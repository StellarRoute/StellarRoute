//! Agent feature endpoints (AI-37)
//!
//! All handlers here return 404 when the AI_AGENT_ENABLED flag is unset or
//! false. No existing route, schema, or error code is modified.

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{
    agent::{list_agent_tools, AgentToolsResponse},
    models::ApiErrorCode,
};

/// Agent health response body
#[derive(Debug, Serialize, ToSchema)]
pub struct AgentHealthResponse {
    /// Whether the agent feature gate is currently open
    pub enabled: bool,
}

fn is_agent_enabled() -> bool {
    std::env::var("AI_AGENT_ENABLED")
        .ok()
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
}

/// Agent health check
///
/// Returns 200 with `{"enabled": true}` when the agent feature flag is set.
/// Returns 404 when the flag is unset or false so the frontend chip shows
/// "off" without any extra configuration.
#[utoipa::path(
    get,
    path = "/api/v1/agent/health",
    tag = "agent",
    responses(
        (status = 200, description = "Agent feature gate is open", body = AgentHealthResponse),
        (status = 404, description = "Agent feature is disabled or not configured", body = ErrorResponse),
    )
)]
pub async fn agent_health() -> impl IntoResponse {
    if !is_agent_enabled() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": {
                    "code": ApiErrorCode::NotFound.as_str(),
                    "message": "Agent feature is disabled"
                }
            })),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({ "enabled": true })),
    )
        .into_response()
}

/// List the static AI agent tool catalog.
///
/// Returns 200 with the seven non-executable tool descriptors when the flag is
/// enabled. Returns 404 when the flag is unset or false.
#[utoipa::path(
    get,
    path = "/api/v1/agent/tools",
    tag = "agent",
    responses(
        (status = 200, description = "Tool catalog returned", body = AgentToolsResponse),
        (status = 404, description = "Agent feature is disabled", body = ErrorResponse),
    )
)]
pub async fn agent_tools() -> impl IntoResponse {
    if !is_agent_enabled() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": {
                    "code": ApiErrorCode::NotFound.as_str(),
                    "message": "Agent feature is disabled"
                }
            })),
        )
            .into_response();
    }

    (StatusCode::OK, Json(AgentToolsResponse { tools: list_agent_tools() })).into_response()
}

/// Validate an agent intent without storing it
///
/// Returns 404 when the agent feature flag is unset or false.
/// Returns 422 when the intent payload fails validation.
#[utoipa::path(
    post,
    path = "/api/v1/agent/intents/validate",
    tag = "agent",
    request_body(
        content = serde_json::Value,
        description = "Agent intent payload. Must include `type` and `amount`."
    ),
    responses(
        (status = 200, description = "Intent is valid; returns normalized fields"),
        (status = 404, description = "Agent feature is disabled", body = ErrorResponse),
        (status = 422, description = "Intent failed validation", body = ErrorResponse),
    )
)]
pub async fn agent_intents_validate(
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    if !is_agent_enabled() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": {
                    "code": ApiErrorCode::NotFound.as_str(),
                    "message": "Agent feature is disabled"
                }
            })),
        )
            .into_response();
    }

    let amount = body
        .get("amount")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();

    if amount.is_empty() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({
                "error": {
                    "code": ApiErrorCode::BadRequest.as_str(),
                    "message": "amount is required"
                }
            })),
        )
            .into_response();
    }

    let parsed: Result<f64, _> = amount.parse();
    match parsed {
        Ok(v) if v > 0.0 => {}
        Ok(_) => {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({
                    "error": {
                        "code": ApiErrorCode::ValidationError.as_str(),
                        "message": format!("amount must be positive, got {}", amount)
                    }
                })),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({
                    "error": {
                        "code": ApiErrorCode::ValidationError.as_str(),
                        "message": format!("amount '{}' is not a valid number", amount)
                    }
                })),
            )
                .into_response();
        }
    }

    let intent_type = body
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "amount": amount,
            "type": intent_type
        })),
    )
        .into_response()
}
