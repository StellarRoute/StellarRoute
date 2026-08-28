//! Production Sepolia ERC-20 allowance probe via bounded `eth_call`.

use alloy_primitives::{Address, U256};
use alloy_sol_types::{sol, SolCall};
use async_trait::async_trait;

use crate::cctp::builders::evm::EvmAllowanceChecker;
use crate::cctp::builders::BuilderError;
use crate::cctp::config::CctpConfig;
use crate::cctp::evm_rpc::EvmRpcClient;

sol! {
    interface IERC20Allowance {
        function allowance(address owner, address spender) external view returns (uint256);
    }
}

pub struct EvmRpcAllowanceChecker {
    rpc: EvmRpcClient,
    usdc: Address,
    token_messenger: Address,
}

impl EvmRpcAllowanceChecker {
    pub fn new(config: &CctpConfig) -> Result<Self, crate::cctp::verifiers::VerifierError> {
        let rpc = EvmRpcClient::new(&config.sepolia_rpc_url)?;
        let usdc =
            config.contracts.sepolia_usdc.trim().parse().map_err(|_| {
                crate::cctp::verifiers::VerifierError::Failed("usdc address".into())
            })?;
        let token_messenger = config
            .contracts
            .sepolia_token_messenger
            .trim()
            .parse()
            .map_err(|_| {
                crate::cctp::verifiers::VerifierError::Failed("token messenger address".into())
            })?;
        Ok(Self {
            rpc,
            usdc,
            token_messenger,
        })
    }

    pub fn is_ready(&self) -> bool {
        !self.rpc.rpc_url.trim().is_empty()
    }
}

#[async_trait]
impl EvmAllowanceChecker for EvmRpcAllowanceChecker {
    async fn has_sufficient_allowance(
        &self,
        owner: &str,
        token: &str,
        spender: &str,
        amount: &str,
    ) -> Result<bool, BuilderError> {
        if !self.is_ready() {
            return Err(BuilderError::NotReady);
        }
        let owner_addr: Address = owner
            .trim()
            .parse()
            .map_err(|_| BuilderError::Validation(format!("invalid owner: {owner}")))?;
        let token_addr: Address = token
            .trim()
            .parse()
            .map_err(|_| BuilderError::Validation(format!("invalid token: {token}")))?;
        let spender_addr: Address = spender
            .trim()
            .parse()
            .map_err(|_| BuilderError::Validation(format!("invalid spender: {spender}")))?;
        if token_addr != self.usdc || spender_addr != self.token_messenger {
            return Err(BuilderError::Validation("wrong token or spender".into()));
        }
        let required = crate::cctp::encoding::decimal_to_cctp_subunits(amount)
            .map_err(|e| BuilderError::Encoding(e.to_string()))?;

        self.rpc.ensure_chain().await.map_err(|e| match e {
            crate::cctp::verifiers::VerifierError::NotReady => BuilderError::NotReady,
            crate::cctp::verifiers::VerifierError::Transient(m) => BuilderError::AccountLookup(m),
            other => BuilderError::AccountLookup(other.to_string()),
        })?;

        let call = IERC20Allowance::allowanceCall {
            owner: owner_addr,
            spender: spender_addr,
        };
        let data = format!("0x{}", hex::encode(call.abi_encode()));
        let result = self
            .rpc
            .eth_call(&format!("{:#x}", self.usdc), &data, "latest")
            .await
            .map_err(|e| BuilderError::AccountLookup(e.to_string()))?;
        let hex = result.trim_start_matches("0x");
        if hex.is_empty() {
            return Ok(false);
        }
        let bytes =
            hex::decode(hex).map_err(|_| BuilderError::Encoding("allowance decode".into()))?;
        let allowance = U256::from_be_slice(&bytes);
        Ok(allowance >= U256::from(required))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn not_ready_without_rpc_url() {
        let mut cfg = CctpConfig::default_testnet();
        cfg.sepolia_rpc_url = String::new();
        assert!(matches!(
            EvmRpcAllowanceChecker::new(&cfg),
            Err(crate::cctp::verifiers::VerifierError::NotReady)
        ));
    }

    #[tokio::test]
    async fn parses_allowance_eth_call_response() {
        let server = MockServer::start().await;
        let mut cfg = CctpConfig::default_testnet();
        cfg.sepolia_rpc_url = server.uri();
        let checker = EvmRpcAllowanceChecker::new(&cfg).unwrap();

        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains("eth_chainId"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": "0xaa36a7"
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(body_string_contains("eth_call"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": "0x000000000000000000000000000000000000000000000000000000000f4240"
            })))
            .mount(&server)
            .await;

        let ok = checker
            .has_sufficient_allowance(
                "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0",
                &cfg.contracts.sepolia_usdc,
                &cfg.contracts.sepolia_token_messenger,
                "1.000000",
            )
            .await
            .unwrap();
        assert!(ok);
    }
}
