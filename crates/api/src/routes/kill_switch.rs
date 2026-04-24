use axum::{extract::State, Json};
use crate::error::Result;
use crate::kill_switch::KillSwitchState;
use crate::state::AppState;
use std::sync::Arc;
use tracing::info;

/// Get current kill switch state
#[utoipa::path(
    get,
    path = "/api/v1/admin/kill-switch",
    tag = "admin",
    responses(
        (status = 200, description = "Current kill switch state", body = KillSwitchState),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
)]
pub async fn get_kill_switch(
    State(state): State<Arc<AppState>>,
) -> Result<Json<KillSwitchState>> {
    let ks_state = state.kill_switch.get_state().await;
    Ok(Json(ks_state))
}

/// Update kill switch state
#[utoipa::path(
    post,
    path = "/api/v1/admin/kill-switch",
    tag = "admin",
    request_body = KillSwitchState,
    responses(
        (status = 200, description = "Kill switch state updated"),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
)]
pub async fn update_kill_switch(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<KillSwitchState>,
) -> Result<Json<serde_json::Value>> {
    info!("Admin updating kill switch state: {:?}", payload);
    
    state.kill_switch.update_state(payload).await
        .map_err(|e| crate::error::ApiError::Internal(e))?;
    
    Ok(Json(serde_json::json!({ "status": "ok" })))
}
