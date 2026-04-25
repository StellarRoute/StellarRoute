//! Shared application state

use sqlx::PgPool;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

use crate::cache::{CacheManager, SingleFlight};

use crate::graph::GraphManager;
use crate::models::{QuoteResponse, RoutesResponse};
use crate::replay::capture::CaptureHook;
use crate::routes::ws::WsState;
use stellarroute_routing::health::circuit_breaker::CircuitBreakerRegistry;

use crate::worker::{JobQueue, RouteWorkerPool, WorkerPoolConfig};

/// Primary database pool for write operations plus an optional replica pool
/// for read-heavy endpoints.
#[derive(Clone, Debug)]
pub struct DatabasePools {
    primary: PgPool,
    replica: Option<PgPool>,
}

impl DatabasePools {
    pub fn new(primary: PgPool, replica: Option<PgPool>) -> Self {
        Self { primary, replica }
    }

    /// Pool used for read-only queries. Falls back to the primary pool when
    /// no replica is configured.
    pub fn read_pool(&self) -> &PgPool {
        self.replica.as_ref().unwrap_or(&self.primary)
    }

    pub fn write_pool(&self) -> &PgPool {
        &self.primary
    }
}

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
    pub db: DatabasePools,
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
    pub quote_single_flight: Arc<SingleFlight<crate::error::Result<(QuoteResponse, bool)>>>,

    /// Optional replay capture hook (None when REPLAY_CAPTURE_ENABLED=false)
    pub replay_capture: Option<Arc<CaptureHook>>,

    /// Single-flight manager for routes
    pub routes_single_flight: Arc<SingleFlight<crate::error::Result<RoutesResponse>>>,
    /// Persistent background synced graph manager
    pub graph_manager: Arc<GraphManager>,
    /// WebSocket shared state
    pub ws: Option<Arc<WsState>>,
    /// Shared circuit breaker registry for liquidity providers
    pub circuit_breaker: Arc<CircuitBreakerRegistry>,
    /// API-level kill switches for sources/venues
    pub kill_switch: Arc<crate::kill_switch::KillSwitchManager>,
    /// Shared liquidity anomaly detector
    pub anomaly_detector: Arc<tokio::sync::Mutex<stellarroute_routing::health::anomaly::LiquidityAnomalyDetector>>,
}

impl AppState {
    /// Create new application state
    pub fn new(db: DatabasePools) -> Self {
        Self::new_with_policy(db, CachePolicy::default())
    }

    pub fn new_with_policy(db: DatabasePools, cache_policy: CachePolicy) -> Self {
        let worker_pool = Self::create_worker_pool(db.write_pool().clone());
        let graph_manager = Arc::new(GraphManager::new(db.write_pool().clone()));
        graph_manager.clone().start_sync();

        let kill_switch = Arc::new(crate::kill_switch::KillSwitchManager::new(None));

        Self {
            db,
            cache: None,
            version: env!("CARGO_PKG_VERSION").to_string(),
            cache_policy,
            cache_metrics: Arc::new(CacheMetrics::default()),
            worker_pool,
            quote_single_flight: Arc::new(SingleFlight::<
                crate::error::Result<(QuoteResponse, bool)>,
            >::new()),
            replay_capture: None,
            routes_single_flight: Arc::new(SingleFlight::new()),
            graph_manager,
            ws: None,
            circuit_breaker: Arc::new(CircuitBreakerRegistry::default()),
            kill_switch,
            anomaly_detector: graph_manager.anomaly_detector.clone(),
        }
    }

    /// Create new application state with cache
    pub fn with_cache(db: DatabasePools, cache: CacheManager) -> Self {
        Self::with_cache_and_policy(db, cache, CachePolicy::default())
    }

    pub fn with_cache_and_policy(
        db: DatabasePools,
        cache: CacheManager,
        cache_policy: CachePolicy,
    ) -> Self {
        let worker_pool = Self::create_worker_pool(db.write_pool().clone());
        let graph_manager = Arc::new(GraphManager::new(db.write_pool().clone()));
        graph_manager.clone().start_sync();

        let cache_arc = Arc::new(Mutex::new(cache));
        let kill_switch = Arc::new(crate::kill_switch::KillSwitchManager::new(Some(
            cache_arc.clone(),
        )));

        // Spawn a task to load initial state from Redis
        let ks = kill_switch.clone();
        tokio::spawn(async move {
            ks.load().await;
            ks.start_sync();
        });

        Self {
            db,
            cache: Some(cache_arc),
            version: env!("CARGO_PKG_VERSION").to_string(),
            cache_policy,
            cache_metrics: Arc::new(CacheMetrics::default()),
            worker_pool,
            quote_single_flight: Arc::new(SingleFlight::<
                crate::error::Result<(QuoteResponse, bool)>,
            >::new()),
            replay_capture: None,
            routes_single_flight: Arc::new(SingleFlight::new()),
            graph_manager,
            ws: None,
            circuit_breaker: Arc::new(CircuitBreakerRegistry::default()),
            kill_switch,
            anomaly_detector: graph_manager.anomaly_detector.clone(),
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

    /// Attach a replay capture hook to this state.
    /// Returns a new `AppState` with the hook set.
    pub fn with_replay_capture(mut self, hook: CaptureHook) -> Self {
        self.replay_capture = Some(Arc::new(hook));
        self
    }

    /// Attach WebSocket state to this state.
    /// Returns a new `AppState` with the state set.
    pub fn with_ws(mut self, ws: Arc<WsState>) -> Self {
        self.ws = Some(ws);
        self
    }
}
