//! Idempotent POST /api/v1/quote
//!
//! Clients supply an `Idempotency-Key` header (UUID or opaque string, max 128
//! chars).  Duplicate requests with the same key return the **identical** body
//! and status code without re-running the quote pipeline.
//!
//! # Key lifecycle
//! - TTL: 5 minutes (configurable via `IDEMPOTENCY_TTL_SECS` env var).
//! - Storage: in-process `DedupeLedger` (same as the existing exactly-once
//!   pipeline).  Keys are normalised to lowercase-trimmed form before lookup.
//! - Collision safety: keys are scoped per-endpoint; a key used on
//!   `POST /api/v1/quote` cannot collide with keys on other endpoints.
//!
//! # Degradation
//! If the `Idempotency-Key` header is absent the request is processed normally
//! (no deduplication).

use axum::{extract::State, http::HeaderMap, Json};
use serde::Deserialize;
use std::sync::Arc;
use tracing::debug;
use utoipa::ToSchema;

use crate::{
    error::{ApiError, Result},
    exactlyonce::RequestIdentity,
    middleware::RequestId,
    models::{
        request::{AssetPath, QuoteParams, QuoteType},
        ApiResponse,
    },
    routes::quote::get_quote_inner,
    state::AppState,
};

/// Header name for the client-supplied idempotency key.
pub const IDEMPOTENCY_KEY_HEADER: &str = "idempotency-key";
/// Maximum allowed length for an idempotency key.
pub const IDEMPOTENCY_KEY_MAX_LEN: usize = 128;
/// Default TTL for stored idempotency entries (5 minutes).
pub const IDEMPOTENCY_TTL_SECS: u64 = 300;

/// Request body for `POST /api/v1/quote`.
#[derive(Debug, Deserialize, Clone, ToSchema)]
pub struct PostQuoteRequest {
    /// Base asset identifier ("native", "CODE", or "CODE:ISSUER").
    pub base: String,
    /// Quote asset identifier ("native", "CODE", or "CODE:ISSUER").
    pub quote: String,
    /// Amount to trade (default: "1").
    pub amount: Option<String>,
    /// Slippage tolerance in basis points (default: 50).
    pub slippage_bps: Option<u32>,
    /// Quote direction: "sell" or "buy" (default: sell).
    pub quote_type: Option<QuoteType>,
}

/// Normalise an idempotency key: trim whitespace, lowercase, reject if empty
/// or too long.
fn normalise_key(raw: &str) -> Result<String> {
    let key = raw.trim().to_lowercase();
    if key.is_empty() {
        return Err(ApiError::Validation(
            "Idempotency-Key must not be empty".to_string(),
        ));
    }
    if key.len() > IDEMPOTENCY_KEY_MAX_LEN {
        return Err(ApiError::Validation(format!(
            "Idempotency-Key exceeds maximum length of {} characters",
            IDEMPOTENCY_KEY_MAX_LEN
        )));
    }
    // Scope the key to this endpoint to prevent cross-endpoint collisions.
    Ok(format!("post_quote:{key}"))
}

/// POST /api/v1/quote — idempotent quote request.
///
/// Supply an `Idempotency-Key` header to enable deduplication.  Retries with
/// the same key within the TTL window return the cached response without
/// re-running the quote pipeline.
#[utoipa::path(
    post,
    path = "/api/v1/quote",
    tag = "trading",
    params(
        ("Idempotency-Key" = Option<String>, Header,
         description = "Client-supplied idempotency key (max 128 chars). \
                        Duplicate requests within the TTL window return the same response."),
    ),
    request_body(
        content = PostQuoteRequest,
        description = "Quote parameters",
    ),
    responses(
        (status = 200, description = "Price quote (may be a cached replay)", body = ApiResponse<crate::models::QuoteResponse>),
        (status = 400, description = "Invalid parameters", body = crate::models::ErrorResponse),
        (status = 404, description = "No route found", body = crate::models::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::models::ErrorResponse),
    )
)]
pub async fn post_quote(
    State(state): State<Arc<AppState>>,
    request_id: RequestId,
    headers: HeaderMap,
    Json(body): Json<PostQuoteRequest>,
) -> Result<Json<ApiResponse<crate::models::QuoteResponse>>> {
    // ── 1. Parse and normalise idempotency key ────────────────────────────
    let idempotency_key = headers
        .get(IDEMPOTENCY_KEY_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(normalise_key)
        .transpose()?;

    // ── 2. Check dedupe ledger for a cached response ──────────────────────
    if let Some(ref key) = idempotency_key {
        let identity = RequestIdentity {
            base_asset: key.clone(),
            quote_asset: String::new(),
            amount: String::new(),
            slippage_bps: 0,
            quote_type: String::new(),
        };
        if let Ok(cached_bytes) = state.idempotency_ledger.lookup(&identity).await {
            debug!("Idempotency cache hit for key: {}", key);
            let quote_resp: crate::models::QuoteResponse =
                serde_json::from_slice(&cached_bytes).map_err(|e| {
                    ApiError::Internal(Arc::new(anyhow::anyhow!(
                        "Failed to deserialise cached quote: {e}"
                    )))
                })?;
            let envelope = ApiResponse::new(quote_resp, request_id.to_string());
            return Ok(Json(envelope));
        }
    }

    // ── 3. Parse assets ───────────────────────────────────────────────────
    let base_asset = AssetPath::parse(&body.base)
        .map_err(|e| ApiError::InvalidAsset(format!("Invalid base asset: {e}")))?;
    let quote_asset = AssetPath::parse(&body.quote)
        .map_err(|e| ApiError::InvalidAsset(format!("Invalid quote asset: {e}")))?;

    let params = QuoteParams {
        amount: body.amount.clone(),
        slippage_bps: body.slippage_bps,
        quote_type: body.quote_type.unwrap_or(QuoteType::Sell),
        explain: None,
    };

    // ── 4. Run quote pipeline ─────────────────────────────────────────────
    let (prepared, _cache_hit) =
        get_quote_inner(state.clone(), base_asset, quote_asset, params, false).await?;

    let quote_resp = prepared.into_quote()?;

    // ── 5. Store in dedupe ledger ─────────────────────────────────────────
    if let Some(ref key) = idempotency_key {
        let ttl = std::env::var("IDEMPOTENCY_TTL_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(IDEMPOTENCY_TTL_SECS);

        let identity = RequestIdentity {
            base_asset: key.clone(),
            quote_asset: String::new(),
            amount: String::new(),
            slippage_bps: 0,
            quote_type: String::new(),
        };

        if let Ok(bytes) = serde_json::to_vec(&quote_resp) {
            let _ = state.idempotency_ledger.record(identity, bytes, ttl).await;
        }
    }

    let envelope = ApiResponse::new(quote_resp, request_id.to_string());
    Ok(Json(envelope))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalise_key_trims_and_lowercases() {
        let key = normalise_key("  MyKey-123  ").unwrap();
        assert_eq!(key, "post_quote:mykey-123");
    }

    #[test]
    fn normalise_key_rejects_empty() {
        assert!(normalise_key("   ").is_err());
    }

    #[test]
    fn normalise_key_rejects_too_long() {
        let long = "a".repeat(IDEMPOTENCY_KEY_MAX_LEN + 1);
        assert!(normalise_key(&long).is_err());
    }

    #[test]
    fn normalise_key_accepts_max_length() {
        let exact = "a".repeat(IDEMPOTENCY_KEY_MAX_LEN);
        assert!(normalise_key(&exact).is_ok());
    }

    #[test]
    fn normalise_key_scopes_to_endpoint() {
        let key = normalise_key("abc").unwrap();
        assert!(key.starts_with("post_quote:"));
    }
}
