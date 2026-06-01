//! Quote compute budget enforcement with per-stage timing limits
//!
//! This module implements per-stage timing budgets for the quote pipeline
//! to prevent runaway latency and protect SLOs.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use prometheus::{IntCounterVec, HistogramVec, register_int_counter_vec, register_histogram_vec};

lazy_static::lazy_static! {
    /// Counter for budget overrun events by stage
    pub static ref BUDGET_OVERRUNS: IntCounterVec = register_int_counter_vec!(
        "stellarroute_quote_budget_overruns_total",
        "Number of times a quote pipeline stage exceeded its budget",
        &["stage"]
    ).expect("Can't create BUDGET_OVERRUNS counter");

    /// Histogram for stage execution durations
    pub static ref STAGE_DURATION: HistogramVec = register_histogram_vec!(
        "stellarroute_quote_stage_duration_seconds",
        "Duration of each quote pipeline stage in seconds",
        &["stage"],
        vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]
    ).expect("Can't create STAGE_DURATION histogram");
}

/// Configuration for per-stage timing budgets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetConfig {
    /// Budget for fetching candidates from data sources (SDEX, AMM)
    pub fetch_candidates_ms: u64,
    /// Budget for freshness evaluation
    pub freshness_eval_ms: u64,
    /// Budget for health scoring
    pub health_scoring_ms: u64,
    /// Budget for policy filtering
    pub policy_filter_ms: u64,
    /// Budget for venue selection
    pub venue_selection_ms: u64,
    /// Total budget for the entire quote pipeline
    pub total_pipeline_ms: u64,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            fetch_candidates_ms: 50,
            freshness_eval_ms: 5,
            health_scoring_ms: 10,
            policy_filter_ms: 5,
            venue_selection_ms: 5,
            total_pipeline_ms: 100,
        }
    }
}

impl BudgetConfig {
    /// Create a strict budget configuration for real-time trading
    pub fn realtime() -> Self {
        Self {
            fetch_candidates_ms: 30,
            freshness_eval_ms: 2,
            health_scoring_ms: 5,
            policy_filter_ms: 3,
            venue_selection_ms: 3,
            total_pipeline_ms: 50,
        }
    }

    /// Create a relaxed budget configuration for analysis
    pub fn analysis() -> Self {
        Self {
            fetch_candidates_ms: 200,
            freshness_eval_ms: 20,
            health_scoring_ms: 50,
            policy_filter_ms: 20,
            venue_selection_ms: 20,
            total_pipeline_ms: 500,
        }
    }
}

/// Stage identifiers for budget tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PipelineStage {
    FetchCandidates,
    FreshnessEval,
    HealthScoring,
    PolicyFilter,
    VenueSelection,
    TotalPipeline,
}

impl PipelineStage {
    pub fn as_str(&self) -> &'static str {
        match self {
            PipelineStage::FetchCandidates => "fetch_candidates",
            PipelineStage::FreshnessEval => "freshness_eval",
            PipelineStage::HealthScoring => "health_scoring",
            PipelineStage::PolicyFilter => "policy_filter",
            PipelineStage::VenueSelection => "venue_selection",
            PipelineStage::TotalPipeline => "total_pipeline",
        }
    }

    pub fn budget_ms(&self, config: &BudgetConfig) -> u64 {
        match self {
            PipelineStage::FetchCandidates => config.fetch_candidates_ms,
            PipelineStage::FreshnessEval => config.freshness_eval_ms,
            PipelineStage::HealthScoring => config.health_scoring_ms,
            PipelineStage::PolicyFilter => config.policy_filter_ms,
            PipelineStage::VenueSelection => config.venue_selection_ms,
            PipelineStage::TotalPipeline => config.total_pipeline_ms,
        }
    }
}

/// Result of a budget check
#[derive(Debug, Clone)]
pub enum BudgetResult {
    /// Stage completed within budget
    WithinBudget { duration: Duration },
    /// Stage exceeded budget but continued
    OverBudget { duration: Duration, budget_ms: u64 },
}

impl BudgetResult {
    pub fn duration(&self) -> Duration {
        match self {
            BudgetResult::WithinBudget { duration } => *duration,
            BudgetResult::OverBudget { duration, .. } => *duration,
        }
    }

    pub fn is_over_budget(&self) -> bool {
        matches!(self, BudgetResult::OverBudget { .. })
    }
}

/// Guard for tracking stage execution time and enforcing budgets
pub struct StageGuard {
    stage: PipelineStage,
    start: Instant,
    budget_ms: u64,
}

impl StageGuard {
    pub fn new(stage: PipelineStage, config: &BudgetConfig) -> Self {
        Self {
            stage,
            start: Instant::now(),
            budget_ms: stage.budget_ms(config),
        }
    }

    /// Complete the stage and check budget
    pub fn complete(self) -> BudgetResult {
        let duration = self.start.elapsed();
        let duration_ms = duration.as_millis() as u64;

        STAGE_DURATION
            .with_label_values(&[self.stage.as_str()])
            .observe(duration.as_secs_f64());

        if duration_ms > self.budget_ms {
            BUDGET_OVERRUNS
                .with_label_values(&[self.stage.as_str()])
                .inc();
            BudgetResult::OverBudget {
                duration,
                budget_ms: self.budget_ms,
            }
        } else {
            BudgetResult::WithinBudget { duration }
        }
    }
}

/// Budget enforcement tracker for the entire quote pipeline
#[derive(Debug)]
pub struct BudgetTracker {
    config: BudgetConfig,
    results: Vec<(PipelineStage, BudgetResult)>,
    total_start: Instant,
}

impl BudgetTracker {
    pub fn new(config: BudgetConfig) -> Self {
        Self {
            config,
            results: Vec::new(),
            total_start: Instant::now(),
        }
    }

    /// Start timing a stage
    pub fn stage(&self, stage: PipelineStage) -> StageGuard {
        StageGuard::new(stage, &self.config)
    }

    /// Record a stage result
    pub fn record(&mut self, stage: PipelineStage, result: BudgetResult) {
        self.results.push((stage, result));
    }

    /// Execute a closure with budget tracking
    pub fn track<F, T>(&mut self, stage: PipelineStage, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let guard = self.stage(stage);
        let result = f();
        let budget_result = guard.complete();
        self.record(stage, budget_result);
        result
    }

    /// Complete the pipeline and return summary
    pub fn finish(self) -> BudgetSummary {
        let total_duration = self.total_start.elapsed();
        let overbudget_stages: Vec<PipelineStage> = self
            .results
            .iter()
            .filter(|(_, r)| r.is_over_budget())
            .map(|(s, _)| *s)
            .collect();

        let total_budget_ms = self.config.total_pipeline_ms;
        let total_overbudget = total_duration.as_millis() as u64 > total_budget_ms;

        if total_overbudget {
            BUDGET_OVERRUNS
                .with_label_values(&[PipelineStage::TotalPipeline.as_str()])
                .inc();
        }

        STAGE_DURATION
            .with_label_values(&[PipelineStage::TotalPipeline.as_str()])
            .observe(total_duration.as_secs_f64());

        BudgetSummary {
            total_duration,
            stage_results: self.results,
            overbudget_stages,
            total_overbudget,
        }
    }
}

/// Summary of budget enforcement for a quote pipeline execution
#[derive(Debug, Clone)]
pub struct BudgetSummary {
    pub total_duration: Duration,
    pub stage_results: Vec<(PipelineStage, BudgetResult)>,
    pub overbudget_stages: Vec<PipelineStage>,
    pub total_overbudget: bool,
}

impl BudgetSummary {
    pub fn has_overruns(&self) -> bool {
        !self.overbudget_stages.is_empty() || self.total_overbudget
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_config_default_is_reasonable() {
        let config = BudgetConfig::default();
        assert!(config.fetch_candidates_ms > 0);
        assert!(config.total_pipeline_ms >= config.fetch_candidates_ms);
    }

    #[test]
    fn stage_guard_tracks_duration() {
        let config = BudgetConfig::default();
        let guard = StageGuard::new(PipelineStage::FreshnessEval, &config);
        std::thread::sleep(Duration::from_millis(1));
        let result = guard.complete();
        assert!(result.duration().as_millis() >= 1);
    }

    #[test]
    fn budget_tracker_aggregates_results() {
        let mut tracker = BudgetTracker::new(BudgetConfig::default());
        tracker.track(PipelineStage::FreshnessEval, || 42);
        let summary = tracker.finish();
        assert!(!summary.stage_results.is_empty());
    }
}
