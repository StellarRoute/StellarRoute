//! Dynamic routing-source weighting based on recent execution quality.
//!
//! # Overview
//!
//! Maintains per-source exponential moving averages (EMAs) for two signals:
//! - **quality_score** – a [0.0, 1.0] measure of route execution fidelity
//!   (e.g. slippage vs. quoted price, fill rate, route consistency).
//! - **error_rate** – fraction of recent requests that resulted in an error.
//!
//! These are combined into a single *reliability weight* that is fed back into
//! [`crate::consensus::ConsensusPolicy::source_weights`].
//!
//! ## Bounds
//! Weights are clamped to [`WeightBounds`] so that no single source can
//! dominate or be starved entirely, preventing runaway feedback loops.
//!
//! ## Observability
//! Every weight update is emitted as a structured `tracing::info!` event so
//! that log aggregators (Grafana Loki, Datadog, etc.) can alert on drift.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{info, warn};

// ── Configuration ────────────────────────────────────────────────────────────

/// Safe bounds for source weights.  Ensures no source is fully silenced
/// and no single source becomes an uncontested monopoly.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WeightBounds {
    /// Minimum allowed weight for any source (default 0.1).
    pub min: f64,
    /// Maximum allowed weight for any source (default 0.95).
    pub max: f64,
}

impl Default for WeightBounds {
    fn default() -> Self {
        Self { min: 0.1, max: 0.95 }
    }
}

/// Configuration for the quality-tracker EMA.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QualityTrackerConfig {
    /// EMA smoothing factor α ∈ (0, 1].  Larger = more responsive to recent
    /// observations, smaller = smoother / more stable.  Default 0.1.
    pub ema_alpha: f64,
    /// Initial weight assigned to unknown sources.
    pub default_weight: f64,
    /// Clamp parameters so weights stay in a safe operating range.
    pub bounds: WeightBounds,
    /// How much the error-rate signal reduces the weight relative to quality.
    /// Combined weight = quality_ema * (1 - error_penalty * error_rate_ema).
    pub error_penalty: f64,
}

impl Default for QualityTrackerConfig {
    fn default() -> Self {
        Self {
            ema_alpha: 0.1,
            default_weight: 0.5,
            bounds: WeightBounds::default(),
            error_penalty: 0.5,
        }
    }
}

// ── Per-source state ──────────────────────────────────────────────────────────

/// Live EMA state for a single routing source.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceState {
    /// Source identifier (matches keys in ConsensusPolicy::source_weights).
    pub source: String,
    /// EMA of quality scores ∈ [0.0, 1.0].
    pub quality_ema: f64,
    /// EMA of error indicator (1.0 on error, 0.0 on success).
    pub error_rate_ema: f64,
    /// Derived weight after applying bounds.
    pub weight: f64,
    /// Total observations recorded for this source.
    pub observation_count: u64,
}

/// An execution quality observation for a single source.
#[derive(Clone, Debug)]
pub struct QualityObservation {
    /// Source being measured.
    pub source: String,
    /// Quality score for this execution ∈ [0.0, 1.0].
    pub quality_score: f64,
    /// Whether this observation was an error (true) or success (false).
    pub is_error: bool,
}

// ── Tracker ───────────────────────────────────────────────────────────────────

/// Thread-safe tracker that maintains EMA state and derives routing weights.
///
/// # Example
/// ```rust
/// use stellarroute_routing::execution_quality::{ExecutionQualityTracker, QualityObservation, QualityTrackerConfig};
///
/// let tracker = ExecutionQualityTracker::new(QualityTrackerConfig::default());
/// tracker.record(QualityObservation { source: "amm".into(), quality_score: 0.9, is_error: false });
/// let weights = tracker.current_weights();
/// assert!(weights.contains_key("amm"));
/// ```
pub struct ExecutionQualityTracker {
    config: QualityTrackerConfig,
    state: Arc<RwLock<HashMap<String, SourceState>>>,
}

impl ExecutionQualityTracker {
    /// Create a new tracker with the given config.
    pub fn new(config: QualityTrackerConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record a new quality observation for a source.
    ///
    /// Updates the EMA state and recomputes the source's weight.  The update
    /// is logged via `tracing::info!` for observability.
    pub fn record(&self, obs: QualityObservation) {
        if !(0.0..=1.0).contains(&obs.quality_score) {
            warn!(
                source = %obs.source,
                quality_score = obs.quality_score,
                "quality_score out of [0,1]; clamping"
            );
        }
        let quality_score = obs.quality_score.clamp(0.0, 1.0);
        let error_signal = if obs.is_error { 1.0 } else { 0.0 };
        let alpha = self.config.ema_alpha;

        let mut guard = self.state.write().expect("weight state lock poisoned");
        let entry = guard.entry(obs.source.clone()).or_insert_with(|| SourceState {
            source: obs.source.clone(),
            quality_ema: self.config.default_weight,
            error_rate_ema: 0.0,
            weight: self.config.default_weight,
            observation_count: 0,
        });

        let prev_weight = entry.weight;
        entry.quality_ema = alpha * quality_score + (1.0 - alpha) * entry.quality_ema;
        entry.error_rate_ema = alpha * error_signal + (1.0 - alpha) * entry.error_rate_ema;
        entry.observation_count += 1;

        let raw_weight = entry.quality_ema * (1.0 - self.config.error_penalty * entry.error_rate_ema);
        entry.weight = raw_weight.clamp(self.config.bounds.min, self.config.bounds.max);

        info!(
            source = %obs.source,
            quality_ema = entry.quality_ema,
            error_rate_ema = entry.error_rate_ema,
            prev_weight = prev_weight,
            new_weight = entry.weight,
            observation_count = entry.observation_count,
            "source weight updated"
        );
    }

    /// Return a snapshot of the current weight for every tracked source.
    pub fn current_weights(&self) -> HashMap<String, f64> {
        self.state
            .read()
            .expect("weight state lock poisoned")
            .iter()
            .map(|(k, v)| (k.clone(), v.weight))
            .collect()
    }

    /// Return the full state snapshot for all tracked sources.
    pub fn source_states(&self) -> Vec<SourceState> {
        self.state
            .read()
            .expect("weight state lock poisoned")
            .values()
            .cloned()
            .collect()
    }

    /// Return the current weight for a single source, or `default_weight` if unseen.
    pub fn weight_for(&self, source: &str) -> f64 {
        self.state
            .read()
            .expect("weight state lock poisoned")
            .get(source)
            .map(|s| s.weight)
            .unwrap_or(self.config.default_weight)
    }

    /// Bulk-apply the current weights into a `ConsensusPolicy`-compatible map.
    ///
    /// Only sources that have been observed at least once are included; unknown
    /// sources will fall back to `ConsensusPolicy`'s built-in default.
    pub fn apply_to_policy_weights(&self, weights: &mut HashMap<String, f64>) {
        for (source, weight) in self.current_weights() {
            weights.insert(source, weight);
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tracker() -> ExecutionQualityTracker {
        ExecutionQualityTracker::new(QualityTrackerConfig {
            ema_alpha: 0.5,
            default_weight: 0.5,
            bounds: WeightBounds { min: 0.1, max: 0.95 },
            error_penalty: 0.5,
        })
    }

    #[test]
    fn test_new_source_starts_at_default() {
        let tracker = make_tracker();
        assert_eq!(tracker.weight_for("amm"), 0.5);
    }

    #[test]
    fn test_high_quality_raises_weight() {
        let tracker = make_tracker();
        for _ in 0..20 {
            tracker.record(QualityObservation {
                source: "amm".into(),
                quality_score: 1.0,
                is_error: false,
            });
        }
        let w = tracker.weight_for("amm");
        assert!(w > 0.5, "weight should increase with consistent high quality; got {w}");
        assert!(w <= 0.95, "weight must not exceed max bound; got {w}");
    }

    #[test]
    fn test_errors_reduce_weight() {
        let tracker = make_tracker();
        // Seed with decent quality first
        for _ in 0..10 {
            tracker.record(QualityObservation {
                source: "sdex".into(),
                quality_score: 0.8,
                is_error: false,
            });
        }
        let w_before = tracker.weight_for("sdex");

        // Now inject errors
        for _ in 0..10 {
            tracker.record(QualityObservation {
                source: "sdex".into(),
                quality_score: 0.2,
                is_error: true,
            });
        }
        let w_after = tracker.weight_for("sdex");
        assert!(w_after < w_before, "errors should reduce weight; before={w_before}, after={w_after}");
    }

    #[test]
    fn test_weight_never_below_min_bound() {
        let tracker = make_tracker();
        for _ in 0..100 {
            tracker.record(QualityObservation {
                source: "bad".into(),
                quality_score: 0.0,
                is_error: true,
            });
        }
        assert!(tracker.weight_for("bad") >= 0.1);
    }

    #[test]
    fn test_weight_never_above_max_bound() {
        let tracker = make_tracker();
        for _ in 0..100 {
            tracker.record(QualityObservation {
                source: "perfect".into(),
                quality_score: 1.0,
                is_error: false,
            });
        }
        assert!(tracker.weight_for("perfect") <= 0.95);
    }

    #[test]
    fn test_apply_to_policy_weights() {
        let tracker = make_tracker();
        tracker.record(QualityObservation {
            source: "amm".into(),
            quality_score: 0.9,
            is_error: false,
        });
        let mut map = HashMap::new();
        map.insert("sdex".to_string(), 0.7_f64);
        tracker.apply_to_policy_weights(&mut map);
        assert!(map.contains_key("amm"), "amm should be injected");
        assert!(map.contains_key("sdex"), "sdex should be preserved");
    }

    #[test]
    fn test_observation_count_increments() {
        let tracker = make_tracker();
        for i in 1..=5u64 {
            tracker.record(QualityObservation {
                source: "amm".into(),
                quality_score: 0.5,
                is_error: false,
            });
            let states = tracker.source_states();
            let amm = states.iter().find(|s| s.source == "amm").unwrap();
            assert_eq!(amm.observation_count, i);
        }
    }

    #[test]
    fn test_out_of_range_quality_clamped() {
        let tracker = make_tracker();
        // Should not panic
        tracker.record(QualityObservation {
            source: "amm".into(),
            quality_score: 1.5,
            is_error: false,
        });
        tracker.record(QualityObservation {
            source: "amm".into(),
            quality_score: -0.3,
            is_error: false,
        });
        let w = tracker.weight_for("amm");
        assert!(w >= 0.1 && w <= 0.95);
    }

    #[test]
    fn test_static_vs_dynamic_weighting() {
        // Static: always weight 0.5 for amm, 0.5 for sdex
        let static_amm = 0.5_f64;
        let static_sdex = 0.5_f64;

        // Dynamic: amm gets high quality, sdex gets errors
        let tracker = make_tracker();
        for _ in 0..30 {
            tracker.record(QualityObservation {
                source: "amm".into(),
                quality_score: 0.95,
                is_error: false,
            });
            tracker.record(QualityObservation {
                source: "sdex".into(),
                quality_score: 0.2,
                is_error: true,
            });
        }

        let dynamic_amm = tracker.weight_for("amm");
        let dynamic_sdex = tracker.weight_for("sdex");

        // Dynamic weights should diverge from static
        assert!(dynamic_amm > static_amm, "dynamic amm weight should exceed static after good performance");
        assert!(dynamic_sdex < static_sdex, "dynamic sdex weight should fall below static after poor performance");
    }
}
