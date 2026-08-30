//! Attestation verification — production Circle CCTP v2 verifier and test seams.

use std::sync::Arc;

use async_trait::async_trait;
use thiserror::Error;
use tokio::task::JoinHandle;

use crate::cctp::attestation_crypto::{verify_attestation_signatures, AttestationCryptoError};
use crate::cctp::attestation_trust::{
    AttestationRefreshDeps, AttestationTrustCache, AttestationTrustError,
};
use crate::cctp::attester_set::{destination_for_message, AttesterSetSnapshot};
use crate::cctp::config::{SEPOLIA_DOMAIN, STELLAR_TESTNET_DOMAIN};
use crate::cctp::message::parse_cctp_v2_message;
use crate::metrics;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AttestationVerifyError {
    #[error("attestation verifier not ready")]
    NotReady,
    #[error("empty attestation")]
    Empty,
    #[error("empty message")]
    EmptyMessage,
    #[error("wrong corridor domains")]
    WrongCorridor,
    #[error("verification failed: {0}")]
    Invalid(String),
}

impl From<AttestationCryptoError> for AttestationVerifyError {
    fn from(e: AttestationCryptoError) -> Self {
        metrics::record_cctp_attestation_verify(e.reason_label());
        Self::Invalid(e.reason_label().into())
    }
}

#[async_trait]
pub trait AttestationVerifier: Send + Sync {
    fn is_ready(&self) -> bool;
    async fn verify_attestation(
        &self,
        raw_message: &[u8],
        attestation: &[u8],
    ) -> Result<(), AttestationVerifyError>;
}

/// Production Circle CCTP v2 attestation verifier.
pub struct CircleAttestationVerifier {
    pub(crate) trust: Arc<AttestationTrustCache>,
    deps: Arc<AttestationRefreshDeps>,
    _refresh_task: Option<JoinHandle<()>>,
}

impl CircleAttestationVerifier {
    pub fn new(trust: Arc<AttestationTrustCache>, deps: AttestationRefreshDeps) -> Self {
        let deps = Arc::new(deps);
        let weak = Arc::downgrade(&trust);
        let refresh_interval = trust.ttl() / 2;
        let refresh_task = Some(AttestationTrustCache::spawn_background_refresh(
            weak,
            deps.clone(),
            refresh_interval.max(std::time::Duration::from_secs(30)),
        ));
        Self {
            trust,
            deps,
            _refresh_task: refresh_task,
        }
    }

    pub async fn bootstrap(&self) -> Result<(), AttestationVerifyError> {
        self.trust
            .full_refresh(self.deps.as_ref())
            .await
            .map_err(|_| AttestationVerifyError::NotReady)?;
        if !self.trust.is_ready() {
            return Err(AttestationVerifyError::NotReady);
        }
        Ok(())
    }

    pub(crate) fn verify_with_snapshot(
        raw_message: &[u8],
        attestation: &[u8],
        snap: &AttesterSetSnapshot,
        iris_set_hash: [u8; 32],
    ) -> Result<(), AttestationVerifyError> {
        if snap.iris_set_hash != iris_set_hash {
            return Err(AttestationVerifyError::NotReady);
        }
        verify_attestation_signatures(
            raw_message,
            attestation,
            snap.signature_threshold,
            &snap.enabled_addresses,
        )?;
        metrics::record_cctp_attestation_verify("ok");
        Ok(())
    }
}

#[async_trait]
impl AttestationVerifier for CircleAttestationVerifier {
    fn is_ready(&self) -> bool {
        self.trust.is_ready()
    }

    async fn verify_attestation(
        &self,
        raw_message: &[u8],
        attestation: &[u8],
    ) -> Result<(), AttestationVerifyError> {
        if raw_message.is_empty() {
            return Err(AttestationVerifyError::EmptyMessage);
        }
        if attestation.is_empty() {
            return Err(AttestationVerifyError::Empty);
        }

        self.trust
            .ensure_fresh(self.deps.as_ref())
            .await
            .map_err(|e| match e {
                AttestationTrustError::Stale | AttestationTrustError::NotReady => {
                    AttestationVerifyError::NotReady
                }
                _ => AttestationVerifyError::NotReady,
            })?;

        let parsed = parse_cctp_v2_message(raw_message)
            .map_err(|_| AttestationVerifyError::Invalid("parse".into()))?;

        let dest = destination_for_message(parsed.source_domain, parsed.destination_domain)
            .ok_or(AttestationVerifyError::WrongCorridor)?;

        let valid_pair = (parsed.source_domain == STELLAR_TESTNET_DOMAIN
            && parsed.destination_domain == SEPOLIA_DOMAIN)
            || (parsed.source_domain == SEPOLIA_DOMAIN
                && parsed.destination_domain == STELLAR_TESTNET_DOMAIN);
        if !valid_pair {
            return Err(AttestationVerifyError::WrongCorridor);
        }

        let generation = self
            .trust
            .generation()
            .ok_or(AttestationVerifyError::NotReady)?;
        let snap = self
            .trust
            .snapshot_for(dest)
            .ok_or(AttestationVerifyError::NotReady)?;
        let iris_hash = generation.iris.set_hash;

        match Self::verify_with_snapshot(raw_message, attestation, &snap, iris_hash) {
            Ok(()) => Ok(()),
            Err(AttestationVerifyError::Invalid(reason))
                if reason == AttestationCryptoError::UnknownSigner.reason_label() =>
            {
                self.trust
                    .force_full_refresh(self.deps.as_ref())
                    .await
                    .map_err(|_| AttestationVerifyError::NotReady)?;
                let generation = self
                    .trust
                    .generation()
                    .ok_or(AttestationVerifyError::NotReady)?;
                let snap = self
                    .trust
                    .snapshot_for(dest)
                    .ok_or(AttestationVerifyError::NotReady)?;
                Self::verify_with_snapshot(
                    raw_message,
                    attestation,
                    &snap,
                    generation.iris.set_hash,
                )
            }
            other => other,
        }
    }
}

/// Production default — wired only when full attestation stack is configured.
pub struct NotReadyAttestationVerifier;

#[async_trait]
impl AttestationVerifier for NotReadyAttestationVerifier {
    fn is_ready(&self) -> bool {
        false
    }

    async fn verify_attestation(
        &self,
        _raw_message: &[u8],
        _attestation: &[u8],
    ) -> Result<(), AttestationVerifyError> {
        Err(AttestationVerifyError::NotReady)
    }
}

/// Test-only verifier accepting non-empty message + attestation pairs.
pub struct FakeAttestationVerifier {
    pub ready: bool,
}

#[async_trait]
impl AttestationVerifier for FakeAttestationVerifier {
    fn is_ready(&self) -> bool {
        self.ready
    }

    async fn verify_attestation(
        &self,
        raw_message: &[u8],
        attestation: &[u8],
    ) -> Result<(), AttestationVerifyError> {
        if !self.ready {
            return Err(AttestationVerifyError::NotReady);
        }
        if raw_message.is_empty() {
            return Err(AttestationVerifyError::EmptyMessage);
        }
        if attestation.is_empty() {
            return Err(AttestationVerifyError::Empty);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cctp::attestation_trust::{AttestationTrustCache, MockClock, SystemClock};
    use crate::cctp::attester_set::{
        AttesterDestination, AttesterSetError, AttesterSetReader, RawOnChainAttesterSet,
    };
    use crate::cctp::fixtures::circle_attestation_v2::{
        ATTESTER_ADDRESS_1, ATTESTER_ADDRESS_2, ATTESTER_ADDRESS_3, FIXTURE_VALID_ATTESTATION,
        FIXTURE_VALID_MESSAGE,
    };
    use crate::cctp::iris_public_keys::{IrisPublicKeyError, IrisPublicKeySource};
    use async_trait::async_trait;
    use std::time::Duration;

    #[tokio::test]
    async fn verifies_official_fixture_via_crypto() {
        let enabled = vec![ATTESTER_ADDRESS_1, ATTESTER_ADDRESS_2, ATTESTER_ADDRESS_3];
        let mut sorted = enabled.clone();
        sorted.sort();
        crate::cctp::attestation_crypto::verify_attestation_signatures(
            FIXTURE_VALID_MESSAGE,
            FIXTURE_VALID_ATTESTATION,
            2,
            &sorted,
        )
        .expect("crypto path");
    }

    #[tokio::test]
    async fn rejects_when_not_ready() {
        let verifier = NotReadyAttestationVerifier;
        let err = verifier.verify_attestation(&[1], &[2]).await.unwrap_err();
        assert_eq!(err, AttestationVerifyError::NotReady);
    }

    struct StaticIris(Vec<[u8; 20]>);

    #[async_trait]
    impl IrisPublicKeySource for StaticIris {
        async fn fetch_public_keys(&self) -> Result<Vec<[u8; 20]>, IrisPublicKeyError> {
            Ok(self.0.clone())
        }
    }

    struct StaticReader {
        dest: AttesterDestination,
        enabled: Vec<[u8; 20]>,
    }

    #[async_trait]
    impl AttesterSetReader for StaticReader {
        fn destination(&self) -> AttesterDestination {
            self.dest
        }

        async fn read_on_chain_set(&self) -> Result<RawOnChainAttesterSet, AttesterSetError> {
            Ok(RawOnChainAttesterSet {
                signature_threshold: 2,
                enabled_addresses: self.enabled.clone(),
                block_or_ledger: Some("mock".into()),
            })
        }
    }

    fn e2e_verifier(enabled: Vec<[u8; 20]>) -> CircleAttestationVerifier {
        let iris = enabled.clone();
        let trust = Arc::new(AttestationTrustCache::new(
            Duration::from_secs(900),
            Duration::from_secs(86_400),
            Arc::new(SystemClock),
        ));
        CircleAttestationVerifier::new(
            trust,
            AttestationRefreshDeps {
                iris_source: Arc::new(StaticIris(iris)),
                readers: vec![
                    Arc::new(StaticReader {
                        dest: AttesterDestination::Sepolia,
                        enabled: enabled.clone(),
                    }),
                    Arc::new(StaticReader {
                        dest: AttesterDestination::StellarTestnet,
                        enabled,
                    }),
                ],
            },
        )
    }

    #[tokio::test]
    async fn e2e_bootstrap_and_verify_with_snapshot_both_destinations() {
        let enabled = vec![ATTESTER_ADDRESS_1, ATTESTER_ADDRESS_2, ATTESTER_ADDRESS_3];
        let verifier = e2e_verifier(enabled);
        verifier.bootstrap().await.expect("bootstrap");
        assert!(verifier.is_ready());

        let generation = verifier.trust.generation().expect("generation");
        for dest in [
            AttesterDestination::Sepolia,
            AttesterDestination::StellarTestnet,
        ] {
            let snap = verifier.trust.snapshot_for(dest).expect("snapshot");
            CircleAttestationVerifier::verify_with_snapshot(
                FIXTURE_VALID_MESSAGE,
                FIXTURE_VALID_ATTESTATION,
                &snap,
                generation.iris.set_hash,
            )
            .expect("verify both destinations");
        }
    }

    #[tokio::test]
    async fn e2e_official_fixture_rejected_by_corridor_gate() {
        let enabled = vec![ATTESTER_ADDRESS_1, ATTESTER_ADDRESS_2, ATTESTER_ADDRESS_3];
        let verifier = e2e_verifier(enabled);
        verifier.bootstrap().await.unwrap();
        let err = verifier
            .verify_attestation(FIXTURE_VALID_MESSAGE, FIXTURE_VALID_ATTESTATION)
            .await
            .unwrap_err();
        assert_eq!(err, AttestationVerifyError::WrongCorridor);
    }

    #[tokio::test]
    async fn e2e_rejects_stale_snapshot() {
        let enabled = vec![ATTESTER_ADDRESS_1, ATTESTER_ADDRESS_2, ATTESTER_ADDRESS_3];
        let clock = Arc::new(MockClock::new());
        let trust = Arc::new(AttestationTrustCache::new(
            Duration::from_secs(60),
            Duration::from_secs(120),
            clock.clone(),
        ));
        let verifier = CircleAttestationVerifier::new(
            trust,
            AttestationRefreshDeps {
                iris_source: Arc::new(StaticIris(enabled.clone())),
                readers: vec![
                    Arc::new(StaticReader {
                        dest: AttesterDestination::Sepolia,
                        enabled: enabled.clone(),
                    }),
                    Arc::new(StaticReader {
                        dest: AttesterDestination::StellarTestnet,
                        enabled,
                    }),
                ],
            },
        );
        verifier.bootstrap().await.unwrap();
        clock.advance(Duration::from_secs(121));
        let err = verifier
            .verify_attestation(FIXTURE_VALID_MESSAGE, FIXTURE_VALID_ATTESTATION)
            .await
            .unwrap_err();
        assert_eq!(err, AttestationVerifyError::NotReady);
    }

    #[tokio::test]
    async fn e2e_verify_attestation_unknown_signer_single_refresh_success() {
        use k256::ecdsa::SigningKey;
        use std::sync::atomic::{AtomicUsize, Ordering};

        use crate::cctp::attestation_crypto::SIGNATURE_LENGTH;

        fn corridor_message() -> Vec<u8> {
            let mut msg = FIXTURE_VALID_MESSAGE.to_vec();
            msg[4..8].copy_from_slice(&27u32.to_be_bytes());
            msg[8..12].copy_from_slice(&0u32.to_be_bytes());
            msg
        }

        fn eth_address(sk: &SigningKey) -> [u8; 20] {
            let encoded = sk.verifying_key().to_encoded_point(false);
            let mut xy = [0u8; 64];
            xy.copy_from_slice(&encoded.as_bytes()[1..65]);
            crate::cctp::attestation_crypto::eth_address_from_pubkey_xy(&xy)
        }

        fn sign_component(digest: &[u8; 32], sk: &SigningKey) -> [u8; 65] {
            let (sig, rid) = sk
                .sign_prehash_recoverable(digest)
                .expect("sign prehash recoverable");
            let mut out = [0u8; 65];
            out[..64].copy_from_slice(&sig.to_bytes());
            out[64] = rid.to_byte() + 27;
            out
        }

        fn build_attestation(message: &[u8], signers: &[SigningKey]) -> Vec<u8> {
            let digest = crate::cctp::attestation_crypto::keccak256(message);
            let mut addrs: Vec<([u8; 20], SigningKey)> = signers
                .iter()
                .map(|sk| (eth_address(sk), sk.clone()))
                .collect();
            addrs.sort_by(|a, b| a.0.cmp(&b.0));
            let mut attestation = Vec::with_capacity(2 * SIGNATURE_LENGTH);
            for (_, sk) in addrs {
                attestation.extend_from_slice(&sign_component(&digest, &sk));
            }
            attestation
        }

        let sk_known = SigningKey::from_bytes(&[0x11; 32].into()).expect("known key");
        let sk_unknown = SigningKey::from_bytes(&[0x22; 32].into()).expect("unknown key");
        let sk_decoy = SigningKey::from_bytes(&[0x55; 32].into()).expect("decoy key");
        let addr_known = eth_address(&sk_known);
        let addr_unknown = eth_address(&sk_unknown);
        let addr_decoy = eth_address(&sk_decoy);
        assert_ne!(addr_known, addr_unknown);

        let message = corridor_message();
        let attestation = build_attestation(&message, &[sk_known.clone(), sk_unknown.clone()]);

        let initial_enabled = {
            let mut v = vec![addr_known, addr_decoy];
            v.sort();
            v
        };
        let refreshed_enabled = {
            let mut v = vec![addr_known, addr_unknown];
            v.sort();
            v
        };

        struct PhasedIris {
            calls: AtomicUsize,
            initial: Vec<[u8; 20]>,
            refreshed: Vec<[u8; 20]>,
        }

        #[async_trait]
        impl IrisPublicKeySource for PhasedIris {
            async fn fetch_public_keys(&self) -> Result<Vec<[u8; 20]>, IrisPublicKeyError> {
                let phase = self.calls.fetch_add(1, Ordering::SeqCst);
                if phase == 0 {
                    Ok(self.initial.clone())
                } else {
                    Ok(self.refreshed.clone())
                }
            }
        }

        struct PhasedReader {
            dest: AttesterDestination,
            calls: AtomicUsize,
            initial: Vec<[u8; 20]>,
            refreshed: Vec<[u8; 20]>,
        }

        #[async_trait]
        impl AttesterSetReader for PhasedReader {
            fn destination(&self) -> AttesterDestination {
                self.dest
            }

            async fn read_on_chain_set(&self) -> Result<RawOnChainAttesterSet, AttesterSetError> {
                let phase = self.calls.fetch_add(1, Ordering::SeqCst);
                let enabled = if phase == 0 {
                    self.initial.clone()
                } else {
                    self.refreshed.clone()
                };
                Ok(RawOnChainAttesterSet {
                    signature_threshold: 2,
                    enabled_addresses: enabled,
                    block_or_ledger: Some("mock".into()),
                })
            }
        }

        let trust = Arc::new(AttestationTrustCache::new(
            Duration::from_secs(900),
            Duration::from_secs(86_400),
            Arc::new(SystemClock),
        ));
        let build_count = trust.generation_build_count.clone();
        let verifier = CircleAttestationVerifier::new(
            trust.clone(),
            AttestationRefreshDeps {
                iris_source: Arc::new(PhasedIris {
                    calls: AtomicUsize::new(0),
                    initial: initial_enabled.clone(),
                    refreshed: refreshed_enabled.clone(),
                }),
                readers: vec![
                    Arc::new(PhasedReader {
                        dest: AttesterDestination::Sepolia,
                        calls: AtomicUsize::new(0),
                        initial: initial_enabled.clone(),
                        refreshed: refreshed_enabled.clone(),
                    }),
                    Arc::new(PhasedReader {
                        dest: AttesterDestination::StellarTestnet,
                        calls: AtomicUsize::new(0),
                        initial: initial_enabled,
                        refreshed: refreshed_enabled.clone(),
                    }),
                ],
            },
        );

        verifier.bootstrap().await.expect("bootstrap");
        assert_eq!(build_count.load(Ordering::SeqCst), 1);

        let gen_before = verifier.trust.generation().expect("generation");
        verifier
            .verify_attestation(&message, &attestation)
            .await
            .expect("unknown signer refresh succeeds");

        assert_eq!(build_count.load(Ordering::SeqCst), 2);
        let gen_after = verifier
            .trust
            .generation()
            .expect("generation after refresh");
        assert!(gen_after.generation > gen_before.generation);
        for dest in [
            AttesterDestination::Sepolia,
            AttesterDestination::StellarTestnet,
        ] {
            let snap = verifier.trust.snapshot_for(dest).expect("snapshot");
            assert!(snap.enabled_addresses.contains(&addr_unknown));
            assert_eq!(snap.iris_set_hash, gen_after.iris.set_hash);
        }
    }

    #[tokio::test]
    async fn e2e_verify_attestation_unknown_signer_fails_after_one_refresh() {
        use k256::ecdsa::SigningKey;
        use std::sync::atomic::Ordering;

        use crate::cctp::attestation_crypto::SIGNATURE_LENGTH;

        fn corridor_message() -> Vec<u8> {
            let mut msg = FIXTURE_VALID_MESSAGE.to_vec();
            msg[4..8].copy_from_slice(&27u32.to_be_bytes());
            msg[8..12].copy_from_slice(&0u32.to_be_bytes());
            msg
        }

        fn eth_address(sk: &SigningKey) -> [u8; 20] {
            let encoded = sk.verifying_key().to_encoded_point(false);
            let mut xy = [0u8; 64];
            xy.copy_from_slice(&encoded.as_bytes()[1..65]);
            crate::cctp::attestation_crypto::eth_address_from_pubkey_xy(&xy)
        }

        fn sign_component(digest: &[u8; 32], sk: &SigningKey) -> [u8; 65] {
            let (sig, rid) = sk
                .sign_prehash_recoverable(digest)
                .expect("sign prehash recoverable");
            let mut out = [0u8; 65];
            out[..64].copy_from_slice(&sig.to_bytes());
            out[64] = rid.to_byte() + 27;
            out
        }

        let sk_known = SigningKey::from_bytes(&[0x33; 32].into()).expect("known key");
        let sk_unknown = SigningKey::from_bytes(&[0x44; 32].into()).expect("unknown key");
        let sk_decoy = SigningKey::from_bytes(&[0x66; 32].into()).expect("decoy key");
        let addr_known = eth_address(&sk_known);
        let addr_unknown = eth_address(&sk_unknown);
        let addr_decoy = eth_address(&sk_decoy);

        let message = corridor_message();
        let digest = crate::cctp::attestation_crypto::keccak256(&message);
        let mut signers = vec![(addr_known, sk_known.clone()), (addr_unknown, sk_unknown)];
        signers.sort_by(|a, b| a.0.cmp(&b.0));
        let mut attestation = Vec::with_capacity(2 * SIGNATURE_LENGTH);
        for (_, sk) in &signers {
            attestation.extend_from_slice(&sign_component(&digest, sk));
        }

        let enabled_without_unknown = {
            let mut v = vec![addr_known, addr_decoy];
            v.sort();
            v
        };

        struct StaticIrisAlways(Vec<[u8; 20]>);
        #[async_trait]
        impl IrisPublicKeySource for StaticIrisAlways {
            async fn fetch_public_keys(&self) -> Result<Vec<[u8; 20]>, IrisPublicKeyError> {
                Ok(self.0.clone())
            }
        }

        struct StaticReaderAlways {
            dest: AttesterDestination,
            enabled: Vec<[u8; 20]>,
        }
        #[async_trait]
        impl AttesterSetReader for StaticReaderAlways {
            fn destination(&self) -> AttesterDestination {
                self.dest
            }
            async fn read_on_chain_set(&self) -> Result<RawOnChainAttesterSet, AttesterSetError> {
                Ok(RawOnChainAttesterSet {
                    signature_threshold: 2,
                    enabled_addresses: self.enabled.clone(),
                    block_or_ledger: Some("mock".into()),
                })
            }
        }

        let trust = Arc::new(AttestationTrustCache::new(
            Duration::from_secs(900),
            Duration::from_secs(86_400),
            Arc::new(SystemClock),
        ));
        let build_count = trust.generation_build_count.clone();
        let verifier = CircleAttestationVerifier::new(
            trust,
            AttestationRefreshDeps {
                iris_source: Arc::new(StaticIrisAlways(enabled_without_unknown.clone())),
                readers: vec![
                    Arc::new(StaticReaderAlways {
                        dest: AttesterDestination::Sepolia,
                        enabled: enabled_without_unknown.clone(),
                    }),
                    Arc::new(StaticReaderAlways {
                        dest: AttesterDestination::StellarTestnet,
                        enabled: enabled_without_unknown,
                    }),
                ],
            },
        );

        verifier.bootstrap().await.unwrap();
        assert_eq!(build_count.load(Ordering::SeqCst), 1);

        let err = verifier
            .verify_attestation(&message, &attestation)
            .await
            .unwrap_err();
        assert_eq!(
            err,
            AttestationVerifyError::Invalid(
                AttestationCryptoError::UnknownSigner.reason_label().into()
            )
        );
        assert_eq!(
            build_count.load(Ordering::SeqCst),
            2,
            "must perform exactly one full refresh, not a retry loop"
        );
    }
}
