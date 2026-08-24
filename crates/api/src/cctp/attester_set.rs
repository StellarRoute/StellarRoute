//! Destination attester-set validation and snapshot types.

use std::collections::BTreeSet;
use std::fmt;
use std::time::Instant;

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::cctp::attestation_crypto::keccak256;
use crate::cctp::bounds::{MAX_ENABLED_ATTESTERS, MAX_SIGNATURE_THRESHOLD};
use crate::cctp::config::{CctpConfig, SEPOLIA_DOMAIN, STELLAR_TESTNET_DOMAIN};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttesterDestination {
    Sepolia,
    StellarTestnet,
}

impl AttesterDestination {
    pub fn domain(self) -> u32 {
        match self {
            Self::Sepolia => SEPOLIA_DOMAIN,
            Self::StellarTestnet => STELLAR_TESTNET_DOMAIN,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Sepolia => "sepolia",
            Self::StellarTestnet => "stellar_testnet",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RawOnChainAttesterSet {
    pub signature_threshold: u32,
    pub enabled_addresses: Vec<[u8; 20]>,
    pub block_or_ledger: Option<String>,
}

#[derive(Clone)]
pub struct AttesterSetSnapshot {
    pub destination: AttesterDestination,
    pub signature_threshold: u32,
    pub enabled_addresses: Vec<[u8; 20]>,
    /// Keccak hash of sorted Iris v2 key-derived addresses used for this generation.
    pub iris_set_hash: [u8; 32],
    /// SHA-256 hash of sorted on-chain enabled addresses.
    pub on_chain_set_hash: [u8; 32],
    pub enabled_count: u32,
    pub verified_at: Instant,
    pub block_or_ledger: Option<String>,
    pub source: &'static str,
    pub generation: u64,
}

impl AttesterSetSnapshot {
    pub fn is_fresh(&self, ttl: std::time::Duration, now: Instant) -> bool {
        now.duration_since(self.verified_at) <= ttl
    }

    pub fn is_stale_beyond(&self, max_stale: std::time::Duration, now: Instant) -> bool {
        now.duration_since(self.verified_at) > max_stale
    }

    pub fn on_chain_set_hash(addresses: &[[u8; 20]]) -> [u8; 32] {
        let mut sorted = addresses.to_vec();
        sorted.sort();
        let mut hasher = Sha256::new();
        for addr in sorted {
            hasher.update(addr);
        }
        let digest = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&digest);
        out
    }
}

impl fmt::Debug for AttesterSetSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AttesterSetSnapshot")
            .field("destination", &self.destination)
            .field("signature_threshold", &self.signature_threshold)
            .field("enabled_count", &self.enabled_count)
            .field("iris_set_hash", &hex::encode(self.iris_set_hash))
            .field("on_chain_set_hash", &hex::encode(self.on_chain_set_hash))
            .field("generation", &self.generation)
            .field("block_or_ledger", &self.block_or_ledger)
            .field("source", &self.source)
            .finish()
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AttesterSetError {
    #[error("not ready")]
    NotReady,
    #[error("transient: {0}")]
    Transient(String),
    #[error("threshold zero")]
    ThresholdZero,
    #[error("threshold exceeds enabled attesters")]
    ThresholdExceedsEnabled,
    #[error("insufficient enabled attesters")]
    InsufficientEnabled,
    #[error("on-chain attester not represented in Iris v2 keys")]
    OnChainNotInIris,
    #[error("empty enabled set")]
    EmptySet,
    #[error("stale snapshot")]
    Stale,
    #[error("iris set hash mismatch")]
    IrisSetHashMismatch,
    #[error("enabled count mismatch")]
    EnabledCountMismatch,
    #[error("enumeration cap exceeded")]
    EnumerationCapExceeded,
    #[error("threshold cap exceeded")]
    ThresholdCapExceeded,
}

#[async_trait]
pub trait AttesterSetReader: Send + Sync {
    fn destination(&self) -> AttesterDestination;
    async fn read_on_chain_set(&self) -> Result<RawOnChainAttesterSet, AttesterSetError>;
}

pub fn iris_candidate_hash(addresses: &[[u8; 20]]) -> [u8; 32] {
    keccak256(
        &addresses
            .iter()
            .flat_map(|a| a.iter().copied())
            .collect::<Vec<_>>(),
    )
}

pub fn validate_attester_set_parity(
    destination: AttesterDestination,
    on_chain: &RawOnChainAttesterSet,
    iris_addresses: &[[u8; 20]],
    iris_set_hash: [u8; 32],
    generation: u64,
    verified_at: Instant,
    source: &'static str,
) -> Result<AttesterSetSnapshot, AttesterSetError> {
    if iris_candidate_hash(iris_addresses) != iris_set_hash {
        return Err(AttesterSetError::IrisSetHashMismatch);
    }

    if on_chain.signature_threshold == 0 {
        return Err(AttesterSetError::ThresholdZero);
    }
    if on_chain.signature_threshold > MAX_SIGNATURE_THRESHOLD {
        return Err(AttesterSetError::ThresholdCapExceeded);
    }

    let mut enabled = on_chain.enabled_addresses.clone();
    if enabled.is_empty() {
        return Err(AttesterSetError::EmptySet);
    }
    if enabled.len() > MAX_ENABLED_ATTESTERS {
        return Err(AttesterSetError::EnumerationCapExceeded);
    }
    enabled.sort();
    enabled.dedup();
    if enabled.len() != on_chain.enabled_addresses.len() {
        return Err(AttesterSetError::EnabledCountMismatch);
    }

    let enabled_count =
        u32::try_from(enabled.len()).map_err(|_| AttesterSetError::EnumerationCapExceeded)?;
    if on_chain.signature_threshold > enabled_count {
        return Err(AttesterSetError::ThresholdExceedsEnabled);
    }

    let iris_set: BTreeSet<[u8; 20]> = iris_addresses.iter().copied().collect();
    for addr in &enabled {
        if !iris_set.contains(addr) {
            return Err(AttesterSetError::OnChainNotInIris);
        }
    }

    Ok(AttesterSetSnapshot {
        destination,
        signature_threshold: on_chain.signature_threshold,
        enabled_addresses: enabled.clone(),
        iris_set_hash,
        on_chain_set_hash: AttesterSetSnapshot::on_chain_set_hash(&enabled),
        enabled_count,
        verified_at,
        block_or_ledger: on_chain.block_or_ledger.clone(),
        source,
        generation,
    })
}

pub fn destination_for_message(
    source_domain: u32,
    destination_domain: u32,
) -> Option<AttesterDestination> {
    if source_domain == STELLAR_TESTNET_DOMAIN && destination_domain == SEPOLIA_DOMAIN {
        Some(AttesterDestination::Sepolia)
    } else if source_domain == SEPOLIA_DOMAIN && destination_domain == STELLAR_TESTNET_DOMAIN {
        Some(AttesterDestination::StellarTestnet)
    } else {
        None
    }
}

pub fn attester_cache_ttl(config: &CctpConfig) -> std::time::Duration {
    std::time::Duration::from_secs(config.attester_snapshot_ttl_secs)
}

pub fn attester_cache_stale_max(config: &CctpConfig) -> std::time::Duration {
    std::time::Duration::from_secs(config.attester_snapshot_stale_max_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_on_chain() -> RawOnChainAttesterSet {
        RawOnChainAttesterSet {
            signature_threshold: 2,
            enabled_addresses: vec![[0x01; 20], [0x02; 20], [0x03; 20]],
            block_or_ledger: Some("latest".into()),
        }
    }

    #[test]
    fn destination_mapping_for_corridor() {
        assert_eq!(
            destination_for_message(27, 0),
            Some(AttesterDestination::Sepolia)
        );
        assert_eq!(
            destination_for_message(0, 27),
            Some(AttesterDestination::StellarTestnet)
        );
        assert_eq!(destination_for_message(1, 2), None);
    }

    #[test]
    fn validates_full_on_chain_subset_of_iris() {
        let iris = vec![[0x01; 20], [0x02; 20], [0x03; 20], [0x04; 20]];
        let hash = iris_candidate_hash(&iris);
        let snap = validate_attester_set_parity(
            AttesterDestination::Sepolia,
            &sample_on_chain(),
            &iris,
            hash,
            1,
            Instant::now(),
            "test",
        )
        .expect("valid");
        assert_eq!(snap.enabled_count, 3);
        assert_eq!(snap.iris_set_hash, hash);
    }

    #[test]
    fn rejects_on_chain_extra_not_in_iris() {
        let iris = vec![[0x01; 20], [0x02; 20]];
        let hash = iris_candidate_hash(&iris);
        let err = validate_attester_set_parity(
            AttesterDestination::Sepolia,
            &sample_on_chain(),
            &iris,
            hash,
            1,
            Instant::now(),
            "test",
        )
        .unwrap_err();
        assert_eq!(err, AttesterSetError::OnChainNotInIris);
    }

    #[test]
    fn rejects_threshold_exceeds_enabled() {
        let mut on_chain = sample_on_chain();
        on_chain.signature_threshold = 5;
        let iris = vec![[0x01; 20], [0x02; 20], [0x03; 20]];
        let hash = iris_candidate_hash(&iris);
        let err = validate_attester_set_parity(
            AttesterDestination::Sepolia,
            &on_chain,
            &iris,
            hash,
            1,
            Instant::now(),
            "test",
        )
        .unwrap_err();
        assert_eq!(err, AttesterSetError::ThresholdExceedsEnabled);
    }

    #[test]
    fn rejects_iris_hash_mismatch() {
        let iris = vec![[0x01; 20], [0x02; 20], [0x03; 20]];
        let err = validate_attester_set_parity(
            AttesterDestination::Sepolia,
            &sample_on_chain(),
            &iris,
            [0xAA; 32],
            1,
            Instant::now(),
            "test",
        )
        .unwrap_err();
        assert_eq!(err, AttesterSetError::IrisSetHashMismatch);
    }

    #[test]
    fn iris_extra_signer_allowed() {
        let iris = vec![[0x01; 20], [0x02; 20], [0x03; 20], [0x99; 20]];
        let hash = iris_candidate_hash(&iris);
        validate_attester_set_parity(
            AttesterDestination::Sepolia,
            &sample_on_chain(),
            &iris,
            hash,
            1,
            Instant::now(),
            "test",
        )
        .expect("iris extra signer is ok");
    }
}
