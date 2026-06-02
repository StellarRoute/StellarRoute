use crate::admin_audit::{build_admin_audit_entry, emit_admin_audit};
use crate::error::Result;
use crate::kill_switch::KillSwitchState;
use crate::middleware::RequestId;
use crate::state::AppState;
use axum::http::HeaderMap;
use axum::{extract::State, Json};
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
    _headers: HeaderMap,
    _request_id: RequestId,
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
    headers: HeaderMap,
    request_id: RequestId,
    Json(payload): Json<KillSwitchState>,
) -> Result<Json<serde_json::Value>> {
    info!("Admin updating kill switch state: {:?}", payload);

    state.kill_switch.update_state(payload).await;

    // Emit admin audit entry
    let entry = build_admin_audit_entry(
        "kill_switch.update",
        request_id.as_str(),
        &headers,
        "kill_switch",
        "success",
    );
    let _ = emit_admin_audit(&entry);

    Ok(Json(serde_json::json!({ "status": "ok" })))
}
