//! Administrative API routes

use anyhow::anyhow;
use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;
use std::sync::Arc;
use tracing::info;

use crate::{
    cache,
    error::{ApiError, Result},
    middleware::admin::AdminAuth,
    models::{CacheFlushResponse, ErrorResponse},
    state::AppState,
};

/// Flush quote and orderbook cache entries for a pair or wildcard pattern.
#[utoipa::path(
    post,
    path = "/api/v1/admin/cache/flush/{base}/{quote}",
    tag = "admin",
    params(
        ("base" = String, Path, description = "Base asset identifier or wildcard '*'."),
        ("quote" = String, Path, description = "Quote asset identifier or wildcard '*'."),
    ),
    responses(
        (status = 200, description = "Cache flush completed", body = CacheFlushResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
)]
pub async fn flush_cache(
    State(state): State<Arc<AppState>>,
    _admin: AdminAuth,
    Path((base, quote)): Path<(String, String)>,
) -> Result<Json<CacheFlushResponse>> {
    let cache = state.cache.as_ref().ok_or_else(|| {
        ApiError::Internal(anyhow::anyhow!("Cache backend is not configured"))
    })?;

    let quote_pattern = build_quote_pattern(&base, &quote);
    let orderbook_pattern = build_orderbook_pattern(&base, &quote);

    let mut cache = cache.lock().await;
    let deleted_quote_keys = cache
        .delete_by_pattern(&quote_pattern)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Cache delete failed: {}", e)))?;
    let deleted_orderbook_keys = cache
        .delete_by_pattern(&orderbook_pattern)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Cache delete failed: {}", e)))?;

    let total_deleted = deleted_quote_keys + deleted_orderbook_keys;

    info!(
        audit = true,
        action = "cache_flush",
        base = %base,
        quote = %quote,
        quote_pattern = %quote_pattern,
        orderbook_pattern = %orderbook_pattern,
        deleted_quote_keys,
        deleted_orderbook_keys,
        total_deleted,
        "Admin cache flush executed"
    );

    Ok(Json(CacheFlushResponse {
        base,
        quote,
        quote_pattern,
        orderbook_pattern,
        deleted_quote_keys,
        deleted_orderbook_keys,
        total_deleted,
    }))
}

fn build_quote_pattern(base: &str, quote: &str) -> String {
    match (base, quote) {
        ("*", "*") => "*quote:*".to_string(),
        ("*", quote) => format!("*quote:*:{}:*", quote),
        (base, "*") => format!("*quote:{}:*:*", base),
        (base, quote) => format!("*quote:{}:{}:*", base, quote),
    }
}

fn build_orderbook_pattern(base: &str, quote: &str) -> String {
    match (base, quote) {
        ("*", "*") => "orderbook:*:*".to_string(),
        ("*", quote) => format!("orderbook:*:{}", quote),
        (base, "*") => format!("orderbook:{}:*", base),
        (base, quote) => format!("orderbook:{}:{}", base, quote),
    }
}
