use serde::{Deserialize, Serialize};

use crate::optimizer::OptimizerDiagnostics;

/// Configuration for the Canary routing pipeline.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CanaryConfig {
    /// Whether canary evaluation is globally enabled.
    pub enabled: bool,
    /// Baseline policy (e.g. "production").
    pub baseline_policy: String,
    /// Candidate policy to evaluate (e.g. "candidate_v2").
    pub candidate_policy: String,
    /// Maximum acceptable added latency in milliseconds.
    pub max_latency_drift_ms: i64,
    /// Maximum acceptable negative output drift in basis points (e.g. 5 means 0.05% worse).
    pub max_output_drift_bps: i64,
    /// Maximum consecutive violations before automatically disabling the canary.
    pub rollback_trigger_threshold: u32,
    /// Percentage of requests to evaluate (0.0 to 1.0).
    pub evaluation_rate: f64,
}

impl Default for CanaryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            baseline_policy: "production".to_string(),
            candidate_policy: "testing".to_string(),
            max_latency_drift_ms: 50,
            max_output_drift_bps: 10,
            rollback_trigger_threshold: 5,
            evaluation_rate: 0.1, // 10%
        }
    }
}

/// Result of a single canary evaluation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CanaryEvaluation {
    pub timestamp: i64,
    pub base_asset: String,
    pub quote_asset: String,
    pub amount_in: i128,
    pub baseline_score: f64,
    pub candidate_score: f64,
    pub baseline_latency_ms: u64,
    pub candidate_latency_ms: u64,
    pub latency_drift_ms: i64,
    pub output_drift_bps: i64,
    pub is_violation: bool,
    pub violation_reasons: Vec<String>,
}

pub struct CanaryEvaluator;

impl CanaryEvaluator {
    /// Compares candidate diagnostics against the baseline diagnostics.
    pub fn evaluate(
        config: &CanaryConfig,
        baseline: &OptimizerDiagnostics,
        candidate: &OptimizerDiagnostics,
        base_asset: &str,
        quote_asset: &str,
        amount_in: i128,
    ) -> CanaryEvaluation {
        let mut violation_reasons = Vec::new();

        // Calculate latency drift
        let baseline_latency = baseline.total_compute_time_ms;
        let candidate_latency = candidate.total_compute_time_ms;
        let latency_drift_ms = candidate_latency as i64 - baseline_latency as i64;

        if latency_drift_ms > config.max_latency_drift_ms {
            violation_reasons.push(format!(
                "Latency drift {}ms exceeds threshold {}ms",
                latency_drift_ms, config.max_latency_drift_ms
            ));
        }

        // Calculate output drift in basis points
        // If candidate outputs less than baseline, that's negative drift
        let baseline_output = baseline.metrics.output_amount as f64;
        let candidate_output = candidate.metrics.output_amount as f64;

        let output_drift_bps = if baseline_output > 0.0 {
            // How much *less* is the candidate? (positive BPS means candidate is worse)
            ((baseline_output - candidate_output) / baseline_output * 10000.0).round() as i64
        } else {
            0
        };

        if output_drift_bps > config.max_output_drift_bps {
            violation_reasons.push(format!(
                "Output drift {}bps exceeds threshold {}bps",
                output_drift_bps, config.max_output_drift_bps
            ));
        }

        CanaryEvaluation {
            timestamp: chrono::Utc::now().timestamp_millis(),
            base_asset: base_asset.to_string(),
            quote_asset: quote_asset.to_string(),
            amount_in,
            baseline_score: baseline.metrics.score,
            candidate_score: candidate.metrics.score,
            baseline_latency_ms: baseline_latency,
            candidate_latency_ms: candidate_latency,
            latency_drift_ms,
            output_drift_bps,
            is_violation: !violation_reasons.is_empty(),
            violation_reasons,
        }
    }
}
