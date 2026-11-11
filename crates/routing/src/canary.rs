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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimizer::{OptimizerPolicy, RouteMetrics};
    use crate::pathfinder::{PathHop, SwapPath};

    /// Build a single-hop fixture path so `OptimizerDiagnostics` can be built
    /// without touching live liquidity data.
    fn fixture_path() -> SwapPath {
        SwapPath {
            hops: vec![PathHop {
                source_asset: "XLM".to_string(),
                destination_asset: "USDC".to_string(),
                venue_type: "sdex".to_string(),
                venue_ref: "sdex_xlm_usdc".to_string(),
                price: 0.1052,
                fee_bps: 30,
                provider: None,
                bridge: None,
            }],
            estimated_output: 1_000_000,
        }
    }

    /// Fixture diagnostics used as canary input. Only `output_amount`, `score`
    /// and `total_compute_time_ms` participate in the comparison, so the rest
    /// is fixed to keep the fixture deterministic.
    fn fixture_diagnostics(
        output_amount: i128,
        score: f64,
        compute_time_ms: u64,
    ) -> OptimizerDiagnostics {
        OptimizerDiagnostics {
            selected_path: fixture_path(),
            metrics: RouteMetrics {
                output_amount,
                impact_bps: 12,
                compute_time_us: compute_time_ms * 1_000,
                hop_count: 1,
                score,
                anomaly_score: 0.0,
                anomaly_reasons: vec![],
            },
            alternatives: vec![],
            policy: OptimizerPolicy::default(),
            total_compute_time_ms: compute_time_ms,
            excluded_routes: vec![],
            active_scorer_name: "default".to_string(),
            flagged_venues: vec![],
            non_finite_scores_dropped: 0,
        }
    }

    /// Evaluate the default (production) thresholds against a baseline and a
    /// candidate quote.
    fn evaluate_default(
        baseline_output: i128,
        candidate_output: i128,
        baseline_ms: u64,
        candidate_ms: u64,
    ) -> CanaryEvaluation {
        let config = CanaryConfig::default();
        CanaryEvaluator::evaluate(
            &config,
            &fixture_diagnostics(baseline_output, 0.9, baseline_ms),
            &fixture_diagnostics(candidate_output, 0.9, candidate_ms),
            "XLM",
            "USDC",
            1_000,
        )
    }

    // ── Default config must stay off ────────────────────────────────────────

    #[test]
    fn test_default_config_is_disabled() {
        let config = CanaryConfig::default();
        assert!(
            !config.enabled,
            "canary must default to disabled so production routing is untouched"
        );
    }

    #[test]
    fn test_default_config_thresholds_and_policies_are_unchanged() {
        let config = CanaryConfig::default();
        assert_eq!(config.baseline_policy, "production");
        assert_eq!(config.candidate_policy, "testing");
        assert_eq!(config.max_latency_drift_ms, 50);
        assert_eq!(config.max_output_drift_bps, 10);
        assert_eq!(config.rollback_trigger_threshold, 5);
        assert!((config.evaluation_rate - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn test_default_config_round_trips_through_json() {
        let config = CanaryConfig::default();
        let json = serde_json::to_string(&config).expect("canary config should serialize");
        let decoded: CanaryConfig =
            serde_json::from_str(&json).expect("canary config should deserialize");
        assert!(!decoded.enabled);
        assert_eq!(decoded.baseline_policy, config.baseline_policy);
        assert_eq!(decoded.candidate_policy, config.candidate_policy);
        assert_eq!(decoded.max_latency_drift_ms, config.max_latency_drift_ms);
        assert_eq!(decoded.max_output_drift_bps, config.max_output_drift_bps);
        assert_eq!(
            decoded.rollback_trigger_threshold,
            config.rollback_trigger_threshold
        );
        assert!((decoded.evaluation_rate - config.evaluation_rate).abs() < f64::EPSILON);
    }

    // ── Equal quotes match ──────────────────────────────────────────────────

    #[test]
    fn test_equal_quotes_produce_match() {
        let evaluation = evaluate_default(1_000_000, 1_000_000, 12, 12);
        assert_eq!(evaluation.latency_drift_ms, 0);
        assert_eq!(evaluation.output_drift_bps, 0);
        assert!(!evaluation.is_violation, "equal quotes must not violate");
        assert!(
            evaluation.violation_reasons.is_empty(),
            "equal quotes must not report reasons"
        );
    }

    #[test]
    fn test_equal_quotes_preserve_context_fields() {
        let evaluation = evaluate_default(1_000_000, 1_000_000, 12, 12);
        assert_eq!(evaluation.base_asset, "XLM");
        assert_eq!(evaluation.quote_asset, "USDC");
        assert_eq!(evaluation.amount_in, 1_000);
        assert_eq!(evaluation.baseline_latency_ms, 12);
        assert_eq!(evaluation.candidate_latency_ms, 12);
        assert!((evaluation.baseline_score - 0.9).abs() < f64::EPSILON);
        assert!((evaluation.candidate_score - 0.9).abs() < f64::EPSILON);
        assert!(evaluation.timestamp > 0, "timestamp must be populated");
    }

    // ── Mismatched price produces a typed diff ──────────────────────────────

    #[test]
    fn test_mismatched_price_produces_output_drift_violation() {
        // 1_100 stroops worse on 1_000_000 => 11 bps drift (> 10 bps budget).
        let evaluation = evaluate_default(1_000_000, 998_900, 12, 12);
        assert_eq!(evaluation.output_drift_bps, 11);
        assert!(evaluation.is_violation, "price regression must violate");
        assert_eq!(evaluation.violation_reasons.len(), 1);
        assert!(
            evaluation.violation_reasons[0].contains("Output drift"),
            "expected a typed output-drift diff, got {:?}",
            evaluation.violation_reasons
        );
    }

    #[test]
    fn test_latency_regression_produces_latency_drift_violation() {
        // +51 ms of drift (> 50 ms budget) with identical output.
        let evaluation = evaluate_default(1_000_000, 1_000_000, 10, 61);
        assert_eq!(evaluation.latency_drift_ms, 51);
        assert!(evaluation.is_violation, "latency regression must violate");
        assert_eq!(evaluation.violation_reasons.len(), 1);
        assert!(
            evaluation.violation_reasons[0].contains("Latency drift"),
            "expected a typed latency-drift diff, got {:?}",
            evaluation.violation_reasons
        );
    }

    #[test]
    fn test_both_regressions_report_two_typed_reasons() {
        let evaluation = evaluate_default(1_000_000, 998_900, 10, 61);
        assert!(evaluation.is_violation);
        assert_eq!(evaluation.violation_reasons.len(), 2);
        assert!(evaluation
            .violation_reasons
            .iter()
            .any(|r| r.contains("Output drift")));
        assert!(evaluation
            .violation_reasons
            .iter()
            .any(|r| r.contains("Latency drift")));
    }

    // ── Boundaries: thresholds are exclusive ────────────────────────────────

    #[test]
    fn test_drift_exactly_at_threshold_is_not_a_violation() {
        // Exactly 10 bps output drift and exactly 50 ms latency drift.
        let evaluation = evaluate_default(1_000_000, 999_000, 10, 60);
        assert_eq!(evaluation.output_drift_bps, 10);
        assert_eq!(evaluation.latency_drift_ms, 50);
        assert!(
            !evaluation.is_violation,
            "drift at (not above) the budget must pass, got {:?}",
            evaluation.violation_reasons
        );
    }

    #[test]
    fn test_small_drift_within_budget_passes() {
        // 5 bps output drift, +20 ms latency drift: both inside the budget.
        let evaluation = evaluate_default(1_000_000, 999_500, 10, 30);
        assert_eq!(evaluation.output_drift_bps, 5);
        assert_eq!(evaluation.latency_drift_ms, 20);
        assert!(!evaluation.is_violation);
    }

    #[test]
    fn test_candidate_beating_baseline_is_never_a_violation() {
        // Better output yields negative drift, and lower latency is fine too.
        let evaluation = evaluate_default(1_000_000, 1_050_000, 40, 10);
        assert_eq!(evaluation.output_drift_bps, -500);
        assert_eq!(evaluation.latency_drift_ms, -30);
        assert!(
            !evaluation.is_violation,
            "candidate improvement must not violate, got {:?}",
            evaluation.violation_reasons
        );
    }

    // ── Degenerate inputs ───────────────────────────────────────────────────

    #[test]
    fn test_zero_baseline_output_reports_zero_drift() {
        // A zero baseline has no denominator, so output drift must stay 0
        // instead of dividing by zero or reporting a bogus violation.
        let evaluation = evaluate_default(0, 0, 12, 12);
        assert_eq!(evaluation.output_drift_bps, 0);
        assert!(!evaluation.is_violation);
    }

    #[test]
    fn test_wide_config_budget_absorbs_regression() {
        // Same regression as the violation test, but with a relaxed budget.
        let mut config = CanaryConfig::default();
        config.max_output_drift_bps = 50;
        config.max_latency_drift_ms = 200;
        let evaluation = CanaryEvaluator::evaluate(
            &config,
            &fixture_diagnostics(1_000_000, 0.9, 10),
            &fixture_diagnostics(998_900, 0.9, 61),
            "XLM",
            "USDC",
            1_000,
        );
        assert!(!evaluation.is_violation);
        assert!(evaluation.violation_reasons.is_empty());
    }

    // ── Evaluation payload stability ────────────────────────────────────────

    #[test]
    fn test_evaluation_round_trips_through_json() {
        let evaluation = evaluate_default(1_000_000, 998_900, 10, 61);
        let json = serde_json::to_string(&evaluation).expect("evaluation should serialize");
        let decoded: CanaryEvaluation =
            serde_json::from_str(&json).expect("evaluation should deserialize");
        assert_eq!(decoded.base_asset, evaluation.base_asset);
        assert_eq!(decoded.quote_asset, evaluation.quote_asset);
        assert_eq!(decoded.amount_in, evaluation.amount_in);
        assert_eq!(decoded.latency_drift_ms, evaluation.latency_drift_ms);
        assert_eq!(decoded.output_drift_bps, evaluation.output_drift_bps);
        assert_eq!(decoded.is_violation, evaluation.is_violation);
        assert_eq!(decoded.violation_reasons, evaluation.violation_reasons);
    }

    #[test]
    fn test_evaluation_json_field_names_are_stable() {
        // Locks the wire field names so the admin/compare surfaces cannot
        // silently drift through a rename.
        let evaluation = evaluate_default(1_000_000, 1_000_000, 12, 12);
        let value = serde_json::to_value(&evaluation).expect("evaluation should serialize");
        let obj = value
            .as_object()
            .expect("evaluation should be a JSON object");
        for field in [
            "timestamp",
            "base_asset",
            "quote_asset",
            "amount_in",
            "baseline_score",
            "candidate_score",
            "baseline_latency_ms",
            "candidate_latency_ms",
            "latency_drift_ms",
            "output_drift_bps",
            "is_violation",
            "violation_reasons",
        ] {
            assert!(
                obj.contains_key(field),
                "missing field `{field}` in canary evaluation JSON"
            );
        }
    }

    #[test]
    fn test_evaluate_does_not_mutate_config() {
        let config = CanaryConfig::default();
        let before = serde_json::to_string(&config).expect("canary config should serialize");
        let _ = CanaryEvaluator::evaluate(
            &config,
            &fixture_diagnostics(1_000_000, 0.9, 10),
            &fixture_diagnostics(998_900, 0.9, 61),
            "XLM",
            "USDC",
            1_000,
        );
        let after = serde_json::to_string(&config).expect("canary config should serialize");
        assert_eq!(before, after, "evaluate must not mutate canary config");
    }

    #[test]
    fn test_evaluate_does_not_flip_the_enabled_flag() {
        // Canary stays off unless an operator explicitly turns it on: a
        // comparison run must never self-enable.
        let config = CanaryConfig::default();
        let _ = CanaryEvaluator::evaluate(
            &config,
            &fixture_diagnostics(1_000_000, 0.9, 10),
            &fixture_diagnostics(998_900, 0.9, 61),
            "XLM",
            "USDC",
            1_000,
        );
        assert!(!config.enabled);
    }
}
