//! Shared application state

use sqlx::PgPool;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

use crate::cache::{CacheManager, SingleFlight};
use crate::models::QuoteResponse;
use crate::worker::{JobQueue, RouteWorkerPool, WorkerPoolConfig};

/// Cache policy configuration
#[derive(Debug, Clone)]
pub struct CachePolicy {
    pub quote_ttl: Duration,
}

impl Default for CachePolicy {
    fn default() -> Self {
        Self {
            quote_ttl: Duration::from_secs(2),
        }
    }
}

/// In-process cache metrics
pub struct CacheMetrics {
    quote_hits: AtomicU64,
    quote_misses: AtomicU64,
    stale_quote_rejections: AtomicU64,
    stale_inputs_excluded: AtomicU64,
}

impl Default for CacheMetrics {
    fn default() -> Self {
        Self {
            quote_hits: AtomicU64::new(0),
            quote_misses: AtomicU64::new(0),
            stale_quote_rejections: AtomicU64::new(0),
            stale_inputs_excluded: AtomicU64::new(0),
        }
    }
}

impl CacheMetrics {
    pub fn inc_quote_hit(&self) {
        self.quote_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_quote_miss(&self) {
        self.quote_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment the stale-quote-rejection counter by one.
    pub fn inc_stale_rejection(&self) {
        self.stale_quote_rejections.fetch_add(1, Ordering::Relaxed);
    }

    /// Add `n` to the stale-inputs-excluded counter.
    pub fn add_stale_inputs_excluded(&self, n: u64) {
        self.stale_inputs_excluded.fetch_add(n, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> (u64, u64) {
        (
            self.quote_hits.load(Ordering::Relaxed),
            self.quote_misses.load(Ordering::Relaxed),
        )
    }

    pub fn snapshot_staleness(&self) -> (u64, u64) {
        (
            self.stale_quote_rejections.load(Ordering::Relaxed),
            self.stale_inputs_excluded.load(Ordering::Relaxed),
        )
    }
}

/// Shared API state
#[derive(Clone)]
pub struct AppState {
    /// Database connection pool
    pub db: PgPool,
    /// Redis cache manager (optional)
    pub cache: Option<Arc<Mutex<CacheManager>>>,
    /// API version
    pub version: String,
    /// Cache policy settings
    pub cache_policy: CachePolicy,
    /// Cache hit/miss counters
    pub cache_metrics: Arc<CacheMetrics>,
    /// Route computation worker pool
    pub worker_pool: Arc<RouteWorkerPool>,
    /// Single-flight manager for quotes to prevent stampedes
    pub quote_single_flight: Arc<SingleFlight<crate::error::Result<QuoteResponse>>>,
}

impl AppState {
    /// Create new application state
    pub fn new(db: PgPool) -> Self {
        Self::new_with_policy(db, CachePolicy::default())
    }

    /// Create new application state with an explicit cache policy
    pub fn new_with_policy(db: PgPool, cache_policy: CachePolicy) -> Self {
        let worker_pool = Self::create_worker_pool(db.clone());

        Self {
            db,
            cache: None,
            version: env!("CARGO_PKG_VERSION").to_string(),
            cache_policy,
            cache_metrics: Arc::new(CacheMetrics::default()),
            worker_pool,
            quote_single_flight: Arc::new(SingleFlight::new()),
        }
    }

    /// Create new application state with cache
    pub fn with_cache(db: PgPool, cache: CacheManager) -> Self {
        Self::with_cache_and_policy(db, cache, CachePolicy::default())
    }

    /// Create new application state with cache and explicit cache policy
    pub fn with_cache_and_policy(
        db: PgPool,
        cache: CacheManager,
        cache_policy: CachePolicy,
    ) -> Self {
        let worker_pool = Self::create_worker_pool(db.clone());

        Self {
            db,
            cache: Some(Arc::new(Mutex::new(cache))),
            version: env!("CARGO_PKG_VERSION").to_string(),
            cache_policy,
            cache_metrics: Arc::new(CacheMetrics::default()),
            worker_pool,
            quote_single_flight: Arc::new(SingleFlight::new()),
        }
    }

    /// Create worker pool with configuration
    fn create_worker_pool(db: PgPool) -> Arc<RouteWorkerPool> {
        let queue = JobQueue::new(db);
        let config = WorkerPoolConfig::default();
        Arc::new(RouteWorkerPool::new(config, queue))
    }

    /// Wrap in Arc for sharing across handlers
    pub fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// Check if caching is enabled
    pub fn has_cache(&self) -> bool {
        self.cache.is_some()
    }
}
