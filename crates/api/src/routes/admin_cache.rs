use crate::admin_audit::{build_admin_audit_entry, emit_admin_audit};
use crate::error::Result;
use crate::middleware::admin::AdminAuth;
use crate::middleware::RequestId;
use crate::state::AppState;
use axum::http::HeaderMap;
use axum::{extract::State, Json};
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CacheFlushRequest {
    pub base: Option<String>,
    pub quote: Option<String>,
}

/// Global cache flush
#[utoipa::path(
    post,
    path = "/api/v1/admin/cache/flush",
    tag = "admin",
    request_body = CacheFlushRequest,
    responses(
        (status = 200, description = "Cache flush completed", body = serde_json::Value),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(
        ("admin_token" = []),
        ("admin_bearer" = [])
    )
)]
pub async fn flush_cache(
    State(state): State<Arc<AppState>>,
    _admin: AdminAuth,
    headers: HeaderMap,
    request_id: RequestId,
    Json(payload): Json<CacheFlushRequest>,
) -> Result<Json<serde_json::Value>> {
    info!("Admin cache flush requested: {:?}", payload);

    let resource = if let (Some(base), Some(quote)) = (&payload.base, &payload.quote) {
        format!("cache:pair:{}:{}", base, quote)
    } else {
        "cache:all".to_string()
    };

    let deleted = if let Some(cache) = &state.cache {
        if let Ok(mut cache) = cache.try_lock() {
            // Determine pattern
            let pattern = if let (Some(base), Some(quote)) = (&payload.base, &payload.quote) {
                crate::cache::keys::quote_pair_pattern(base, quote)
            } else {
                // Delete all quote keys
                "*quote:*".to_string()
            };

            match cache.delete_by_pattern(&pattern).await {
                Ok(n) => n,
                Err(_) => 0,
            }
        } else {
            0
        }
    } else {
        0
    };

    // Emit admin audit
    let entry = build_admin_audit_entry(
        "cache.flush",
        request_id.as_str(),
        &headers,
        resource.clone(),
        "success",
    );
    let _ = emit_admin_audit(&entry);

    Ok(Json(
        serde_json::json!({ "status": "ok", "deleted": deleted }),
    ))
}
