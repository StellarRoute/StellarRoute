//! Asset metadata endpoints

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use sqlx::Row;
use std::sync::Arc;
use tracing::debug;

use crate::{
    error::{ApiError, Result},
    middleware::RequestId,
    models::{ApiResponse, AssetMetadataBulkResponse, AssetMetadataResponse},
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct AssetMetadataParams {
    pub issuer: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BulkAssetMetadataParams {
    pub codes: String,
}

/// Get metadata for a single asset
#[utoipa::path(
    get,
    path = "/api/v1/assets/{code}",
    tag = "assets",
    params(
        ("code" = String, Path, description = "Asset code (e.g. 'XLM', 'USDC')"),
        ("issuer" = Option<String>, Query, description = "Optional asset issuer address")
    ),
    responses(
        (status = 200, description = "Asset metadata", body = AssetMetadataResponse),
        (status = 404, description = "Asset not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_asset_metadata(
    State(state): State<Arc<AppState>>,
    Path(code): Path<String>,
    Query(params): Query<AssetMetadataParams>,
    request_id: crate::middleware::RequestId,
) -> Result<Json<ApiResponse<AssetMetadataResponse>>> {
    debug!(code = %code, issuer = ?params.issuer, "Fetching asset metadata");

    let metadata = fetch_metadata(&state, &code, params.issuer.as_deref())
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Asset metadata not found for {}", code)))?;

    Ok(Json(ApiResponse::new(metadata, request_id.to_string())))
}

/// Get metadata for multiple assets (bulk)
#[utoipa::path(
    get,
    path = "/api/v1/assets",
    tag = "assets",
    params(
        ("codes" = String, Query, description = "Comma-separated list of asset codes")
    ),
    responses(
        (status = 200, description = "List of asset metadata", body = AssetMetadataBulkResponse),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_assets_metadata(
    State(state): State<Arc<AppState>>,
    Query(params): Query<BulkAssetMetadataParams>,
    request_id: crate::middleware::RequestId,
) -> Result<Json<ApiResponse<AssetMetadataBulkResponse>>> {
    let codes: Vec<&str> = params.codes.split(',').map(|s| s.trim()).collect();
    debug!(codes = ?codes, "Fetching bulk asset metadata");

    let mut assets = Vec::new();
    for code in codes {
        if let Ok(Some(meta)) = fetch_metadata(&state, code, None).await {
            assets.push(meta);
        }
    }

    Ok(Json(ApiResponse::new(
        AssetMetadataBulkResponse { assets },
        request_id.to_string(),
    )))
}

async fn fetch_metadata(
    state: &AppState,
    code: &str,
    issuer: Option<&str>,
) -> Result<Option<AssetMetadataResponse>> {
    // Special case for native XLM
    if code.eq_ignore_ascii_case("XLM") || code.eq_ignore_ascii_case("native") {
        return Ok(Some(AssetMetadataResponse {
            code: "XLM".to_string(),
            issuer: None,
            decimals: 7,
            asset_type: "native".to_string(),
            display_name: Some("Stellar Lumens".to_string()),
            icon_url: Some("https://stellar.org/images/lumens-logo.svg".to_string()),
            domain: Some("stellar.org".to_string()),
        }));
    }

    // Query database for issued assets
    let row = if let Some(issuer_addr) = issuer {
        sqlx::query(
            r#"
            SELECT asset_code, asset_issuer, asset_type, decimals, domain, icon_url
            FROM asset_metadata
            WHERE asset_code = $1 AND asset_issuer = $2
            "#,
        )
        .bind(code)
        .bind(issuer_addr)
        .fetch_optional(state.db.read_pool())
        .await
        .map_err(|e| ApiError::Database(Arc::new(e)))?
    } else {
        // If no issuer provided, pick the most "trustworthy" one (e.g. the one with decimals or just the first one)
        sqlx::query(
            r#"
            SELECT asset_code, asset_issuer, asset_type, decimals, domain, icon_url
            FROM asset_metadata
            WHERE asset_code = $1
            ORDER BY decimals DESC NULLS LAST, fetched_at DESC
            LIMIT 1
            "#,
        )
        .bind(code)
        .fetch_optional(state.db.read_pool())
        .await
        .map_err(|e| ApiError::Database(Arc::new(e)))?
    };

    Ok(row.map(|r| AssetMetadataResponse {
        code: r
            .get::<Option<String>, _>("asset_code")
            .unwrap_or_else(|| code.to_string()),
        issuer: r.get("asset_issuer"),
        decimals: r.get::<Option<i16>, _>("decimals").unwrap_or(7),
        asset_type: r.get("asset_type"),
        display_name: r.get("asset_code"), // Use code as display name if nothing else
        icon_url: r.get("icon_url"),
        domain: r.get("domain"),
    }))
}
