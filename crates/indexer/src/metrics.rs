//! Prometheus metrics for the StellarRoute indexer.
//!
//! Exposes counters and gauges for:
//! - Horizon throttle events (429 responses)
//! - Throttle wait time
//! - Indexer lag

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
        "Total number of SSE events received from Horizon",
        &["source"]
    )
    .expect("Can't create SSE_EVENTS_RECEIVED counter");
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

/// Encode all metrics in Prometheus text format.
pub fn encode_metrics() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer)?;
    Ok(String::from_utf8(buffer)?)
}
