use std::sync::Arc;

use axum::{extract::State, http::HeaderMap, Json};
use uuid::Uuid;

use crate::{
    error::{ApiError, Result},
    middleware::RequestId,
    models::{
        ApiResponse, QuoteExpirationWebhookRegistrationRequest,
        QuoteExpirationWebhookRegistrationResponse,
    },
    state::AppState,
};

fn resolve_consumer_id(headers: &HeaderMap) -> Result<String> {
    headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("api_key:{value}"))
        .ok_or(ApiError::BadRequest(
            "Missing X-API-Key header for webhook registration".to_string(),
        ))
}

#[utoipa::path(
    post,
    path = "/api/v1/integrator/webhooks/quote-expiration",
    tag = "integrator",
    request_body = QuoteExpirationWebhookRegistrationRequest,
    responses(
        (status = 200, description = "Webhook registration updated", body = QuoteExpirationWebhookRegistrationResponse),
        (status = 400, description = "Invalid input", body = crate::models::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::models::ErrorResponse),
    )
)]
pub async fn upsert_quote_expiration_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    request_id: RequestId,
    Json(body): Json<QuoteExpirationWebhookRegistrationRequest>,
) -> Result<Json<ApiResponse<QuoteExpirationWebhookRegistrationResponse>>> {
    let consumer_id = resolve_consumer_id(&headers)?;

    if body.webhook_url.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "webhook_url must not be empty".to_string(),
        ));
    }

    if !body.webhook_url.starts_with("https://") {
        return Err(ApiError::BadRequest(
            "webhook_url must use https".to_string(),
        ));
    }

    let generated_signing_secret = if body
        .signing_secret
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        Some(Uuid::new_v4().to_string())
    } else {
        None
    };

    let signing_secret = body
        .signing_secret
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| generated_signing_secret.as_deref().unwrap_or_default());

    let enabled = body.enabled.unwrap_or(true);

    state
        .quote_expiration_webhooks
        .upsert_registration(&consumer_id, &body.webhook_url, signing_secret, enabled)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(ApiResponse::new(
        QuoteExpirationWebhookRegistrationResponse {
            consumer_id,
            webhook_url: body.webhook_url,
            enabled,
            generated_signing_secret,
        },
        request_id.to_string(),
    )))
}
