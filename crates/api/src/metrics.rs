//! Prometheus metrics for StellarRoute API
//!
//! Exposes metrics for:
//! - Quote request latency (p50/p95)
//! - Route computation time
//! - Cache hit ratio

use lazy_static::lazy_static;
use prometheus::{
    register_histogram_vec, register_int_counter_vec, register_int_gauge_vec, Encoder,
    HistogramVec, IntCounterVec, IntGaugeVec, TextEncoder,
};
use std::time::Duration;

lazy_static! {
    /// Quote request latency histogram
    /// Labels: outcome (success/error), cache_hit (true/false)
    pub static ref QUOTE_LATENCY: HistogramVec = register_histogram_vec!(
        "stellarroute_quote_request_duration_seconds",
        "Quote request latency in seconds",
        &["outcome", "cache_hit"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    )
    .expect("Can't create QUOTE_LATENCY histogram");

    /// Route computation time histogram
    /// Labels: environment (production/analysis/realtime/testing)
    pub static ref ROUTE_COMPUTE_TIME: HistogramVec = register_histogram_vec!(
        "stellarroute_route_compute_duration_seconds",
        "Route computation time in seconds",
        &["environment"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]
    )
    .expect("Can't create ROUTE_COMPUTE_TIME histogram");

    /// Cache operations counters
    pub static ref CACHE_HITS: IntCounterVec = register_int_counter_vec!(
        "stellarroute_cache_hits_total",
        "Total number of cache hits",
        &["type"]
    )
    .expect("Can't create CACHE_HITS counter");

    pub static ref CACHE_MISSES: IntCounterVec = register_int_counter_vec!(
        "stellarroute_cache_misses_total",
        "Total number of cache misses",
        &["type"]
    )
    .expect("Can't create CACHE_MISSES counter");

    /// Quote request counter
    pub static ref QUOTE_REQUESTS: IntCounterVec = register_int_counter_vec!(
        "stellarroute_quote_requests_total",
        "Total number of quote requests",
        &["outcome", "cache_hit"]
    )
    .expect("Can't create QUOTE_REQUESTS counter");

    pub static ref KILL_SWITCH_STATUS: IntGaugeVec = register_int_gauge_vec!(
        "stellarroute_kill_switch_status",
        "Kill switch status (1 for disabled, 0 for enabled)",
        &["type", "name"]
    )
    .expect("Can't create KILL_SWITCH_STATUS gauge");

    /// Adaptive timeout value in milliseconds
    pub static ref ADAPTIVE_TIMEOUT_MS: IntGaugeVec = register_int_gauge_vec!(
        "stellarroute_adaptive_timeout_ms",
        "Current adaptive timeout value in milliseconds",
        &["environment"]
    )
    .expect("Can't create ADAPTIVE_TIMEOUT_MS gauge");

    /// EMA latency in milliseconds
    pub static ref EMA_LATENCY_MS: IntGaugeVec = register_int_gauge_vec!(
        "stellarroute_ema_latency_ms",
        "Current EMA latency in milliseconds",
        &["environment"]
    )
    .expect("Can't create EMA_LATENCY_MS gauge");

    // ── Priority queue metrics ────────────────────────────────────────────

    /// Total jobs submitted to the priority queue, labelled by priority band.
    pub static ref QUEUE_SUBMISSIONS: IntCounterVec = register_int_counter_vec!(
        "stellarroute_queue_submissions_total",
        "Total jobs submitted to the priority queue",
        &["priority"]
    )
    .expect("Can't create QUEUE_SUBMISSIONS counter");

    /// Total jobs completed from the priority queue, labelled by priority band.
    pub static ref QUEUE_COMPLETIONS: IntCounterVec = register_int_counter_vec!(
        "stellarroute_queue_completions_total",
        "Total jobs completed from the priority queue",
        &["priority"]
    )
    .expect("Can't create QUEUE_COMPLETIONS counter");

    /// Current depth of the pending queue, labelled by priority band.
    pub static ref QUEUE_DEPTH: IntGaugeVec = register_int_gauge_vec!(
        "stellarroute_queue_depth",
        "Current number of pending jobs in the priority queue",
        &["priority"]
    )
    .expect("Can't create QUEUE_DEPTH gauge");

    /// Job processing latency histogram, labelled by priority band.
    pub static ref QUEUE_JOB_LATENCY: HistogramVec = register_histogram_vec!(
        "stellarroute_queue_job_duration_seconds",
        "Time from job submission to completion, by priority band",
        &["priority"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    )
    .expect("Can't create QUEUE_JOB_LATENCY histogram");

    /// WFQ virtual clock value (monotonically increasing).
    pub static ref QUEUE_VIRTUAL_CLOCK: IntGaugeVec = register_int_gauge_vec!(
        "stellarroute_queue_virtual_clock",
        "Current WFQ virtual clock value used for starvation prevention",
        &["instance"]
    )
    .expect("Can't create QUEUE_VIRTUAL_CLOCK gauge");

    // ── Indexer lag metrics ───────────────────────────────────────────────

    /// Indexer lag in ledger counts relative to Horizon.
    /// Labels: source (sdex / amm)
    pub static ref INDEXER_LAG_LEDGERS: IntGaugeVec = register_int_gauge_vec!(
        "stellarroute_indexer_lag_ledgers",
        "Number of ledgers the local index is behind the live Horizon sequence",
        &["source"]
    )
    .expect("Can't create INDEXER_LAG_LEDGERS gauge");

    /// Indexer lag in estimated wall-clock seconds.
    /// Labels: source (sdex / amm)
    pub static ref INDEXER_LAG_SECONDS: prometheus::GaugeVec = prometheus::register_gauge_vec!(
        "stellarroute_indexer_lag_seconds",
        "Estimated wall-clock lag of the local index behind Horizon (seconds)",
        &["source"]
    )
    .expect("Can't create INDEXER_LAG_SECONDS gauge");

    /// Most recently indexed ledger sequence number.
    /// Labels: source (sdex / amm)
    pub static ref INDEXER_LAST_LEDGER: IntGaugeVec = register_int_gauge_vec!(
        "stellarroute_indexer_last_indexed_ledger",
        "Most recently indexed ledger sequence number",
        &["source"]
    )
    .expect("Can't create INDEXER_LAST_LEDGER gauge");

    /// Current Horizon latest ledger sequence (cached from last measurement).
    pub static ref INDEXER_HORIZON_LEDGER: IntGaugeVec = register_int_gauge_vec!(
        "stellarroute_indexer_horizon_ledger",
        "Current Horizon latest ledger sequence number (cached)",
        &["instance"]
    )
    .expect("Can't create INDEXER_HORIZON_LEDGER gauge");

    /// Sync status gauge: 1 = ok, 0 = warning, -1 = critical, -2 = unknown.
    /// Labels: source (sdex / amm)
    pub static ref INDEXER_SYNC_STATUS: IntGaugeVec = register_int_gauge_vec!(
        "stellarroute_indexer_sync_status",
        "Indexer sync health: 1=ok, 0=warning, -1=critical, -2=unknown",
        &["source"]
    )
    .expect("Can't create INDEXER_SYNC_STATUS gauge");
}

/// Record kill switch status
pub fn record_kill_switch_status(ks_type: &str, name: &str, disabled: bool) {
    let value = if disabled { 1 } else { 0 };
    KILL_SWITCH_STATUS
        .with_label_values(&[ks_type, name])
        .set(value);
}

/// Record quote latency metric
pub fn record_quote_latency(duration: Duration, outcome: &str, cache_hit: bool) {
    let outcome_label = match outcome {
        "none" => "success",
        _ => "error",
    };
    let cache_hit_label = if cache_hit { "true" } else { "false" };

    QUOTE_LATENCY
        .with_label_values(&[outcome_label, cache_hit_label])
        .observe(duration.as_secs_f64());

    QUOTE_REQUESTS
        .with_label_values(&[outcome_label, cache_hit_label])
        .inc();
}

/// Record route compute time metric
pub fn record_route_compute_time(duration: Duration, environment: &str) {
    ROUTE_COMPUTE_TIME
        .with_label_values(&[environment])
        .observe(duration.as_secs_f64());
}

/// Record cache hit
pub fn record_cache_hit(cache_type: &str) {
    CACHE_HITS.with_label_values(&[cache_type]).inc();
}

/// Record cache miss
pub fn record_cache_miss(cache_type: &str) {
    CACHE_MISSES.with_label_values(&[cache_type]).inc();
}

/// Record adaptive timeout metrics
pub fn record_adaptive_timeout(timeout_ms: u64, ema_ms: u64, environment: &str) {
    ADAPTIVE_TIMEOUT_MS
        .with_label_values(&[environment])
        .set(timeout_ms as i64);
    EMA_LATENCY_MS
        .with_label_values(&[environment])
        .set(ema_ms as i64);
}

// ── Priority queue metric helpers ─────────────────────────────────────────────

/// Increment the submission counter for a priority band.
pub fn record_queue_submission(priority: &str) {
    QUEUE_SUBMISSIONS.with_label_values(&[priority]).inc();
}

/// Increment the completion counter for a priority band.
pub fn record_queue_completion(priority: &str) {
    QUEUE_COMPLETIONS.with_label_values(&[priority]).inc();
}

/// Record job processing latency for a priority band.
pub fn record_queue_job_latency(duration: Duration, priority: &str) {
    QUEUE_JOB_LATENCY
        .with_label_values(&[priority])
        .observe(duration.as_secs_f64());
}

/// Update the pending queue depth gauges from a metrics snapshot.
///
/// Call this periodically (e.g. from a background task) to keep the
/// Prometheus gauges in sync with the actual queue state.
pub fn update_queue_depth_gauges(pending_by_priority: &[usize; 4]) {
    const BANDS: [&str; 4] = ["critical", "high", "normal", "low"];
    for (i, &depth) in pending_by_priority.iter().enumerate() {
        QUEUE_DEPTH.with_label_values(&[BANDS[i]]).set(depth as i64);
    }
}

/// Update the WFQ virtual clock gauge.
pub fn update_virtual_clock(value: i64) {
    QUEUE_VIRTUAL_CLOCK
        .with_label_values(&["default"])
        .set(value);
}

// ── Indexer lag metric helpers ────────────────────────────────────────────────

/// Update all indexer lag gauges for a single source in one call.
///
/// Called by [`crate::indexer_lag::IndexerLagMonitor`] after each measurement.
pub fn update_indexer_lag(
    source: &str,
    lag_ledgers: u64,
    lag_seconds: f64,
    last_indexed_ledger: u64,
    horizon_ledger: u64,
    status: crate::indexer_lag::SyncStatus,
) {
    INDEXER_LAG_LEDGERS
        .with_label_values(&[source])
        .set(lag_ledgers as i64);

    INDEXER_LAG_SECONDS
        .with_label_values(&[source])
        .set(lag_seconds);

    INDEXER_LAST_LEDGER
        .with_label_values(&[source])
        .set(last_indexed_ledger as i64);

    INDEXER_HORIZON_LEDGER
        .with_label_values(&["default"])
        .set(horizon_ledger as i64);

    INDEXER_SYNC_STATUS
        .with_label_values(&[source])
        .set(status.as_gauge_value());
}

/// Get cache hit ratio for a given cache type
pub fn get_cache_hit_ratio(cache_type: &str) -> f64 {
    let hits = CACHE_HITS.with_label_values(&[cache_type]).get() as f64;
    let misses = CACHE_MISSES.with_label_values(&[cache_type]).get() as f64;
    let total = hits + misses;
    if total == 0.0 {
        0.0
    } else {
        hits / total
    }
}

/// Encode metrics in Prometheus text format
pub fn encode_metrics() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer)?;
    Ok(String::from_utf8(buffer)?)
}
