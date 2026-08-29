//! Prometheus metrics for the StellarRoute indexer.
//!
//! Exposes counters and gauges for:
//! - Horizon throttle events (429 responses)
//! - Throttle wait time
//! - Indexer lag
//! - AMM pool refresh failure streaks

use lazy_static::lazy_static;
use prometheus::{
    register_int_counter, register_int_counter_vec, register_int_gauge_vec, Encoder, IntCounter,
    IntCounterVec, IntGaugeVec, TextEncoder,
};

lazy_static! {
    /// Total number of Horizon 429 rate-limit responses received.
    pub static ref HORIZON_THROTTLE_EVENTS: IntCounter = register_int_counter!(
        "stellarroute_indexer_horizon_throttle_events_total",
        "Total number of Horizon 429 rate-limit responses received"
    )
    .expect("Can't create HORIZON_THROTTLE_EVENTS counter");

    /// Total milliseconds spent waiting due to Horizon rate-limiting.
    pub static ref HORIZON_THROTTLE_WAIT_MS: IntCounter = register_int_counter!(
        "stellarroute_indexer_horizon_throttle_wait_ms_total",
        "Total milliseconds spent waiting due to Horizon rate-limiting"
    )
    .expect("Can't create HORIZON_THROTTLE_WAIT_MS counter");

    /// Current consecutive 429 count (gauge, resets on success).
    pub static ref HORIZON_CONSECUTIVE_429S: IntGaugeVec = register_int_gauge_vec!(
        "stellarroute_indexer_horizon_consecutive_429s",
        "Current number of consecutive Horizon 429 responses",
        &["source"]
    )
    .expect("Can't create HORIZON_CONSECUTIVE_429S gauge");

    /// Indexer ingestion lag in ledgers.
    pub static ref INDEXER_LAG_LEDGERS: IntGaugeVec = register_int_gauge_vec!(
        "stellarroute_indexer_lag_ledgers",
        "Number of ledgers the local index is behind the live Horizon sequence",
        &["source"]
    )
    .expect("Can't create INDEXER_LAG_LEDGERS gauge");

    /// Total number of offers indexed from Horizon.
    pub static ref OFFERS_INDEXED: IntCounterVec = register_int_counter_vec!(
        "stellarroute_indexer_offers_indexed_total",
        "Total number of offers indexed from Horizon",
        &["source"]
    )
    .expect("Can't create OFFERS_INDEXED counter");

    /// Total number of SSE stream disconnects.
    pub static ref SSE_DISCONNECTS: IntCounterVec = register_int_counter_vec!(
        "stellarroute_indexer_sse_disconnects_total",
        "Total number of SSE stream disconnects",
        &["source"]
    )
    .expect("Can't create SSE_DISCONNECTS counter");

    /// Total number of SSE events received.
    pub static ref SSE_EVENTS_RECEIVED: IntCounterVec = register_int_counter_vec!(
        "stellarroute_indexer_sse_events_received_total",
        "Total number of SSE events received",
        &["source"]
    )
    .expect("Can't create SSE_EVENTS_RECEIVED counter");

    /// Number of times the AMM refresh loop has hit a run of consecutive
    /// failures long enough to be worth paging on.
    ///
    /// Incremented once per streak, when the consecutive-failure count reaches
    /// [`AMM_REFRESH_FAILURE_STREAK_THRESHOLD`], and again on every further
    /// failure while the streak continues. It never resets — use `rate()`/
    /// `increase()` to alert. Observability only: the refresh loop keeps its
    /// cadence and the process does not exit.
    pub static ref AMM_REFRESH_FAILURE_STREAKS: IntCounterVec = register_int_counter_vec!(
        "stellarroute_indexer_amm_refresh_failure_streaks_total",
        "Total AMM pool refresh cycles that failed while a consecutive-failure streak was active",
        &["source"]
    )
    .expect("Can't create AMM_REFRESH_FAILURE_STREAKS counter");

    /// Current number of consecutive AMM refresh failures (resets to 0 on the
    /// next successful cycle).
    pub static ref AMM_CONSECUTIVE_REFRESH_FAILURES: IntGaugeVec = register_int_gauge_vec!(
        "stellarroute_indexer_amm_consecutive_refresh_failures",
        "Current number of consecutive AMM pool refresh failures",
        &["source"]
    )
    .expect("Can't create AMM_CONSECUTIVE_REFRESH_FAILURES gauge");

    /// Queue depth per partition (placeholder for future implementation)
    pub static ref PARTITION_QUEUE_DEPTH: IntGaugeVec = register_int_gauge_vec!(
        "stellarroute_indexer_partition_queue_depth",
        "Queue depth per partition",
        &["partition"]
    )
    .expect("Can't create PARTITION_QUEUE_DEPTH gauge");

    /// Fairness score per partition (e.g., lag variance)
    pub static ref FAIRNESS_SCORE: IntGaugeVec = register_int_gauge_vec!(
        "stellarroute_indexer_fairness_score",
        "Fairness score per partition",
        &["partition"]
    )
    .expect("Can't create FAIRNESS_SCORE gauge");
}

/// Record a Horizon throttle event.
pub fn record_throttle_event(wait_ms: u64, consecutive: u64, source: &str) {
    HORIZON_THROTTLE_EVENTS.inc();
    HORIZON_THROTTLE_WAIT_MS.inc_by(wait_ms);
    HORIZON_CONSECUTIVE_429S
        .with_label_values(&[source])
        .set(consecutive as i64);
}

/// Reset the consecutive 429 gauge after a successful request.
pub fn record_throttle_success(source: &str) {
    HORIZON_CONSECUTIVE_429S.with_label_values(&[source]).set(0);
}

/// Update the indexer lag gauge.
pub fn update_lag(source: &str, lag_ledgers: i64) {
    INDEXER_LAG_LEDGERS
        .with_label_values(&[source])
        .set(lag_ledgers);
}

/// Record offers indexed.
pub fn record_offers_indexed(source: &str, count: u64) {
    OFFERS_INDEXED.with_label_values(&[source]).inc_by(count);
}

/// Record an SSE disconnect.
pub fn record_sse_disconnect(source: &str) {
    SSE_DISCONNECTS.with_label_values(&[source]).inc();
}

/// Record an SSE event received.
pub fn record_sse_event(source: &str) {
    SSE_EVENTS_RECEIVED.with_label_values(&[source]).inc();
}

/// Number of back-to-back AMM refresh failures before the streak counter starts
/// incrementing. One transient RPC blip should not page anyone; a sustained run
/// should.
pub const AMM_REFRESH_FAILURE_STREAK_THRESHOLD: u64 = 3;

/// Record a failed AMM refresh cycle.
///
/// `consecutive` is the running count of back-to-back failures including this
/// one. Once it reaches [`AMM_REFRESH_FAILURE_STREAK_THRESHOLD`] the streak
/// counter is incremented, so `increase(...[15m]) > 0` is a usable alert.
///
/// Observability only — the caller keeps polling on its normal interval.
pub fn record_amm_refresh_failure(source: &str, consecutive: u64) {
    AMM_CONSECUTIVE_REFRESH_FAILURES
        .with_label_values(&[source])
        .set(consecutive as i64);

    if consecutive >= AMM_REFRESH_FAILURE_STREAK_THRESHOLD {
        AMM_REFRESH_FAILURE_STREAKS
            .with_label_values(&[source])
            .inc();
    }
}

/// Reset the consecutive AMM refresh failure gauge after a successful cycle.
pub fn record_amm_refresh_success(source: &str) {
    AMM_CONSECUTIVE_REFRESH_FAILURES
        .with_label_values(&[source])
        .set(0);
}

/// Encode all metrics in Prometheus text format.
pub fn encode_metrics() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer)?;
    Ok(String::from_utf8(buffer)?)
}
