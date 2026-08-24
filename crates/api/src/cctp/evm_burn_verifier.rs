//! Production EVM Sepolia burn verifier via JSON-RPC.
//!
//! Verifies tx `input` calldata independently of event logs, then cross-checks exactly one
//! `DepositForBurn` event from TokenMessengerV2.

use alloy_primitives::{Address, Log, B256};
use alloy_sol_types::{sol, SolCall, SolEvent};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

use crate::cctp::builders::evm::{ITokenMessengerV2, SEPOLIA_CHAIN_ID_NUM};
use crate::cctp::config::CctpConfig;
use crate::cctp::verifiers::{EvmBurnVerifier, VerifiedBurnFacts, VerifierError};

/// `depositForBurn(uint256,uint32,bytes32,address,bytes32,uint256,uint32)` — Circle TokenMessengerV2.
/// Cross-check: `cast sig "depositForBurn(uint256,uint32,bytes32,address,bytes32,uint256,uint32)"` => 0x8e0250ee
pub const DEPOSIT_FOR_BURN_SELECTOR: [u8; 4] = [0x8e, 0x02, 0x50, 0xee];

/// `depositForBurnWithHook(uint256,uint32,bytes32,address,bytes32,uint256,uint32,bytes)`.
/// Cross-check: `cast sig "depositForBurnWithHook(uint256,uint32,bytes32,address,bytes32,uint256,uint32,bytes)"` => 0x779b432d
pub const DEPOSIT_FOR_BURN_WITH_HOOK_SELECTOR: [u8; 4] = [0x77, 0x9b, 0x43, 0x2d];

const DEFAULT_MIN_CONFIRMATIONS: u64 = 1;
const MAX_JSON_BODY_BYTES: usize = 256 * 1024;

sol! {
    event DepositForBurn(
        address indexed burnToken,
        uint256 amount,
        address indexed depositor,
        bytes32 mintRecipient,
        uint32 destinationDomain,
        bytes32 destinationTokenMessenger,
        bytes32 destinationCaller,
        uint256 maxFee,
        uint32 indexed minFinalityThreshold,
        bytes hookData
    );
}

#[derive(Debug)]
pub struct EvmRpcBurnVerifier {
    client: Client,
    rpc_url: String,
    token_messenger: Address,
    chain_id: u64,
    min_confirmations: u64,
    probe_ok: bool,
}

impl EvmRpcBurnVerifier {
    pub fn new(config: &CctpConfig) -> Result<Self, VerifierError> {
        Self::with_confirmations(config, DEFAULT_MIN_CONFIRMATIONS)
    }

    pub fn with_confirmations(
        config: &CctpConfig,
        min_confirmations: u64,
    ) -> Result<Self, VerifierError> {
        if config.sepolia_rpc_url.trim().is_empty() {
            return Err(VerifierError::NotReady);
        }
        let token_messenger = config
            .contracts
            .sepolia_token_messenger
            .trim()
            .parse()
            .map_err(|_| VerifierError::Failed("token messenger address".into()))?;
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .map_err(|e| VerifierError::Transient(e.to_string()))?,
            rpc_url: config.sepolia_rpc_url.clone(),
            token_messenger,
            chain_id: SEPOLIA_CHAIN_ID_NUM,
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

    fn normalize_hash(hash: &str) -> String {
        let trimmed = hash.trim();
        let hex = trimmed
            .strip_prefix("0x")
            .or_else(|| trimmed.strip_prefix("0X"))
            .unwrap_or(trimmed);
        format!("0x{}", hex.to_ascii_lowercase())
    }

    fn bound_body(body: &str) -> Result<(), VerifierError> {
        if body.len() > MAX_JSON_BODY_BYTES {
            return Err(VerifierError::Failed("rpc response too large".into()));
        }
        Ok(())
    }

    async fn rpc_call<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<T, VerifierError> {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });
        let body_str = body.to_string();
        if body_str.len() > MAX_JSON_BODY_BYTES {
            return Err(VerifierError::Failed("rpc request too large".into()));
        }
        let resp = self
            .client
            .post(&self.rpc_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| VerifierError::Transient(e.to_string()))?;
        let text = resp
            .text()
            .await
            .map_err(|e| VerifierError::Transient(e.to_string()))?;
        Self::bound_body(&text)?;
        let payload: RpcResponse<T> =
            serde_json::from_str(&text).map_err(|e| VerifierError::Failed(e.to_string()))?;
        if let Some(err) = payload.error {
            if err.message.to_ascii_lowercase().contains("rate limit") {
                return Err(VerifierError::Transient(err.message));
            }
            return Err(VerifierError::Failed(err.message));
        }
        payload.result.ok_or(VerifierError::TxNotFound)
    }

    fn decode_calldata(input: &str) -> Result<DecodedBurnCall, VerifierError> {
        let hex = input
            .strip_prefix("0x")
            .or_else(|| input.strip_prefix("0X"))
            .unwrap_or(input);
        let bytes = hex::decode(hex).map_err(|_| VerifierError::Failed("calldata hex".into()))?;
        if bytes.len() < 4 {
            return Err(VerifierError::Failed("calldata too short".into()));
        }
        let selector: [u8; 4] = bytes[0..4].try_into().unwrap();
        if selector == ITokenMessengerV2::depositForBurnCall::SELECTOR {
            let call = ITokenMessengerV2::depositForBurnCall::abi_decode(&bytes, true)
                .map_err(|e| VerifierError::Failed(e.to_string()))?;
            Ok(DecodedBurnCall {
                amount: call.amount,
                destination_domain: call.destinationDomain,
                mint_recipient: call.mintRecipient.0,
                burn_token: call.burnToken,
                destination_caller: call.destinationCaller.0,
                max_fee: call.maxFee,
                min_finality: call.minFinalityThreshold,
                hook_data: None,
            })
        } else if selector == ITokenMessengerV2::depositForBurnWithHookCall::SELECTOR {
            let call = ITokenMessengerV2::depositForBurnWithHookCall::abi_decode(&bytes, true)
                .map_err(|e| VerifierError::Failed(e.to_string()))?;
            let hook = if call.hookData.is_empty() {
                None
            } else {
                Some(call.hookData.to_vec())
            };
            Ok(DecodedBurnCall {
                amount: call.amount,
                destination_domain: call.destinationDomain,
                mint_recipient: call.mintRecipient.0,
                burn_token: call.burnToken,
                destination_caller: call.destinationCaller.0,
                max_fee: call.maxFee,
                min_finality: call.minFinalityThreshold,
                hook_data: hook,
            })
        } else {
            Err(VerifierError::Failed("unknown burn selector".into()))
        }
    }

    fn calldata_matches_event(call: &DecodedBurnCall, event: &DepositForBurn) -> bool {
        call.amount == event.amount
            && call.destination_domain == event.destinationDomain
            && call.mint_recipient == event.mintRecipient.0
            && call.burn_token == event.burnToken
            && call.destination_caller == event.destinationCaller.0
            && call.max_fee == event.maxFee
            && call.min_finality == event.minFinalityThreshold
            && call.hook_data.as_deref().unwrap_or(&[]) == event.hookData.as_ref()
    }
}

struct DecodedBurnCall {
    amount: alloy_primitives::U256,
    destination_domain: u32,
    mint_recipient: [u8; 32],
    burn_token: Address,
    destination_caller: [u8; 32],
    max_fee: alloy_primitives::U256,
    min_finality: u32,
    hook_data: Option<Vec<u8>>,
}

#[derive(Debug, Deserialize)]
struct RpcResponse<T> {
    result: Option<T>,
    error: Option<RpcError>,
}

#[derive(Debug, Deserialize)]
struct RpcError {
    message: String,
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

#[async_trait]
impl EvmBurnVerifier for EvmRpcBurnVerifier {
    fn is_ready(&self) -> bool {
        self.probe_ok
    }

    async fn verify_burn(&self, tx_hash: &str) -> Result<VerifiedBurnFacts, VerifierError> {
        if !self.is_ready() {
            return Err(VerifierError::NotReady);
        }
        let hash = Self::normalize_hash(tx_hash);
        let tx: EthTransaction = self
            .rpc_call("eth_getTransactionByHash", json!([hash]))
            .await?;
        let receipt: EthReceipt = self
            .rpc_call("eth_getTransactionReceipt", json!([hash]))
            .await?;

        if receipt.status.as_deref() != Some("0x1") {
            return Err(VerifierError::Failed("tx failed".into()));
        }

        let from = tx
            .from
            .as_deref()
            .ok_or_else(|| VerifierError::Failed("missing from".into()))?;
        let to = tx
            .to
            .as_deref()
            .ok_or_else(|| VerifierError::Failed("missing to".into()))?
            .to_ascii_lowercase();
        if to != format!("{:#x}", self.token_messenger).to_ascii_lowercase()
            && to != format!("0x{:x}", self.token_messenger).to_ascii_lowercase()
        {
            return Err(VerifierError::Failed("wrong contract".into()));
        }

        let input = tx
            .input
            .as_deref()
            .ok_or_else(|| VerifierError::Failed("missing input".into()))?;
        let decoded_call = Self::decode_calldata(input)?;

        let chain_id_resp: String = self.rpc_call("eth_chainId", json!([])).await?;
        let parsed_chain = u64::from_str_radix(chain_id_resp.trim_start_matches("0x"), 16)
            .map_err(|_| VerifierError::Failed("chain id parse".into()))?;
        if parsed_chain != self.chain_id {
            return Err(VerifierError::Failed("wrong chain".into()));
        }

        let latest: EthBlockNumber = self.rpc_call("eth_blockNumber", json!([])).await?;
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

        let logs = receipt.logs.unwrap_or_default();
        let mut matches = 0usize;
        let mut parsed_event: Option<DepositForBurn> = None;
        for log in logs {
            if !log
                .address
                .eq_ignore_ascii_case(&format!("{:#x}", self.token_messenger))
            {
                continue;
            }
            let topics: Vec<B256> = log.topics.iter().filter_map(|t| t.parse().ok()).collect();
            if topics.is_empty() {
                continue;
            }
            let alloy_log = Log {
                address: self.token_messenger,
                data: alloy_primitives::LogData::new_unchecked(
                    topics,
                    log.data.parse().unwrap_or_default(),
                ),
            };
            if let Ok(decoded) = DepositForBurn::decode_log(&alloy_log, true) {
                matches += 1;
                parsed_event = Some(decoded.data);
            }
        }
        if matches != 1 {
            return Err(VerifierError::Failed("ambiguous burn logs".into()));
        }
        let event = parsed_event.ok_or_else(|| VerifierError::Failed("no burn event".into()))?;

        if !Self::calldata_matches_event(&decoded_call, &event) {
            return Err(VerifierError::Failed("calldata/event mismatch".into()));
        }

        let hook_data = if event.hookData.is_empty() {
            None
        } else {
            Some(event.hookData.to_vec())
        };

        Ok(VerifiedBurnFacts {
            tx_hash: hash,
            source_chain_id: format!("eip155:{}", self.chain_id),
            source_domain: 0,
            destination_domain: event.destinationDomain,
            sender: from.to_string(),
            amount_cctp_subunits: event.amount.try_into().unwrap_or(0),
            burn_token_bytes32: address_to_bytes32(event.burnToken),
            mint_recipient_bytes32: event.mintRecipient.0,
            destination_caller_bytes32: event.destinationCaller.0,
            min_finality_threshold: event.minFinalityThreshold,
            hook_data,
            token_messenger_bytes32: address_to_bytes32(self.token_messenger),
            block_or_ledger: receipt.block_number,
        })
    }
}

fn address_to_bytes32(addr: Address) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[12..32].copy_from_slice(addr.as_slice());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cctp::builders::evm::ProductionEvmCctpBuilder;
    use crate::cctp::config::CctpConfig;
    use crate::cctp::encoding::evm_address_to_bytes32;
    use crate::cctp::expectations::ANY_DESTINATION_CALLER;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn selector_literals_match_alloy_codegen() {
        assert_eq!(
            DEPOSIT_FOR_BURN_SELECTOR,
            ITokenMessengerV2::depositForBurnCall::SELECTOR
        );
        assert_eq!(
            DEPOSIT_FOR_BURN_WITH_HOOK_SELECTOR,
            ITokenMessengerV2::depositForBurnWithHookCall::SELECTOR
        );
        // Event topic0 is keccak256(event signature) — distinct from function selectors.
        assert_ne!(
            &DepositForBurn::SIGNATURE_HASH.as_slice()[0..4],
            &ITokenMessengerV2::depositForBurnCall::SELECTOR
        );
    }

    #[test]
    fn decodes_synthetic_deposit_for_burn_calldata() {
        let mint = evm_address_to_bytes32("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0").unwrap();
        let data = ProductionEvmCctpBuilder::encode_deposit_for_burn(
            1_000_000,
            27,
            mint,
            crate::cctp::config::SEPOLIA_USDC,
            ANY_DESTINATION_CALLER,
            "1",
            crate::cctp::config::FINALITY_STANDARD,
        )
        .unwrap();
        let input = format!("0x{}", hex::encode(&data));
        let decoded = EvmRpcBurnVerifier::decode_calldata(&input).unwrap();
        assert_eq!(decoded.amount, alloy_primitives::U256::from(1_000_000u64));
        assert_eq!(decoded.destination_domain, 27);
    }

    #[test]
    fn not_ready_without_rpc_url() {
        let mut cfg = CctpConfig::default_testnet();
        cfg.sepolia_rpc_url = String::new();
        assert!(matches!(
            EvmRpcBurnVerifier::new(&cfg),
            Err(VerifierError::NotReady)
        ));
    }

    #[tokio::test]
    async fn rejects_failed_receipt() {
        let server = MockServer::start().await;
        let mut cfg = CctpConfig::default_testnet();
        cfg.sepolia_rpc_url = server.uri();
        let verifier = EvmRpcBurnVerifier::with_confirmations_for_test(&cfg, 1).unwrap();

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": {
                    "from": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0",
                    "to": cfg.contracts.sepolia_token_messenger,
                    "input": "0x"
                }
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": { "status": "0x0", "logs": [] }
            })))
            .mount(&server)
            .await;

        let err = verifier
            .verify_burn("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
            .await
            .unwrap_err();
        assert_eq!(err, VerifierError::Failed("tx failed".into()));
    }

    #[tokio::test]
    async fn accepts_synthetic_deposit_for_burn_fixture() {
        use alloy_primitives::{Address, Bytes, FixedBytes, B256, U256};
        use alloy_sol_types::SolEvent;
        use wiremock::matchers::{body_string_contains, method, path};

        let server = MockServer::start().await;
        let mut cfg = CctpConfig::default_testnet();
        cfg.sepolia_rpc_url = server.uri();
        let verifier = EvmRpcBurnVerifier::with_confirmations_for_test(&cfg, 1).unwrap();

        let from: Address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0"
            .parse()
            .unwrap();
        let burn_token: Address = crate::cctp::config::SEPOLIA_USDC.parse().unwrap();
        let mint_recipient =
            evm_address_to_bytes32("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0").unwrap();
        let amount = 1_000_000u128;
        let dest_domain = crate::cctp::config::STELLAR_TESTNET_DOMAIN;
        let data = ProductionEvmCctpBuilder::encode_deposit_for_burn(
            amount,
            dest_domain,
            mint_recipient,
            crate::cctp::config::SEPOLIA_USDC,
            ANY_DESTINATION_CALLER,
            "1",
            crate::cctp::config::FINALITY_STANDARD,
        )
        .unwrap();
        let input = format!("0x{}", hex::encode(&data));
        let tx_hash = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

        let max_fee = U256::from(crate::cctp::encoding::decimal_to_cctp_subunits("1").unwrap());
        let event = DepositForBurn {
            burnToken: burn_token,
            amount: U256::from(amount),
            depositor: from,
            mintRecipient: FixedBytes::from_slice(&mint_recipient),
            destinationDomain: dest_domain,
            destinationTokenMessenger: B256::ZERO,
            destinationCaller: FixedBytes::from_slice(&ANY_DESTINATION_CALLER),
            maxFee: max_fee,
            minFinalityThreshold: crate::cctp::config::FINALITY_STANDARD,
            hookData: Bytes::new(),
        };
        let log_data = event.encode_log_data();
        let topics: Vec<String> = log_data
            .topics()
            .iter()
            .map(|t| format!("{:#x}", t))
            .collect();
        let log_body = format!("0x{}", hex::encode(log_data.data));

        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains("eth_getTransactionByHash"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": {
                    "from": format!("{:#x}", from),
                    "to": cfg.contracts.sepolia_token_messenger,
                    "input": input
                }
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(body_string_contains("eth_getTransactionReceipt"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": {
                    "status": "0x1",
                    "blockNumber": "0x10",
                    "logs": [{
                        "address": cfg.contracts.sepolia_token_messenger,
                        "topics": topics,
                        "data": log_body
                    }]
                }
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(body_string_contains("eth_chainId"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": "0xaa36a7"
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(body_string_contains("eth_blockNumber"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": "0x10"
            })))
            .mount(&server)
            .await;

        let facts = verifier.verify_burn(tx_hash).await.unwrap();
        assert_eq!(facts.tx_hash, tx_hash);
        assert_eq!(facts.amount_cctp_subunits, amount as u128);
        assert_eq!(facts.destination_domain, dest_domain);
        assert_eq!(facts.sender.to_ascii_lowercase(), format!("{:#x}", from));
    }

    #[tokio::test]
    async fn accepts_independent_cast_calldata_fixture() {
        use crate::cctp::fixtures::circle_evm_burn_v2::{
            FIXTURE_AMOUNT, FIXTURE_BURN_TOKEN, FIXTURE_DEPOSITOR, FIXTURE_DESTINATION_DOMAIN,
            FIXTURE_MAX_FEE, FIXTURE_MIN_FINALITY, INDEPENDENT_DEPOSIT_FOR_BURN_INPUT,
        };
        use alloy_primitives::{Address, Bytes, FixedBytes, B256, U256};
        use alloy_sol_types::SolEvent;
        use wiremock::matchers::{body_string_contains, method, path};

        let server = MockServer::start().await;
        let mut cfg = CctpConfig::default_testnet();
        cfg.sepolia_rpc_url = server.uri();
        let verifier = EvmRpcBurnVerifier::with_confirmations_for_test(&cfg, 1).unwrap();

        let from: Address = FIXTURE_DEPOSITOR.parse().unwrap();
        let burn_token: Address = FIXTURE_BURN_TOKEN.parse().unwrap();
        let mint_recipient =
            evm_address_to_bytes32("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0").unwrap();
        let input = INDEPENDENT_DEPOSIT_FOR_BURN_INPUT;
        let tx_hash = "0xabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd";

        let event = DepositForBurn {
            burnToken: burn_token,
            amount: U256::from(FIXTURE_AMOUNT),
            depositor: from,
            mintRecipient: FixedBytes::from_slice(&mint_recipient),
            destinationDomain: FIXTURE_DESTINATION_DOMAIN,
            destinationTokenMessenger: B256::ZERO,
            destinationCaller: FixedBytes::from_slice(&ANY_DESTINATION_CALLER),
            maxFee: U256::from(FIXTURE_MAX_FEE),
            minFinalityThreshold: FIXTURE_MIN_FINALITY,
            hookData: Bytes::new(),
        };
        let log_data = event.encode_log_data();
        let topics: Vec<String> = log_data
            .topics()
            .iter()
            .map(|t| format!("{:#x}", t))
            .collect();
        let log_body = format!("0x{}", hex::encode(log_data.data));

        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains("eth_getTransactionByHash"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0", "id": 1,
                "result": {
                    "from": FIXTURE_DEPOSITOR,
                    "to": cfg.contracts.sepolia_token_messenger,
                    "input": input
                }
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(body_string_contains("eth_getTransactionReceipt"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0", "id": 1,
                "result": {
                    "status": "0x1",
                    "blockNumber": "0x10",
                    "logs": [{
                        "address": cfg.contracts.sepolia_token_messenger,
                        "topics": topics,
                        "data": log_body
                    }]
                }
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(body_string_contains("eth_chainId"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0", "id": 1, "result": "0xaa36a7"
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(body_string_contains("eth_blockNumber"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0", "id": 1, "result": "0x10"
            })))
            .mount(&server)
            .await;

        let facts = verifier.verify_burn(tx_hash).await.unwrap();
        assert_eq!(facts.amount_cctp_subunits, FIXTURE_AMOUNT);
        assert_eq!(facts.destination_domain, FIXTURE_DESTINATION_DOMAIN);
        let decoded = EvmRpcBurnVerifier::decode_calldata(input).unwrap();
        assert_eq!(decoded.amount, U256::from(FIXTURE_AMOUNT));
        assert_eq!(decoded.destination_domain, FIXTURE_DESTINATION_DOMAIN);
        assert_eq!(decoded.min_finality, FIXTURE_MIN_FINALITY);
    }

    #[tokio::test]
    async fn insufficient_confirmations_rejected() {
        use wiremock::matchers::{body_string_contains, method, path};

        let server = MockServer::start().await;
        let mut cfg = CctpConfig::default_testnet();
        cfg.sepolia_rpc_url = server.uri();
        let verifier = EvmRpcBurnVerifier::with_confirmations_for_test(&cfg, 3).unwrap();
        let tx_hash = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let mint_recipient =
            evm_address_to_bytes32("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0").unwrap();
        let data = ProductionEvmCctpBuilder::encode_deposit_for_burn(
            1_000_000,
            crate::cctp::config::STELLAR_TESTNET_DOMAIN,
            mint_recipient,
            crate::cctp::config::SEPOLIA_USDC,
            ANY_DESTINATION_CALLER,
            "1",
            crate::cctp::config::FINALITY_STANDARD,
        )
        .unwrap();
        let input = format!("0x{}", hex::encode(&data));

        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains("eth_getTransactionByHash"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": {
                    "from": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0",
                    "to": cfg.contracts.sepolia_token_messenger,
                    "input": input
                }
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(body_string_contains("eth_getTransactionReceipt"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": {
                    "status": "0x1",
                    "blockNumber": "0x10",
                    "logs": []
                }
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(body_string_contains("eth_chainId"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": "0xaa36a7"
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(body_string_contains("eth_blockNumber"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": "0x11"
            })))
            .mount(&server)
            .await;

        let err = verifier.verify_burn(tx_hash).await.unwrap_err();
        assert_eq!(
            err,
            VerifierError::Failed("insufficient confirmations".into())
        );
    }
}
