//! Production Sepolia MessageTransmitterV2 mint verifier.
//!
//! ABI source: [circlefin/evm-cctp-contracts `MessageTransmitterV2.sol`](https://github.com/circlefin/evm-cctp-contracts/blob/master/src/v2/MessageTransmitterV2.sol)
//! @ commit master; event `MessageReceived` and `usedNonces(bytes32)` from `BaseMessageTransmitter.sol`.

use alloy_primitives::{Address, FixedBytes, Log, B256, U256};
use alloy_sol_types::{sol, SolCall, SolEvent};
use async_trait::async_trait;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::cctp::builders::evm::ProductionEvmCctpBuilder;
use crate::cctp::config::CctpConfig;
use crate::cctp::evm_rpc::EvmRpcClient;
use crate::cctp::message::{parse_cctp_v2_message, MESSAGE_HEADER_LEN};
use crate::cctp::verifiers::{
    EvmMintVerifier, MintVerifyOutcome, VerifiedMintFacts, VerifierError,
};
use crate::models::v2_cctp::SEPOLIA_CHAIN_ID;

/// `receiveMessage(bytes,bytes)` — MessageTransmitterV2.
/// Cross-check: `cast sig "receiveMessage(bytes,bytes)"` => 0x57ecfd28
pub const RECEIVE_MESSAGE_SELECTOR: [u8; 4] = [0x57, 0xec, 0xfd, 0x28];

/// Topic0 for official V2 `MessageReceived` event.
/// Cross-check: `cast keccak "MessageReceived(address,uint32,bytes32,bytes32,uint32,bytes)"`
pub const MESSAGE_RECEIVED_TOPIC0: &str =
    "0xff48c13eda96b1cceacc6b9edeedc9e9db9d6226afbc30146b720c19d3addb1c";

/// `BaseMessageTransmitter.NONCE_USED` constant.
pub const NONCE_USED_VALUE: u64 = 1;

const DEFAULT_MIN_CONFIRMATIONS: u64 = 1;

sol! {
    interface IMessageTransmitterV2 {
        function receiveMessage(bytes message, bytes attestation) external returns (bool);
        function usedNonces(bytes32 nonce) external view returns (uint256);
    }

    /// Official V2 event — NOT the legacy uint64 nonce shape.
    event MessageReceived(
        address indexed caller,
        uint32 sourceDomain,
        bytes32 indexed nonce,
        bytes32 sender,
        uint32 indexed finalityThresholdExecuted,
        bytes messageBody
    );
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EthTransaction {
    from: Option<String>,
    to: Option<String>,
    input: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EthReceipt {
    status: Option<String>,
    logs: Option<Vec<EthLog>>,
    block_number: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EthLog {
    address: String,
    topics: Vec<String>,
    data: String,
}

#[derive(Debug, Deserialize)]
struct EthBlockNumber(String);

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedMessageFields {
    source_domain: u32,
    nonce: [u8; 32],
    sender: [u8; 32],
    message_body: Vec<u8>,
}

pub struct EvmRpcMintVerifier {
    rpc: EvmRpcClient,
    message_transmitter: Address,
    /// Configured wire `to` string — must match mint builder for payload hash parity.
    message_transmitter_configured: String,
    min_confirmations: u64,
    probe_ok: bool,
}

impl EvmRpcMintVerifier {
    pub fn new(config: &CctpConfig) -> Result<Self, VerifierError> {
        Self::with_confirmations(config, DEFAULT_MIN_CONFIRMATIONS)
    }

    pub fn with_confirmations(
        config: &CctpConfig,
        min_confirmations: u64,
    ) -> Result<Self, VerifierError> {
        let rpc = EvmRpcClient::new(&config.sepolia_rpc_url)?;
        let message_transmitter = config
            .contracts
            .sepolia_message_transmitter
            .trim()
            .parse()
            .map_err(|_| VerifierError::Failed("message transmitter address".into()))?;
        Ok(Self {
            rpc,
            message_transmitter,
            message_transmitter_configured: config.contracts.sepolia_message_transmitter.clone(),
            min_confirmations,
            probe_ok: false,
        })
    }

    pub async fn try_new(config: &CctpConfig) -> Result<Self, VerifierError> {
        let probe = crate::cctp::evm_readiness_probes::probe_sepolia_with_failover(config).await;
        if !probe.all_ok() {
            return Err(VerifierError::NotReady);
        }
        let mut verifier = Self::with_confirmations(config, DEFAULT_MIN_CONFIRMATIONS)?;
        verifier.probe_ok = true;
        Ok(verifier)
    }

    /// Wiremock/unit tests bypass live Sepolia probes but still exercise verifier logic.
    #[cfg(test)]
    pub fn with_confirmations_for_test(
        config: &CctpConfig,
        min_confirmations: u64,
    ) -> Result<Self, VerifierError> {
        let mut verifier = Self::with_confirmations(config, min_confirmations)?;
        verifier.probe_ok = true;
        Ok(verifier)
    }

    fn hash32(data: &[u8]) -> [u8; 32] {
        Sha256::digest(data).into()
    }

    fn parse_message_fields(message: &[u8]) -> Result<ParsedMessageFields, VerifierError> {
        let parsed =
            parse_cctp_v2_message(message).map_err(|e| VerifierError::Failed(e.to_string()))?;
        if message.len() < MESSAGE_HEADER_LEN {
            return Err(VerifierError::Failed("message too short".into()));
        }
        Ok(ParsedMessageFields {
            source_domain: parsed.source_domain,
            nonce: parsed.nonce,
            sender: parsed.sender,
            message_body: message[MESSAGE_HEADER_LEN..].to_vec(),
        })
    }

    pub(crate) fn parse_stored_nonce(nonce: &str) -> Result<[u8; 32], VerifierError> {
        let trimmed = nonce.trim();
        if let Some(hex) = trimmed
            .strip_prefix("0x")
            .or_else(|| trimmed.strip_prefix("0X"))
        {
            if hex.len() != 64 {
                return Err(VerifierError::Failed("nonce hex length".into()));
            }
            let bytes = hex::decode(hex).map_err(|_| VerifierError::Failed("nonce hex".into()))?;
            let mut out = [0u8; 32];
            out.copy_from_slice(&bytes);
            return Ok(out);
        }
        let n: u128 = trimmed
            .parse()
            .map_err(|_| VerifierError::Failed("nonce format".into()))?;
        let mut out = [0u8; 32];
        out[16..32].copy_from_slice(&n.to_be_bytes());
        Ok(out)
    }

    fn decode_receive_input(input: &str) -> Result<(Vec<u8>, Vec<u8>), VerifierError> {
        let bytes = hex::decode(input.trim_start_matches("0x"))
            .map_err(|_| VerifierError::Failed("calldata hex".into()))?;
        let call = IMessageTransmitterV2::receiveMessageCall::abi_decode(&bytes, true)
            .map_err(|e| VerifierError::Failed(e.to_string()))?;
        Ok((call.message.to_vec(), call.attestation.to_vec()))
    }

    async fn confirmations_ok(&self, receipt: &EthReceipt) -> Result<(), VerifierError> {
        let latest: EthBlockNumber = self
            .rpc
            .call("eth_blockNumber", serde_json::json!([]))
            .await?;
        let latest_num = u64::from_str_radix(latest.0.trim_start_matches("0x"), 16)
            .map_err(|_| VerifierError::Failed("block parse".into()))?;
        let tx_block = receipt
            .block_number
            .as_deref()
            .ok_or_else(|| VerifierError::Failed("pending".into()))?;
        let tx_num = u64::from_str_radix(tx_block.trim_start_matches("0x"), 16)
            .map_err(|_| VerifierError::Failed("tx block parse".into()))?;
        if latest_num.saturating_sub(tx_num) + 1 < self.min_confirmations {
            return Err(VerifierError::Failed("insufficient confirmations".into()));
        }
        Ok(())
    }

    fn decode_message_received_logs(
        &self,
        logs: &[EthLog],
        expected: &ParsedMessageFields,
    ) -> Result<usize, VerifierError> {
        let mut matches = 0usize;
        for log in logs {
            if !log
                .address
                .eq_ignore_ascii_case(&format!("{:#x}", self.message_transmitter))
            {
                continue;
            }
            let topics: Vec<B256> = log.topics.iter().filter_map(|t| t.parse().ok()).collect();
            if topics.is_empty() {
                continue;
            }
            let alloy_log = Log {
                address: self.message_transmitter,
                data: alloy_primitives::LogData::new_unchecked(
                    topics,
                    log.data.parse().unwrap_or_default(),
                ),
            };
            let Ok(decoded) = MessageReceived::decode_log(&alloy_log, true) else {
                continue;
            };
            if decoded.data.sourceDomain != expected.source_domain {
                continue;
            }
            if decoded.data.nonce.0 != expected.nonce {
                continue;
            }
            if decoded.data.sender.0 != expected.sender {
                continue;
            }
            if decoded.data.messageBody.as_ref() != expected.message_body.as_slice() {
                continue;
            }
            matches += 1;
        }
        Ok(matches)
    }

    async fn query_nonce_used(&self, nonce: [u8; 32]) -> Result<bool, VerifierError> {
        self.rpc.ensure_chain().await?;
        let call = IMessageTransmitterV2::usedNoncesCall {
            nonce: FixedBytes::from(nonce),
        };
        let data = format!("0x{}", hex::encode(call.abi_encode()));
        let result = self
            .rpc
            .eth_call(&format!("{:#x}", self.message_transmitter), &data, "latest")
            .await?;
        let hex = result.trim_start_matches("0x");
        if hex.is_empty() {
            return Ok(false);
        }
        let bytes =
            hex::decode(hex).map_err(|_| VerifierError::Failed("usedNonces decode".into()))?;
        let value = U256::from_be_slice(&bytes);
        Ok(value == U256::from(NONCE_USED_VALUE))
    }

    async fn completion_outcome(
        &self,
        tx_hash: &str,
        message: &[u8],
        nonce: &str,
    ) -> Result<MintVerifyOutcome, VerifierError> {
        let expected_msg = Self::parse_message_fields(message)?;
        let expected_nonce = Self::parse_stored_nonce(nonce)?;
        if expected_msg.nonce != expected_nonce {
            return Err(VerifierError::Failed("nonce/message mismatch".into()));
        }

        let hash = EvmRpcClient::normalize_hash(tx_hash);
        let receipt: EthReceipt = self
            .rpc
            .call("eth_getTransactionReceipt", serde_json::json!([hash]))
            .await?;

        if receipt.status.as_deref() != Some("0x1") {
            return Ok(MintVerifyOutcome::FailedRetryable {
                reason: "tx failed".into(),
            });
        }

        self.confirmations_ok(&receipt).await?;

        let logs = receipt.logs.unwrap_or_default();
        let matched = self.decode_message_received_logs(&logs, &expected_msg)?;
        match matched {
            1 => return Ok(MintVerifyOutcome::Succeeded),
            n if n > 1 => return Err(VerifierError::Failed("ambiguous mint logs".into())),
            _ => {}
        }

        match self.query_nonce_used(expected_msg.nonce).await {
            Ok(true) => Ok(MintVerifyOutcome::ReconciliationNonceConsumed),
            Ok(false) => Ok(MintVerifyOutcome::Pending),
            Err(VerifierError::Transient(m)) => Err(VerifierError::Transient(m)),
            Err(VerifierError::NotReady) => Err(VerifierError::NotReady),
            Err(e) => Err(e),
        }
    }
}

#[async_trait]
impl EvmMintVerifier for EvmRpcMintVerifier {
    fn is_ready(&self) -> bool {
        self.probe_ok
    }

    async fn verify_mint_submission(
        &self,
        tx_hash: &str,
        message: &[u8],
        attestation: &[u8],
        nonce: &str,
        expected_payload_hash: &str,
    ) -> Result<VerifiedMintFacts, VerifierError> {
        if !self.is_ready() {
            return Err(VerifierError::NotReady);
        }
        self.rpc.ensure_chain().await?;
        let hash = EvmRpcClient::normalize_hash(tx_hash);
        let tx: EthTransaction = self
            .rpc
            .call("eth_getTransactionByHash", serde_json::json!([hash]))
            .await?;
        let receipt: EthReceipt = self
            .rpc
            .call("eth_getTransactionReceipt", serde_json::json!([hash]))
            .await?;

        if receipt.status.as_deref() != Some("0x1") {
            return Err(VerifierError::Failed("tx failed".into()));
        }
        self.confirmations_ok(&receipt).await?;

        let to = tx
            .to
            .as_deref()
            .ok_or_else(|| VerifierError::Failed("missing to".into()))?
            .to_ascii_lowercase();
        if to != format!("{:#x}", self.message_transmitter).to_ascii_lowercase() {
            return Err(VerifierError::Failed("wrong contract".into()));
        }

        let input = tx
            .input
            .as_deref()
            .ok_or_else(|| VerifierError::Failed("missing input".into()))?;
        let (tx_message, tx_attestation) = Self::decode_receive_input(input)?;
        if tx_message != message || tx_attestation != attestation {
            return Err(VerifierError::Failed("message/attestation mismatch".into()));
        }

        let payload = ProductionEvmCctpBuilder::encode_receive_message(message, attestation);
        let payload_wallet = crate::models::v2_cctp::PreparedWalletPayload::EvmTransaction {
            chain_id: SEPOLIA_CHAIN_ID.into(),
            to: self.message_transmitter_configured.clone(),
            data: format!("0x{}", hex::encode(&payload)),
            value: "0".into(),
        };
        let computed_hash = crate::cctp::builders::evm::hash_payload(&payload_wallet);
        if computed_hash != expected_payload_hash {
            return Err(VerifierError::Failed("payload hash mismatch".into()));
        }

        let outcome = self
            .completion_outcome(tx_hash, message, nonce)
            .await
            .unwrap_or(MintVerifyOutcome::Pending);

        Ok(VerifiedMintFacts {
            tx_hash: hash,
            destination_chain_id: SEPOLIA_CHAIN_ID.into(),
            contract_address: format!("{:#x}", self.message_transmitter),
            function_selector: hex::encode(RECEIVE_MESSAGE_SELECTOR),
            message_hash: Self::hash32(message),
            attestation_hash: Self::hash32(attestation),
            nonce: nonce.to_string(),
            payload_hash: expected_payload_hash.to_string(),
            outcome,
            recipient_evidence: tx.from.clone(),
        })
    }

    async fn verify_mint_completion(
        &self,
        tx_hash: &str,
        message: &[u8],
        nonce: &str,
        _recipient: &str,
        _quoted_finality: crate::models::v2_cctp::CctpFinality,
    ) -> Result<MintVerifyOutcome, VerifierError> {
        if !self.is_ready() {
            return Err(VerifierError::NotReady);
        }
        self.completion_outcome(tx_hash, message, nonce).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cctp::config::CctpConfig;
    use alloy_primitives::Bytes;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sample_message_body() -> Vec<u8> {
        vec![0u8; 228]
    }

    fn sample_cctp_message() -> Vec<u8> {
        let body = sample_message_body();
        let mut msg = vec![0u8; MESSAGE_HEADER_LEN + body.len()];
        msg[0..4].copy_from_slice(&1u32.to_be_bytes());
        msg[4..8].copy_from_slice(&27u32.to_be_bytes());
        msg[8..12].copy_from_slice(&0u32.to_be_bytes());
        let mut nonce = [0u8; 32];
        nonce[31] = 42;
        msg[12..44].copy_from_slice(&nonce);
        msg[140..144].copy_from_slice(&2000u32.to_be_bytes());
        msg[144..148].copy_from_slice(&2000u32.to_be_bytes());
        msg[MESSAGE_HEADER_LEN..].copy_from_slice(&body);
        msg
    }

    fn encode_message_received_log(
        caller: Address,
        source_domain: u32,
        nonce: [u8; 32],
        sender: [u8; 32],
        finality: u32,
        body: &[u8],
    ) -> (Vec<String>, String) {
        let event = MessageReceived {
            caller,
            sourceDomain: source_domain,
            nonce: FixedBytes::from(nonce),
            sender: FixedBytes::from(sender),
            finalityThresholdExecuted: finality,
            messageBody: Bytes::copy_from_slice(body),
        };
        let log_data = event.encode_log_data();
        let topics: Vec<String> = log_data
            .topics()
            .iter()
            .map(|t| format!("{:#x}", t))
            .collect();
        let data = format!("0x{}", hex::encode(log_data.data));
        (topics, data)
    }

    #[test]
    fn message_received_topic0_matches_official_abi() {
        assert_eq!(
            format!("{:#x}", MessageReceived::SIGNATURE_HASH),
            MESSAGE_RECEIVED_TOPIC0
        );
        assert_eq!(
            RECEIVE_MESSAGE_SELECTOR,
            IMessageTransmitterV2::receiveMessageCall::SELECTOR
        );
    }

    #[test]
    fn used_nonces_selector_matches_cast() {
        assert_eq!(
            hex::encode(IMessageTransmitterV2::usedNoncesCall::SELECTOR),
            "feb61724"
        );
    }

    #[tokio::test]
    async fn positive_receive_message_fixture_succeeds() {
        let server = MockServer::start().await;
        let mut cfg = CctpConfig::default_testnet();
        cfg.sepolia_rpc_url = server.uri();
        let verifier = EvmRpcMintVerifier::with_confirmations_for_test(&cfg, 1).unwrap();
        let message = sample_cctp_message();
        let attestation = vec![0xab; 65];
        let receive_call = IMessageTransmitterV2::receiveMessageCall {
            message: Bytes::copy_from_slice(&message),
            attestation: Bytes::copy_from_slice(&attestation),
        };
        let input = format!("0x{}", hex::encode(receive_call.abi_encode()));
        let tx_hash = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let caller: Address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0"
            .parse()
            .unwrap();
        let fields = EvmRpcMintVerifier::parse_message_fields(&message).unwrap();
        let (topics, data) = encode_message_received_log(
            caller,
            fields.source_domain,
            fields.nonce,
            fields.sender,
            2000,
            &fields.message_body,
        );

        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains("eth_getTransactionByHash"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0", "id": 1,
                "result": {
                    "from": format!("{:#x}", caller),
                    "to": cfg.contracts.sepolia_message_transmitter,
                    "input": input
                }
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(body_string_contains("eth_getTransactionReceipt"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0", "id": 1,
                "result": {
                    "status": "0x1",
                    "blockNumber": "0x10",
                    "logs": [{
                        "address": cfg.contracts.sepolia_message_transmitter,
                        "topics": topics,
                        "data": data
                    }]
                }
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(body_string_contains("eth_chainId"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0", "id": 1, "result": "0xaa36a7"
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(body_string_contains("eth_blockNumber"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0", "id": 1, "result": "0x10"
            })))
            .mount(&server)
            .await;

        let outcome = verifier
            .verify_mint_completion(
                &tx_hash,
                &message,
                "42",
                "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0",
                crate::models::v2_cctp::CctpFinality::Standard,
            )
            .await
            .unwrap();
        assert_eq!(outcome, MintVerifyOutcome::Succeeded);
    }

    async fn mount_completion_mocks(
        server: &MockServer,
        cfg: &CctpConfig,
        tx_hash: &str,
        logs: serde_json::Value,
        used_nonces_result: &str,
    ) {
        Mock::given(method("POST"))
            .and(body_string_contains("eth_getTransactionReceipt"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0", "id": 1,
                "result": { "status": "0x1", "blockNumber": "0x10", "logs": logs }
            })))
            .mount(server)
            .await;
        Mock::given(method("POST"))
            .and(body_string_contains("eth_chainId"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0", "id": 1, "result": "0xaa36a7"
            })))
            .mount(server)
            .await;
        Mock::given(method("POST"))
            .and(body_string_contains("eth_blockNumber"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0", "id": 1, "result": "0x10"
            })))
            .mount(server)
            .await;
        Mock::given(method("POST"))
            .and(body_string_contains("eth_call"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0", "id": 1, "result": used_nonces_result
            })))
            .mount(server)
            .await;
        let _ = (tx_hash, cfg);
    }

    #[tokio::test]
    async fn wrong_topic0_stays_pending() {
        let server = MockServer::start().await;
        let mut cfg = CctpConfig::default_testnet();
        cfg.sepolia_rpc_url = server.uri();
        let verifier = EvmRpcMintVerifier::with_confirmations_for_test(&cfg, 1).unwrap();
        let message = sample_cctp_message();
        let fields = EvmRpcMintVerifier::parse_message_fields(&message).unwrap();
        let caller: Address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0"
            .parse()
            .unwrap();
        let (mut topics, data) = encode_message_received_log(
            caller,
            fields.source_domain,
            fields.nonce,
            fields.sender,
            2000,
            &fields.message_body,
        );
        topics[0] = "0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef".into();
        let tx_hash = "0x2222222222222222222222222222222222222222222222222222222222222222";
        mount_completion_mocks(
            &server,
            &cfg,
            tx_hash,
            serde_json::json!([{
                "address": cfg.contracts.sepolia_message_transmitter,
                "topics": topics,
                "data": data
            }]),
            "0x0000000000000000000000000000000000000000000000000000000000000000",
        )
        .await;
        let outcome = verifier
            .verify_mint_completion(
                tx_hash,
                &message,
                "42",
                "0x0",
                crate::models::v2_cctp::CctpFinality::Standard,
            )
            .await
            .unwrap();
        assert_eq!(outcome, MintVerifyOutcome::Pending);
    }

    #[tokio::test]
    async fn wrong_log_address_stays_pending() {
        let server = MockServer::start().await;
        let mut cfg = CctpConfig::default_testnet();
        cfg.sepolia_rpc_url = server.uri();
        let verifier = EvmRpcMintVerifier::with_confirmations_for_test(&cfg, 1).unwrap();
        let message = sample_cctp_message();
        let fields = EvmRpcMintVerifier::parse_message_fields(&message).unwrap();
        let caller: Address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0"
            .parse()
            .unwrap();
        let (topics, data) = encode_message_received_log(
            caller,
            fields.source_domain,
            fields.nonce,
            fields.sender,
            2000,
            &fields.message_body,
        );
        let tx_hash = "0x3333333333333333333333333333333333333333333333333333333333333333";
        mount_completion_mocks(
            &server,
            &cfg,
            tx_hash,
            serde_json::json!([{
                "address": "0x0000000000000000000000000000000000000001",
                "topics": topics,
                "data": data
            }]),
            "0x0000000000000000000000000000000000000000000000000000000000000000",
        )
        .await;
        let outcome = verifier
            .verify_mint_completion(
                tx_hash,
                &message,
                "42",
                "0x0",
                crate::models::v2_cctp::CctpFinality::Standard,
            )
            .await
            .unwrap();
        assert_eq!(outcome, MintVerifyOutcome::Pending);
    }

    #[tokio::test]
    async fn wrong_nonce_in_log_stays_pending() {
        let server = MockServer::start().await;
        let mut cfg = CctpConfig::default_testnet();
        cfg.sepolia_rpc_url = server.uri();
        let verifier = EvmRpcMintVerifier::with_confirmations_for_test(&cfg, 1).unwrap();
        let message = sample_cctp_message();
        let fields = EvmRpcMintVerifier::parse_message_fields(&message).unwrap();
        let mut wrong_nonce = fields.nonce;
        wrong_nonce[31] ^= 0xff;
        let caller: Address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0"
            .parse()
            .unwrap();
        let (topics, data) = encode_message_received_log(
            caller,
            fields.source_domain,
            wrong_nonce,
            fields.sender,
            2000,
            &fields.message_body,
        );
        let tx_hash = "0x4444444444444444444444444444444444444444444444444444444444444444";
        mount_completion_mocks(
            &server,
            &cfg,
            tx_hash,
            serde_json::json!([{
                "address": cfg.contracts.sepolia_message_transmitter,
                "topics": topics,
                "data": data
            }]),
            "0x0000000000000000000000000000000000000000000000000000000000000000",
        )
        .await;
        let outcome = verifier
            .verify_mint_completion(
                tx_hash,
                &message,
                "42",
                "0x0",
                crate::models::v2_cctp::CctpFinality::Standard,
            )
            .await
            .unwrap();
        assert_eq!(outcome, MintVerifyOutcome::Pending);
    }

    #[tokio::test]
    async fn wrong_sender_in_log_stays_pending() {
        let server = MockServer::start().await;
        let mut cfg = CctpConfig::default_testnet();
        cfg.sepolia_rpc_url = server.uri();
        let verifier = EvmRpcMintVerifier::with_confirmations_for_test(&cfg, 1).unwrap();
        let message = sample_cctp_message();
        let fields = EvmRpcMintVerifier::parse_message_fields(&message).unwrap();
        let mut wrong_sender = fields.sender;
        wrong_sender[0] ^= 0xff;
        let caller: Address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0"
            .parse()
            .unwrap();
        let (topics, data) = encode_message_received_log(
            caller,
            fields.source_domain,
            fields.nonce,
            wrong_sender,
            2000,
            &fields.message_body,
        );
        let tx_hash = "0x5555555555555555555555555555555555555555555555555555555555555555";
        mount_completion_mocks(
            &server,
            &cfg,
            tx_hash,
            serde_json::json!([{
                "address": cfg.contracts.sepolia_message_transmitter,
                "topics": topics,
                "data": data
            }]),
            "0x0000000000000000000000000000000000000000000000000000000000000000",
        )
        .await;
        let outcome = verifier
            .verify_mint_completion(
                tx_hash,
                &message,
                "42",
                "0x0",
                crate::models::v2_cctp::CctpFinality::Standard,
            )
            .await
            .unwrap();
        assert_eq!(outcome, MintVerifyOutcome::Pending);
    }

    #[tokio::test]
    async fn wrong_message_body_in_log_stays_pending() {
        let server = MockServer::start().await;
        let mut cfg = CctpConfig::default_testnet();
        cfg.sepolia_rpc_url = server.uri();
        let verifier = EvmRpcMintVerifier::with_confirmations_for_test(&cfg, 1).unwrap();
        let message = sample_cctp_message();
        let fields = EvmRpcMintVerifier::parse_message_fields(&message).unwrap();
        let mut wrong_body = fields.message_body.clone();
        wrong_body[0] ^= 0xff;
        let caller: Address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0"
            .parse()
            .unwrap();
        let (topics, data) = encode_message_received_log(
            caller,
            fields.source_domain,
            fields.nonce,
            fields.sender,
            2000,
            &wrong_body,
        );
        let tx_hash = "0x6666666666666666666666666666666666666666666666666666666666666666";
        mount_completion_mocks(
            &server,
            &cfg,
            tx_hash,
            serde_json::json!([{
                "address": cfg.contracts.sepolia_message_transmitter,
                "topics": topics,
                "data": data
            }]),
            "0x0000000000000000000000000000000000000000000000000000000000000000",
        )
        .await;
        let outcome = verifier
            .verify_mint_completion(
                tx_hash,
                &message,
                "42",
                "0x0",
                crate::models::v2_cctp::CctpFinality::Standard,
            )
            .await
            .unwrap();
        assert_eq!(outcome, MintVerifyOutcome::Pending);
    }

    #[tokio::test]
    async fn multiple_matching_events_fails() {
        let server = MockServer::start().await;
        let mut cfg = CctpConfig::default_testnet();
        cfg.sepolia_rpc_url = server.uri();
        let verifier = EvmRpcMintVerifier::with_confirmations_for_test(&cfg, 1).unwrap();
        let message = sample_cctp_message();
        let fields = EvmRpcMintVerifier::parse_message_fields(&message).unwrap();
        let caller: Address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0"
            .parse()
            .unwrap();
        let (topics, data) = encode_message_received_log(
            caller,
            fields.source_domain,
            fields.nonce,
            fields.sender,
            2000,
            &fields.message_body,
        );
        let tx_hash = "0x7777777777777777777777777777777777777777777777777777777777777777";
        mount_completion_mocks(
            &server,
            &cfg,
            tx_hash,
            serde_json::json!([
                { "address": cfg.contracts.sepolia_message_transmitter, "topics": topics.clone(), "data": data.clone() },
                { "address": cfg.contracts.sepolia_message_transmitter, "topics": topics, "data": data }
            ]),
            "0x0000000000000000000000000000000000000000000000000000000000000000",
        )
        .await;
        let err = verifier
            .verify_mint_completion(
                tx_hash,
                &message,
                "42",
                "0x0",
                crate::models::v2_cctp::CctpFinality::Standard,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, VerifierError::Failed(_)));
    }

    #[test]
    fn malformed_nonce_rejected() {
        assert!(EvmRpcMintVerifier::parse_stored_nonce("not-a-nonce").is_err());
        assert!(EvmRpcMintVerifier::parse_stored_nonce("0xabcd").is_err());
    }

    #[tokio::test]
    async fn nonce_message_mismatch_rejected() {
        let server = MockServer::start().await;
        let mut cfg = CctpConfig::default_testnet();
        cfg.sepolia_rpc_url = server.uri();
        let verifier = EvmRpcMintVerifier::with_confirmations_for_test(&cfg, 1).unwrap();
        let message = sample_cctp_message();
        let tx_hash = "0x8888888888888888888888888888888888888888888888888888888888888888";
        let err = verifier
            .verify_mint_completion(
                tx_hash,
                &message,
                "99",
                "0x0",
                crate::models::v2_cctp::CctpFinality::Standard,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, VerifierError::Failed(_)));
    }

    #[tokio::test]
    async fn wrong_chain_returns_failed() {
        let server = MockServer::start().await;
        let mut cfg = CctpConfig::default_testnet();
        cfg.sepolia_rpc_url = server.uri();
        let verifier = EvmRpcMintVerifier::with_confirmations_for_test(&cfg, 1).unwrap();
        let message = sample_cctp_message();
        let tx_hash = "0x9999999999999999999999999999999999999999999999999999999999999999";
        Mock::given(method("POST"))
            .and(body_string_contains("eth_getTransactionReceipt"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0", "id": 1,
                "result": { "status": "0x1", "blockNumber": "0x10", "logs": [] }
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(body_string_contains("eth_chainId"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0", "id": 1, "result": "0x1"
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(body_string_contains("eth_blockNumber"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0", "id": 1, "result": "0x10"
            })))
            .mount(&server)
            .await;
        let err = verifier
            .verify_mint_completion(
                tx_hash,
                &message,
                "42",
                "0x0",
                crate::models::v2_cctp::CctpFinality::Standard,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, VerifierError::Failed(_)));
    }

    #[tokio::test]
    async fn rpc_error_on_used_nonces_propagates() {
        let server = MockServer::start().await;
        let mut cfg = CctpConfig::default_testnet();
        cfg.sepolia_rpc_url = server.uri();
        let verifier = EvmRpcMintVerifier::with_confirmations_for_test(&cfg, 1).unwrap();
        let message = sample_cctp_message();
        let tx_hash = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

        Mock::given(method("POST"))
            .and(body_string_contains("eth_getTransactionReceipt"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0", "id": 1,
                "result": { "status": "0x1", "blockNumber": "0x10", "logs": [] }
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(body_string_contains("eth_chainId"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0", "id": 1, "result": "0xaa36a7"
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(body_string_contains("eth_blockNumber"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0", "id": 1, "result": "0x10"
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(body_string_contains("eth_call"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0", "id": 1, "error": { "code": -32000, "message": "execution reverted" }
            })))
            .mount(&server)
            .await;
        let err = verifier
            .verify_mint_completion(
                tx_hash,
                &message,
                "42",
                "0x0",
                crate::models::v2_cctp::CctpFinality::Standard,
            )
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            VerifierError::Failed(_) | VerifierError::Transient(_)
        ));
    }

    #[tokio::test]
    async fn nonce_used_via_used_nonces_mapping_when_event_missing() {
        let server = MockServer::start().await;
        let mut cfg = CctpConfig::default_testnet();
        cfg.sepolia_rpc_url = server.uri();
        let verifier = EvmRpcMintVerifier::with_confirmations_for_test(&cfg, 1).unwrap();
        let message = sample_cctp_message();
        let tx_hash = "0xabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd";
        let fields = EvmRpcMintVerifier::parse_message_fields(&message).unwrap();
        let nonce_topic = format!("{:#x}", B256::from(fields.nonce));

        Mock::given(method("POST"))
            .and(body_string_contains("eth_getTransactionReceipt"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0", "id": 1,
                "result": { "status": "0x1", "blockNumber": "0x10", "logs": [] }
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(body_string_contains("eth_chainId"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0", "id": 1, "result": "0xaa36a7"
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(body_string_contains("eth_blockNumber"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0", "id": 1, "result": "0x10"
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(body_string_contains("eth_call"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0", "id": 1,
                "result": "0x0000000000000000000000000000000000000000000000000000000000000001"
            })))
            .mount(&server)
            .await;

        let outcome = verifier
            .verify_mint_completion(
                &tx_hash,
                &message,
                "42",
                "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0",
                crate::models::v2_cctp::CctpFinality::Standard,
            )
            .await
            .unwrap();
        assert_eq!(outcome, MintVerifyOutcome::ReconciliationNonceConsumed);
        let _ = nonce_topic;
    }

    #[tokio::test]
    async fn wrong_source_domain_in_log_stays_pending() {
        let server = MockServer::start().await;
        let mut cfg = CctpConfig::default_testnet();
        cfg.sepolia_rpc_url = server.uri();
        let verifier = EvmRpcMintVerifier::with_confirmations_for_test(&cfg, 1).unwrap();
        let message = sample_cctp_message();
        let fields = EvmRpcMintVerifier::parse_message_fields(&message).unwrap();
        let caller: Address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0"
            .parse()
            .unwrap();
        let (topics, data) = encode_message_received_log(
            caller,
            99,
            fields.nonce,
            fields.sender,
            2000,
            &fields.message_body,
        );
        let tx_hash = "0x1111111111111111111111111111111111111111111111111111111111111111";

        Mock::given(method("POST"))
            .and(body_string_contains("eth_getTransactionReceipt"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0", "id": 1,
                "result": {
                    "status": "0x1", "blockNumber": "0x10",
                    "logs": [{ "address": cfg.contracts.sepolia_message_transmitter, "topics": topics, "data": data }]
                }
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(body_string_contains("eth_chainId"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0", "id": 1, "result": "0xaa36a7"
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(body_string_contains("eth_blockNumber"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0", "id": 1, "result": "0x10"
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(body_string_contains("eth_call"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0", "id": 1,
                "result": "0x0000000000000000000000000000000000000000000000000000000000000000"
            })))
            .mount(&server)
            .await;

        let outcome = verifier
            .verify_mint_completion(
                &tx_hash,
                &message,
                "42",
                "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0",
                crate::models::v2_cctp::CctpFinality::Standard,
            )
            .await
            .unwrap();
        assert_eq!(outcome, MintVerifyOutcome::Pending);
    }
}
