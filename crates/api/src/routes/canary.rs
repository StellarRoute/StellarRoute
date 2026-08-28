use axum::{extract::State, Json};
use serde_json::Value;
use std::sync::Arc;

use crate::models::{
    LiveCompareIngestResponse, LiveCompareOutcome, LiveCompareReportResponse, LiveCompareResult,
};
use crate::{error::Result, middleware::AdminAuth, state::AppState};
use stellarroute_routing::canary::CanaryConfig;

/// GET /api/v1/system/canary/report
///
/// Returns the current canary configuration and the history of recent evaluations.
pub async fn get_report(State(state): State<Arc<AppState>>) -> Result<Json<Value>> {
    let config = state.canary_config.read().await.clone();
    let history_guard = state.canary_history.read().await;
    // Clone the evaluations into a vector to return them
    let history: Vec<_> = history_guard.iter().cloned().collect();

    Ok(Json(serde_json::json!({
        "config": config,
        "total_evaluations": history.len(),
        "recent_evaluations": history,
    })))
}

/// POST /api/v1/system/canary/config
///
/// Updates the current canary configuration.
pub async fn update_config(
    State(state): State<Arc<AppState>>,
    _admin: AdminAuth,
    Json(new_config): Json<CanaryConfig>,
) -> Result<Json<CanaryConfig>> {
    let mut config_guard = state.canary_config.write().await;
    *config_guard = new_config.clone();
    Ok(Json(new_config))
}

/// POST /api/v1/system/canary/live-compare
///
/// Accepts a comparison result from the external canary script, updates
/// Prometheus metrics, and appends to the in-memory history buffer.
/// Requires AdminAuth. Returns HTTP 422 automatically when the JSON body
/// is missing required fields or contains unexpected types (serde rejection).
pub async fn ingest_live_compare(
    State(state): State<Arc<AppState>>,
    _admin: AdminAuth,
    Json(result): Json<LiveCompareResult>,
) -> Result<Json<LiveCompareIngestResponse>> {
    let outcome_str = match result.outcome {
        LiveCompareOutcome::Ok => "ok",
        LiveCompareOutcome::Diverged => "diverged",
        LiveCompareOutcome::Error => "error",
    };

    // When outcome is error, divergence is unknown — record 0 to avoid a
    // misleading gauge value. Never allow negative bps.
    let bps = if result.outcome == LiveCompareOutcome::Error {
        0.0
    } else {
        result.divergence_bps.max(0.0)
    };

    crate::metrics::record_live_compare_result(&result.pair, bps, outcome_str);

    let mut history = state.live_compare_history.write().await;
    if history.len() == 1000 {
        history.pop_front(); // evict oldest to keep buffer at ≤ 1,000
    }
    history.push_back(result);
    let entries = history.len();

    Ok(Json(LiveCompareIngestResponse {
        status: "ok".to_string(),
        entries,
    }))
}

/// GET /api/v1/system/canary/live-compare/report
///
/// Returns recent comparison history newest first. Requires AdminAuth.
pub async fn live_compare_report(
    State(state): State<Arc<AppState>>,
    _admin: AdminAuth,
) -> Result<Json<LiveCompareReportResponse>> {
    let history = state.live_compare_history.read().await;
    let results: Vec<LiveCompareResult> = history.iter().rev().cloned().collect();
    let total_entries = results.len();
    Ok(Json(LiveCompareReportResponse {
        total_entries,
        results,
    }))
}
