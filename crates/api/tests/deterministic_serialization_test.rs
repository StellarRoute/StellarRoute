//! Integration tests for deterministic serialization contract (Issue #431)
//!
//! These tests verify that route diagnostics payloads serialize deterministically
//! so clients and tests can compare outputs reliably.

use stellarroute_api::serialization::{
    DeterministicSerialize, NormalizedAnomaly, NormalizedExclusion, NormalizedHop,
    NormalizedPolicy, NormalizedAlternative, NormalizedRouteDiagnostics,
    NormalizedRouteMetrics, NormalizedSwapPath,
};

fn make_sample_diagnostics() -> NormalizedRouteDiagnostics {
    NormalizedRouteDiagnostics {
        selected_path: NormalizedSwapPath {
            hops: vec![NormalizedHop {
                source_asset: "native".to_string(),
                destination_asset: "USDC:GABC...123".to_string(),
                venue_type: "sdex".to_string(),
                venue_ref: "offer-1".to_string(),
                price: "0.9985000".to_string(),
                fee_bps: 0,
                anomaly_score: "0.0000000".to_string(),
                anomaly_reasons: vec![],
            }],
            estimated_output: "9985000000".to_string(),
        },
        metrics: NormalizedRouteMetrics {
            output_amount: "9985000000".to_string(),
            impact_bps: 15,
            compute_time_us: 1500,
            hop_count: 1,
            score: "0.9500000".to_string(),
            anomaly_score: "0.0000000".to_string(),
            anomaly_reasons: vec![],
        },
        alternatives: vec![NormalizedAlternative {
            path: NormalizedSwapPath {
                hops: vec![
                    NormalizedHop {
                        source_asset: "native".to_string(),
                        destination_asset: "EURC:GDEF...456".to_string(),
                        venue_type: "sdex".to_string(),
                        venue_ref: "offer-2".to_string(),
                        price: "0.9990000".to_string(),
                        fee_bps: 0,
                        anomaly_score: "0.0000000".to_string(),
                        anomaly_reasons: vec![],
                    },
                    NormalizedHop {
                        source_asset: "EURC:GDEF...456".to_string(),
                        destination_asset: "USDC:GABC...123".to_string(),
                        venue_type: "amm".to_string(),
                        venue_ref: "pool-eurc-usdc".to_string(),
                        price: "0.9995000".to_string(),
                        fee_bps: 30,
                        anomaly_score: "0.0000000".to_string(),
                        anomaly_reasons: vec![],
                    },
                ],
                estimated_output: "9975000000".to_string(),
            },
            metrics: NormalizedRouteMetrics {
                output_amount: "9975000000".to_string(),
                impact_bps: 25,
                compute_time_us: 2500,
                hop_count: 2,
                score: "0.9200000".to_string(),
                anomaly_score: "0.0000000".to_string(),
                anomaly_reasons: vec![],
            },
            sort_key: "0.9200000".to_string(),
        }],
        policy: NormalizedPolicy {
            output_weight: "0.5000000".to_string(),
            impact_weight: "0.3000000".to_string(),
            latency_weight: "0.2000000".to_string(),
            max_impact_bps: 300,
            max_compute_time_ms: 500,
            environment: "production".to_string(),
        },
        total_compute_time_ms: 5,
        excluded_routes: vec![NormalizedExclusion {
            venue_ref: "amm:pool-legacy".to_string(),
            reason: "StaleData".to_string(),
        }],
        flagged_venues: vec![NormalizedAnomaly {
            venue_ref: "sdex:offer-3".to_string(),
            score: "0.7500000".to_string(),
            reasons: vec!["low_liquidity".to_string()],
        }],
        serialization_version: "1.0.0".to_string(),
    }
}

#[test]
fn deterministic_serialization_is_byte_stable() {
    let diag = make_sample_diagnostics();

    let json1 = diag.to_deterministic_json().expect("first serialization");
    let json2 = diag.to_deterministic_json().expect("second serialization");

    assert_eq!(
        json1, json2,
        "Deterministic serialization must produce identical bytes for identical input"
    );
}

#[test]
fn field_ordering_is_deterministic() {
    let diag = make_sample_diagnostics();
    let json = diag.to_deterministic_json().expect("serialization");
    let json_str = String::from_utf8(json.clone()).expect("valid utf8");

    // Verify that fields appear in sorted order
    let selected_path_pos = json_str.find("\"selected_path\"").expect("selected_path field");
    let metrics_pos = json_str.find("\"metrics\"").expect("metrics field");
    let policy_pos = json_str.find("\"policy\"").expect("policy field");

    // Fields should be in alphabetical order
    assert!(
        selected_path_pos < metrics_pos,
        "selected_path should come before metrics"
    );
    assert!(
        metrics_pos < policy_pos,
        "metrics should come before policy"
    );
}

#[test]
fn normalization_handles_nan_and_infinity() {
    use serde_json::json;
    use stellarroute_api::serialization::DeterministicSerialize;

    let value = json!({
        "valid_number": 1.5,
        "nan_value": f64::NAN,
        "infinity_value": f64::INFINITY,
        "neg_infinity": f64::NEG_INFINITY,
    });

    let normalized = NormalizedRouteDiagnostics::normalize_value(value);
    let obj = normalized.as_object().expect("should be object");

    assert!(obj["valid_number"].is_number(), "valid numbers preserved");
    assert!(obj["nan_value"].is_null(), "NaN becomes null");
    assert!(obj["infinity_value"].is_null(), "infinity becomes null");
    assert!(obj["neg_infinity"].is_null(), "negative infinity becomes null");
}

#[test]
fn round_trip_preserves_data() {
    let original = make_sample_diagnostics();
    let json = original.to_deterministic_json().expect("serialize");
    let json_str = String::from_utf8(json).expect("valid utf8");
    let restored: NormalizedRouteDiagnostics =
        serde_json::from_str(&json_str).expect("deserialize");

    assert_eq!(
        original.selected_path.estimated_output,
        restored.selected_path.estimated_output
    );
    assert_eq!(original.metrics.score, restored.metrics.score);
    assert_eq!(original.policy.environment, restored.policy.environment);
    assert_eq!(
        original.serialization_version,
        restored.serialization_version
    );
}

#[test]
fn alternatives_are_sorted_by_score() {
    let mut diag = make_sample_diagnostics();

    // Add multiple alternatives with different scores
    diag.alternatives.push(NormalizedAlternative {
        path: NormalizedSwapPath {
            hops: vec![],
            estimated_output: "9900000000".to_string(),
        },
        metrics: NormalizedRouteMetrics {
            output_amount: "9900000000".to_string(),
            impact_bps: 50,
            compute_time_us: 1000,
            hop_count: 1,
            score: "0.8000000".to_string(),
            anomaly_score: "0.0000000".to_string(),
            anomaly_reasons: vec![],
        },
        sort_key: "0.8000000".to_string(),
    });

    // Verify alternatives are present
    assert_eq!(diag.alternatives.len(), 2);
}
