use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use crate::{
    error::{ApiError, Result},
    models::ApiResponse,
    state::AppState,
};

/// AI intent request
#[derive(Debug, Deserialize, ToSchema)]
pub struct ValidateIntentRequest {
    /// Intent type (send, swap, bridge, etc.)
    pub r#type: String,
    /// Amount as a string
    pub amount: String,
    /// Asset code
    pub asset: String,
    /// Optional recipient address
    pub recipient: Option<String>,
    /// Optional destination chain/asset
    pub destination: Option<String>,
    /// Optional source chain/asset
    pub source: Option<String>,
    /// Optional currency
    pub currency: Option<String>,
    /// Optional recurrence
    pub recurrence: Option<String>,
}

/// Validated intent response
#[derive(Debug, Serialize, ToSchema)]
pub struct ValidateIntentResponse {
    /// Normalized amount string
    pub amount: String,
    /// Intent type
    pub r#type: String,
}

/// Validate an AI intent without storing it
///
/// POST /api/v1/agent/intents/validate
///
/// Returns 200 with the validated intent, 422 if validation fails, or 404 if feature is disabled.
#[utoipa::path(
    post,
    path = "/api/v1/agent/intents/validate",
    tag = "agent",
    request_body(content = ValidateIntentRequest, description = "Intent to validate"),
    responses(
        (status = 200, description = "Validated intent", body = ApiResponse<ValidateIntentResponse>),
        (status = 404, description = "Agent feature disabled", body = crate::models::ErrorResponse),
        (status = 422, description = "Validation failed", body = crate::models::ErrorResponse),
    )
)]
pub async fn validate_intent(
    State(_state): State<Arc<AppState>>,
    request_id: crate::middleware::RequestId,
    Json(body): Json<ValidateIntentRequest>,
) -> Result<impl IntoResponse> {
    // Check if agent is enabled
    if !is_agent_enabled() {
        return Err(ApiError::NotFound("Agent feature is disabled".to_string()));
    }

    // Validate amount
    let amount = body.amount.trim();
    if amount.is_empty() {
        return Err(ApiError::Validation("amount is required".to_string()));
    }

    // Parse amount to ensure it's a valid number
    let _parsed_amount: f64 = amount
        .parse()
        .map_err(|_| ApiError::Validation(format!("amount '{}' is not a valid number", amount)))?;

    // Check for negative amounts
    if _parsed_amount <= 0.0 {
        return Err(ApiError::Validation(format!(
            "amount must be positive, got {}",
            amount
        )));
    }

    // Normalize the amount string (trim whitespace)
    let normalized_amount = amount.to_string();

    Ok((
        StatusCode::OK,
        Json(ApiResponse::new(
            ValidateIntentResponse {
                amount: normalized_amount,
                r#type: body.r#type,
            },
            request_id.to_string(),
        )),
    ))
}

fn is_agent_enabled() -> bool {
    std::env::var("AI_AGENT_ENABLED")
        .ok()
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or_default()
}
