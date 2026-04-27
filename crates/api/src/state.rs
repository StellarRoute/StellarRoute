//! Shared application state

use sqlx::PgPool;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

use crate::cache::{CacheManager, SingleFlight};
use crate::models::{QuoteResponse, RoutesResponse};
use crate::replay::capture::CaptureHook;
use crate::routes::ws::WsState;
use stellarroute_routing::health::circuit_breaker::{CircuitBreakerRegistry, BreakerConfig};

use crate::audit::AuditWriter;
use crate::indexer_lag::IndexerLagMonitor;
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

    pub fn inc_stale_rejection(&self) {
        self.stale_quote_rejections.fetch_add(1, Ordering::Relaxed);
    }

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
    pub db: PgPool,
    pub cache: Option<Arc<Mutex<CacheManager>>>,
    pub version: String,
    pub cache_policy: CachePolicy,
    pub cache_metrics: Arc<CacheMetrics>,
    pub worker_pool: Arc<RouteWorkerPool>,
    pub quote_single_flight: Arc<SingleFlight<crate::error::Result<QuoteResponse>>>,
    pub replay_capture: Option<Arc<CaptureHook>>,
    pub routes_single_flight: Arc<SingleFlight<crate::error::Result<RoutesResponse>>>,
    pub graph_manager: Arc<GraphManager>,
    pub ws: Option<Arc<WsState>>,
    pub circuit_breaker: Arc<CircuitBreakerRegistry>,
    /// API-level kill switches for sources/venues
    pub kill_switch: Arc<crate::kill_switch::KillSwitchManager>,
    /// Shared liquidity anomaly detector
    pub anomaly_detector:
        Arc<tokio::sync::Mutex<stellarroute_routing::health::anomaly::LiquidityAnomalyDetector>>,
    /// Canary configuration for side-by-side policy evaluation
    pub canary_config: Arc<tokio::sync::RwLock<CanaryConfig>>,
    /// Canary history buffer for operator reporting
    pub canary_history: Arc<tokio::sync::RwLock<std::collections::VecDeque<CanaryEvaluation>>>,
    /// Dynamic timeout controller for quote discovery
    pub timeout_controller: Arc<TimeoutController>,
    /// Non-blocking audit log writer for route decisions
    pub audit_writer: Arc<AuditWriter>,
    /// Indexer lag monitor for sync drift detection
    pub indexer_lag: Arc<IndexerLagMonitor>,
}

impl AppState {
    pub fn new(db: PgPool) -> Self {
        Self::new_with_policy(db, CachePolicy::default())
    }

    pub fn new_with_policy(db: DatabasePools, cache_policy: CachePolicy) -> Self {
        let worker_pool = Self::create_worker_pool(db.write_pool().clone());
        let graph_manager = Arc::new(GraphManager::new(db.write_pool().clone()));
        graph_manager.clone().start_sync();

        let kill_switch = Arc::new(crate::kill_switch::KillSwitchManager::new(None));
        let audit_writer = Arc::new(AuditWriter::from_env(db.write_pool().clone()));
        let indexer_lag = Arc::new(IndexerLagMonitor::from_env(db.write_pool().clone()));
        indexer_lag
            .clone()
            .start_polling(std::time::Duration::from_secs(30));

        Self {
            db,
            cache: None,
            version: env!("CARGO_PKG_VERSION").to_string(),
            cache_policy,
            cache_metrics: Arc::new(CacheMetrics::default()),
            worker_pool,
            quote_single_flight: Arc::new(SingleFlight::new()),
            replay_capture: None,
            routes_single_flight: Arc::new(SingleFlight::new()),
            anomaly_detector: graph_manager.anomaly_detector.clone(),
            graph_manager,
            ws: None,
            circuit_breaker: Arc::new(CircuitBreakerRegistry::default()),
            kill_switch,
            canary_config: Arc::new(tokio::sync::RwLock::new(CanaryConfig::default())),
            canary_history: Arc::new(tokio::sync::RwLock::new(
                std::collections::VecDeque::with_capacity(1000),
            )),
            timeout_controller: Arc::new(TimeoutController::new(Default::default())),
            audit_writer,
            indexer_lag,
        }
    }

    pub fn with_cache(db: PgPool, cache: CacheManager) -> Self {
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
        let audit_writer = Arc::new(AuditWriter::from_env(db.write_pool().clone()));
        let indexer_lag = Arc::new(IndexerLagMonitor::from_env(db.write_pool().clone()));
        indexer_lag
            .clone()
            .start_polling(std::time::Duration::from_secs(30));

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
            quote_single_flight: Arc::new(SingleFlight::new()),
            replay_capture: None,
            routes_single_flight: Arc::new(SingleFlight::new()),
            anomaly_detector: graph_manager.anomaly_detector.clone(),
            graph_manager,
            ws: None,
            circuit_breaker: Arc::new(CircuitBreakerRegistry::default()),
            kill_switch,
            canary_config: Arc::new(tokio::sync::RwLock::new(CanaryConfig::default())),
            canary_history: Arc::new(tokio::sync::RwLock::new(
                std::collections::VecDeque::with_capacity(1000),
            )),
            timeout_controller: Arc::new(TimeoutController::new(Default::default())),
            audit_writer,
            indexer_lag,
        }
    }

    fn create_worker_pool(db: PgPool) -> Arc<RouteWorkerPool> {
        let queue = JobQueue::new(db);
        let config = WorkerPoolConfig::default();
        let pool = Arc::new(RouteWorkerPool::new(config, queue));

        // Spawn a background task that periodically pushes per-priority queue
        // depth and virtual-clock values to Prometheus gauges.
        let pool_ref = pool.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
            loop {
                interval.tick().await;
                let snapshot = pool_ref.metrics().await;
                crate::metrics::update_queue_depth_gauges(&snapshot.pending_by_priority);
                crate::metrics::update_virtual_clock(snapshot.virtual_clock);
            }
        });

        pool
    }

    pub fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }

    pub fn has_cache(&self) -> bool {
        self.cache.is_some()
    }

    pub fn with_replay_capture(mut self, hook: CaptureHook) -> Self {
        self.replay_capture = Some(Arc::new(hook));
        self
    }

    pub fn with_ws(mut self, ws: Arc<WsState>) -> Self {
        self.ws = Some(ws);
        self
    }
}
