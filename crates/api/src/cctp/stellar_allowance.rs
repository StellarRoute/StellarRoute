//! Production Stellar Testnet SEP-41 allowance probe via bounded Soroban simulation.

use async_trait::async_trait;
use std::sync::Arc;

use crate::cctp::builders::stellar::StellarAllowanceChecker;
use crate::cctp::builders::BuilderError;
use crate::cctp::config::CctpConfig;
use crate::cctp::stellar_readiness_probes::simulate_allowance;
use crate::cctp::stellar_rpc::StellarRpcClient;

pub struct StellarRpcAllowanceChecker {
    rpc: Arc<StellarRpcClient>,
    usdc: String,
    token_messenger: String,
    probe_ok: bool,
}

impl StellarRpcAllowanceChecker {
    pub async fn new(config: &CctpConfig) -> Result<Self, crate::cctp::verifiers::VerifierError> {
        if config.stellar_rpc_url.trim().is_empty() {
            return Err(crate::cctp::verifiers::VerifierError::NotReady);
        }
        let rpc = Arc::new(StellarRpcClient::new(config)?);
        let probe_ok = rpc.latest_ledger().await.is_ok();
        Ok(Self {
            rpc,
            usdc: config.contracts.stellar_usdc.clone(),
            token_messenger: config.contracts.stellar_token_messenger.clone(),
            probe_ok,
        })
    }

    pub fn is_ready(&self) -> bool {
        self.probe_ok && self.rpc.is_ready()
    }
}

#[async_trait]
impl StellarAllowanceChecker for StellarRpcAllowanceChecker {
    async fn has_sufficient_allowance(
        &self,
        owner: &str,
        token: &str,
        spender: &str,
        amount: i128,
    ) -> Result<bool, BuilderError> {
        if !self.is_ready() {
            return Err(BuilderError::NotReady);
        }
        if token != self.usdc || spender != self.token_messenger {
            return Err(BuilderError::Validation("wrong token or spender".into()));
        }
        let allowance = simulate_allowance(&self.rpc, token, owner, spender)
            .await
            .map_err(|e| BuilderError::AccountLookup(e.to_string()))?;
        Ok(allowance >= amount)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_string_contains, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn not_ready_without_rpc_url() {
        let mut cfg = CctpConfig::default_testnet();
        cfg.stellar_rpc_url = String::new();
        assert!(matches!(
            StellarRpcAllowanceChecker::new(&cfg).await,
            Err(crate::cctp::verifiers::VerifierError::NotReady)
        ));
    }

    #[tokio::test]
    async fn rejects_wrong_token_contract() {
        let server = MockServer::start().await;
        let mut cfg = CctpConfig::default_testnet();
        cfg.stellar_rpc_url = server.uri();
        Mock::given(method("POST"))
            .and(body_string_contains("getLatestLedger"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0", "id": 1, "result": { "sequence": 100 }
            })))
            .mount(&server)
            .await;
        let checker = StellarRpcAllowanceChecker::new(&cfg).await.unwrap();
        let err = checker
            .has_sufficient_allowance(
                "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                "CWRONG",
                &cfg.contracts.stellar_token_messenger,
                1,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, BuilderError::Validation(_)));
    }
}
