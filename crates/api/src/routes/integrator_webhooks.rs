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
    //! Regression tests for `POST /api/v1/integrator/webhooks/quote-expiration`
    //! validation rules.
    //!
    //! All four acceptance-criteria cases run without a live database or
    //! external network calls:
    //!
    //! 1. Missing `X-API-Key` header → 400 Bad Request
    //! 2. Empty `webhook_url` → 400 Bad Request
    //! 3. Non-HTTPS `webhook_url` (http://) → 400 Bad Request
    //! 4. Valid HTTPS URL with key present → validation passes and execution
    //!    reaches the persistence layer (proves no validation gate blocked it)
    //!
    //! The `AppState` is constructed with a lazy pool that never dials the
    //! database, so cases 1–3 complete in-process. Case 4 confirms validation
    //! is fully cleared by observing a `Database` error (not a `BadRequest`)
    //! from the handler — the only way to reach that error is to pass every
    //! prior check.

    use std::sync::Arc;

    use axum::http::{HeaderMap, HeaderValue};

    use crate::{
        error::ApiError,
        middleware::RequestId,
        models::QuoteExpirationWebhookRegistrationRequest,
        routes::integrator_webhooks::{resolve_consumer_id, upsert_quote_expiration_webhook},
        state::{AppState, DatabasePools},
    };

    use axum::extract::State;

    // ── helpers ──────────────────────────────────────────────────────────────

    /// Build an `AppState` backed by a lazy pool that never opens a real
    /// connection. Safe to use for tests that do not reach DB execution.
    fn lazy_app_state() -> Arc<AppState> {
        // connect_lazy never dials the server; the pool is valid but unusable.
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/stellarroute_test")
            .expect("lazy pool");
        Arc::new(AppState::new(DatabasePools::new(pool, None)))
    }

    fn make_body(url: &str) -> QuoteExpirationWebhookRegistrationRequest {
        QuoteExpirationWebhookRegistrationRequest {
            webhook_url: url.to_string(),
            signing_secret: None,
            enabled: None,
        }
    }

    fn headers_with_api_key(key: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(
            "x-api-key",
            HeaderValue::from_str(key).expect("valid header value"),
        );
        h
    }

    // ── Case 1: missing X-API-Key ────────────────────────────────────────────

    /// A request with no `X-API-Key` header must be rejected with
    /// `ApiError::BadRequest` before any body validation runs.
    #[test]
    fn missing_api_key_is_rejected() {
        let empty_headers = HeaderMap::new();
        let result = resolve_consumer_id(&empty_headers);

        assert!(
            matches!(result, Err(ApiError::BadRequest(_))),
            "expected BadRequest for missing X-API-Key, got: {:?}",
            result
        );
    }

    /// The rejection message must mention `X-API-Key` so clients know what
    /// is missing.
    #[test]
    fn missing_api_key_error_message_names_header() {
        let result = resolve_consumer_id(&HeaderMap::new());
        if let Err(ApiError::BadRequest(msg)) = result {
            assert!(
                msg.contains("X-API-Key"),
                "error message should mention 'X-API-Key', got: {msg}"
            );
        } else {
            panic!("expected BadRequest, got: {:?}", result);
        }
    }

    /// A header present but containing only whitespace is treated as missing.
    #[test]
    fn whitespace_only_api_key_is_rejected() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", HeaderValue::from_static("   "));
        let result = resolve_consumer_id(&headers);
        assert!(
            matches!(result, Err(ApiError::BadRequest(_))),
            "whitespace-only X-API-Key must be rejected"
        );
    }

    /// A valid non-empty key is accepted and prefixed with `api_key:`.
    #[test]
    fn valid_api_key_is_accepted_and_prefixed() {
        let headers = headers_with_api_key("my-secret-key");
        let result = resolve_consumer_id(&headers);
        assert_eq!(result.unwrap(), "api_key:my-secret-key");
    }

    // ── Case 2: empty webhook_url ────────────────────────────────────────────

    /// An empty `webhook_url` must be rejected with `ApiError::BadRequest`.
    #[tokio::test]
    async fn empty_webhook_url_is_rejected() {
        let state = lazy_app_state();
        let headers = headers_with_api_key("test-key");
        let request_id = RequestId::generate();
        let body = make_body("");

        let result = upsert_quote_expiration_webhook(
            State(state),
            headers,
            request_id,
            axum::Json(body),
        )
        .await;

        assert!(
            matches!(result, Err(ApiError::BadRequest(_))),
            "expected BadRequest for empty webhook_url, got: {:?}",
            result
        );
    }

    /// A `webhook_url` that is only whitespace is also rejected.
    #[tokio::test]
    async fn whitespace_only_webhook_url_is_rejected() {
        let state = lazy_app_state();
        let headers = headers_with_api_key("test-key");
        let request_id = RequestId::generate();
        let body = make_body("   ");

        let result = upsert_quote_expiration_webhook(
            State(state),
            headers,
            request_id,
            axum::Json(body),
        )
        .await;

        assert!(
            matches!(result, Err(ApiError::BadRequest(_))),
            "whitespace-only webhook_url must be rejected as empty"
        );
    }

    // ── Case 3: non-HTTPS URL ────────────────────────────────────────────────

    /// An `http://` URL must be rejected — only `https://` is allowed.
    #[tokio::test]
    async fn http_url_is_rejected() {
        let state = lazy_app_state();
        let headers = headers_with_api_key("test-key");
        let request_id = RequestId::generate();
        let body = make_body("http://example.com/webhook");

        let result = upsert_quote_expiration_webhook(
            State(state),
            headers,
            request_id,
            axum::Json(body),
        )
        .await;

        assert!(
            matches!(result, Err(ApiError::BadRequest(_))),
            "expected BadRequest for http:// URL, got: {:?}",
            result
        );
    }

    /// The rejection message for a non-HTTPS URL must mention `https`.
    #[tokio::test]
    async fn http_url_error_message_mentions_https() {
        let state = lazy_app_state();
        let headers = headers_with_api_key("test-key");
        let request_id = RequestId::generate();
        let body = make_body("http://example.com/webhook");

        let result = upsert_quote_expiration_webhook(
            State(state),
            headers,
            request_id,
            axum::Json(body),
        )
        .await;

        if let Err(ApiError::BadRequest(msg)) = result {
            assert!(
                msg.contains("https"),
                "error message should mention 'https', got: {msg}"
            );
        } else {
            panic!("expected BadRequest, got: {:?}", result);
        }
    }

    /// A plain URL with no scheme must also be rejected (does not start with
    /// "https://").
    #[tokio::test]
    async fn no_scheme_url_is_rejected() {
        let state = lazy_app_state();
        let headers = headers_with_api_key("test-key");
        let request_id = RequestId::generate();
        let body = make_body("example.com/webhook");

        let result = upsert_quote_expiration_webhook(
            State(state),
            headers,
            request_id,
            axum::Json(body),
        )
        .await;

        assert!(
            matches!(result, Err(ApiError::BadRequest(_))),
            "URL without https:// scheme must be rejected"
        );
    }

    // ── Case 4: happy-path HTTPS registration ────────────────────────────────

    /// When all validation gates pass (valid key + valid HTTPS URL), execution
    /// reaches the persistence layer. With a lazy pool this surfaces as a
    /// `Database` error — proving no `BadRequest` gate blocked the request.
    ///
    /// No external network call is made: the pool is lazy and never connects.
    #[tokio::test]
    async fn valid_https_url_passes_all_validation_gates() {
        let state = lazy_app_state();
        let headers = headers_with_api_key("integrator-key-123");
        let request_id = RequestId::generate();
        let body = make_body("https://integrator.example.com/hooks/quote-expired");

        let result = upsert_quote_expiration_webhook(
            State(state),
            headers,
            request_id,
            axum::Json(body),
        )
        .await;

        // Must NOT be a BadRequest — that would mean a validation gate fired.
        // A Database error confirms the request cleared all validation and
        // reached the persistence layer.
        assert!(
            !matches!(result, Err(ApiError::BadRequest(_))),
            "a valid HTTPS request must not be rejected by validation: {:?}",
            result
        );
        assert!(
            matches!(result, Err(ApiError::Database(_))),
            "expected a Database error (lazy pool) after validation passes, got: {:?}",
            result
        );
    }

    /// An HTTPS URL with a path and query-string-like characters still passes
    /// the HTTPS check — the handler only validates the scheme prefix.
    #[tokio::test]
    async fn https_url_with_path_and_port_passes_scheme_check() {
        let state = lazy_app_state();
        let headers = headers_with_api_key("key");
        let request_id = RequestId::generate();
        let body = make_body("https://hooks.example.com:8443/v2/quote/expired");

        let result = upsert_quote_expiration_webhook(
            State(state),
            headers,
            request_id,
            axum::Json(body),
        )
        .await;

        assert!(
            !matches!(result, Err(ApiError::BadRequest(_))),
            "https:// URL with port and path must pass scheme validation: {:?}",
            result
        );
    }
}
