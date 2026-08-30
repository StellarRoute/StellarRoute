//! Sepolia MessageTransmitterV2 on-chain attester-set reader.
//!
//! Pinned ABI: `circlefin/evm-cctp-contracts` `Attestable.sol` @ `a92a2b4e7e6e`.

use std::sync::Arc;

use alloy_primitives::{Address, U256};
use alloy_sol_types::{sol, SolCall};
use async_trait::async_trait;

use crate::cctp::attester_set::{
    AttesterDestination, AttesterSetError, AttesterSetReader, RawOnChainAttesterSet,
};
use crate::cctp::bounds::MAX_ENABLED_ATTESTERS;
use crate::cctp::builders::evm::SEPOLIA_CHAIN_ID_NUM;
use crate::cctp::config::CctpConfig;
use crate::cctp::evm_rpc::EvmRpcClient;

sol! {
    interface IMessageTransmitterAttestable {
        function signatureThreshold() external view returns (uint256);
        function isEnabledAttester(address attester) external view returns (bool);
        function getNumEnabledAttesters() external view returns (uint256);
        function getEnabledAttester(uint256 index) external view returns (address);
    }
}

pub struct EvmAttesterSetReader {
    rpc: EvmRpcClient,
    contract: Address,
}

impl EvmAttesterSetReader {
    pub fn new(config: &CctpConfig) -> Result<Self, AttesterSetError> {
        if config.sepolia_rpc_url.trim().is_empty() {
            return Err(AttesterSetError::NotReady);
        }
        let rpc = EvmRpcClient::new(&config.sepolia_rpc_url)
            .map_err(|e| AttesterSetError::Transient(e.to_string()))?;
        let contract: Address = config
            .contracts
            .sepolia_message_transmitter
            .trim()
            .parse()
            .map_err(|_| AttesterSetError::Transient("contract address".into()))?;
        Ok(Self { rpc, contract })
    }

    async fn eth_call(&self, data: &str) -> Result<Vec<u8>, AttesterSetError> {
        let result = self
            .rpc
            .eth_call(&format!("{:#x}", self.contract), data, "latest")
            .await
            .map_err(|e| AttesterSetError::Transient(e.to_string()))?;
        let trimmed = result.trim_start_matches("0x");
        hex::decode(trimmed).map_err(|_| AttesterSetError::Transient("decode hex".into()))
    }

    async fn eth_call_u256(&self, data: &str) -> Result<u64, AttesterSetError> {
        let bytes = self.eth_call(data).await?;
        if bytes.len() > 32 {
            return Err(AttesterSetError::Transient("value too large".into()));
        }
        let mut padded = [0u8; 32];
        padded[32 - bytes.len()..].copy_from_slice(&bytes);
        let value = U256::from_be_bytes(padded);
        u64::try_from(value).map_err(|_| AttesterSetError::Transient("uint overflow".into()))
    }

    pub(crate) fn decode_abi_bool(bytes: &[u8]) -> Result<bool, AttesterSetError> {
        if bytes.len() != 32 {
            return Err(AttesterSetError::Transient("bool word length".into()));
        }
        if bytes[..31].iter().any(|&b| b != 0) {
            return Err(AttesterSetError::Transient("bool high bytes".into()));
        }
        match bytes[31] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(AttesterSetError::Transient("bool invalid".into())),
        }
    }

    pub(crate) fn decode_abi_address(bytes: &[u8]) -> Result<[u8; 20], AttesterSetError> {
        if bytes.len() != 32 {
            return Err(AttesterSetError::Transient("address word length".into()));
        }
        let mut out = [0u8; 20];
        out.copy_from_slice(&bytes[12..32]);
        Ok(out)
    }

    async fn eth_call_bool(&self, data: &str) -> Result<bool, AttesterSetError> {
        let bytes = self.eth_call(data).await?;
        Self::decode_abi_bool(&bytes)
    }

    async fn eth_call_address(&self, data: &str) -> Result<[u8; 20], AttesterSetError> {
        let bytes = self.eth_call(data).await?;
        Self::decode_abi_address(&bytes)
    }
}

#[async_trait]
impl AttesterSetReader for EvmAttesterSetReader {
    fn destination(&self) -> AttesterDestination {
        AttesterDestination::Sepolia
    }

    async fn read_on_chain_set(&self) -> Result<RawOnChainAttesterSet, AttesterSetError> {
        self.rpc
            .ensure_chain()
            .await
            .map_err(|e| AttesterSetError::Transient(e.to_string()))?;
        if self.rpc.chain_id != SEPOLIA_CHAIN_ID_NUM {
            return Err(AttesterSetError::Transient("wrong chain".into()));
        }

        let threshold_data = IMessageTransmitterAttestable::signatureThresholdCall {}.abi_encode();
        let threshold = self
            .eth_call_u256(&format!("0x{}", hex::encode(threshold_data)))
            .await?;
        let threshold_u32 = u32::try_from(threshold)
            .map_err(|_| AttesterSetError::Transient("threshold overflow".into()))?;

        let count_data = IMessageTransmitterAttestable::getNumEnabledAttestersCall {}.abi_encode();
        let count = self
            .eth_call_u256(&format!("0x{}", hex::encode(count_data)))
            .await?;
        let count_usize =
            usize::try_from(count).map_err(|_| AttesterSetError::EnumerationCapExceeded)?;
        if count_usize > MAX_ENABLED_ATTESTERS {
            return Err(AttesterSetError::EnumerationCapExceeded);
        }

        let mut enabled = Vec::with_capacity(count_usize);
        for index in 0..count_usize {
            let data = IMessageTransmitterAttestable::getEnabledAttesterCall {
                index: U256::from(index),
            }
            .abi_encode();
            let addr = self
                .eth_call_address(&format!("0x{}", hex::encode(data)))
                .await?;
            enabled.push(addr);
        }

        // Cross-check membership via isEnabledAttester for each enumerated address.
        for addr in &enabled {
            let call = IMessageTransmitterAttestable::isEnabledAttesterCall {
                attester: Address::from_slice(addr),
            }
            .abi_encode();
            let ok = self
                .eth_call_bool(&format!("0x{}", hex::encode(call)))
                .await?;
            if !ok {
                return Err(AttesterSetError::EnabledCountMismatch);
            }
        }

        Ok(RawOnChainAttesterSet {
            signature_threshold: threshold_u32,
            enabled_addresses: enabled,
            block_or_ledger: Some("latest".into()),
        })
    }
}

pub fn evm_reader_arc(config: &CctpConfig) -> Result<Arc<dyn AttesterSetReader>, AttesterSetError> {
    Ok(Arc::new(EvmAttesterSetReader::new(config)?))
}

#[cfg(test)]
mod live_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires live Sepolia RPC"]
    async fn live_sepolia_enumeration() {
        let mut cfg = CctpConfig::default_testnet();
        if let Ok(url) = std::env::var("SEPOLIA_RPC_URL") {
            cfg.sepolia_rpc_url = url;
        } else {
            cfg.sepolia_rpc_url = "https://ethereum-sepolia-rpc.publicnode.com".into();
        }
        eprintln!("sepolia rpc={}", cfg.sepolia_rpc_url);
        let reader = EvmAttesterSetReader::new(&cfg).expect("reader");
        let set = reader.read_on_chain_set().await.expect("enumeration");
        let hash = crate::cctp::attester_set::AttesterSetSnapshot::on_chain_set_hash(
            &set.enabled_addresses,
        );
        eprintln!(
            "sepolia chain={} threshold={} enabled_count={} set_hash={}",
            SEPOLIA_CHAIN_ID_NUM,
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
    use alloy_sol_types::SolCall;

    #[test]
    fn rejects_missing_rpc() {
        let mut cfg = CctpConfig::default_testnet();
        cfg.sepolia_rpc_url.clear();
        assert!(matches!(
            EvmAttesterSetReader::new(&cfg),
            Err(AttesterSetError::NotReady)
        ));
    }

    #[test]
    fn selector_literals_match_attestable_abi() {
        assert_eq!(
            IMessageTransmitterAttestable::signatureThresholdCall::SELECTOR.len(),
            4
        );
        assert_eq!(
            IMessageTransmitterAttestable::getNumEnabledAttestersCall::SELECTOR.len(),
            4
        );
        assert_eq!(
            IMessageTransmitterAttestable::getEnabledAttesterCall::SELECTOR.len(),
            4
        );
        assert_eq!(
            IMessageTransmitterAttestable::isEnabledAttesterCall::SELECTOR.len(),
            4
        );
        assert_ne!(
            IMessageTransmitterAttestable::signatureThresholdCall::SELECTOR,
            IMessageTransmitterAttestable::getNumEnabledAttestersCall::SELECTOR
        );
    }

    #[test]
    fn strict_abi_bool_decode() {
        let mut word = [0u8; 32];
        word[31] = 1;
        assert!(EvmAttesterSetReader::decode_abi_bool(&word).unwrap());
        word[31] = 0;
        assert!(!EvmAttesterSetReader::decode_abi_bool(&word).unwrap());
        word[30] = 1;
        assert!(EvmAttesterSetReader::decode_abi_bool(&word).is_err());
        word[30] = 0;
        word[31] = 2;
        assert!(EvmAttesterSetReader::decode_abi_bool(&word).is_err());
    }

    #[test]
    fn rejects_ends_with_one_style_bool() {
        let bad = vec![0u8; 31];
        assert!(EvmAttesterSetReader::decode_abi_bool(&bad).is_err());
    }

    #[test]
    fn mocked_full_enumeration_roundtrip() {
        let addr1 = [0x11; 20];
        let addr2 = [0x22; 20];
        let mut enabled = vec![addr1, addr2];
        enabled.sort();
        let raw = RawOnChainAttesterSet {
            signature_threshold: 2,
            enabled_addresses: enabled.clone(),
            block_or_ledger: Some("latest".into()),
        };
        assert_eq!(raw.enabled_addresses.len(), 2);
        assert_eq!(raw.signature_threshold, 2);
    }
}
