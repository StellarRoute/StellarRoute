//! Shared bounded JSON-RPC client for Sepolia EVM CCTP components.

use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;

use crate::cctp::builders::evm::SEPOLIA_CHAIN_ID_NUM;
use crate::cctp::verifiers::VerifierError;

pub const MAX_JSON_BODY_BYTES: usize = 256 * 1024;

#[derive(Debug, Deserialize)]
struct RpcResponse<T> {
    result: Option<T>,
    error: Option<RpcError>,
}

#[derive(Debug, Deserialize)]
struct RpcError {
    message: String,
}

#[derive(Debug, Clone)]
pub struct EvmRpcClient {
    pub client: Client,
    pub rpc_url: String,
    pub chain_id: u64,
}

impl EvmRpcClient {
    pub fn new(rpc_url: &str) -> Result<Self, VerifierError> {
        if rpc_url.trim().is_empty() {
            return Err(VerifierError::NotReady);
        }
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .map_err(|e| VerifierError::Transient(e.to_string()))?,
            rpc_url: rpc_url.to_string(),
            chain_id: SEPOLIA_CHAIN_ID_NUM,
        })
    }

    pub fn is_ready(&self) -> bool {
        false
    }

    pub fn bound_body(body: &str) -> Result<(), VerifierError> {
        if body.len() > MAX_JSON_BODY_BYTES {
            return Err(VerifierError::Failed("rpc response too large".into()));
        }
        Ok(())
    }

    pub async fn call<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        params: Value,
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

    pub async fn eth_call(
        &self,
        to: &str,
        data: &str,
        block: &str,
    ) -> Result<String, VerifierError> {
        #[derive(Deserialize)]
        struct CallResult(String);
        let result: CallResult = self
            .call(
                "eth_call",
                json!([{
                    "to": to,
                    "data": data,
                }, block]),
            )
            .await?;
        Ok(result.0)
    }

    pub async fn chain_id(&self) -> Result<u64, VerifierError> {
        let chain_id_resp: String = self.call("eth_chainId", json!([])).await?;
        u64::from_str_radix(chain_id_resp.trim_start_matches("0x"), 16)
            .map_err(|_| VerifierError::Failed("chain id parse".into()))
    }

    pub async fn get_code(&self, address: &str) -> Result<String, VerifierError> {
        self.call("eth_getCode", json!([address, "latest"])).await
    }

    pub async fn ensure_chain(&self) -> Result<(), VerifierError> {
        let parsed = self.chain_id().await?;
        if parsed != self.chain_id {
            return Err(VerifierError::Failed("wrong chain".into()));
        }
        Ok(())
    }

    pub fn normalize_hash(hash: &str) -> String {
        let trimmed = hash.trim();
        let hex = trimmed
            .strip_prefix("0x")
            .or_else(|| trimmed.strip_prefix("0X"))
            .unwrap_or(trimmed);
        format!("0x{}", hex.to_ascii_lowercase())
    }
}
