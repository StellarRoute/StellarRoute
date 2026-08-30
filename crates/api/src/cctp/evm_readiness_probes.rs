//! Non-mutating Sepolia EVM CCTP contract readiness probes.

use alloy_primitives::Address;
use alloy_sol_types::{sol, SolCall};

use crate::cctp::builders::evm::SEPOLIA_CHAIN_ID_NUM;
use crate::cctp::config::{CctpConfig, SEPOLIA_USDC_DECIMALS};
use crate::cctp::evm_rpc::EvmRpcClient;
use crate::cctp::rpc_env;
use crate::cctp::verifiers::VerifierError;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EvmContractProbeResult {
    pub rpc_ok: bool,
    pub chain_id_ok: bool,
    pub token_messenger_ok: bool,
    pub message_transmitter_ok: bool,
    pub usdc_ok: bool,
    pub linkage_ok: bool,
}

impl EvmContractProbeResult {
    pub fn all_ok(&self) -> bool {
        self.rpc_ok
            && self.chain_id_ok
            && self.token_messenger_ok
            && self.message_transmitter_ok
            && self.usdc_ok
            && self.linkage_ok
    }
}

sol! {
    interface IProbePausable {
        function paused() external view returns (bool);
    }
    interface ITokenMessengerProbe {
        function localMessageTransmitter() external view returns (address);
        function paused() external view returns (bool);
    }
    interface IERC20Meta {
        function decimals() external view returns (uint8);
    }
}

fn parse_address(configured: &str) -> Result<Address, VerifierError> {
    configured
        .trim()
        .parse()
        .map_err(|_| VerifierError::Failed("invalid evm address".into()))
}

fn decode_bool_result(hex: &str) -> Result<bool, VerifierError> {
    let stripped = hex.trim().strip_prefix("0x").unwrap_or(hex.trim());
    if stripped.is_empty() {
        return Err(VerifierError::Failed("empty eth_call result".into()));
    }
    let bytes = hex::decode(stripped).map_err(|_| VerifierError::Failed("bool decode".into()))?;
    if bytes.len() != 32 {
        return Err(VerifierError::Failed("bool width".into()));
    }
    Ok(bytes[31] == 1)
}

fn decode_address_result(hex: &str) -> Result<Address, VerifierError> {
    let stripped = hex.trim().strip_prefix("0x").unwrap_or(hex.trim());
    let bytes =
        hex::decode(stripped).map_err(|_| VerifierError::Failed("address decode".into()))?;
    if bytes.len() != 32 {
        return Err(VerifierError::Failed("address width".into()));
    }
    Ok(Address::from_slice(&bytes[12..32]))
}

fn decode_u8_result(hex: &str) -> Result<u8, VerifierError> {
    let stripped = hex.trim().strip_prefix("0x").unwrap_or(hex.trim());
    let bytes = hex::decode(stripped).map_err(|_| VerifierError::Failed("u8 decode".into()))?;
    if bytes.len() != 32 {
        return Err(VerifierError::Failed("u8 width".into()));
    }
    Ok(bytes[31])
}

async fn contract_has_code(rpc: &EvmRpcClient, address: &str) -> Result<bool, VerifierError> {
    let code: String = rpc
        .call("eth_getCode", serde_json::json!([address, "latest"]))
        .await?;
    let stripped = code.trim();
    Ok(!stripped.is_empty() && stripped != "0x" && stripped != "0x0")
}

async fn probe_paused(rpc: &EvmRpcClient, address: &str) -> Result<(), VerifierError> {
    let call = IProbePausable::pausedCall {};
    let data = format!("0x{}", hex::encode(call.abi_encode()));
    let result = rpc.eth_call(address, &data, "latest").await?;
    if decode_bool_result(&result)? {
        return Err(VerifierError::Failed("contract paused".into()));
    }
    Ok(())
}

async fn probe_usdc_decimals(rpc: &EvmRpcClient, usdc: &str) -> Result<(), VerifierError> {
    let call = IERC20Meta::decimalsCall {};
    let data = format!("0x{}", hex::encode(call.abi_encode()));
    let result = rpc.eth_call(usdc, &data, "latest").await?;
    let decimals = decode_u8_result(&result)?;
    if u32::from(decimals) != SEPOLIA_USDC_DECIMALS {
        return Err(VerifierError::Failed("usdc decimals mismatch".into()));
    }
    Ok(())
}

async fn probe_token_messenger_linkage(
    rpc: &EvmRpcClient,
    token_messenger: &str,
    expected_transmitter: &str,
) -> Result<(), VerifierError> {
    let call = ITokenMessengerProbe::localMessageTransmitterCall {};
    let data = format!("0x{}", hex::encode(call.abi_encode()));
    let result = rpc.eth_call(token_messenger, &data, "latest").await?;
    let linked = decode_address_result(&result)?;
    let expected = parse_address(expected_transmitter)?;
    if linked != expected {
        return Err(VerifierError::Failed("message transmitter linkage".into()));
    }
    Ok(())
}

pub async fn probe_sepolia_contracts(config: &CctpConfig, rpc_url: &str) -> EvmContractProbeResult {
    let mut out = EvmContractProbeResult::default();
    let Ok(rpc) = EvmRpcClient::new(rpc_url) else {
        return out;
    };
    out.rpc_ok = true;
    if rpc.chain_id().await.ok() != Some(SEPOLIA_CHAIN_ID_NUM) {
        return out;
    }
    out.chain_id_ok = true;

    let tm = &config.contracts.sepolia_token_messenger;
    let mt = &config.contracts.sepolia_message_transmitter;
    let usdc = &config.contracts.sepolia_usdc;

    if contract_has_code(&rpc, tm).await.unwrap_or(false) {
        out.token_messenger_ok = true;
    }
    if contract_has_code(&rpc, mt).await.unwrap_or(false) && probe_paused(&rpc, mt).await.is_ok() {
        out.message_transmitter_ok = true;
    }
    if contract_has_code(&rpc, usdc).await.unwrap_or(false)
        && probe_usdc_decimals(&rpc, usdc).await.is_ok()
    {
        out.usdc_ok = true;
    }
    if out.token_messenger_ok && probe_token_messenger_linkage(&rpc, tm, mt).await.is_ok() {
        out.linkage_ok = true;
    }
    out
}

/// Try primary + fallback Sepolia RPC URLs until a full probe succeeds.
pub async fn probe_sepolia_with_failover(config: &CctpConfig) -> EvmContractProbeResult {
    let urls = rpc_env::resolve_sepolia_rpc_urls().unwrap_or_default();
    let mut last = EvmContractProbeResult::default();
    for url in urls {
        let result = probe_sepolia_contracts(config, &url).await;
        if result.all_ok() {
            return result;
        }
        last = result;
    }
    last
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{body_string_contains, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_config(rpc_url: &str) -> CctpConfig {
        let mut cfg = CctpConfig::default_testnet();
        cfg.sepolia_rpc_url = rpc_url.to_string();
        cfg
    }

    #[tokio::test]
    async fn rejects_wrong_chain_id() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(body_string_contains("eth_chainId"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0", "id": 1, "result": "0x1"
            })))
            .mount(&server)
            .await;
        let cfg = test_config(&server.uri());
        let out = probe_sepolia_contracts(&cfg, &server.uri()).await;
        assert!(out.rpc_ok);
        assert!(!out.chain_id_ok);
        assert!(!out.all_ok());
    }

    #[tokio::test]
    async fn rejects_empty_contract_code() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(body_string_contains("eth_chainId"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0", "id": 1, "result": "0xaa36a7"
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(body_string_contains("eth_getCode"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0", "id": 1, "result": "0x"
            })))
            .expect(3)
            .mount(&server)
            .await;
        let cfg = test_config(&server.uri());
        let out = probe_sepolia_contracts(&cfg, &server.uri()).await;
        assert!(out.chain_id_ok);
        assert!(!out.token_messenger_ok);
        assert!(!out.all_ok());
    }
}
