//! Integration tests for quote compute budget enforcement (Issue #430)
//!
//! These tests verify that per-stage timing budgets are enforced correctly
//! and that overruns are properly detected and reported.

use std::time::Duration;
use stellarroute_api::budget::{
    BudgetConfig, BudgetResult, BudgetTracker, PipelineStage, StageGuard,
};

#[test]
fn budget_config_realtime_is_stricter_than_default() {
    let realtime = BudgetConfig::realtime();
    let default = BudgetConfig::default();

    assert!(
        realtime.fetch_candidates_ms < default.fetch_candidates_ms,
        "Realtime fetch budget should be stricter"
    );
    assert!(
        realtime.total_pipeline_ms < default.total_pipeline_ms,
        "Realtime total budget should be stricter"
    );
}

#[test]
fn budget_config_analysis_is_more_relaxed() {
    let analysis = BudgetConfig::analysis();
    let default = BudgetConfig::default();

    assert!(
        analysis.fetch_candidates_ms > default.fetch_candidates_ms,
        "Analysis fetch budget should be more relaxed"
    );
    assert!(
        analysis.total_pipeline_ms > default.total_pipeline_ms,
        "Analysis total budget should be more relaxed"
    );
}

#[test]
fn stage_guard_detects_overrun() {
    let config = BudgetConfig {
        fetch_candidates_ms: 1, // Very tight budget
        ..Default::default()
    };

    let guard = StageGuard::new(PipelineStage::FetchCandidates, &config);
    std::thread::sleep(Duration::from_millis(5)); // Exceed budget
    let result = guard.complete();

    assert!(result.is_over_budget(), "Should detect budget overrun");
    if let BudgetResult::OverBudget { budget_ms, .. } = result {
        assert_eq!(budget_ms, 1);
    } else {
        panic!("Expected OverBudget result");
    }
}

#[test]
fn stage_guard_detects_within_budget() {
    let config = BudgetConfig {
        fetch_candidates_ms: 1000, // Generous budget
        ..Default::default()
    };

    let guard = StageGuard::new(PipelineStage::FetchCandidates, &config);
    std::thread::sleep(Duration::from_millis(1)); // Well within budget
    let result = guard.complete();

    assert!(!result.is_over_budget(), "Should be within budget");
}

#[test]
fn budget_tracker_aggregates_multiple_stages() {
    let config = BudgetConfig {
        fetch_candidates_ms: 100,
        freshness_eval_ms: 50,
        health_scoring_ms: 50,
        policy_filter_ms: 50,
        venue_selection_ms: 50,
        total_pipeline_ms: 500,
    };

    let mut tracker = BudgetTracker::new(config);

    // Track multiple stages
    let guard1 = tracker.stage(PipelineStage::FetchCandidates);
    std::thread::sleep(Duration::from_millis(1));
    tracker.record(PipelineStage::FetchCandidates, guard1.complete());

    let guard2 = tracker.stage(PipelineStage::FreshnessEval);
    std::thread::sleep(Duration::from_millis(1));
    tracker.record(PipelineStage::FreshnessEval, guard2.complete());

    let guard3 = tracker.stage(PipelineStage::HealthScoring);
    std::thread::sleep(Duration::from_millis(1));
    tracker.record(PipelineStage::HealthScoring, guard3.complete());

    let summary = tracker.finish();

    assert_eq!(summary.stage_results.len(), 3);
    assert!(!summary.has_overruns(), "All stages should be within budget");
}

#[test]
fn budget_tracker_detects_overruns() {
    let config = BudgetConfig {
        fetch_candidates_ms: 1, // Tight budget
        freshness_eval_ms: 100,
        health_scoring_ms: 100,
        policy_filter_ms: 100,
        venue_selection_ms: 100,
        total_pipeline_ms: 500,
    };

    let mut tracker = BudgetTracker::new(config);

    // This stage will overrun
    let guard = tracker.stage(PipelineStage::FetchCandidates);
    std::thread::sleep(Duration::from_millis(5));
    tracker.record(PipelineStage::FetchCandidates, guard.complete());

    let summary = tracker.finish();

    assert!(summary.has_overruns(), "Should detect overruns");
    assert!(
        summary.overbudget_stages.contains(&PipelineStage::FetchCandidates),
        "FetchCandidates should be over budget"
    );
}

#[test]
fn pipeline_stage_budget_mapping() {
    let config = BudgetConfig::default();

    assert_eq!(
        PipelineStage::FetchCandidates.budget_ms(&config),
        config.fetch_candidates_ms
    );
    assert_eq!(
        PipelineStage::FreshnessEval.budget_ms(&config),
        config.freshness_eval_ms
    );
    assert_eq!(
        PipelineStage::HealthScoring.budget_ms(&config),
        config.health_scoring_ms
    );
    assert_eq!(
        PipelineStage::PolicyFilter.budget_ms(&config),
        config.policy_filter_ms
    );
    assert_eq!(
        PipelineStage::VenueSelection.budget_ms(&config),
        config.venue_selection_ms
    );
    assert_eq!(
        PipelineStage::TotalPipeline.budget_ms(&config),
        config.total_pipeline_ms
    );
}

#[test]
fn track_closure_returns_value() {
    let config = BudgetConfig::default();
    let mut tracker = BudgetTracker::new(config);

    let result = tracker.track(PipelineStage::FreshnessEval, || 42);
    assert_eq!(result, 42);

    let summary = tracker.finish();
    assert_eq!(summary.stage_results.len(), 1);
}
