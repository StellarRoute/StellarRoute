use axum::{extract::State, Json};
use serde_json::Value;
use std::sync::Arc;

use crate::{error::Result, state::AppState};
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
    Json(new_config): Json<CanaryConfig>,
) -> Result<Json<CanaryConfig>> {
    let mut config_guard = state.canary_config.write().await;
    *config_guard = new_config.clone();
    Ok(Json(new_config))
}
