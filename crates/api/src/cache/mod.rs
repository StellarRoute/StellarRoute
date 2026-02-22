//! Redis caching layer

use redis::{aio::ConnectionManager, AsyncCommands, RedisError};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;
use tracing::{debug, warn};

/// Cache manager for Redis operations
#[derive(Clone)]
pub struct CacheManager {
    client: ConnectionManager,
}

impl CacheManager {
    /// Create a new cache manager
    pub async fn new(redis_url: &str) -> Result<Self, RedisError> {
        let client = redis::Client::open(redis_url)?;
        let conn = ConnectionManager::new(client).await?;

        debug!("Redis cache manager initialized");
        Ok(Self { client: conn })
    }

    /// Get a cached value
    pub async fn get<T: DeserializeOwned>(&mut self, key: &str) -> Option<T> {
        match self.client.get::<_, String>(key).await {
            Ok(json) => match serde_json::from_str(&json) {
                Ok(value) => {
                    debug!("Cache hit for key: {}", key);
                    Some(value)
                }
                Err(e) => {
                    warn!("Failed to deserialize cached value for {}: {}", key, e);
                    None
                }
            },
            Err(_) => {
                debug!("Cache miss for key: {}", key);
                None
            }
        }
    }

    /// Set a cached value with TTL
    pub async fn set<T: Serialize>(
        &mut self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> Result<(), RedisError> {
        let json = serde_json::to_string(value).map_err(|e| {
            RedisError::from((
                redis::ErrorKind::TypeError,
                "serialization error",
                e.to_string(),
            ))
        })?;

        self.client
            .set_ex::<_, _, ()>(key, json, ttl.as_secs())
            .await?;

        debug!("Cached key: {} with TTL: {:?}", key, ttl);
        Ok(())
    }

    /// Delete a cached value
    pub async fn delete(&mut self, key: &str) -> Result<(), RedisError> {
        self.client.del::<_, ()>(key).await?;
        debug!("Deleted cache key: {}", key);
        Ok(())
    }

    /// Check if cache is healthy
    pub async fn is_healthy(&mut self) -> bool {
        self.client
            .get::<_, Option<String>>("_health")
            .await
            .is_ok()
    }
}

/// Cache key builders
pub mod keys {
    /// Cache key for trading pairs list
    pub fn pairs_list() -> String {
        "pairs:list".to_string()
    }

    /// Cache key for orderbook
    pub fn orderbook(base: &str, quote: &str) -> String {
        format!("orderbook:{}:{}", base, quote)
    }

    /// Cache key for quote
    pub fn quote(base: &str, quote: &str, amount: &str) -> String {
        format!("quote:{}:{}:{}", base, quote, amount)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_keys() {
        assert_eq!(keys::pairs_list(), "pairs:list");
        assert_eq!(keys::orderbook("XLM", "USDC"), "orderbook:XLM:USDC");
        assert_eq!(keys::quote("XLM", "USDC", "100"), "quote:XLM:USDC:100");
    }
}
