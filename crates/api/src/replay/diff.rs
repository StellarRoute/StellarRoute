//! Field-level diff between original and replayed quote outputs.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::artifact::ReplayArtifact;
use super::engine::ReplayOutput;

/// Tolerance for numeric string comparisons (avoids float formatting false positives).
const NUMERIC_TOLERANCE: f64 = 1e-7;

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// A single field that diverged between the original and replayed outputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDivergence {
    /// Dot-path of the diverging field, e.g. `"price"` or `"path[0].source"`.
    pub field: String,
    /// Value from the stored original output.
    pub original: serde_json::Value,
    /// Value produced by the replay engine.
    pub replayed: serde_json::Value,
}

/// Structured comparison of original vs replayed outputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffReport {
    pub artifact_id: Uuid,
    /// `true` when no divergences were found.
    pub is_identical: bool,
    /// List of fields that differ. Empty when `is_identical` is `true`.
    pub divergences: Vec<FieldDivergence>,
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// Compares a `ReplayOutput` against the `original_output` stored in an artifact.
pub struct DiffEngine;

impl DiffEngine {
    /// Compare `replay` against the original output embedded in `artifact`.
    ///
    /// Fields compared:
    /// - `price` (numeric tolerance `1e-7`)
    /// - `selected_source`
    /// - `path[0].source` (if path is non-empty)
    pub fn diff(artifact: &ReplayArtifact, replay: &ReplayOutput) -> DiffReport {
        let mut divergences: Vec<FieldDivergence> = Vec::new();
        let orig = &artifact.original_output;

        // ── price ────────────────────────────────────────────────────────────
        let orig_price = orig.get("price").cloned().unwrap_or(serde_json::Value::Null);
        let replay_price = serde_json::Value::String(replay.price.clone());
        if !numeric_values_equal(&orig_price, &replay_price) {
            divergences.push(FieldDivergence {
                field: "price".to_string(),
                original: orig_price,
                replayed: replay_price,
            });
        }

        // ── selected_source ──────────────────────────────────────────────────
        let orig_source = orig
            .get("selected_source")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let replay_source = serde_json::Value::String(replay.selected_source.clone());
        if orig_source != replay_source {
            divergences.push(FieldDivergence {
                field: "selected_source".to_string(),
                original: orig_source,
                replayed: replay_source,
            });
        }

        // ── path[0].source ───────────────────────────────────────────────────
        let orig_path_source = orig
            .get("path")
            .and_then(|p| p.get(0))
            .and_then(|s| s.get("source"))
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let replay_path_source = replay
            .path
            .first()
            .map(|s| serde_json::Value::String(s.source.clone()))
            .unwrap_or(serde_json::Value::Null);

        // Only compare if the original has a path (some artifacts may not)
        if orig_path_source != serde_json::Value::Null
            && orig_path_source != replay_path_source
        {
            divergences.push(FieldDivergence {
                field: "path[0].source".to_string(),
                original: orig_path_source,
                replayed: replay_path_source,
            });
        }

        let is_identical = divergences.is_empty();
        DiffReport {
            artifact_id: artifact.id,
            is_identical,
            divergences,
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns `true` if two JSON values represent numerically equal strings
/// (within `NUMERIC_TOLERANCE`), or are equal as JSON values.
fn numeric_values_equal(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    if a == b {
        return true;
    }
    // Try numeric comparison for string values
    if let (Some(sa), Some(sb)) = (a.as_str(), b.as_str()) {
        if let (Ok(fa), Ok(fb)) = (sa.parse::<f64>(), sb.parse::<f64>()) {
            return (fa - fb).abs() < NUMERIC_TOLERANCE;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AssetInfo, PathStep};
    use crate::replay::artifact::{
        HealthConfigSnapshot, LiquidityCandidate, ReplayArtifact, CURRENT_SCHEMA_VERSION,
    };
    use chrono::Utc;
    use proptest::prelude::*;

    fn make_artifact(price: &str, source: &str) -> ReplayArtifact {
        ReplayArtifact {
            id: Uuid::new_v4(),
            schema_version: CURRENT_SCHEMA_VERSION,
            incident_id: None,
            captured_at: Utc::now(),
            base: "native".to_string(),
            quote: "USDC:[REDACTED]".to_string(),
            amount: "100.0000000".to_string(),
            slippage_bps: 50,
            quote_type: "sell".to_string(),
            liquidity_snapshot: vec![LiquidityCandidate {
                venue_type: "sdex".to_string(),
                venue_ref: "offer1".to_string(),
                price: price.to_string(),
                available_amount: "100.0000000".to_string(),
            }],
            health_config_snapshot: HealthConfigSnapshot {
                freshness_threshold_secs_sdex: 30,
                freshness_threshold_secs_amm: 60,
                staleness_threshold_secs: 30,
                min_tvl_threshold_e7: 1_000_000_000,
            },
            original_output: serde_json::json!({
                "price": price,
                "selected_source": source,
            }),
        }
    }

    fn make_replay(artifact: &ReplayArtifact, price: &str, source: &str) -> ReplayOutput {
        ReplayOutput {
            artifact_id: artifact.id,
            selected_source: source.to_string(),
            price: price.to_string(),
            path: vec![PathStep {
                from_asset: AssetInfo::native(),
                to_asset: AssetInfo::native(),
                price: price.to_string(),
                source: source.to_string(),
            }],
            is_deterministic: true,
            replayed_at: Utc::now(),
        }
    }

    // ── Unit tests ──────────────────────────────────────────────────────────

    #[test]
    fn identical_outputs_produce_empty_diff() {
        let artifact = make_artifact("1.0000000", "sdex:offer1");
        let replay = make_replay(&artifact, "1.0000000", "sdex:offer1");
        let report = DiffEngine::diff(&artifact, &replay);
        assert!(report.is_identical);
        assert!(report.divergences.is_empty());
    }

    #[test]
    fn differing_price_produces_one_divergence() {
        let artifact = make_artifact("1.0000000", "sdex:offer1");
        let replay = make_replay(&artifact, "1.0500000", "sdex:offer1");
        let report = DiffEngine::diff(&artifact, &replay);
        assert!(!report.is_identical);
        assert_eq!(report.divergences.len(), 1);
        assert_eq!(report.divergences[0].field, "price");
    }

    #[test]
    fn prices_within_tolerance_are_equal() {
        let artifact = make_artifact("1.0000000", "sdex:offer1");
        // Difference of 5e-8 < 1e-7 tolerance
        let replay = make_replay(&artifact, "1.0000001", "sdex:offer1");
        let report = DiffEngine::diff(&artifact, &replay);
        // 1.0000000 vs 1.0000001 → diff = 1e-7, which is NOT strictly less than 1e-7
        // so this should diverge (boundary case)
        // Let's use a value clearly within tolerance
        let replay2 = make_replay(&artifact, "1.00000005", "sdex:offer1");
        let report2 = DiffEngine::diff(&artifact, &replay2);
        assert!(report2.is_identical, "diff within tolerance should be identical");
        let _ = report; // suppress unused warning
    }

    #[test]
    fn differing_selected_source_produces_divergence() {
        let artifact = make_artifact("1.0000000", "sdex:offer1");
        let replay = make_replay(&artifact, "1.0000000", "amm:pool1");
        let report = DiffEngine::diff(&artifact, &replay);
        assert!(!report.is_identical);
        assert!(report.divergences.iter().any(|d| d.field == "selected_source"));
    }

    // ── Property-based tests ────────────────────────────────────────────────

    proptest! {
        /// Property 4: Diff of identical outputs is empty (reflexivity).
        ///
        /// Feature: quote-replay-system, Property 4: diff of identical outputs is empty
        #[test]
        fn prop_diff_reflexive(
            price in "[0-9]{1,5}\\.[0-9]{7}",
            source in "[a-z]{3,6}:[a-z0-9]{4,12}",
        ) {
            let artifact = make_artifact(&price, &source);
            let replay = make_replay(&artifact, &price, &source);
            let report = DiffEngine::diff(&artifact, &replay);
            prop_assert!(report.is_identical, "self-diff must be identical");
            prop_assert!(report.divergences.is_empty());
        }

        /// Property 6: Numeric diff tolerance — values within 1e-7 are treated as equal.
        ///
        /// Feature: quote-replay-system, Property 6: numeric diff tolerance
        #[test]
        fn prop_numeric_diff_within_tolerance(base in 0.001f64..1_000.0f64) {
            let a = serde_json::Value::String(format!("{:.7}", base));
            let b = serde_json::Value::String(format!("{:.7}", base + 5e-8));
            prop_assert!(numeric_values_equal(&a, &b),
                "values within tolerance should be equal: {} vs {}", a, b);
        }
    }
}
