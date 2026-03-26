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

    /// Delete all cached values that match a Redis glob pattern
    pub async fn delete_by_pattern(&mut self, pattern: &str) -> Result<u64, RedisError> {
        let keys: Vec<String> = self.client.keys(pattern).await?;
        if keys.is_empty() {
            return Ok(0);
        }

        let deleted: u64 = self.client.del(keys).await?;
        debug!(
            "Deleted {} cache keys matching pattern: {}",
            deleted, pattern
        );
        Ok(deleted)
    }

    /// Check if cache is healthy
    pub async fn is_healthy(&mut self) -> bool {
        self.client
            .get::<_, Option<String>>("_health")
            .await
            .is_ok()
    }
}

/// SingleFlight manager to prevent cache stampedes
pub struct SingleFlight<T> {
    inflight: tokio::sync::Mutex<std::collections::HashMap<String, Arc<InFlight<T>>>>,
}

struct InFlight<T> {
    result: tokio::sync::RwLock<Option<Arc<T>>>,
    notify: tokio::sync::Notify,
}

impl<T: Send + Sync + 'static> SingleFlight<T> {
    /// Create a new SingleFlight manager
    pub fn new() -> Self {
        Self {
            inflight: tokio::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Execute a function with single-flight protection
    /// Identical concurrent requests for the same key will share the same computation
    pub async fn execute<F, Fut>(&self, key: &str, f: F) -> Arc<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Arc<T>>,
    {
        // 1. Check if already in flight
        let mut mg = self.inflight.lock().await;
        if let Some(inflight) = mg.get(key) {
            let inflight = Arc::clone(inflight);
            drop(mg);

            // Create notification future BEFORE checking the result to avoid race
            let notified = inflight.notify.notified();

            // Check if already finished
            {
                let res = inflight.result.read().await;
                if let Some(result) = res.as_ref() {
                    return Arc::clone(result);
                }
            }

            // Wait for notification if not finished yet
            notified.await;

            // Return the result
            let res = inflight.result.read().await;
            return Arc::clone(
                res.as_ref()
                    .expect("Result must be present after notification"),
            );
        }

        // 2. Not in flight, start the work
        let inflight = Arc::new(InFlight {
            result: tokio::sync::RwLock::new(None),
            notify: tokio::sync::Notify::new(),
        });
        mg.insert(key.to_string(), Arc::clone(&inflight));
        drop(mg);

        // 3. Perform the computation
        let result = f().await;

        // 4. Save result and notify others
        {
            let mut res_mg = inflight.result.write().await;
            *res_mg = Some(Arc::clone(&result));
        }
        inflight.notify.notify_waiters();

        // 5. Cleanup inflight map
        let mut mg = self.inflight.lock().await;
        mg.remove(key);
        drop(mg);

        result
    }
}

impl<T: Send + Sync + 'static> Default for SingleFlight<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache key builders
///
/// Current version: v1
/// Documented key formats:
/// - pairs:list -> List of all active trading pairs
/// - orderbook:{base}:{quote} -> Orderbook for a specific pair
/// - v1:quote:{base}:{quote}:{amount}:{slippage_bps}:{quote_type} -> Result of a quote request
/// - liquidity:revision:{base}:{quote} -> Latest observed ledger revision for a pair
pub mod keys {
    /// Cache key for trading pairs list
    pub fn pairs_list() -> String {
        "pairs:list".to_string()
    }

    /// Cache key for orderbook
    pub fn orderbook(base: &str, quote: &str) -> String {
        format!("orderbook:{}:{}", base, quote)
    }

    /// Cache key for quote (versioned: v1)
    pub fn quote(
        base: &str,
        quote: &str,
        amount: &str,
        slippage_bps: u32,
        quote_type: &str,
    ) -> String {
        format!(
            "v1:quote:{}:{}:{}:{}:{}",
            base, quote, amount, slippage_bps, quote_type
        )
    }

    /// Key used to track the latest liquidity revision observed for a pair
    pub fn liquidity_revision(base: &str, quote: &str) -> String {
        format!("liquidity:revision:{}:{}", base, quote)
    }

    /// Pattern that matches all cached quotes for a pair
    pub fn quote_pair_pattern(base: &str, quote: &str) -> String {
        format!("*quote:{}:{}:*", base, quote)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_keys() {
        assert_eq!(keys::pairs_list(), "pairs:list");
        assert_eq!(keys::orderbook("XLM", "USDC"), "orderbook:XLM:USDC");
        assert_eq!(
            keys::quote("XLM", "USDC", "100", 50, "sell"),
            "v1:quote:XLM:USDC:100:50:sell"
        );
        assert_eq!(
            keys::liquidity_revision("XLM", "USDC"),
            "liquidity:revision:XLM:USDC"
        );
        assert_eq!(keys::quote_pair_pattern("XLM", "USDC"), "*quote:XLM:USDC:*");
    }

    #[tokio::test]
    async fn test_single_flight() {
        use std::sync::atomic::{AtomicU64, Ordering};

        let sf = SingleFlight::<u64>::new();
        let counter = Arc::new(AtomicU64::new(0));
        let mut handlers = vec![];

        for _ in 0..10 {
            let sf_ref = &sf;
            let counter_ref = counter.clone();
            handlers.push(tokio::spawn(async move {
                sf_ref
                    .execute("test", || async move {
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                        counter_ref.fetch_add(1, Ordering::Relaxed);
                        Arc::new(42u64)
                    })
                    .await
            }));
        }

        let mut results = vec![];
        for h in handlers {
            results.push(h.await.expect("task failed"));
        }

        assert_eq!(counter.load(Ordering::Relaxed), 1);
        for r in results {
            assert_eq!(*r, 42);
        }
    }
}
