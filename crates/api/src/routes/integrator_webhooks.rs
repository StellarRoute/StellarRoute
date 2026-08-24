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

pub(crate) fn resolve_consumer_id(headers: &HeaderMap) -> Result<String> {
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

#[cfg(test)]
mod tests {
    //! Regression tests for `POST /api/v1/integrator/webhooks/quote-expiration`.
    //!
    //! All four acceptance-criteria cases are covered:
    //!
    //! 1. Missing `X-API-Key` header → `BadRequest`
    //! 2. Empty `webhook_url` → `BadRequest`
    //! 3. Non-HTTPS `webhook_url` (`http://`) → `BadRequest`
    //! 4. Valid HTTPS URL with key present → validation cleared, execution
    //!    reaches the persistence layer (`Database` error from a lazy pool)
    //!
    //! No external network calls are made. Cases 1–3 return before any DB
    //! access. Case 4 uses a `connect_lazy` pool that never dials a server;
    //! the resulting `Database` error proves every validation gate passed.

    use std::sync::Arc;

    use axum::extract::State;
    use axum::http::{HeaderMap, HeaderValue};

    use crate::{
        error::ApiError,
        middleware::RequestId,
        models::QuoteExpirationWebhookRegistrationRequest,
        routes::integrator_webhooks::{resolve_consumer_id, upsert_quote_expiration_webhook},
        state::{AppState, DatabasePools},
    };

    // ── helpers ──────────────────────────────────────────────────────────────

    /// `AppState` backed by a lazy pool that never opens a connection.
    /// Safe for tests that hit a validation error before any DB call.
    fn lazy_state() -> Arc<AppState> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/stellarroute_test")
            .expect("lazy pool");
        Arc::new(AppState::new(DatabasePools::new(pool, None)))
    }

    fn headers_with_key(key: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(
            "x-api-key",
            HeaderValue::from_str(key).expect("valid header value"),
        );
        h
    }

    fn body(url: &str) -> QuoteExpirationWebhookRegistrationRequest {
        QuoteExpirationWebhookRegistrationRequest {
            webhook_url: url.to_string(),
            signing_secret: None,
            enabled: None,
        }
    }

    // ── Case 1: missing X-API-Key ────────────────────────────────────────────

    /// No `X-API-Key` header → `BadRequest` before body is inspected.
    #[test]
    fn missing_api_key_returns_bad_request() {
        let result = resolve_consumer_id(&HeaderMap::new());
        assert!(
            matches!(result, Err(ApiError::BadRequest(_))),
            "expected BadRequest for missing X-API-Key, got: {:?}",
            result
        );
    }

    /// The rejection message must name `X-API-Key` so callers know what is missing.
    #[test]
    fn missing_api_key_error_mentions_header_name() {
        if let Err(ApiError::BadRequest(msg)) = resolve_consumer_id(&HeaderMap::new()) {
            assert!(
                msg.contains("X-API-Key"),
                "message must mention 'X-API-Key', got: {msg}"
            );
        } else {
            panic!("expected BadRequest");
        }
    }

    /// Whitespace-only value is treated as absent.
    #[test]
    fn whitespace_only_api_key_is_rejected() {
        let mut h = HeaderMap::new();
        h.insert("x-api-key", HeaderValue::from_static("   "));
        assert!(
            matches!(resolve_consumer_id(&h), Err(ApiError::BadRequest(_))),
            "whitespace-only X-API-Key must be rejected"
        );
    }

    /// A valid non-empty key is accepted and gets the `api_key:` prefix.
    #[test]
    fn valid_api_key_is_accepted_with_prefix() {
        let result = resolve_consumer_id(&headers_with_key("my-key"));
        assert_eq!(result.unwrap(), "api_key:my-key");
    }

    // ── Case 2: empty webhook_url ────────────────────────────────────────────

    #[tokio::test]
    async fn empty_webhook_url_returns_bad_request() {
        let result = upsert_quote_expiration_webhook(
            State(lazy_state()),
            headers_with_key("k"),
            RequestId::generate(),
            axum::Json(body("")),
        )
        .await;
        assert!(
            matches!(result, Err(ApiError::BadRequest(_))),
            "empty webhook_url must be rejected: {:?}",
            result
        );
    }

    /// Whitespace-only URL is also empty.
    #[tokio::test]
    async fn whitespace_only_webhook_url_returns_bad_request() {
        let result = upsert_quote_expiration_webhook(
            State(lazy_state()),
            headers_with_key("k"),
            RequestId::generate(),
            axum::Json(body("   ")),
        )
        .await;
        assert!(
            matches!(result, Err(ApiError::BadRequest(_))),
            "whitespace-only webhook_url must be rejected"
        );
    }

    // ── Case 3: non-HTTPS URL ────────────────────────────────────────────────

    /// Plain `http://` URL must be rejected.
    #[tokio::test]
    async fn http_url_returns_bad_request() {
        let result = upsert_quote_expiration_webhook(
            State(lazy_state()),
            headers_with_key("k"),
            RequestId::generate(),
            axum::Json(body("http://example.com/hook")),
        )
        .await;
        assert!(
            matches!(result, Err(ApiError::BadRequest(_))),
            "http:// URL must be rejected: {:?}",
            result
        );
    }

    /// The rejection message must mention `https`.
    #[tokio::test]
    async fn http_url_error_mentions_https() {
        let result = upsert_quote_expiration_webhook(
            State(lazy_state()),
            headers_with_key("k"),
            RequestId::generate(),
            axum::Json(body("http://example.com/hook")),
        )
        .await;
        if let Err(ApiError::BadRequest(msg)) = result {
            assert!(
                msg.contains("https"),
                "error must mention 'https', got: {msg}"
            );
        } else {
            panic!("expected BadRequest");
        }
    }

    /// A URL with no scheme is also rejected (does not start with `https://`).
    #[tokio::test]
    async fn no_scheme_url_returns_bad_request() {
        let result = upsert_quote_expiration_webhook(
            State(lazy_state()),
            headers_with_key("k"),
            RequestId::generate(),
            axum::Json(body("example.com/hook")),
        )
        .await;
        assert!(
            matches!(result, Err(ApiError::BadRequest(_))),
            "URL without https:// scheme must be rejected"
        );
    }

    // ── Case 4: happy-path HTTPS registration ────────────────────────────────

    /// A valid key + valid HTTPS URL clears every validation gate. Execution
    /// reaches the persistence layer, which fails with `Database` (lazy pool).
    /// The result must NOT be `BadRequest`.
    #[tokio::test]
    async fn valid_https_url_passes_all_validation_gates() {
        let result = upsert_quote_expiration_webhook(
            State(lazy_state()),
            headers_with_key("integrator-key"),
            RequestId::generate(),
            axum::Json(body("https://hooks.example.com/quote-expired")),
        )
        .await;
        assert!(
            !matches!(result, Err(ApiError::BadRequest(_))),
            "valid HTTPS request must not be blocked by validation: {:?}",
            result
        );
        assert!(
            matches!(result, Err(ApiError::Database(_))),
            "expected Database error after validation passes (lazy pool), got: {:?}",
            result
        );
    }

    /// HTTPS URL with port and path still passes scheme validation.
    #[tokio::test]
    async fn https_url_with_port_passes_validation() {
        let result = upsert_quote_expiration_webhook(
            State(lazy_state()),
            headers_with_key("k"),
            RequestId::generate(),
            axum::Json(body("https://hooks.example.com:8443/v2/expired")),
        )
        .await;
        assert!(
            !matches!(result, Err(ApiError::BadRequest(_))),
            "https:// URL with port must pass scheme validation: {:?}",
            result
        );
    }
}
