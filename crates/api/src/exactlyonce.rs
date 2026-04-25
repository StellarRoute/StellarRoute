//! Exactly-once quote request pipeline with dedupe ledger

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::instrument;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RequestIdentity {
    pub base_asset: String,
    pub quote_asset: String,
    pub amount: String, // Canonical decimal string
    pub slippage_bps: u32,
    pub quote_type: String,
}

impl RequestIdentity {
    pub fn canonical_key(&self) -> String {
        format!(
            "{}/{}:{}:{}:{}",
            self.base_asset, self.quote_asset, self.amount, self.slippage_bps, self.quote_type
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupeEntry {
    pub identity: RequestIdentity,
    pub response_bytes: Vec<u8>,
    pub created_at: u64,
    pub ttl_secs: u64,
}

impl DedupeEntry {
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now.saturating_sub(self.created_at) > self.ttl_secs
    }
}

#[derive(Error, Debug)]
pub enum ExactlyOnceError {
    #[error("Request not found in ledger")]
    NotFound,
    #[error("Ledger operation failed: {0}")]
    LedgerError(String),
}

pub struct DedupeLedger {
    entries: Arc<RwLock<HashMap<String, DedupeEntry>>>,
    cleanup_interval_secs: u64,
}

impl DedupeLedger {
    pub fn new(cleanup_interval_secs: u64) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            cleanup_interval_secs,
        }
    }

    #[instrument(skip(self, response_bytes))]
    pub async fn record(
        &self,
        identity: RequestIdentity,
        response_bytes: Vec<u8>,
        ttl_secs: u64,
    ) -> Result<(), ExactlyOnceError> {
        let key = identity.canonical_key();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let entry = DedupeEntry {
            identity: identity.clone(),
            response_bytes,
            created_at: now,
            ttl_secs,
        };

        let mut ledger = self.entries.write().await;
        ledger.insert(key, entry);

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn lookup(&self, identity: &RequestIdentity) -> Result<Vec<u8>, ExactlyOnceError> {
        let key = identity.canonical_key();
        let ledger = self.entries.read().await;

        if let Some(entry) = ledger.get(&key) {
            if entry.is_expired() {
                return Err(ExactlyOnceError::NotFound);
            }
            return Ok(entry.response_bytes.clone());
        }

        Err(ExactlyOnceError::NotFound)
    }

    pub async fn cleanup(&self) {
        let mut ledger = self.entries.write().await;
        ledger.retain(|_, entry| !entry.is_expired());
    }

    // For testing: spawn background cleanup task
    pub fn spawn_cleanup_task(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(self.cleanup_interval_secs))
                    .await;
                self.cleanup().await;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_exactly_once_semantics() {
        let ledger = Arc::new(DedupeLedger::new(10));
        let identity = RequestIdentity {
            base_asset: "XLM".to_string(),
            quote_asset: "USDC".to_string(),
            amount: "100.0000000".to_string(),
            slippage_bps: 100,
            quote_type: "buy".to_string(),
        };

        let response = b"test_response".to_vec();
        ledger
            .record(identity.clone(), response.clone(), 100)
            .await
            .unwrap();

        let retrieved = ledger.lookup(&identity).await.unwrap();
        assert_eq!(retrieved, response);

        // Second lookup returns identical response
        let retrieved2 = ledger.lookup(&identity).await.unwrap();
        assert_eq!(retrieved2, response);
    }

    #[tokio::test]
    async fn test_expired_entry_cleanup() {
        let ledger = DedupeLedger::new(1);
        let identity = RequestIdentity {
            base_asset: "XLM".to_string(),
            quote_asset: "BTC".to_string(),
            amount: "50.0000000".to_string(),
            slippage_bps: 50,
            quote_type: "sell".to_string(),
        };

        ledger
            .record(identity.clone(), b"response".to_vec(), 1)
            .await
            .unwrap();

        // Immediate lookup succeeds
        assert!(ledger.lookup(&identity).await.is_ok());

        // After expiration, cleanup removes it
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        ledger.cleanup().await;

        assert!(matches!(
            ledger.lookup(&identity).await,
            Err(ExactlyOnceError::NotFound)
        ));
    }

    #[test]
    fn test_canonical_key_determinism() {
        let id1 = RequestIdentity {
            base_asset: "XLM".to_string(),
            quote_asset: "USDC".to_string(),
            amount: "100.0000000".to_string(),
            slippage_bps: 100,
            quote_type: "buy".to_string(),
        };

        let id2 = RequestIdentity {
            base_asset: "XLM".to_string(),
            quote_asset: "USDC".to_string(),
            amount: "100.0000000".to_string(),
            slippage_bps: 100,
            quote_type: "buy".to_string(),
        };

        assert_eq!(id1.canonical_key(), id2.canonical_key());
    }
}
