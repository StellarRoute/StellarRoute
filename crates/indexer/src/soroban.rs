//! Soroban RPC client support for AMM pool indexing.
//!
//! This module provides:
//! - a client abstraction (`SorobanRpc`) for dependency injection and testing
//! - configurable retry/backoff and timeout policies
//! - built-in endpoint presets for Stellar testnet and pubnet

use crate::error::{IndexerError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tracing::debug;

pub const TESTNET_SOROBAN_RPC_URL: &str = "https://soroban-testnet.stellar.org";
pub const PUBNET_SOROBAN_RPC_URL: &str = "https://soroban-rpc.stellar.org";

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum StellarNetwork {
    Testnet,
    Pubnet,
}

impl StellarNetwork {
    pub fn rpc_url(self) -> &'static str {
        match self {
            Self::Testnet => TESTNET_SOROBAN_RPC_URL,
            Self::Pubnet => PUBNET_SOROBAN_RPC_URL,
        }
    }
}

/// Retry policy for Soroban RPC calls.
#[derive(Clone, Debug)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
        }
    }
}

/// Runtime configuration for Soroban RPC client behavior.
#[derive(Clone, Debug)]
pub struct SorobanRpcConfig {
    pub base_url: String,
    pub timeout_secs: u64,
    pub retry: RetryPolicy,
}

impl SorobanRpcConfig {
    pub fn for_network(network: StellarNetwork) -> Self {
        Self {
            base_url: network.rpc_url().to_string(),
            timeout_secs: 30,
            retry: RetryPolicy::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

#[derive(Debug, Serialize)]
struct JsonRpcRequest<'a> {
    jsonrpc: &'static str,
    id: &'a str,
    method: &'a str,
    params: serde_json::Value,
}

/// Narrow abstraction so indexing logic can be tested with a mock client.
#[async_trait]
pub trait SorobanRpc: Send + Sync {
    async fn get_latest_ledger(&self) -> Result<u64>;
    async fn get_pool_state(&self, contract_id: &str) -> Result<serde_json::Value>;
    async fn get_events(
        &self,
        start_ledger: u64,
        end_ledger: Option<u64>,
        filters: Vec<EventFilter>,
    ) -> Result<Vec<SorobanEvent>>;
    async fn request(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value>;
}

#[derive(Debug, Serialize, Clone)]
pub struct EventFilter {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(rename = "contractIds")]
    pub contract_ids: Vec<String>,
    pub topics: Vec<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SorobanEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(rename = "ledger")]
    pub ledger: u64,
    #[serde(rename = "ledgerClosedAt")]
    pub ledger_closed_at: String,
    #[serde(rename = "contractId")]
    pub contract_id: String,
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "pagingToken")]
    pub paging_token: String,
    #[serde(rename = "topic")]
    pub topics: Vec<String>,
    #[serde(rename = "value")]
    pub value: SorobanEventValue,
    #[serde(rename = "inSuccessfulContractCall")]
    pub in_successful_contract_call: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SorobanEventValue {
    pub xdr: String,
}

#[derive(Clone)]
pub struct SorobanRpcClient {
    config: SorobanRpcConfig,
    http: reqwest::Client,
}

impl SorobanRpcClient {
    pub fn new(config: SorobanRpcConfig) -> Result<Self> {
        let base_url = config.base_url.trim_end_matches('/').to_string();
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| IndexerError::SorobanRpc(format!("failed to create HTTP client: {e}")))?;

        Ok(Self {
            config: SorobanRpcConfig { base_url, ..config },
            http,
        })
    }

    pub fn for_network(network: StellarNetwork) -> Result<Self> {
        Self::new(SorobanRpcConfig::for_network(network))
    }

    pub fn config(&self) -> &SorobanRpcConfig {
        &self.config
    }

    async fn retry_request<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut attempt = 0;
        let mut delay_ms = self.config.retry.initial_delay_ms;

        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(err) => {
                    attempt += 1;
                    if !err.is_retryable() || attempt > self.config.retry.max_retries {
                        return Err(err);
                    }

                    debug!(
                        "Soroban RPC request failed (attempt {}/{}), retrying in {}ms: {}",
                        attempt, self.config.retry.max_retries, delay_ms, err
                    );

                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    delay_ms = ((delay_ms as f64) * self.config.retry.backoff_multiplier) as u64;
                    delay_ms = delay_ms.min(self.config.retry.max_delay_ms);
                }
            }
        }
    }
}

#[async_trait]
impl SorobanRpc for SorobanRpcClient {
    async fn get_latest_ledger(&self) -> Result<u64> {
        let result = self.request("getLatestLedger", json!({})).await?;
        result
            .get("sequence")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| {
                IndexerError::SorobanRpc(
                    "missing `sequence` in getLatestLedger response".to_string(),
                )
            })
    }

    async fn get_pool_state(&self, contract_id: &str) -> Result<serde_json::Value> {
        self.request(
            "getContractData",
            json!({
                "contractId": contract_id,
            }),
        )
        .await
    }

    async fn get_events(
        &self,
        start_ledger: u64,
        end_ledger: Option<u64>,
        filters: Vec<EventFilter>,
    ) -> Result<Vec<SorobanEvent>> {
        let mut params = json!({
            "startLedger": start_ledger,
            "filters": filters,
        });

        if let Some(end) = end_ledger {
            params["endLedger"] = json!(end);
        }

        let result = self.request("getEvents", params).await?;

        // Handle paginated response if necessary, but for simplicity we assume it fits
        let events: Vec<SorobanEvent> =
            serde_json::from_value(result.get("events").cloned().unwrap_or(json!([])))
                .map_err(|e| IndexerError::SorobanRpc(format!("failed to parse events: {e}")))?;

        Ok(events)
    }

    async fn request(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let url = self.config.base_url.clone();
        let http = self.http.clone();
        let body = JsonRpcRequest {
            jsonrpc: "2.0",
            id: "stellarroute-indexer",
            method,
            params,
        };

        self.retry_request(|| {
            let url = url.clone();
            let http = http.clone();
            let body = JsonRpcRequest {
                jsonrpc: body.jsonrpc,
                id: body.id,
                method: body.method,
                params: body.params.clone(),
            };
            async move {
                let resp = http.post(&url).json(&body).send().await?;
                let status = resp.status();
                if !status.is_success() {
                    let error_body = resp.text().await.unwrap_or_default();
                    return Err(IndexerError::StellarApi {
                        endpoint: url,
                        status: status.as_u16(),
                        message: error_body,
                    });
                }

                let payload: JsonRpcResponse = resp.json().await.map_err(|e| {
                    IndexerError::SorobanRpc(format!("invalid JSON-RPC response: {e}"))
                })?;

                if let Some(err) = payload.error {
                    return Err(IndexerError::SorobanRpc(format!(
                        "JSON-RPC error {}: {}",
                        err.code, err.message
                    )));
                }

                payload.result.ok_or_else(|| {
                    IndexerError::SorobanRpc("missing JSON-RPC `result`".to_string())
                })
            }
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_partial_json, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn endpoint_presets_cover_testnet_and_pubnet() {
        assert_eq!(
            StellarNetwork::Testnet.rpc_url(),
            "https://soroban-testnet.stellar.org"
        );
        assert_eq!(
            StellarNetwork::Pubnet.rpc_url(),
            "https://soroban-rpc.stellar.org"
        );
    }

    #[test]
    fn default_config_for_network_uses_retry_and_timeout() {
        let cfg = SorobanRpcConfig::for_network(StellarNetwork::Testnet);
        assert_eq!(cfg.base_url, TESTNET_SOROBAN_RPC_URL);
        assert_eq!(cfg.timeout_secs, 30);
        assert_eq!(cfg.retry.max_retries, 3);
    }

    #[tokio::test]
    async fn request_success_returns_result() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(body_partial_json(json!({"method":"getLatestLedger"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc":"2.0",
                "id":"stellarroute-indexer",
                "result":{"sequence":12345}
            })))
            .mount(&server)
            .await;

        let client = SorobanRpcClient::new(SorobanRpcConfig {
            base_url: server.uri(),
            timeout_secs: 5,
            retry: RetryPolicy {
                max_retries: 0,
                initial_delay_ms: 0,
                max_delay_ms: 0,
                backoff_multiplier: 1.0,
            },
        })
        .unwrap();

        let seq = client.get_latest_ledger().await.unwrap();
        assert_eq!(seq, 12345);
    }

    #[tokio::test]
    async fn request_returns_json_rpc_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc":"2.0",
                "id":"stellarroute-indexer",
                "error":{"code":-32602,"message":"invalid params"}
            })))
            .mount(&server)
            .await;

        let client = SorobanRpcClient::new(SorobanRpcConfig {
            base_url: server.uri(),
            timeout_secs: 5,
            retry: RetryPolicy {
                max_retries: 0,
                initial_delay_ms: 0,
                max_delay_ms: 0,
                backoff_multiplier: 1.0,
            },
        })
        .unwrap();

        let err = client
            .request("getLatestLedger", json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, IndexerError::SorobanRpc(_)));
    }

    #[tokio::test]
    async fn request_retries_transient_http_failures() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(500).set_body_string("temporary failure"))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc":"2.0",
                "id":"stellarroute-indexer",
                "result":{"xdr":"AAAABBBB"}
            })))
            .mount(&server)
            .await;

        let client = SorobanRpcClient::new(SorobanRpcConfig {
            base_url: server.uri(),
            timeout_secs: 5,
            retry: RetryPolicy {
                max_retries: 2,
                initial_delay_ms: 1,
                max_delay_ms: 2,
                backoff_multiplier: 1.0,
            },
        })
        .unwrap();

        let pool = client.get_pool_state("CDUMMYPOOL").await.unwrap();
        assert_eq!(
            pool.get("xdr").and_then(serde_json::Value::as_str),
            Some("AAAABBBB")
        );
    }

    #[tokio::test]
    async fn request_exhausts_retries_on_http_failure() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(503).set_body_string("unavailable"))
            .mount(&server)
            .await;

        let client = SorobanRpcClient::new(SorobanRpcConfig {
            base_url: server.uri(),
            timeout_secs: 5,
            retry: RetryPolicy {
                max_retries: 1,
                initial_delay_ms: 1,
                max_delay_ms: 1,
                backoff_multiplier: 1.0,
            },
        })
        .unwrap();

        let err = client.get_pool_state("CDUMMYPOOL").await.unwrap_err();
        assert!(matches!(err, IndexerError::StellarApi { status: 503, .. }));
    }
}
