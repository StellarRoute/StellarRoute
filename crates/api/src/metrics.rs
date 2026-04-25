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

    /// Kill switch status gauge
    pub static ref KILL_SWITCH_STATUS: IntGaugeVec = register_int_gauge_vec!(
        "stellarroute_kill_switch_status",
        "Kill switch status (1 for disabled, 0 for enabled)",
        &["type", "name"]
    )
    .expect("Can't create KILL_SWITCH_STATUS gauge");
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
