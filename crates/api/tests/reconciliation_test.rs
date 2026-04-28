//! Integration tests for quote cache reconciliation (Issue #432)
//!
//! These tests verify that the background reconciliation job correctly
//! detects drift between cached quotes and live compute results.

use stellarroute_api::reconciliation::{
    calculate_drift, ReconciliationAction, ReconciliationConfig, ReconciliationResult,
};

#[test]
fn calculate_drift_handles_equal_prices() {
    let drift = calculate_drift(100.0, 100.0);
    assert!((drift - 0.0).abs() < 0.001, "Equal prices should have zero drift");
}

#[test]
fn calculate_drift_handles_small_difference() {
    let drift = calculate_drift(100.0, 100.5);
    assert!((drift - 0.5).abs() < 0.001, "Should calculate 0.5% drift");
}

#[test]
fn calculate_drift_handles_large_difference() {
    let drift = calculate_drift(100.0, 110.0);
    assert!((drift - 10.0).abs() < 0.001, "Should calculate 10% drift");
}

#[test]
fn calculate_drift_handles_zero_cached_price() {
    let drift = calculate_drift(0.0, 100.0);
    assert_eq!(drift, 100.0, "Zero cached price should return 100% drift");
}

#[test]
fn calculate_drift_handles_both_zero() {
    let drift = calculate_drift(0.0, 0.0);
    assert_eq!(drift, 0.0, "Both zero should return zero drift");
}

#[test]
fn calculate_drift_handles_negative_drift() {
    let drift = calculate_drift(100.0, 99.0);
    assert!((drift - 1.0).abs() < 0.001, "Should calculate 1% drift for price decrease");
}

#[test]
fn reconciliation_config_defaults_are_reasonable() {
    let config = ReconciliationConfig::default();

    assert!(config.interval_secs > 0, "Interval should be positive");
    assert!(
        config.sample_rate > 0.0 && config.sample_rate <= 1.0,
        "Sample rate should be between 0 and 1"
    );
    assert!(
        config.drift_threshold_pct > 0.0,
        "Drift threshold should be positive"
    );
    assert!(
        config.alert_threshold_pct > config.drift_threshold_pct,
        "Alert threshold should be higher than drift threshold"
    );
    assert!(
        config.max_samples_per_run > 0,
        "Max samples should be positive"
    );
}

#[test]
fn reconciliation_result_serialization() {
    let result = ReconciliationResult {
        pair: "native/USDC".to_string(),
        cached_price: 100.0,
        live_price: 100.5,
        drift_pct: 0.5,
        exceeded_threshold: false,
        timestamp: chrono::Utc::now(),
        action: ReconciliationAction::None,
    };

    let json = serde_json::to_string(&result).expect("should serialize");
    let restored: ReconciliationResult = serde_json::from_str(&json).expect("should deserialize");

    assert_eq!(result.pair, restored.pair);
    assert_eq!(result.cached_price, restored.cached_price);
    assert_eq!(result.drift_pct, restored.drift_pct);
}

#[test]
fn reconciliation_action_equality() {
    assert_eq!(ReconciliationAction::None, ReconciliationAction::None);
    assert_eq!(
        ReconciliationAction::CacheInvalidated,
        ReconciliationAction::CacheInvalidated
    );
    assert_ne!(ReconciliationAction::None, ReconciliationAction::AlertTriggered);
}

#[test]
fn drift_threshold_detection() {
    let config = ReconciliationConfig {
        drift_threshold_pct: 0.5,
        ..Default::default()
    };

    // Below threshold
    let drift_below = calculate_drift(100.0, 100.3);
    assert!(drift_below < config.drift_threshold_pct);

    // Above threshold
    let drift_above = calculate_drift(100.0, 101.0);
    assert!(drift_above > config.drift_threshold_pct);
}

#[test]
fn alert_threshold_detection() {
    let config = ReconciliationConfig {
        alert_threshold_pct: 2.0,
        ..Default::default()
    };

    // Below alert threshold
    let drift_below = calculate_drift(100.0, 101.0);
    assert!(drift_below < config.alert_threshold_pct);

    // Above alert threshold
    let drift_above = calculate_drift(100.0, 103.0);
    assert!(drift_above > config.alert_threshold_pct);
}
