//! Background reconciliation job for quote cache vs live compute drift
//!
//! Periodically compares cached quotes with fresh compute results to detect
//! drift and trigger corrective actions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::cache::CacheManager;
use crate::error::Result;
use crate::models::QuoteResponse;
use crate::state::AppState;

lazy_static::lazy_static! {
    /// Counter for drift detection events
    pub static ref DRIFT_DETECTIONS: prometheus::IntCounterVec = prometheus::register_int_counter_vec!(
        "stellarroute_quote_drift_detections_total",
        "Number of drift detections between cached and live quotes",
        &["severity"]
    ).expect("Can't create DRIFT_DETECTIONS counter");

    /// Gauge for current drift magnitude
    pub static ref DRIFT_MAGNITUDE: prometheus::GaugeVec = prometheus::register_gauge_vec!(
        "stellarroute_quote_drift_magnitude",
        "Current drift magnitude between cached and live quotes",
        &["pair"]
    ).expect("Can't create DRIFT_MAGNITUDE gauge");

    /// Counter for cache invalidations triggered by drift
    pub static ref DRIFT_INVALIDATIONS: prometheus::IntCounter = prometheus::register_int_counter!(
        "stellarroute_quote_drift_invalidations_total",
        "Number of cache invalidations triggered by drift detection"
    ).expect("Can't create DRIFT_INVALIDATIONS counter");
}

/// Configuration for the reconciliation job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationConfig {
    /// Interval between reconciliation runs
    pub interval_secs: u64,
    /// Sample rate (0.0 to 1.0) for checking cached quotes
    pub sample_rate: f64,
    /// Price drift threshold (percentage) for triggering invalidation
    pub drift_threshold_pct: f64,
    /// Maximum drift threshold before alerting
    pub alert_threshold_pct: f64,
    /// Maximum number of samples per run
    pub max_samples_per_run: usize,
    /// Whether to automatically invalidate on drift
    pub auto_invalidate: bool,
}

impl Default for ReconciliationConfig {
    fn default() -> Self {
        Self {
            interval_secs: 60,
            sample_rate: 0.1,
            drift_threshold_pct: 0.5,
            alert_threshold_pct: 2.0,
            max_samples_per_run: 100,
            auto_invalidate: true,
        }
    }
}

/// Result of a single reconciliation check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationResult {
    /// Trading pair checked
    pub pair: String,
    /// Cached price
    pub cached_price: f64,
    /// Live computed price
    pub live_price: f64,
    /// Drift percentage
    pub drift_pct: f64,
    /// Whether drift exceeded threshold
    pub exceeded_threshold: bool,
    /// Timestamp of the check
    pub timestamp: DateTime<Utc>,
    /// Action taken
    pub action: ReconciliationAction,
}

/// Action taken during reconciliation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReconciliationAction {
    None,
    CacheInvalidated,
    AlertTriggered,
    Logged,
}

/// Summary of a reconciliation run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationSummary {
    /// Number of samples checked
    pub samples_checked: usize,
    /// Number of drifts detected
    pub drifts_detected: usize,
    /// Number of cache invalidations
    pub invalidations: usize,
    /// Number of alerts triggered
    pub alerts: usize,
    /// Maximum drift observed
    pub max_drift_pct: f64,
    /// Average drift observed
    pub avg_drift_pct: f64,
    /// Timestamp of the run
    pub timestamp: DateTime<Utc>,
    /// Duration of the run in milliseconds
    pub duration_ms: u64,
}

/// Background reconciliation job manager
pub struct ReconciliationJob {
    config: ReconciliationConfig,
    state: Arc<AppState>,
    running: Arc<std::sync::atomic::AtomicBool>,
    last_run: Arc<RwLock<Option<DateTime<Utc>>>>,
    summary: Arc<RwLock<Option<ReconciliationSummary>>>,
}

impl ReconciliationJob {
    pub fn new(config: ReconciliationConfig, state: Arc<AppState>) -> Self {
        Self {
            config,
            state,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            last_run: Arc::new(RwLock::new(None)),
            summary: Arc::new(RwLock::new(None)),
        }
    }

    /// Start the background reconciliation job
    pub fn start(self: Arc<Self>) {
        if self
            .running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            warn!("Reconciliation job already running");
            return;
        }

        info!(
            interval_secs = self.config.interval_secs,
            sample_rate = self.config.sample_rate,
            "Starting quote cache reconciliation job"
        );

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(Duration::from_secs(self.config.interval_secs));

            loop {
                interval.tick().await;

                if !self.running.load(Ordering::Relaxed) {
                    break;
                }

                if let Err(e) = self.run_reconciliation().await {
                    error!("Reconciliation job failed: {:?}", e);
                }

                let mut last_run = self.last_run.write().await;
                *last_run = Some(Utc::now());
            }
        });
    }

    /// Stop the reconciliation job
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Run a single reconciliation pass
    async fn run_reconciliation(&self) -> Result<()> {
        let start = std::time::Instant::now();
        let mut results = Vec::new();

        // Get sample of cached quotes to check
        let samples = self.sample_cached_quotes().await?;

        for sample in samples.into_iter().take(self.config.max_samples_per_run) {
            if let Some(result) = self.check_single_quote(&sample).await? {
                results.push(result);
            }
        }

        // Process results and take actions
        let summary = self.process_results(&results, start.elapsed()).await;

        // Update summary
        let mut current_summary = self.summary.write().await;
        *current_summary = Some(summary);

        Ok(())
    }

    /// Sample cached quotes for reconciliation
    async fn sample_cached_quotes(&self) -> Result<Vec<CachedQuoteSample>> {
        // This would typically query Redis for cached quote keys
        // For now, return empty vec - implementation depends on cache structure
        Ok(Vec::new())
    }

    /// Check a single cached quote against live computation
    async fn check_single_quote(&self, sample: &CachedQuoteSample) -> Result<Option<ReconciliationResult>> {
        // This would:
        // 1. Parse the cached quote
        // 2. Compute a fresh quote for the same parameters
        // 3. Compare prices and calculate drift
        // For now, return None - implementation depends on quote computation
        Ok(None)
    }

    /// Process reconciliation results and take actions
    async fn process_results(
        &self,
        results: &[ReconciliationResult],
        duration: Duration,
    ) -> ReconciliationSummary {
        let drifts: Vec<_> = results.iter().filter(|r| r.exceeded_threshold).collect();
        let invalidations = results
            .iter()
            .filter(|r| r.action == ReconciliationAction::CacheInvalidated)
            .count();
        let alerts = results
            .iter()
            .filter(|r| r.action == ReconciliationAction::AlertTriggered)
            .count();

        let max_drift = results
            .iter()
            .map(|r| r.drift_pct)
            .fold(0.0, |a, b| a.max(b));

        let avg_drift = if results.is_empty() {
            0.0
        } else {
            results.iter().map(|r| r.drift_pct).sum::<f64>() / results.len() as f64
        };

        // Record metrics
        for result in results {
            let severity = if result.drift_pct >= self.config.alert_threshold_pct {
                "high"
            } else if result.drift_pct >= self.config.drift_threshold_pct {
                "medium"
            } else {
                "low"
            };
            DRIFT_DETECTIONS
                .with_label_values(&[severity])
                .inc();
            DRIFT_MAGNITUDE
                .with_label_values(&[&result.pair])
                .set(result.drift_pct);
        }

        if invalidations > 0 {
            DRIFT_INVALIDATIONS.inc_by(invalidations as u64);
        }

        ReconciliationSummary {
            samples_checked: results.len(),
            drifts_detected: drifts.len(),
            invalidations,
            alerts,
            max_drift_pct: max_drift,
            avg_drift_pct: avg_drift,
            timestamp: Utc::now(),
            duration_ms: duration.as_millis() as u64,
        }
    }

    /// Get the last reconciliation summary
    pub async fn get_summary(&self) -> Option<ReconciliationSummary> {
        self.summary.read().await.clone()
    }
}

/// Sample of a cached quote for reconciliation
#[derive(Debug, Clone)]
struct CachedQuoteSample {
    key: String,
    base: String,
    quote: String,
    amount: f64,
    cached_price: f64,
}

/// Calculate drift percentage between cached and live prices
pub fn calculate_drift(cached: f64, live: f64) -> f64 {
    if cached == 0.0 {
        return if live == 0.0 { 0.0 } else { 100.0 };
    }
    ((live - cached).abs() / cached) * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_drift_handles_zero() {
        assert_eq!(calculate_drift(0.0, 0.0), 0.0);
        assert_eq!(calculate_drift(0.0, 1.0), 100.0);
    }

    #[test]
    fn calculate_drift_computes_correctly() {
        let drift = calculate_drift(100.0, 101.0);
        assert!((drift - 1.0).abs() < 0.001);
    }

    #[test]
    fn reconciliation_config_defaults_are_reasonable() {
        let config = ReconciliationConfig::default();
        assert!(config.interval_secs > 0);
        assert!(config.sample_rate > 0.0 && config.sample_rate <= 1.0);
        assert!(config.drift_threshold_pct > 0.0);
    }
}
