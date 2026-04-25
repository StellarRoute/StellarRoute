//! Integration tests for quote freshness guardrails (Issue #150).
//!
//! These tests verify the four acceptance criteria:
//!   AC1 – Quote generation checks source timestamps against freshness threshold.
//!   AC2 – Stale data returns a typed error response (HTTP 422, error="stale_market_data").
//!   AC3 – Metrics track stale-rejection and stale-inputs-excluded counts independently.
//!   AC4 – Tests cover mixed-freshness input scenarios.
//!
//! All tests are pure (no live database or network) so they run in CI without infrastructure.

use chrono::{Duration, Utc};
use stellarroute_api::{
    error::ApiError,
    models::{DataFreshness, ExclusionReason},
    state::CacheMetrics,
};
use stellarroute_routing::health::{
    freshness::FreshnessGuard,
    scorer::{FreshnessThresholds, VenueScorerInput, VenueType},
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn sdex_input(staleness_secs: i64) -> VenueScorerInput {
    let ts = if staleness_secs < 0 {
        // Future timestamp → 0 staleness (treated as fresh)
        Some(Utc::now() + Duration::seconds(-staleness_secs))
    } else {
        Some(Utc::now() - Duration::seconds(staleness_secs))
    };
    VenueScorerInput {
        venue_ref: format!("sdex:offer-{}", staleness_secs),
        venue_type: VenueType::Sdex,
        best_bid_e7: Some(9_990_000),
        best_ask_e7: Some(10_010_000),
        depth_top_n_e7: Some(5_000_000_000),
        reserve_a_e7: None,
        reserve_b_e7: None,
        tvl_e7: None,
        last_updated_at: ts,
    }
}

fn amm_input(staleness_secs: i64) -> VenueScorerInput {
    let ts = Some(Utc::now() - Duration::seconds(staleness_secs));
    VenueScorerInput {
        venue_ref: format!("amm:pool-{}", staleness_secs),
        venue_type: VenueType::Amm,
        best_bid_e7: None,
        best_ask_e7: None,
        depth_top_n_e7: None,
        reserve_a_e7: Some(1_000_000_000),
        reserve_b_e7: Some(1_000_000_000),
        tvl_e7: Some(2_000_000_000),
        last_updated_at: ts,
    }
}

fn default_thresholds() -> FreshnessThresholds {
    FreshnessThresholds { sdex: 30, amm: 60 }
}

// ---------------------------------------------------------------------------
// AC1: Source timestamps are checked against per-venue-type thresholds
// ---------------------------------------------------------------------------

/// SDEX inputs older than 30 s are classified stale; AMM inputs older than 60 s are stale.
/// This confirms that quote generation would exclude such inputs before pricing.
#[test]
fn freshness_guard_applies_sdex_threshold_of_30_seconds() {
    let now = Utc::now();
    let thresholds = default_thresholds();

    let inputs = vec![
        sdex_input(29), // 29 s old → fresh (≤ 30 s)
        sdex_input(31), // 31 s old → stale (> 30 s)
    ];
    let outcome = FreshnessGuard::evaluate(&inputs, &thresholds, now);

    assert_eq!(outcome.fresh, vec![0], "29 s SDEX input must be fresh");
    assert_eq!(outcome.stale, vec![1], "31 s SDEX input must be stale");
}

#[test]
fn freshness_guard_applies_amm_threshold_of_60_seconds() {
    let now = Utc::now();
    let thresholds = default_thresholds();

    let inputs = vec![
        amm_input(59), // 59 s old → fresh (≤ 60 s)
        amm_input(61), // 61 s old → stale (> 60 s)
    ];
    let outcome = FreshnessGuard::evaluate(&inputs, &thresholds, now);

    assert_eq!(outcome.fresh, vec![0], "59 s AMM input must be fresh");
    assert_eq!(outcome.stale, vec![1], "61 s AMM input must be stale");
}

/// SDEX and AMM thresholds are evaluated independently.
/// An input at 45 s is stale for SDEX (> 30 s) but fresh for AMM (≤ 60 s).
#[test]
fn freshness_guard_sdex_and_amm_thresholds_are_independent() {
    let now = Utc::now();
    let thresholds = default_thresholds();

    let inputs = vec![
        sdex_input(45), // stale for SDEX (threshold 30)
        amm_input(45),  // fresh for AMM  (threshold 60)
    ];
    let outcome = FreshnessGuard::evaluate(&inputs, &thresholds, now);

    assert_eq!(outcome.stale, vec![0], "45 s SDEX must be stale");
    assert_eq!(outcome.fresh, vec![1], "45 s AMM must be fresh");
}

/// Input with no timestamp is unconditionally stale (Requirement 2.5).
#[test]
fn freshness_guard_missing_timestamp_is_always_stale() {
    let now = Utc::now();
    let thresholds = default_thresholds();

    let inputs = vec![VenueScorerInput {
        venue_ref: "sdex:no-ts".to_string(),
        venue_type: VenueType::Sdex,
        best_bid_e7: Some(9_990_000),
        best_ask_e7: Some(10_010_000),
        depth_top_n_e7: Some(5_000_000_000),
        reserve_a_e7: None,
        reserve_b_e7: None,
        tvl_e7: None,
        last_updated_at: None, // missing
    }];
    let outcome = FreshnessGuard::evaluate(&inputs, &thresholds, now);

    assert!(
        outcome.fresh.is_empty(),
        "input with no timestamp must not be fresh"
    );
    assert_eq!(outcome.stale, vec![0]);
    assert_eq!(
        outcome.max_staleness_secs,
        u64::MAX,
        "missing timestamp must give MAX staleness"
    );
}

/// Inputs at exactly the threshold boundary are considered fresh (≤, not <).
#[test]
fn freshness_guard_at_threshold_boundary_is_fresh() {
    let now = Utc::now();
    let thresholds = default_thresholds();

    let inputs = vec![
        sdex_input(30), // exactly 30 s → fresh
        amm_input(60),  // exactly 60 s → fresh
    ];
    let outcome = FreshnessGuard::evaluate(&inputs, &thresholds, now);

    assert_eq!(
        outcome.fresh,
        vec![0, 1],
        "inputs exactly at threshold must be fresh"
    );
    assert!(outcome.stale.is_empty());
}

// ---------------------------------------------------------------------------
// AC2: Stale data returns a typed error response
// ---------------------------------------------------------------------------

/// ApiError::StaleMarketData serialises all required detail fields.
#[tokio::test]
async fn stale_market_data_error_produces_typed_json_details() {
    use axum::{body::to_bytes, response::IntoResponse};

    let err = ApiError::StaleMarketData {
        stale_count: 3,
        fresh_count: 0,
        threshold_secs_sdex: 30,
        threshold_secs_amm: 60,
    };
    let resp = err.into_response();
    let status = resp.status().as_u16();
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        status, 422,
        "all-stale quote must return HTTP 422 Unprocessable Entity"
    );
    assert_eq!(json["v"], 1, "envelope version must be 1");
    assert_eq!(
        json["data"]["error"], "stale_market_data",
        "error field must be stale_market_data"
    );
    assert_eq!(json["data"]["details"]["stale_count"], 3);
    assert_eq!(json["data"]["details"]["fresh_count"], 0);
    assert_eq!(json["data"]["details"]["threshold_secs_sdex"], 30);
    assert_eq!(json["data"]["details"]["threshold_secs_amm"], 60);
}

/// ExclusionReason::StaleData serialises with tag type = "stale_data" (snake_case).
#[test]
fn stale_data_exclusion_reason_serializes_as_snake_case_type_tag() {
    let reason = ExclusionReason::StaleData;
    let json = serde_json::to_value(&reason).expect("serialize ExclusionReason::StaleData");
    assert_eq!(
        json["type"], "stale_data",
        "StaleData must serialize as {{\"type\": \"stale_data\"}}"
    );
}

/// ExclusionReason::PolicyThreshold serialises with its threshold value.
#[test]
fn policy_threshold_exclusion_reason_serializes_with_threshold() {
    let reason = ExclusionReason::PolicyThreshold { threshold: 0.5 };
    let json = serde_json::to_value(&reason).expect("serialize PolicyThreshold");
    assert_eq!(json["type"], "policy_threshold");
    assert!(
        (json["threshold"].as_f64().unwrap() - 0.5).abs() < f64::EPSILON,
        "threshold value must be preserved"
    );
}

// ---------------------------------------------------------------------------
// AC3: Metrics track stale-rejection and stale-inputs-excluded counts
// ---------------------------------------------------------------------------

/// stale_quote_rejections increments exactly once per all-stale rejection.
#[test]
fn metrics_stale_rejection_counter_tracks_full_rejections() {
    let m = CacheMetrics::default();
    let (rej, _) = m.snapshot_staleness();
    assert_eq!(rej, 0);

    m.inc_stale_rejection(); // quote 1: all stale
    m.inc_stale_rejection(); // quote 2: all stale again

    let (rej, excl) = m.snapshot_staleness();
    assert_eq!(
        rej, 2,
        "two full-rejection events must increment counter twice"
    );
    assert_eq!(
        excl, 0,
        "stale_inputs_excluded must not move on full rejections"
    );
}

/// stale_inputs_excluded accumulates the count of stale inputs across multiple quotes.
#[test]
fn metrics_stale_inputs_excluded_tracks_partial_exclusions() {
    let m = CacheMetrics::default();

    m.add_stale_inputs_excluded(2); // quote 1 excluded 2 stale inputs
    m.add_stale_inputs_excluded(1); // quote 2 excluded 1 stale input

    let (rej, excl) = m.snapshot_staleness();
    assert_eq!(
        excl, 3,
        "stale_inputs_excluded must accumulate across calls"
    );
    assert_eq!(
        rej, 0,
        "stale_quote_rejections must not move on partial exclusions"
    );
}

/// The two staleness counters are entirely independent.
#[test]
fn metrics_staleness_counters_are_independent() {
    let m = CacheMetrics::default();

    m.inc_stale_rejection(); // 1 full rejection
    m.add_stale_inputs_excluded(5); // 5 partially excluded inputs

    let (rej, excl) = m.snapshot_staleness();
    assert_eq!(rej, 1);
    assert_eq!(excl, 5);
}

/// stale_inputs_excluded is NOT incremented when stale_count == 0 (all fresh).
#[test]
fn metrics_stale_inputs_excluded_not_incremented_when_all_fresh() {
    let m = CacheMetrics::default();

    // Simulate the guard in get_quote(): only add when stale_count > 0
    let stale_count = 0usize;
    if stale_count > 0 {
        m.add_stale_inputs_excluded(stale_count as u64);
    }

    let (_, excl) = m.snapshot_staleness();
    assert_eq!(excl, 0);
}

// ---------------------------------------------------------------------------
// AC4: Mixed-freshness input scenarios
// ---------------------------------------------------------------------------

/// Mixed inputs: the fresh subset is identified correctly; stale inputs appear in a separate list.
#[test]
fn mixed_freshness_partitions_fresh_and_stale_correctly() {
    let now = Utc::now();
    let thresholds = default_thresholds();

    // Index 0: SDEX 10 s old  → fresh
    // Index 1: AMM  90 s old  → stale
    // Index 2: SDEX 5 s old   → fresh
    let inputs = vec![sdex_input(10), amm_input(90), sdex_input(5)];
    let outcome = FreshnessGuard::evaluate(&inputs, &thresholds, now);

    assert_eq!(outcome.fresh, vec![0, 2], "indices 0 and 2 are fresh");
    assert_eq!(outcome.stale, vec![1], "index 1 (90 s AMM) is stale");
    assert_eq!(outcome.max_staleness_secs, 90);
}

/// DataFreshness correctly reflects a mixed outcome: stale_count > 0, fresh_count > 0.
#[test]
fn data_freshness_populated_from_mixed_freshness_outcome() {
    let now = Utc::now();
    let thresholds = default_thresholds();

    let inputs = vec![
        sdex_input(10), // fresh
        amm_input(90),  // stale
        sdex_input(20), // fresh
    ];
    let outcome = FreshnessGuard::evaluate(&inputs, &thresholds, now);

    // Mirror the mapping done in get_quote()
    let df = DataFreshness {
        fresh_count: outcome.fresh.len(),
        stale_count: outcome.stale.len(),
        max_staleness_secs: outcome.max_staleness_secs,
    };

    assert_eq!(df.fresh_count, 2, "two fresh inputs in mixed scenario");
    assert_eq!(df.stale_count, 1, "one stale input in mixed scenario");
    assert_eq!(df.max_staleness_secs, 90);
}

/// When ALL inputs are fresh, data_freshness.stale_count must be 0.
#[test]
fn data_freshness_stale_count_zero_when_all_inputs_fresh() {
    let now = Utc::now();
    let thresholds = default_thresholds();

    let inputs = vec![sdex_input(5), amm_input(10), sdex_input(15)];
    let outcome = FreshnessGuard::evaluate(&inputs, &thresholds, now);

    let df = DataFreshness {
        fresh_count: outcome.fresh.len(),
        stale_count: outcome.stale.len(),
        max_staleness_secs: outcome.max_staleness_secs,
    };

    assert_eq!(
        df.stale_count, 0,
        "all-fresh inputs must produce stale_count of zero"
    );
    assert_eq!(df.fresh_count, 3);
}

/// When ALL inputs are stale, fresh is empty → triggers StaleMarketData path.
/// The stale_quote_rejections counter must be incremented exactly once.
#[test]
fn all_stale_inputs_increments_rejection_counter_once() {
    let m = CacheMetrics::default();

    // Simulate the guard in find_best_price() after all inputs stale
    let now = Utc::now();
    let thresholds = default_thresholds();
    let inputs = vec![sdex_input(60), amm_input(120)]; // both stale
    let outcome = FreshnessGuard::evaluate(&inputs, &thresholds, now);

    assert!(outcome.fresh.is_empty(), "all inputs must be stale");

    // Simulate the rejection branch in find_best_price()
    if outcome.fresh.is_empty() && !inputs.is_empty() {
        m.inc_stale_rejection();
    }

    let (rej, _) = m.snapshot_staleness();
    assert_eq!(rej, 1, "exactly one rejection must be recorded");
}

/// Mixed inputs: after freshness filtering, stale_inputs_excluded is incremented by stale count.
#[test]
fn mixed_freshness_increments_excluded_counter_by_stale_count() {
    let m = CacheMetrics::default();

    let now = Utc::now();
    let thresholds = default_thresholds();

    // 1 fresh SDEX + 2 stale AMM inputs
    let inputs = vec![sdex_input(5), amm_input(90), amm_input(120)];
    let outcome = FreshnessGuard::evaluate(&inputs, &thresholds, now);

    assert_eq!(outcome.stale.len(), 2, "two AMM inputs should be stale");

    // Simulate what get_quote() does
    let stale_count = outcome.stale.len();
    if stale_count > 0 {
        m.add_stale_inputs_excluded(stale_count as u64);
    }

    let (rej, excl) = m.snapshot_staleness();
    assert_eq!(
        excl, 2,
        "two stale inputs must be recorded in stale_inputs_excluded"
    );
    assert_eq!(
        rej, 0,
        "no full rejection occurred — some inputs were fresh"
    );
}

/// max_staleness_secs in FreshnessOutcome is the maximum across all evaluated inputs.
#[test]
fn max_staleness_secs_is_maximum_across_all_inputs() {
    let now = Utc::now();
    let thresholds = default_thresholds();

    let inputs = vec![
        sdex_input(5),  //   5 s old
        amm_input(90),  //  90 s old
        sdex_input(45), //  45 s old
    ];
    let outcome = FreshnessGuard::evaluate(&inputs, &thresholds, now);

    assert_eq!(
        outcome.max_staleness_secs, 90,
        "max_staleness_secs must equal the oldest input's age"
    );
}
