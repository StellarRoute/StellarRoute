//! Authoritative Stellar account sequence lookup for CCTP builders.
//!
//! Prefers Soroban RPC `getLedgerEntries` (account ledger key); falls back to configured
//! Horizon when RPC entry lookup fails. Never accepts client-supplied sequence.

use async_trait::async_trait;
use std::sync::Arc;

use crate::cctp::config::{parse_service_url, CctpConfig};
use crate::cctp::stellar_rpc::StellarRpcClient;
use crate::swap::tx::{AccountSequenceSource, HorizonAccountSequences, TxBuildError};

fn enforce_horizon_testnet_host(host: &str) -> Result<(), TxBuildError> {
    if host != "horizon-testnet.stellar.org" {
        return Err(TxBuildError::AccountLookup(
            "horizon host must match configured testnet endpoint".into(),
        ));
    }
    Ok(())
}

fn validated_horizon_base(config: &CctpConfig) -> Result<String, TxBuildError> {
    let raw = if config.stellar_horizon_url.trim().is_empty() {
        "https://horizon-testnet.stellar.org".to_string()
    } else {
        config
            .stellar_horizon_url
            .trim()
            .trim_end_matches('/')
            .to_string()
    };
    let parsed = parse_service_url(&raw).map_err(|e| TxBuildError::AccountLookup(e.to_string()))?;
    if !cfg!(test) && parsed.scheme != "https" {
        return Err(TxBuildError::AccountLookup(
            "horizon must be https outside loopback".into(),
        ));
    }
    enforce_horizon_testnet_host(&parsed.host)?;
    Ok(raw)
}

pub struct RpcAccountSequenceSource {
    rpc: Arc<StellarRpcClient>,
    horizon: HorizonAccountSequences,
}

impl RpcAccountSequenceSource {
    pub fn new(config: &CctpConfig, rpc: Arc<StellarRpcClient>) -> Self {
        let horizon_urls = match validated_horizon_base(config) {
            Ok(url) => vec![url],
            Err(_) if cfg!(test) => vec!["https://horizon-testnet.stellar.org".to_string()],
            Err(_) => Vec::new(),
        };
        let horizon = HorizonAccountSequences::new(
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap_or_default(),
            horizon_urls,
        );
        Self { rpc, horizon }
    }
}

#[async_trait]
impl AccountSequenceSource for RpcAccountSequenceSource {
    async fn current_sequence(&self, account_id: &str) -> Result<i64, TxBuildError> {
        let rpc_result = self.rpc.get_account_sequence(account_id).await;
        let horizon_result = self.horizon.current_sequence(account_id).await;
        match (rpc_result, horizon_result) {
            (Ok(rpc), Ok(horizon)) => Ok(rpc.max(horizon)),
            (Ok(rpc), Err(_)) => Ok(rpc),
            (Err(_), Ok(horizon)) => Ok(horizon),
            (Err(_), Err(e)) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cctp::config::CctpConfig;

    #[test]
    fn rejects_non_testnet_horizon_host_in_production_policy() {
        assert!(enforce_horizon_testnet_host("horizon.stellar.org").is_err());
        assert!(enforce_horizon_testnet_host("horizon-testnet.stellar.org").is_ok());
    }

    #[tokio::test]
    #[ignore = "live Horizon network lookup"]
    async fn rpc_sequence_falls_back_to_horizon() {
        let mut cfg = CctpConfig::default_testnet();
        cfg.stellar_rpc_url = "http://127.0.0.1:1".into();
        cfg.stellar_horizon_url = "".into();
        let rpc = Arc::new(StellarRpcClient::new(&cfg).unwrap());
        let source = RpcAccountSequenceSource::new(&cfg, rpc);
        let err = source
            .current_sequence("GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF")
            .await
            .unwrap_err();
        assert!(matches!(err, TxBuildError::AccountLookup(_)));
    }
}
