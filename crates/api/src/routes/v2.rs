//! Minimal `/api/v2` seam for chain-aware asset canonicalization.
//!
//! Intentionally small: proves the versioned surface and typed models without
//! porting the full quote/orderbook stack (still `/api/v1`).
//!
//! Bridge edges remain metadata-only / non-executable. Compaction preserves
//! bridge/provider identity. Provider kill-switches apply only when candidates
//! carry provider metadata (forward-compatible; current Stellar quote ingest
//! leaves `provider: None`).

use axum::extract::State;
use axum::Json;
use std::sync::Arc;

use crate::cctp::gate::cctp_public_executability_snapshot;
use crate::error::{ApiError, Result};
use crate::middleware::RequestId;
use crate::models::compat::{chain_asset_to_v2, parse_asset_input};
use crate::models::v2::{ApiV2Info, CanonicalizeAssetRequest, CanonicalizeAssetResponse};
use crate::models::ApiResponse;
use crate::state::AppState;

/// `GET /api/v2` — capability descriptor for the chain-aware seam.
#[utoipa::path(
    get,
    path = "/api/v2",
    tag = "v2",
    responses(
        (status = 200, description = "API v2 capability info", body = ApiV2Info),
    )
)]
pub async fn api_v2_info(
    State(state): State<Arc<AppState>>,
    request_id: RequestId,
) -> Result<Json<ApiResponse<ApiV2Info>>> {
    let (corridors, executable) = if let Some(ctx) = &state.cctp {
        if ctx.config.enabled {
            cctp_public_executability_snapshot(
                &ctx.service,
                &state.kill_switch,
                &state.external_dependency_health,
            )
            .await
        } else {
            (vec![], false)
        }
    } else {
        (vec![], false)
    };

    Ok(Json(ApiResponse::with_version(
        2,
        ApiV2Info {
            version: 2,
            chain_aware_assets: true,
            bridge_venues_metadata_only: !executable,
            bridge_settlement_executable: executable,
            supported_chain_namespaces: vec![
                "stellar".into(),
                "eip155".into(),
                "solana".into(),
                "bip122".into(),
                "tron".into(),
            ],
            supported_corridors: corridors,
        },
        request_id.as_str(),
    )))
}

/// `POST /api/v2/assets/canonicalize` — normalize legacy or CAIP asset ids.
#[utoipa::path(
    post,
    path = "/api/v2/assets/canonicalize",
    tag = "v2",
    request_body = CanonicalizeAssetRequest,
    responses(
        (status = 200, description = "Canonical chain-scoped asset", body = CanonicalizeAssetResponse),
        (status = 400, description = "Invalid asset identifier"),
    )
)]
pub async fn canonicalize_asset(
    request_id: RequestId,
    Json(body): Json<CanonicalizeAssetRequest>,
) -> Result<Json<ApiResponse<CanonicalizeAssetResponse>>> {
    let (asset, input_form) = parse_asset_input(&body.asset)
        .map_err(|e| ApiError::InvalidAssetFormat(format!("invalid asset identifier: {e}")))?;

    Ok(Json(ApiResponse::with_version(
        2,
        CanonicalizeAssetResponse {
            asset: chain_asset_to_v2(&asset),
            input_form: input_form.to_string(),
        },
        request_id.as_str(),
    )))
}
