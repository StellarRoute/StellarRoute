//! Stellar Testnet MessageTransmitterV2 on-chain attester-set reader.
//!
//! Pinned getters: `circlefin/stellar-cctp` `attestable/storage.rs` @ `45746f2c8031`.

use std::sync::Arc;

use async_trait::async_trait;

use crate::cctp::attester_set::{
    AttesterDestination, AttesterSetError, AttesterSetReader, RawOnChainAttesterSet,
};
use crate::cctp::bounds::MAX_ENABLED_ATTESTERS;
use crate::cctp::config::CctpConfig;
use crate::cctp::stellar_rpc::{
    bytes20_scval, scval_to_bool, scval_to_bytes20, scval_to_option_u32, scval_to_u32, u32_scval,
    StellarRpcClient,
};

pub struct StellarAttesterSetReader {
    rpc: StellarRpcClient,
    contract: String,
}

impl StellarAttesterSetReader {
    pub fn new(config: &CctpConfig) -> Result<Self, AttesterSetError> {
        if config.stellar_rpc_url.trim().is_empty() {
            return Err(AttesterSetError::NotReady);
        }
        let rpc = StellarRpcClient::new(config)
            .map_err(|e| AttesterSetError::Transient(e.to_string()))?;
        Ok(Self {
            rpc,
            contract: config.contracts.stellar_message_transmitter.clone(),
        })
    }
}

#[async_trait]
impl AttesterSetReader for StellarAttesterSetReader {
    fn destination(&self) -> AttesterDestination {
        AttesterDestination::StellarTestnet
    }

    async fn read_on_chain_set(&self) -> Result<RawOnChainAttesterSet, AttesterSetError> {
        let ledger = self
            .rpc
            .latest_ledger()
            .await
            .map_err(|e| AttesterSetError::Transient(e.to_string()))?;

        let threshold_val = self
            .rpc
            .simulate_scval(&self.contract, "get_signature_threshold", vec![])
            .await
            .map_err(|e| AttesterSetError::Transient(e.to_string()))?;
        let threshold = scval_to_option_u32(&threshold_val)
            .map_err(|e| AttesterSetError::Transient(e.to_string()))?
            .ok_or(AttesterSetError::ThresholdZero)?;

        let count_val = self
            .rpc
            .simulate_scval(&self.contract, "get_num_enabled_attesters", vec![])
            .await
            .map_err(|e| AttesterSetError::Transient(e.to_string()))?;
        let count = scval_to_u32(&count_val)
            .map_err(|e| AttesterSetError::Transient(e.to_string()))? as usize;
        if count > MAX_ENABLED_ATTESTERS {
            return Err(AttesterSetError::EnumerationCapExceeded);
        }

        let mut enabled = Vec::with_capacity(count);
        for index in 0..count {
            let val = self
                .rpc
                .simulate_scval(
                    &self.contract,
                    "get_enabled_attester",
                    vec![u32_scval(index as u32)],
                )
                .await
                .map_err(|e| AttesterSetError::Transient(e.to_string()))?;
            enabled.push(
                scval_to_bytes20(&val).map_err(|e| AttesterSetError::Transient(e.to_string()))?,
            );
        }

        for addr in &enabled {
            let val = self
                .rpc
                .simulate_scval(
                    &self.contract,
                    "is_enabled_attester",
                    vec![bytes20_scval(*addr)],
                )
                .await
                .map_err(|e| AttesterSetError::Transient(e.to_string()))?;
            if !scval_to_bool(&val).map_err(|e| AttesterSetError::Transient(e.to_string()))? {
                return Err(AttesterSetError::EnabledCountMismatch);
            }
        }

        Ok(RawOnChainAttesterSet {
            signature_threshold: threshold,
            enabled_addresses: enabled,
            block_or_ledger: Some(ledger.to_string()),
        })
    }
}

pub fn stellar_reader_arc(
    config: &CctpConfig,
) -> Result<Arc<dyn AttesterSetReader>, AttesterSetError> {
    Ok(Arc::new(StellarAttesterSetReader::new(config)?))
}

#[cfg(test)]
mod live_tests {
    use super::*;
    use crate::cctp::config::STELLAR_MESSAGE_TRANSMITTER;

    #[tokio::test]
    #[ignore = "requires live Stellar testnet RPC"]
    async fn live_stellar_enumeration() {
        let cfg = CctpConfig::default_testnet();
        eprintln!("stellar contract={STELLAR_MESSAGE_TRANSMITTER}");
        eprintln!("stellar rpc={}", cfg.stellar_rpc_url);
        let reader = StellarAttesterSetReader::new(&cfg).expect("reader");
        let set = reader.read_on_chain_set().await.expect("enumeration");
        let hash = crate::cctp::attester_set::AttesterSetSnapshot::on_chain_set_hash(
            &set.enabled_addresses,
        );
        eprintln!(
            "stellar threshold={} enabled_count={} set_hash={}",
            set.signature_threshold,
            set.enabled_addresses.len(),
            hex::encode(hash)
        );
        assert!(set.signature_threshold > 0);
        assert!(!set.enabled_addresses.is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stellar_xdr::curr::{ReadXdr, ScVal, WriteXdr};

    #[test]
    fn rejects_missing_rpc() {
        let mut cfg = CctpConfig::default_testnet();
        cfg.stellar_rpc_url.clear();
        assert!(matches!(
            StellarAttesterSetReader::new(&cfg),
            Err(AttesterSetError::NotReady)
        ));
    }

    #[test]
    fn parses_realistic_simulate_envelope_for_threshold() {
        let val = ScVal::U32(2);
        let xdr = val
            .to_xdr_base64(stellar_xdr::curr::Limits::none())
            .unwrap();
        let decoded = ScVal::from_xdr_base64(&xdr, stellar_xdr::curr::Limits::none()).unwrap();
        assert_eq!(scval_to_option_u32(&decoded).unwrap(), Some(2));
    }
}
