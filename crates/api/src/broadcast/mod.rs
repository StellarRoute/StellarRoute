//! Transaction broadcast abstraction for swap submission.
//!
//! Distinguishes permanent Horizon result codes from transient transport failures.
//! Ambiguous timeout-after-accept must reconcile via [`TransactionBroadcaster::lookup`]
//! before any rebroadcast.

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BroadcastResult {
    pub tx_hash: String,
    /// `"pending"` when Horizon accepted the tx; `"success"` when included.
    pub status: String,
    pub ledger: Option<u64>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BroadcastError {
    #[error("validation: {0}")]
    Validation(String),
    /// Transport timeout — may be ambiguous if the request left the client.
    #[error("timeout")]
    Timeout,
    /// Transient network / 5xx failure before a definitive Horizon result.
    #[error("transient rpc error: {0}")]
    TransientRpc(String),
    #[error("insufficient fee")]
    InsufficientFee,
    #[error("insufficient balance")]
    InsufficientBalance,
    #[error("slippage exceeded (op_under_dest_min)")]
    SlippageExceeded,
    #[error("bad signature")]
    BadSignature,
    #[error("bad sequence")]
    BadSequence,
    #[error("malformed transaction")]
    Malformed,
    #[error("permanent horizon failure: {0}")]
    Permanent(String),
}

impl BroadcastError {
    pub fn metrics_class(&self) -> &'static str {
        match self {
            Self::Validation(_) => "validation",
            Self::Timeout => "timeout",
            Self::TransientRpc(_) => "rpc_error",
            Self::InsufficientFee => "insufficient_fee",
            Self::InsufficientBalance => "insufficient_balance",
            Self::SlippageExceeded => "slippage_exceeded",
            Self::BadSignature => "bad_signature",
            Self::BadSequence => "bad_sequence",
            Self::Malformed => "malformed",
            Self::Permanent(_) => "permanent",
        }
    }

    /// True only for pre-accept transport failures that may safely retry prepare/submit
    /// after reconciliation proves the tx is absent.
    pub fn is_transient_transport(&self) -> bool {
        matches!(self, Self::Timeout | Self::TransientRpc(_))
    }

    pub fn requires_fresh_prepare(&self) -> bool {
        matches!(
            self,
            Self::BadSequence
                | Self::BadSignature
                | Self::Malformed
                | Self::SlippageExceeded
                | Self::InsufficientFee
                | Self::InsufficientBalance
                | Self::Permanent(_)
                | Self::Validation(_)
        )
    }
}

/// Map Horizon transaction / operation result codes to typed errors.
pub fn map_horizon_result_codes(
    tx_code: Option<&str>,
    op_codes: Option<&[String]>,
) -> Option<BroadcastError> {
    if let Some(code) = tx_code {
        match code {
            "tx_insufficient_fee" => return Some(BroadcastError::InsufficientFee),
            "tx_bad_auth" | "tx_bad_auth_extra" => return Some(BroadcastError::BadSignature),
            "tx_bad_seq" => return Some(BroadcastError::BadSequence),
            "tx_malformed" => return Some(BroadcastError::Malformed),
            "tx_failed" => {
                // Inspect ops below.
            }
            "tx_too_late" | "tx_insufficient_balance" => {
                return Some(BroadcastError::Permanent(code.to_string()));
            }
            other if other.starts_with("tx_") => {
                // Unknown tx_* codes are permanent unless clearly retryable.
            }
            _ => {}
        }
    }
    if let Some(ops) = op_codes {
        for c in ops {
            if c.contains("under_dest_min") || c == "op_under_dest_min" {
                return Some(BroadcastError::SlippageExceeded);
            }
            if c.contains("underfunded") || c == "op_underfunded" {
                return Some(BroadcastError::InsufficientBalance);
            }
            if c == "op_malformed" || c.contains("malformed") {
                return Some(BroadcastError::Malformed);
            }
            if c.starts_with("op_") {
                return Some(BroadcastError::Permanent(c.clone()));
            }
        }
    }
    if tx_code == Some("tx_failed") {
        return Some(BroadcastError::Permanent("tx_failed".into()));
    }
    None
}

#[async_trait]
pub trait TransactionBroadcaster: Send + Sync {
    async fn submit(&self, signed_xdr: &str) -> Result<BroadcastResult, BroadcastError>;

    /// Look up a transaction by hash for timeout reconciliation.
    async fn lookup(&self, tx_hash: &str) -> Result<Option<BroadcastResult>, BroadcastError>;
}

#[derive(Clone)]
pub struct HorizonTransactionBroadcaster {
    client: Client,
    horizon_urls: Vec<String>,
}

impl HorizonTransactionBroadcaster {
    pub fn new(client: Client, horizon_urls: Vec<String>) -> Self {
        Self {
            client,
            horizon_urls,
        }
    }

    pub fn from_env() -> Self {
        // Path payments can take longer than a simple /health probe under load;
        // 10s was racing Horizon accept → false dependency_unavailable / stuck submitting.
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        let mut urls: Vec<String> = std::env::var("STELLAR_HORIZON_URL")
            .ok()
            .map(|u| u.trim().trim_end_matches('/').to_string())
            .into_iter()
            .filter(|u| !u.is_empty())
            .collect();

        if let Ok(extra) = std::env::var("STELLAR_HORIZON_FALLBACK_URLS") {
            for u in extra.split(',') {
                let u = u.trim().trim_end_matches('/').to_string();
                if !u.is_empty() {
                    urls.push(u);
                }
            }
        }

        if urls.is_empty() {
            urls.push("https://horizon-testnet.stellar.org".to_string());
        }

        Self::new(client, urls)
    }
}

#[derive(Debug, Deserialize)]
struct HorizonTxResponse {
    hash: String,
    #[serde(default)]
    successful: Option<bool>,
    ledger: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct HorizonErrorBody {
    #[serde(default)]
    detail: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    extras: Option<HorizonErrorExtras>,
}

#[derive(Debug, Deserialize)]
struct HorizonErrorExtras {
    #[serde(default)]
    result_codes: Option<HorizonResultCodes>,
}

#[derive(Debug, Deserialize)]
struct HorizonResultCodes {
    #[serde(default)]
    transaction: Option<String>,
    #[serde(default)]
    operations: Option<Vec<String>>,
}

#[async_trait]
impl TransactionBroadcaster for HorizonTransactionBroadcaster {
    async fn submit(&self, signed_xdr: &str) -> Result<BroadcastResult, BroadcastError> {
        if signed_xdr.trim().is_empty() {
            return Err(BroadcastError::Validation(
                "signed_xdr must be non-empty".to_string(),
            ));
        }

        let body = format!("tx={}", urlencoding::encode(signed_xdr.trim()));
        let mut last_err = BroadcastError::TransientRpc("no horizon URLs configured".to_string());

        for base in &self.horizon_urls {
            let url = format!("{base}/transactions");
            let response = match self
                .client
                .post(&url)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(body.clone())
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    if e.is_timeout() {
                        last_err = BroadcastError::Timeout;
                    } else {
                        last_err = BroadcastError::TransientRpc(e.to_string());
                    }
                    continue;
                }
            };

            if response.status().is_success() {
                let parsed: HorizonTxResponse = response.json().await.map_err(|e| {
                    BroadcastError::TransientRpc(format!("invalid horizon response: {e}"))
                })?;
                let status = if parsed.successful == Some(true) {
                    "success".to_string()
                } else {
                    "pending".to_string()
                };
                return Ok(BroadcastResult {
                    tx_hash: parsed.hash,
                    status,
                    ledger: parsed.ledger,
                });
            }

            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            if let Ok(err_body) = serde_json::from_str::<HorizonErrorBody>(&text) {
                if let Some(codes) = err_body.extras.and_then(|e| e.result_codes) {
                    if let Some(mapped) = map_horizon_result_codes(
                        codes.transaction.as_deref(),
                        codes.operations.as_deref(),
                    ) {
                        return Err(mapped);
                    }
                }
                let msg = err_body
                    .detail
                    .or(err_body.title)
                    .unwrap_or_else(|| format!("HTTP {status}"));
                if status.is_server_error() {
                    last_err = BroadcastError::TransientRpc(msg);
                } else {
                    return Err(BroadcastError::Permanent(msg));
                }
            } else if status.is_server_error() {
                last_err = BroadcastError::TransientRpc(format!("HTTP {status}: {text}"));
            } else {
                return Err(BroadcastError::Permanent(format!("HTTP {status}: {text}")));
            }
        }

        Err(last_err)
    }

    async fn lookup(&self, tx_hash: &str) -> Result<Option<BroadcastResult>, BroadcastError> {
        let mut last_err = BroadcastError::TransientRpc("no horizon URLs".into());
        for base in &self.horizon_urls {
            let url = format!("{base}/transactions/{tx_hash}");
            match self.client.get(&url).send().await {
                Ok(resp) if resp.status().as_u16() == 404 => return Ok(None),
                Ok(resp) if resp.status().is_success() => {
                    let parsed: HorizonTxResponse = resp
                        .json()
                        .await
                        .map_err(|e| BroadcastError::TransientRpc(e.to_string()))?;
                    let status = if parsed.successful == Some(true) {
                        "success".to_string()
                    } else {
                        "pending".to_string()
                    };
                    return Ok(Some(BroadcastResult {
                        tx_hash: parsed.hash,
                        status,
                        ledger: parsed.ledger,
                    }));
                }
                Ok(resp) if resp.status().is_server_error() => {
                    last_err = BroadcastError::TransientRpc(format!("HTTP {}", resp.status()));
                }
                Ok(resp) => {
                    return Err(BroadcastError::Permanent(format!(
                        "lookup HTTP {}",
                        resp.status()
                    )));
                }
                Err(e) if e.is_timeout() => last_err = BroadcastError::Timeout,
                Err(e) => last_err = BroadcastError::TransientRpc(e.to_string()),
            }
        }
        Err(last_err)
    }
}

pub struct MockTransactionBroadcaster {
    pub result: std::sync::Mutex<Option<Result<BroadcastResult, BroadcastError>>>,
    pub calls: std::sync::Mutex<Vec<String>>,
    pub lookup: std::sync::Mutex<Option<Option<BroadcastResult>>>,
}

impl Default for MockTransactionBroadcaster {
    fn default() -> Self {
        Self {
            result: std::sync::Mutex::new(None),
            calls: std::sync::Mutex::new(Vec::new()),
            lookup: std::sync::Mutex::new(None),
        }
    }
}

impl MockTransactionBroadcaster {
    /// Successful accept. Pass an empty `tx_hash` to let the submit path keep the
    /// cryptographically bound hash; non-empty values must match that hash.
    pub fn succeed(tx_hash: impl Into<String>) -> Self {
        Self {
            result: std::sync::Mutex::new(Some(Ok(BroadcastResult {
                tx_hash: tx_hash.into(),
                status: "pending".to_string(),
                ledger: None,
            }))),
            calls: std::sync::Mutex::new(Vec::new()),
            lookup: std::sync::Mutex::new(None),
        }
    }

    pub fn fail(err: BroadcastError) -> Self {
        Self {
            result: std::sync::Mutex::new(Some(Err(err))),
            calls: std::sync::Mutex::new(Vec::new()),
            lookup: std::sync::Mutex::new(None),
        }
    }

    pub fn with_lookup(self, found: Option<BroadcastResult>) -> Self {
        *self.lookup.lock().unwrap() = Some(found);
        self
    }

    pub fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}

#[async_trait]
impl TransactionBroadcaster for MockTransactionBroadcaster {
    async fn submit(&self, signed_xdr: &str) -> Result<BroadcastResult, BroadcastError> {
        self.calls.lock().unwrap().push(signed_xdr.to_string());
        let mut guard = self.result.lock().unwrap();
        if let Some(result) = guard.take() {
            return result;
        }
        Ok(BroadcastResult {
            tx_hash: "mock-tx-hash".to_string(),
            status: "pending".to_string(),
            ledger: None,
        })
    }

    async fn lookup(&self, _tx_hash: &str) -> Result<Option<BroadcastResult>, BroadcastError> {
        Ok(self.lookup.lock().unwrap().clone().unwrap_or(None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_code_matrix() {
        assert!(matches!(
            map_horizon_result_codes(Some("tx_bad_seq"), None),
            Some(BroadcastError::BadSequence)
        ));
        assert!(matches!(
            map_horizon_result_codes(Some("tx_bad_auth"), None),
            Some(BroadcastError::BadSignature)
        ));
        assert!(matches!(
            map_horizon_result_codes(Some("tx_malformed"), None),
            Some(BroadcastError::Malformed)
        ));
        assert!(matches!(
            map_horizon_result_codes(
                Some("tx_failed"),
                Some(&["op_under_dest_min".to_string()][..])
            ),
            Some(BroadcastError::SlippageExceeded)
        ));
        assert!(matches!(
            map_horizon_result_codes(Some("tx_insufficient_fee"), None),
            Some(BroadcastError::InsufficientFee)
        ));
        assert!(BroadcastError::Timeout.is_transient_transport());
        assert!(BroadcastError::TransientRpc("x".into()).is_transient_transport());
        assert!(!BroadcastError::SlippageExceeded.is_transient_transport());
        assert!(BroadcastError::BadSequence.requires_fresh_prepare());
    }
}
