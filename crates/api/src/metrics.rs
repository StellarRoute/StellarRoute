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

    /// Redis infrastructure errors (connection, timeout, etc.) — distinct from cache misses.
    pub static ref REDIS_ERRORS: IntCounterVec = register_int_counter_vec!(
        "stellarroute_redis_errors_total",
        "Total Redis infrastructure errors by operation",
        &["operation"]
    )
    .expect("Can't create REDIS_ERRORS counter");

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

    /// Total single-flight coalesced requests (stampede prevention).
    pub static ref SINGLE_FLIGHT_COALESCED: IntCounterVec = register_int_counter_vec!(
        "stellarroute_single_flight_coalesced_total",
        "Total requests coalesced by single-flight (stampede prevention)",
        &["type"]
    )
    .expect("Can't create SINGLE_FLIGHT_COALESCED counter");

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

    // ── Dependency circuit breaker metrics ────────────────────────────────

    /// Whether a dependency's circuit breaker is currently open (1) or not (0).
    /// Labels: dependency (horizon / soroban_rpc / database)
    pub static ref DEPENDENCY_BREAKER_OPEN: IntGaugeVec = register_int_gauge_vec!(
        "stellarroute_dependency_breaker_open",
        "Dependency circuit breaker state: 1 = open (failing fast), 0 = closed/half-open",
        &["dependency"]
    )
    .expect("Can't create DEPENDENCY_BREAKER_OPEN gauge");

    /// Requests rejected on the live path because a dependency breaker was open.
    /// Labels: dependency (horizon / soroban_rpc / database)
    pub static ref DEPENDENCY_FAIL_FAST: IntCounterVec = register_int_counter_vec!(
        "stellarroute_dependency_fail_fast_total",
        "Requests rejected fast because a dependency circuit breaker was open",
        &["dependency"]
    )
    .expect("Can't create DEPENDENCY_FAIL_FAST counter");

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

    // ── Health score recomputation metrics ──────────────────────────────────

    /// Histogram of health score recomputation job duration.
    pub static ref HEALTH_SCORE_JOB_DURATION: HistogramVec = register_histogram_vec!(
        "stellarroute_health_score_job_duration_seconds",
        "Duration of health score recomputation cycles",
        &[],
        vec![0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0]
    )
    .expect("Can't create HEALTH_SCORE_JOB_DURATION histogram");

    /// Counter of health score recomputation failures.
    pub static ref HEALTH_SCORE_JOB_FAILURES: IntCounterVec = register_int_counter_vec!(
        "stellarroute_health_score_job_failures_total",
        "Total number of health score recomputation failures",
        &[]
    )
    .expect("Can't create HEALTH_SCORE_JOB_FAILURES counter");
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

/// Record a Redis infrastructure error (connection loss, timeout, etc.).
pub fn record_redis_error(operation: &str) {
    REDIS_ERRORS.with_label_values(&[operation]).inc();
}

/// Snapshot total Redis errors across all operations (for JSON metrics endpoints).
pub fn redis_error_total() -> u64 {
    REDIS_ERRORS.with_label_values(&["get"]).get()
        + REDIS_ERRORS.with_label_values(&["get_json"]).get()
        + REDIS_ERRORS.with_label_values(&["set"]).get()
        + REDIS_ERRORS.with_label_values(&["set_json"]).get()
        + REDIS_ERRORS.with_label_values(&["delete"]).get()
        + REDIS_ERRORS.with_label_values(&["delete_by_pattern"]).get()
        + REDIS_ERRORS.with_label_values(&["health"]).get()
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

/// Record a single-flight coalesced request.
pub fn record_single_flight_coalesced(request_type: &str) {
    SINGLE_FLIGHT_COALESCED
        .with_label_values(&[request_type])
        .inc();
}

/// Record a single-flight unique request (not coalesced).
pub fn record_single_flight_unique(request_type: &str) {
    // For now, this is a no-op or could increment a different metric
    // If you don't have SINGLE_FLIGHT_UNIQUE counter, just log or ignore
    let _ = request_type; // Suppress unused warning
}

/// Record quote response size in bytes.
pub fn record_quote_response_bytes(bytes: usize) {
    // For now, this is a no-op or could record to a histogram
    // If you don't have a corresponding metric, just log or ignore
    let _ = bytes; // Suppress unused warning
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

// ── Health score metric helpers ─────────────────────────────────────────────

/// Record health score recomputation job duration.
pub fn record_health_score_duration(duration: Duration) {
    HEALTH_SCORE_JOB_DURATION
        .with_label_values::<&str>(&[])
        .observe(duration.as_secs_f64());
}

/// Increment the health score recomputation failure counter.
pub fn record_health_score_failure() {
    HEALTH_SCORE_JOB_FAILURES
        .with_label_values::<&str>(&[])
        .inc();
}

// ── Webhook metrics ───────────────────────────────────────────────────────────

lazy_static! {
    /// Webhook delivery success counter
    pub static ref WEBHOOK_DELIVERY_SUCCESS: IntCounterVec = register_int_counter_vec!(
        "stellarroute_webhook_delivery_success_total",
        "Total number of successful webhook deliveries",
        &["integrator_id"]
    ).expect("Can't create WEBHOOK_DELIVERY_SUCCESS counter");

    /// Webhook delivery failure counter
    pub static ref WEBHOOK_DELIVERY_FAILURE: IntCounterVec = register_int_counter_vec!(
        "stellarroute_webhook_delivery_failure_total",
        "Total number of failed webhook deliveries",
        &["integrator_id", "failure_reason"]
    ).expect("Can't create WEBHOOK_DELIVERY_FAILURE counter");

    /// Webhook delivery attempt counter
    pub static ref WEBHOOK_DELIVERY_ATTEMPT: IntCounterVec = register_int_counter_vec!(
        "stellarroute_webhook_delivery_attempt_total",
        "Total number of webhook delivery attempts",
        &["integrator_id", "attempt"]
    ).expect("Can't create WEBHOOK_DELIVERY_ATTEMPT counter");

    /// Webhook delivery duration histogram
    pub static ref WEBHOOK_DELIVERY_DURATION: HistogramVec = register_histogram_vec!(
        "stellarroute_webhook_delivery_duration_seconds",
        "Duration of webhook deliveries",
        &["integrator_id", "success"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    ).expect("Can't create WEBHOOK_DELIVERY_DURATION histogram");

    /// Webhook poll cycle duration histogram
    pub static ref WEBHOOK_POLL_CYCLE_DURATION: HistogramVec = register_histogram_vec!(
        "stellarroute_webhook_poll_cycle_duration_seconds",
        "Duration of webhook poll cycles",
        &[]
    ).expect("Can't create WEBHOOK_POLL_CYCLE_DURATION histogram");

    /// Webhook pending quotes gauge
    pub static ref WEBHOOK_PENDING_QUOTES: IntGaugeVec = register_int_gauge_vec!(
        "stellarroute_webhook_pending_quotes_total",
        "Number of quotes pending webhook delivery",
        &[]
    ).expect("Can't create WEBHOOK_PENDING_QUOTES gauge");
}

lazy_static! {
    // ── Canary live-compare metrics ───────────────────────────────────────────────

    /// Latest canary quote divergence from Horizon reference price, in basis points.
    /// Label: pair (e.g. "native/USDC:GA5Z…")
    /// Updated on every POST /api/v1/system/canary/live-compare call.
    /// Set to 0 when outcome is "error" (divergence unknown).
    pub static ref CANARY_QUOTE_DIVERGENCE_BPS: prometheus::GaugeVec = prometheus::register_gauge_vec!(
        "stellarroute_canary_quote_divergence_bps",
        "Latest canary quote divergence from Horizon reference price in basis points",
        &["pair"]
    )
    .expect("Can't create CANARY_QUOTE_DIVERGENCE_BPS gauge");

    /// Total canary live-comparison runs by outcome.
    /// Labels: pair, outcome ("ok" | "diverged" | "error")
    pub static ref CANARY_COMPARISON_TOTAL: IntCounterVec = register_int_counter_vec!(
        "stellarroute_canary_comparison_total",
        "Total canary live-comparison runs by outcome",
        &["pair", "outcome"]
    )
    .expect("Can't create CANARY_COMPARISON_TOTAL counter");
}

pub fn record_webhook_delivery_success(integrator_id: &str, duration: Duration) {
    WEBHOOK_DELIVERY_SUCCESS
        .with_label_values(&[integrator_id])
        .inc();
    WEBHOOK_DELIVERY_DURATION
        .with_label_values(&[integrator_id, "true"])
        .observe(duration.as_secs_f64());
}

pub fn record_webhook_delivery_failure(
    integrator_id: &str,
    failure_reason: &str,
    duration: Duration,
) {
    WEBHOOK_DELIVERY_FAILURE
        .with_label_values(&[integrator_id, failure_reason])
        .inc();
    WEBHOOK_DELIVERY_DURATION
        .with_label_values(&[integrator_id, "false"])
        .observe(duration.as_secs_f64());
}

pub fn record_webhook_delivery_attempt(integrator_id: &str, attempt: u32) {
    WEBHOOK_DELIVERY_ATTEMPT
        .with_label_values(&[integrator_id, &attempt.to_string()])
        .inc();
}

pub fn update_webhook_pending_quotes(count: i64) {
    WEBHOOK_PENDING_QUOTES
        .with_label_values::<&str>(&[])
        .set(count);
}

pub fn record_webhook_poll_cycle_duration(duration: Duration) {
    WEBHOOK_POLL_CYCLE_DURATION
        .with_label_values::<&str>(&[])
        .observe(duration.as_secs_f64());
}

// ── Swap prepare/submit metrics ─────────────────────────────────────────────

lazy_static! {
    /// Swap prepare request counter.
    ///
    /// Labels:
    /// - `outcome`: "success" or "error"
    /// - `error_class`: machine-readable error category (or "none" on success)
    ///
    /// Error classes:
    /// - `none`                 — successful prepare
    /// - `validation`           — request validation failed
    /// - `quote_expired`        — the referenced quote is stale/expired
    /// - `quote_not_found`      — referenced quote_id does not exist
    /// - `simulation_failed`    — Soroban simulation failed
    /// - `build_failed`         — transaction build failed
    /// - `timeout`              — upstream Soroban/Horizon timeout
    /// - `rpc_error`            — generic Soroban RPC error
    /// - `internal`             — internal/unexpected error
    pub static ref SWAP_PREPARE_REQUESTS: IntCounterVec = register_int_counter_vec!(
        "stellarroute_swap_prepare_total",
        "Total number of swap prepare requests",
        &["outcome", "error_class"]
    )
    .expect("Can't create SWAP_PREPARE_REQUESTS counter");

    /// Swap prepare latency histogram.
    ///
    /// Label: `outcome` ("success" or "error")
    pub static ref SWAP_PREPARE_LATENCY: HistogramVec = register_histogram_vec!(
        "stellarroute_swap_prepare_duration_seconds",
        "Swap prepare request duration in seconds",
        &["outcome"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    )
    .expect("Can't create SWAP_PREPARE_LATENCY histogram");

    /// Swap submit request counter.
    ///
    /// Labels:
    /// - `outcome`: "success" or "error"
    /// - `error_class`: machine-readable error category (or "none" on success)
    ///
    /// Error classes:
    /// - `none`                 — successful submission
    /// - `validation`           — request validation failed
    /// - `duplicate_quote`      — quote_id already submitted (idempotency violation)
    /// - `bad_signature`        — supplied signature is invalid
    /// - `insufficient_fee`     — transaction fee too low
    /// - `insufficient_balance` — source account lacks funds
    /// - `slippage_exceeded`    — on-chain execution exceeded slippage
    /// - `timeout`              — upstream Soroban/Horizon timeout
    /// - `rpc_error`            — generic RPC error
    /// - `internal`             — internal/unexpected error
    pub static ref SWAP_SUBMIT_REQUESTS: IntCounterVec = register_int_counter_vec!(
        "stellarroute_swap_submit_total",
        "Total number of swap submit requests",
        &["outcome", "error_class"]
    )
    .expect("Can't create SWAP_SUBMIT_REQUESTS counter");

    /// Swap submit latency histogram.
    ///
    /// Label: `outcome` ("success" or "error")
    pub static ref SWAP_SUBMIT_LATENCY: HistogramVec = register_histogram_vec!(
        "stellarroute_swap_submit_duration_seconds",
        "Swap submit request duration in seconds",
        &["outcome"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    )
    .expect("Can't create SWAP_SUBMIT_LATENCY histogram");

    /// Number of swap requests currently in flight.
    ///
    /// Label: `phase` ("prepare" or "submit")
    pub static ref SWAP_INFLIGHT: IntGaugeVec = register_int_gauge_vec!(
        "stellarroute_swap_inflight",
        "Current number of in-flight swap requests by phase",
        &["phase"]
    )
    .expect("Can't create SWAP_INFLIGHT gauge");

    /// CCTP saga state transitions.
    pub static ref CCTP_TRANSITIONS: IntCounterVec = register_int_counter_vec!(
        "stellarroute_cctp_transitions_total",
        "CCTP transfer state transitions",
        &["to_status"]
    )
    .expect("Can't create CCTP_TRANSITIONS counter");

    /// Iris client outcomes.
    pub static ref CCTP_IRIS_OUTCOMES: IntCounterVec = register_int_counter_vec!(
        "stellarroute_cctp_iris_outcomes_total",
        "CCTP Iris client outcomes",
        &["operation", "outcome"]
    )
    .expect("Can't create CCTP_IRIS_OUTCOMES counter");

    /// Iris request latency.
    pub static ref CCTP_IRIS_LATENCY: HistogramVec = register_histogram_vec!(
        "stellarroute_cctp_iris_duration_seconds",
        "CCTP Iris request latency",
        &["operation"],
        vec![0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0]
    )
    .expect("Can't create CCTP_IRIS_LATENCY histogram");

    pub static ref CCTP_INVALID_MESSAGE: IntCounterVec = register_int_counter_vec!(
        "stellarroute_cctp_invalid_message_total",
        "CCTP raw message validation failures",
        &["reason"]
    )
    .expect("Can't create CCTP_INVALID_MESSAGE counter");

    pub static ref CCTP_RATE_LIMITED: IntCounterVec = register_int_counter_vec!(
        "stellarroute_cctp_rate_limited_total",
        "CCTP Iris rate limit events",
        &["source"]
    )
    .expect("Can't create CCTP_RATE_LIMITED counter");

    pub static ref CCTP_VERIFIER_MISMATCH: IntCounterVec = register_int_counter_vec!(
        "stellarroute_cctp_verifier_mismatch_total",
        "CCTP burn verifier mismatches",
        &["kind"]
    )
    .expect("Can't create CCTP_VERIFIER_MISMATCH counter");

    pub static ref CCTP_PROVIDER_KILLED_NEW: IntCounterVec = register_int_counter_vec!(
        "stellarroute_cctp_provider_killed_new_transfer_total",
        "CCTP new transfers blocked by provider kill switch",
        &["provider"]
    )
    .expect("Can't create CCTP_PROVIDER_KILLED_NEW counter");

    pub static ref CCTP_ATTESTATION_VERIFY: IntCounterVec = register_int_counter_vec!(
        "stellarroute_cctp_attestation_verify_total",
        "CCTP attestation verification outcomes",
        &["reason"]
    )
    .expect("Can't create CCTP_ATTESTATION_VERIFY counter");

    pub static ref CCTP_IRIS_KEYS_REFRESH: IntCounterVec = register_int_counter_vec!(
        "stellarroute_cctp_iris_keys_refresh_total",
        "Iris public keys cache refresh outcomes",
        &["outcome", "detail"]
    )
    .expect("Can't create CCTP_IRIS_KEYS_REFRESH counter");

    pub static ref CCTP_ATTESTER_SNAPSHOT_REFRESH: IntCounterVec = register_int_counter_vec!(
        "stellarroute_cctp_attester_snapshot_refresh_total",
        "Destination attester snapshot refresh outcomes",
        &["destination", "outcome"]
    )
    .expect("Can't create CCTP_ATTESTER_SNAPSHOT_REFRESH counter");

    pub static ref CCTP_STELLAR_VERIFIER_READINESS: IntCounterVec = register_int_counter_vec!(
        "stellarroute_cctp_stellar_verifier_readiness_total",
        "Stellar CCTP verifier bootstrap readiness outcomes",
        &["component", "outcome"]
    )
    .expect("Can't create CCTP_STELLAR_VERIFIER_READINESS counter");

    pub static ref CCTP_ENDPOINT_OUTCOMES: IntCounterVec = register_int_counter_vec!(
        "stellarroute_cctp_endpoint_outcomes_total",
        "CCTP HTTP endpoint gate/handler outcomes",
        &["endpoint", "outcome"]
    )
    .expect("Can't create CCTP_ENDPOINT_OUTCOMES counter");

    pub static ref CCTP_DIRECTION_READINESS: IntCounterVec = register_int_counter_vec!(
        "stellarroute_cctp_direction_readiness_total",
        "CCTP direction readiness snapshots at bootstrap",
        &["direction", "outcome"]
    )
    .expect("Can't create CCTP_DIRECTION_READINESS counter");

    pub static ref CCTP_POLL_LEASE_OUTCOMES: IntCounterVec = register_int_counter_vec!(
        "stellarroute_cctp_poll_lease_outcomes_total",
        "CCTP status poll lease outcomes",
        &["outcome"]
    )
    .expect("Can't create CCTP_POLL_LEASE_OUTCOMES counter");
}

/// Record a swap prepare outcome.
///
/// `error_class` should be `"none"` for success; any other value maps to
/// `outcome="error"` and the literal class is preserved as a label.
pub fn record_swap_prepare(duration: Duration, error_class: &str) {
    let outcome_label = if error_class == "none" {
        "success"
    } else {
        "error"
    };

    SWAP_PREPARE_LATENCY
        .with_label_values(&[outcome_label])
        .observe(duration.as_secs_f64());

    SWAP_PREPARE_REQUESTS
        .with_label_values(&[outcome_label, error_class])
        .inc();
}

/// Record a swap submit outcome.
///
/// `error_class` should be `"none"` for success; any other value maps to
/// `outcome="error"` and the literal class is preserved as a label.
pub fn record_swap_submit(duration: Duration, error_class: &str) {
    let outcome_label = if error_class == "none" {
        "success"
    } else {
        "error"
    };

    SWAP_SUBMIT_LATENCY
        .with_label_values(&[outcome_label])
        .observe(duration.as_secs_f64());

    SWAP_SUBMIT_REQUESTS
        .with_label_values(&[outcome_label, error_class])
        .inc();
}

/// Increment the in-flight swap gauge for a phase.
pub fn swap_inflight_inc(phase: &str) {
    SWAP_INFLIGHT.with_label_values(&[phase]).inc();
}

/// Decrement the in-flight swap gauge for a phase.
pub fn swap_inflight_dec(phase: &str) {
    SWAP_INFLIGHT.with_label_values(&[phase]).dec();
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

// ── Canary live-compare metric helpers ───────────────────────────────────────

/// Update canary live-compare Prometheus metrics after a result is ingested.
///
/// `outcome` must be one of `"ok"`, `"diverged"`, `"error"`.
/// When outcome is `"error"`, `divergence_bps` should be `0.0` (unknown reading).
pub fn record_live_compare_result(pair: &str, divergence_bps: f64, outcome: &str) {
    CANARY_QUOTE_DIVERGENCE_BPS
        .with_label_values(&[pair])
        .set(divergence_bps);
    CANARY_COMPARISON_TOTAL
        .with_label_values(&[pair, outcome])
        .inc();
}

pub fn record_cctp_transition(to_status: &str) {
    CCTP_TRANSITIONS.with_label_values(&[to_status]).inc();
}

pub fn record_cctp_iris_latency(duration: Duration, operation: &str) {
    CCTP_IRIS_LATENCY
        .with_label_values(&[operation])
        .observe(duration.as_secs_f64());
    CCTP_IRIS_OUTCOMES
        .with_label_values(&[operation, "ok"])
        .inc();
}

pub fn record_cctp_invalid_message() {
    CCTP_INVALID_MESSAGE.with_label_values(&["corridor"]).inc();
}

pub fn record_cctp_rate_limited() {
    CCTP_RATE_LIMITED.with_label_values(&["iris"]).inc();
}

pub fn record_cctp_verifier_mismatch() {
    CCTP_VERIFIER_MISMATCH.with_label_values(&["burn"]).inc();
}

pub fn record_cctp_provider_killed_new_transfer() {
    CCTP_PROVIDER_KILLED_NEW
        .with_label_values(&["circle-cctp"])
        .inc();
}

pub fn record_cctp_attestation_verify(reason: &str) {
    CCTP_ATTESTATION_VERIFY.with_label_values(&[reason]).inc();
}

pub fn record_cctp_iris_keys_refresh(outcome: &str, detail: &str) {
    CCTP_IRIS_KEYS_REFRESH
        .with_label_values(&[outcome, detail])
        .inc();
}

pub fn record_cctp_attester_snapshot_refresh(destination: &str, outcome: &str) {
    CCTP_ATTESTER_SNAPSHOT_REFRESH
        .with_label_values(&[destination, outcome])
        .inc();
}

pub fn record_cctp_stellar_verifier_readiness(component: &str, outcome: &str) {
    CCTP_STELLAR_VERIFIER_READINESS
        .with_label_values(&[component, outcome])
        .inc();
}

pub fn record_cctp_endpoint_outcome(endpoint: &str, outcome: &str) {
    CCTP_ENDPOINT_OUTCOMES
        .with_label_values(&[endpoint, outcome])
        .inc();
}

pub fn record_cctp_direction_readiness(direction: &str, outcome: &str) {
    CCTP_DIRECTION_READINESS
        .with_label_values(&[direction, outcome])
        .inc();
}

pub fn record_cctp_poll_lease(outcome: &str) {
    CCTP_POLL_LEASE_OUTCOMES.with_label_values(&[outcome]).inc();
}
