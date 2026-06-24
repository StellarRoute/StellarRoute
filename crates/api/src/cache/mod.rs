//! Redis caching layer

pub mod adaptive_ttl;
pub mod invalidation;
pub mod invalidation_graph;
pub mod jitter;
pub mod prewarm_job;
pub mod prewarmer;

use redis::{aio::ConnectionManager, AsyncCommands, RedisError};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, instrument, warn};

pub use invalidation::{CacheInvalidationManager, LiquidityUpdateEvent};

pub use adaptive_ttl::{
    AdaptiveTtlConfig, AdaptiveTtlEngine, AdaptiveTtlStats, DepthAggregator, MarketMetrics,
    TtlDecision, TtlReason, VolatilityCalculator,
};

pub use jitter::JitteredTtl;

pub use prewarm_job::{PrewarmConfig, PrewarmJob};
pub use prewarmer::{
    CachePrewarmer, DemandForecaster, KeyDemandEntry, PrewarmError, PrewarmMetrics,
};

/// Outcome of a cache lookup used to distinguish misses from Redis outages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheLookupOutcome {
    Hit,
    Miss,
    Unavailable,
}

/// Result of a cache read operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheResult<T> {
    Hit(T),
    Miss,
    Unavailable,
}

impl<T> CacheResult<T> {
    pub fn into_option(self) -> Option<T> {
        match self {
            Self::Hit(value) => Some(value),
            Self::Miss | Self::Unavailable => None,
        }
    }

    pub fn outcome(&self) -> CacheLookupOutcome {
        match self {
            Self::Hit(_) => CacheLookupOutcome::Hit,
            Self::Miss => CacheLookupOutcome::Miss,
            Self::Unavailable => CacheLookupOutcome::Unavailable,
        }
    }
}

/// Redis cache subsystem status for health probes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheHealthStatus {
    Healthy,
    Degraded,
    NotConfigured,
}

impl CacheHealthStatus {
    pub fn as_component_status(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::NotConfigured => "not_configured",
        }
    }
}

/// Returns true when a Redis error reflects infrastructure failure rather than a cache miss.
pub fn is_redis_infrastructure_error(err: &RedisError) -> bool {
    match err.kind() {
        redis::ErrorKind::TypeError => false,
        redis::ErrorKind::IoError | redis::ErrorKind::ClientError => true,
        _ => err.is_connection_dropped() || err.is_io_error(),
    }
}

fn record_lookup_outcome(outcome: CacheLookupOutcome, operation: &str) {
    if outcome == CacheLookupOutcome::Unavailable {
        crate::metrics::record_redis_error(operation);
    }
}

/// Cache manager for Redis operations
#[derive(Clone)]
pub struct CacheManager {
    client: Option<ConnectionManager>,
}

impl CacheManager {
    /// Create a new cache manager
    pub async fn new(redis_url: &str) -> Result<Self, RedisError> {
        let client = redis::Client::open(redis_url)?;
        let conn = ConnectionManager::new(client).await?;

        debug!("Redis cache manager initialized");
        Ok(Self {
            client: Some(conn),
        })
    }

    /// Create a cache manager from an existing connection (used in tests).
    pub fn from_connection(client: ConnectionManager) -> Self {
        Self {
            client: Some(client),
        }
    }

    /// Create a cache manager that simulates a Redis outage for chaos tests.
    pub fn simulated_outage() -> Self {
        Self { client: None }
    }

    fn record_unavailable(&self, operation: &str) {
        record_lookup_outcome(CacheLookupOutcome::Unavailable, operation);
    }

    /// Probe Redis availability for health checks.
    pub async fn health_status(&mut self) -> CacheHealthStatus {
        if self.client.is_none() {
            return CacheHealthStatus::Degraded;
        }
        if self.is_healthy().await {
            CacheHealthStatus::Healthy
        } else {
            CacheHealthStatus::Degraded
        }
    }

    /// Get a cached value
    #[instrument(skip(self), fields(cache.hit = tracing::field::Empty))]
    pub async fn get<T: DeserializeOwned>(&mut self, key: &str) -> CacheResult<T> {
        let Some(client) = self.client.as_mut() else {
            warn!("Redis unavailable during cache get for {}: simulated outage", key);
            self.record_unavailable("get");
            return CacheResult::Unavailable;
        };

        match client.get::<_, String>(key).await {
            Ok(json) => match serde_json::from_str(&json) {
                Ok(value) => {
                    tracing::Span::current().record("cache.hit", true);
                    debug!("Cache hit for key: {}", key);
                    CacheResult::Hit(value)
                }
                Err(e) => {
                    tracing::Span::current().record("cache.hit", false);
                    warn!("Failed to deserialize cached value for {}: {}", key, e);
                    CacheResult::Miss
                }
            },
            Err(e) => {
                tracing::Span::current().record("cache.hit", false);
                if is_redis_infrastructure_error(&e) {
                    warn!("Redis unavailable during cache get for {}: {}", key, e);
                    record_lookup_outcome(CacheLookupOutcome::Unavailable, "get");
                    CacheResult::Unavailable
                } else {
                    debug!("Cache miss for key: {}", key);
                    CacheResult::Miss
                }
            }
        }
    }

    /// Get a cached JSON payload without deserializing.
    #[instrument(skip(self), fields(cache.hit = tracing::field::Empty))]
    pub async fn get_json(&mut self, key: &str) -> CacheResult<String> {
        let Some(client) = self.client.as_mut() else {
            warn!(
                "Redis unavailable during cache get_json for {}: simulated outage",
                key
            );
            self.record_unavailable("get_json");
            return CacheResult::Unavailable;
        };

        match client.get::<_, String>(key).await {
            Ok(json) => {
                tracing::Span::current().record("cache.hit", true);
                debug!("Raw JSON cache hit for key: {}", key);
                CacheResult::Hit(json)
            }
            Err(e) => {
                tracing::Span::current().record("cache.hit", false);
                if is_redis_infrastructure_error(&e) {
                    warn!("Redis unavailable during cache get_json for {}: {}", key, e);
                    record_lookup_outcome(CacheLookupOutcome::Unavailable, "get_json");
                    CacheResult::Unavailable
                } else {
                    debug!("Raw JSON cache miss for key: {}", key);
                    CacheResult::Miss
                }
            }
        }
    }

    /// Set a cached value with TTL
    #[instrument(skip(self, value), fields(cache.ttl_ms = ttl.as_millis() as u64))]
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

        let Some(client) = self.client.as_mut() else {
            warn!("Redis unavailable during cache set for {}: simulated outage", key);
            self.record_unavailable("set");
            return Err(RedisError::from((
                redis::ErrorKind::IoError,
                "simulated redis outage",
            )));
        };

        client
            .set_ex::<_, _, ()>(key, json, ttl.as_secs())
            .await
            .map_err(|e| {
                if is_redis_infrastructure_error(&e) {
                    warn!("Redis unavailable during cache set for {}: {}", key, e);
                    record_lookup_outcome(CacheLookupOutcome::Unavailable, "set");
                }
                e
            })?;

        debug!("Cached key: {} with TTL: {:?}", key, ttl);
        Ok(())
    }

    /// Set a pre-serialized JSON payload with TTL.
    #[instrument(skip(self, json), fields(cache.ttl_ms = ttl.as_millis() as u64))]
    pub async fn set_json(
        &mut self,
        key: &str,
        json: &str,
        ttl: Duration,
    ) -> Result<(), RedisError> {
        let Some(client) = self.client.as_mut() else {
            warn!(
                "Redis unavailable during cache set_json for {}: simulated outage",
                key
            );
            self.record_unavailable("set_json");
            return Err(RedisError::from((
                redis::ErrorKind::IoError,
                "simulated redis outage",
            )));
        };

        client
            .set_ex::<_, _, ()>(key, json, ttl.as_secs())
            .await
            .map_err(|e| {
                if is_redis_infrastructure_error(&e) {
                    warn!("Redis unavailable during cache set_json for {}: {}", key, e);
                    record_lookup_outcome(CacheLookupOutcome::Unavailable, "set_json");
                }
                e
            })?;

        debug!("Cached raw JSON key: {} with TTL: {:?}", key, ttl);
        Ok(())
    }

    /// Delete a cached value
    pub async fn delete(&mut self, key: &str) -> Result<(), RedisError> {
        let Some(client) = self.client.as_mut() else {
            warn!(
                "Redis unavailable during cache delete for {}: simulated outage",
                key
            );
            self.record_unavailable("delete");
            return Err(RedisError::from((
                redis::ErrorKind::IoError,
                "simulated redis outage",
            )));
        };

        client.del::<_, ()>(key).await.map_err(|e| {
            if is_redis_infrastructure_error(&e) {
                warn!("Redis unavailable during cache delete for {}: {}", key, e);
                record_lookup_outcome(CacheLookupOutcome::Unavailable, "delete");
            }
            e
        })?;
        debug!("Deleted cache key: {}", key);
        Ok(())
    }

    /// Delete all cached values that match a Redis glob pattern
    pub async fn delete_by_pattern(&mut self, pattern: &str) -> Result<u64, RedisError> {
        let Some(client) = self.client.as_mut() else {
            warn!(
                "Redis unavailable during cache delete_by_pattern for {}: simulated outage",
                pattern
            );
            self.record_unavailable("delete_by_pattern");
            return Err(RedisError::from((
                redis::ErrorKind::IoError,
                "simulated redis outage",
            )));
        };

        let keys: Vec<String> = match client.keys(pattern).await {
            Ok(keys) => keys,
            Err(e) => {
                if is_redis_infrastructure_error(&e) {
                    warn!(
                        "Redis unavailable during cache keys lookup for {}: {}",
                        pattern, e
                    );
                    record_lookup_outcome(CacheLookupOutcome::Unavailable, "delete_by_pattern");
                }
                return Err(e);
            }
        };
        if keys.is_empty() {
            return Ok(0);
        }

        let deleted: u64 = client.del(keys).await.map_err(|e| {
            if is_redis_infrastructure_error(&e) {
                warn!(
                    "Redis unavailable during cache delete_by_pattern for {}: {}",
                    pattern, e
                );
                record_lookup_outcome(CacheLookupOutcome::Unavailable, "delete_by_pattern");
            }
            e
        })?;
        debug!(
            "Deleted {} cache keys matching pattern: {}",
            deleted, pattern
        );
        Ok(deleted)
    }

    /// Check if cache is healthy
    pub async fn is_healthy(&mut self) -> bool {
        let Some(client) = self.client.as_mut() else {
            self.record_unavailable("health");
            return false;
        };

        match client.get::<_, Option<String>>("_health").await {
            Ok(_) => true,
            Err(e) => {
                if is_redis_infrastructure_error(&e) {
                    record_lookup_outcome(CacheLookupOutcome::Unavailable, "health");
                }
                false
            }
        }
    }
}

/// SingleFlight manager to prevent cache stampedes
pub struct SingleFlight<T> {
    inflight: Arc<tokio::sync::Mutex<std::collections::HashMap<String, Arc<InFlight<T>>>>>,
}

struct InFlight<T> {
    result: tokio::sync::RwLock<Option<Arc<T>>>,
    notify: tokio::sync::Notify,
}

impl<T: Send + Sync + 'static> SingleFlight<T> {
    /// Create a new SingleFlight manager
    pub fn new() -> Self {
        Self {
            inflight: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Execute a function with single-flight protection
    /// Identical concurrent requests for the same key will share the same computation
    pub async fn execute<F, Fut>(&self, key: &str, f: F) -> Arc<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Arc<T>>,
    {
        // We'll loop until we either return a cached/finished result or become the leader
        // that runs the computation. This allows followers to retry becoming the leader
        // if the previous leader was cancelled and removed the inflight entry.
        let mut maybe_f = Some(f);
        loop {
            // 1. Fast-path: check if already in flight
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

                // After being notified, loop and re-check state: either the result
                // is present (return it) or the inflight entry was removed and we
                // should attempt to become the leader (next loop iteration will do so).
                let res = inflight.result.read().await;
                if let Some(result) = res.as_ref() {
                    return Arc::clone(result);
                }

                // No result present: previous leader likely cancelled. Try again.
                continue;
            }

            // 2. Not in flight: become the leader
            let inflight = Arc::new(InFlight {
                result: tokio::sync::RwLock::new(None),
                notify: tokio::sync::Notify::new(),
            });
            mg.insert(key.to_string(), Arc::clone(&inflight));
            drop(mg);

            // Create a guard to ensure cleanup on drop (cancellation/panic)
            struct LeaderGuard<T: Send + Sync + 'static> {
                inflight_map:
                    Arc<tokio::sync::Mutex<std::collections::HashMap<String, Arc<InFlight<T>>>>>,
                key: String,
                inflight: Arc<InFlight<T>>,
            }

            impl<T: Send + Sync + 'static> Drop for LeaderGuard<T> {
                fn drop(&mut self) {
                    // Notify waiters so they can re-check state instead of hanging.
                    self.inflight.notify.notify_waiters();

                    let inflight_map = self.inflight_map.clone();
                    let key = self.key.clone();
                    tokio::spawn(async move {
                        let mut mg = inflight_map.lock().await;
                        mg.remove(&key);
                    });
                }
            }

            let _guard = LeaderGuard {
                inflight_map: self.inflight.clone(),
                key: key.to_string(),
                inflight: Arc::clone(&inflight),
            };

            // Perform the computation as leader by taking and calling the closure.
            let f_taken = maybe_f
                .take()
                .expect("closure must be available when becoming leader");
            let result = f_taken().await;

            // Save result and notify others
            {
                let mut res_mg = inflight.result.write().await;
                *res_mg = Some(Arc::clone(&result));
            }
            // Notify waiters that result is present
            inflight.notify.notify_waiters();

            return result;
        }
    }

    /// Execute a function with single-flight protection and record metrics with `label`.
    /// Identical concurrent requests for the same key will share the same computation.
    pub async fn execute_with_label<F, Fut>(&self, key: &str, label: &str, f: F) -> Arc<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Arc<T>>,
    {
        let mut maybe_f = Some(f);
        loop {
            let mut mg = self.inflight.lock().await;
            if let Some(inflight) = mg.get(key) {
                let inflight = Arc::clone(inflight);
                drop(mg);

                let notified = inflight.notify.notified();

                {
                    let res = inflight.result.read().await;
                    if let Some(result) = res.as_ref() {
                        // follower returning existing result -> coalesced
                        crate::metrics::record_single_flight_coalesced(label);
                        return Arc::clone(result);
                    }
                }

                notified.await;

                let res = inflight.result.read().await;
                if let Some(result) = res.as_ref() {
                    crate::metrics::record_single_flight_coalesced(label);
                    return Arc::clone(result);
                }

                continue;
            }

            let inflight = Arc::new(InFlight {
                result: tokio::sync::RwLock::new(None),
                notify: tokio::sync::Notify::new(),
            });
            mg.insert(key.to_string(), Arc::clone(&inflight));
            drop(mg);

            struct LeaderGuard<T: Send + Sync + 'static> {
                inflight_map:
                    Arc<tokio::sync::Mutex<std::collections::HashMap<String, Arc<InFlight<T>>>>>,
                key: String,
                inflight: Arc<InFlight<T>>,
            }

            impl<T: Send + Sync + 'static> Drop for LeaderGuard<T> {
                fn drop(&mut self) {
                    self.inflight.notify.notify_waiters();

                    let inflight_map = self.inflight_map.clone();
                    let key = self.key.clone();
                    tokio::spawn(async move {
                        let mut mg = inflight_map.lock().await;
                        mg.remove(&key);
                    });
                }
            }

            let _guard = LeaderGuard {
                inflight_map: self.inflight.clone(),
                key: key.to_string(),
                inflight: Arc::clone(&inflight),
            };

            let f_taken = maybe_f
                .take()
                .expect("closure must be available when becoming leader");
            // leader -> unique
            crate::metrics::record_single_flight_unique(label);
            let result = f_taken().await;

            {
                let mut res_mg = inflight.result.write().await;
                *res_mg = Some(Arc::clone(&result));
            }
            inflight.notify.notify_waiters();

            return result;
        }
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
/// - price-history:{base}:{quote} -> 24h historical price series for a pair
/// - v1:quote:{base}:{quote}:{amount}:{slippage_bps}:{quote_type} -> Result of a quote request
/// - liquidity:revision:{base}:{quote} -> Latest observed ledger revision for a pair
pub mod keys {
    /// Cache key for trading pairs list
    pub fn pairs_list() -> String {
        "pairs:list".to_string()
    }

    /// Cache key for a paginated trading pairs list.
    pub fn pairs_list_page(limit: usize, offset: usize) -> String {
        format!("pairs:list:{}:{}", limit, offset)
    }

    /// Cache key for orderbook
    ///
    /// Uses canonical pair ordering so that `orderbook(A,B)` and `orderbook(B,A)`
    /// produce the same key.
    pub fn orderbook(base: &str, quote: &str) -> String {
        let (norm_base, norm_quote) = normalize_pair_assets(base, quote);
        format!("orderbook:{}:{}", norm_base, norm_quote)
    }

    /// Cache key for 24h price history
    pub fn price_history(base: &str, quote: &str) -> String {
        format!("price-history:{}:{}", base, quote)
    }

    /// Cache key for quote (versioned: v2)
    /// Normalizes assets and amounts for deterministic lookups.
    pub fn quote(
        base: &str,
        quote: &str,
        amount: &str,
        slippage_bps: u32,
        quote_type: &str,
        explain: bool,
    ) -> String {
        let norm_base = normalize_asset(base);
        let norm_quote = normalize_asset(quote);
        let norm_amount = normalize_amount(amount);

        format!(
            "v2:quote:{}:{}:{}:{}:{}:{}",
            norm_base, norm_quote, norm_amount, slippage_bps, quote_type, explain
        )
    }

    /// Normalize asset identifiers (e.g. XLM/xlm -> native)
    ///
    /// Delegates to the shared [`stellarroute_routing::normalize_asset`] so
    /// that all crates use the same canonical representation.
    fn normalize_asset(asset: &str) -> String {
        stellarroute_routing::normalize_asset(asset)
    }

    /// Normalize amounts to a canonical string (7 decimal precision)
    fn normalize_amount(amount: &str) -> String {
        match amount.parse::<f64>() {
            Ok(val) => format!("{:.7}", val),
            Err(_) => amount.to_string(), // Fallback if invalid
        }
    }

    /// Normalize two assets individually then return them in canonical pair order.
    ///
    /// Delegates to the shared [`stellarroute_routing::normalize_pair_owned`]
    /// so that all crates use the same canonical representation.
    fn normalize_pair_assets(a: &str, b: &str) -> (String, String) {
        stellarroute_routing::normalize_pair_owned(a, b)
    }

    /// Key used to track the latest liquidity revision observed for a pair
    ///
    /// Uses canonical pair ordering so that revision checks are consistent
    /// regardless of asset ordering.
    pub fn liquidity_revision(base: &str, quote: &str) -> String {
        let (norm_base, norm_quote) = normalize_pair_assets(base, quote);
        format!("liquidity:revision:{}:{}", norm_base, norm_quote)
    }

    /// Pattern that matches all cached quotes for a pair
    ///
    /// Uses canonical pair ordering so that invalidation covers both orderings.
    pub fn quote_pair_pattern(base: &str, quote: &str) -> String {
        let (norm_base, norm_quote) = normalize_pair_assets(base, quote);
        format!("*quote:{}:{}:*", norm_base, norm_quote)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_keys() {
        assert_eq!(keys::pairs_list(), "pairs:list");
        assert_eq!(keys::pairs_list_page(25, 50), "pairs:list:25:50");
        // orderbook uses canonical pair ordering: "native" < "USDC" lexicographically
        assert_eq!(keys::orderbook("USDC", "XLM"), "orderbook:native:USDC");
        assert_eq!(keys::orderbook("XLM", "USDC"), "orderbook:native:USDC");
        assert_eq!(keys::price_history("XLM", "USDC"), "price-history:XLM:USDC");
        assert_eq!(
            keys::quote("xlm", "usdc", "100.0", 50, "sell", true),
            "v2:quote:native:USDC:100.0000000:50:sell:true"
        );
        assert_eq!(
            keys::liquidity_revision("USDC", "xlm"),
            "liquidity:revision:native:USDC"
        );
        assert_eq!(
            keys::liquidity_revision("xlm", "USDC"),
            "liquidity:revision:native:USDC"
        );
        assert_eq!(
            keys::quote_pair_pattern("USDC", "XLM"),
            "*quote:native:USDC:*"
        );
        assert_eq!(
            keys::quote_pair_pattern("XLM", "usdc"),
            "*quote:native:USDC:*"
        );
    }

    #[tokio::test]
    async fn test_cache_normalization() {
        // Equivalent inputs should map to same key
        let key1 = keys::quote("XLM", "USDC", "100", 50, "sell", false);
        let key2 = keys::quote("xlm", "usdc", "100.000", 50, "sell", false);
        let key3 = keys::quote("native", "USDC", "100.0000000", 50, "sell", false);

        assert_eq!(key1, "v2:quote:native:USDC:100.0000000:50:sell:false");
        assert_eq!(key1, key2);
        assert_eq!(key2, key3);
    }

    #[tokio::test]
    async fn test_canonical_pair_ordering_in_cache_keys() {
        // Reversed URL parameters produce the same pair-oriented cache keys
        assert_eq!(
            keys::orderbook("USDC", "native"),
            keys::orderbook("native", "USDC"),
        );
        assert_eq!(
            keys::liquidity_revision("USDC:GA5ZSEJ", "XLM"),
            keys::liquidity_revision("XLM", "USDC:GA5ZSEJ"),
        );
        assert_eq!(
            keys::quote_pair_pattern("BTC", "ETH"),
            keys::quote_pair_pattern("ETH", "BTC"),
        );
    }

    #[tokio::test]
    async fn test_single_flight() {
        use std::sync::atomic::{AtomicU64, Ordering};

        let sf = Arc::new(SingleFlight::<u64>::new());
        let counter = Arc::new(AtomicU64::new(0));
        let mut handlers = vec![];

        for _ in 0..10 {
            let sf_ref = sf.clone();
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
        for res in results {
            assert_eq!(*res, 42);
        }
    }

    #[tokio::test]
    async fn test_single_flight_cancellation_cleanup() {
        let sf = Arc::new(SingleFlight::<u64>::new());
        let sf_c = sf.clone();

        // Start a leader that will be cancelled
        let handle = tokio::spawn(async move {
            sf_c.execute("cancel-test", || async move {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                Arc::new(0u64)
            })
            .await
        });

        // Give it a moment to start and register in-flight
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        // Start a follower
        let sf_f = sf.clone();
        let follower = tokio::spawn(async move {
            sf_f.execute("cancel-test", || async move {
                Arc::new(42u64) // This shouldn't run if it's following
            })
            .await
        });

        // Cancel the leader
        handle.abort();

        // The follower should NOT hang. It should either get a "Result must be present" panic
        // (if we don't handle None better) or we should handle the None case.
        // Actually, my current implementation panics for followers if leader didn't set result.
        // Let's refine the implementation to handle this or just verify it doesn't hang.

        // Wait for follower with timeout
        let result = tokio::time::timeout(std::time::Duration::from_secs(1), follower).await;
        assert!(result.is_ok(), "Follower hung after leader cancellation");
    }

    #[test]
    fn infrastructure_errors_exclude_cache_miss_kinds() {
        let miss = RedisError::from((redis::ErrorKind::TypeError, "not a string"));
        assert!(!is_redis_infrastructure_error(&miss));
    }

    #[tokio::test]
    async fn unreachable_redis_get_degrades_to_unavailable() {
        let before = crate::metrics::redis_error_total();
        let mut cache = CacheManager::simulated_outage();

        let outcome = cache.get::<String>("missing-key").await;
        assert_eq!(outcome, CacheResult::Unavailable);
        assert!(
            crate::metrics::redis_error_total() > before,
            "redis error metric should increment on infrastructure failure"
        );
        assert_eq!(cache.health_status().await, CacheHealthStatus::Degraded);
    }
}
