//! Trading pairs endpoint

use axum::{extract::State, Json};
use sqlx::Row;
use std::{sync::Arc, time::Duration};
use tracing::debug;

use crate::{
    cache,
    error::{ApiError, Result},
    models::{AssetInfo, PairsResponse, TradingPair},
    state::AppState,
};

/// List all available trading pairs
///
/// Returns a list of trading pairs with active offers in the orderbook.
/// Each pair exposes human-readable `base`/`counter` codes alongside
/// canonical Stellar asset identifiers (`base_asset`/`counter_asset`).
#[utoipa::path(
    get,
    path = "/api/v1/pairs",
    tag = "trading",
    responses(
        (status = 200, description = "List of trading pairs", body = PairsResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
)]
pub async fn list_pairs(State(state): State<Arc<AppState>>) -> Result<Json<PairsResponse>> {
    debug!("Fetching trading pairs");

    // Try to get from cache first
    if let Some(cache) = &state.cache {
        if let Ok(mut cache) = cache.try_lock() {
            if let Some(cached) = cache.get::<PairsResponse>(&cache::keys::pairs_list()).await {
                debug!("Returning cached pairs");
                return Ok(Json(cached));
            }
        }
    }

    // Query distinct trading pairs that have active offers in the orderbook.
    // Results are ranked by offer depth so the most liquid pairs appear first.
    let rows = sqlx::query(
        r#"
        select
            sa.asset_type as selling_type,
            sa.asset_code as selling_code,
            sa.asset_issuer as selling_issuer,
            ba.asset_type as buying_type,
            ba.asset_code as buying_code,
            ba.asset_issuer as buying_issuer,
            count(*) as offer_count,
            max(o.updated_at) as last_updated
        from sdex_offers o
        join assets sa on o.selling_asset_id = sa.id
        join assets ba on o.buying_asset_id = ba.id
        group by
            sa.asset_type, sa.asset_code, sa.asset_issuer,
            ba.asset_type, ba.asset_code, ba.asset_issuer
        order by offer_count desc
        limit 100
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;

    let mut pairs = Vec::new();

    for row in rows {
        let selling_type: String = row.get("selling_type");
        let buying_type: String = row.get("buying_type");

        // Build AssetInfo helpers so we can derive both display names and
        // canonical identifiers from a single source of truth.
        let base_info = if selling_type == "native" {
            AssetInfo::native()
        } else {
            AssetInfo::credit(
                row.get::<Option<String>, _>("selling_code")
                    .unwrap_or_default(),
                row.get("selling_issuer"),
            )
        };

        let counter_info = if buying_type == "native" {
            AssetInfo::native()
        } else {
            AssetInfo::credit(
                row.get::<Option<String>, _>("buying_code")
                    .unwrap_or_default(),
                row.get("buying_issuer"),
            )
        };

        let offer_count: i64 = row.get("offer_count");
        let last_updated: Option<chrono::DateTime<chrono::Utc>> = row.get("last_updated");

        pairs.push(TradingPair {
            base: base_info.display_name(),
            counter: counter_info.display_name(),
            base_asset: base_info.to_canonical(),
            counter_asset: counter_info.to_canonical(),
            offer_count,
            last_updated: last_updated.map(|dt| dt.to_rfc3339()),
        });
    }

    debug!("Found {} trading pairs", pairs.len());

    let response = PairsResponse {
        total: pairs.len(),
        pairs,
    };

    // Cache the response for 10 s to keep latency well under the 100 ms SLA.
    if let Some(cache) = &state.cache {
        if let Ok(mut cache) = cache.try_lock() {
            let _ = cache
                .set(
                    &cache::keys::pairs_list(),
                    &response,
                    Duration::from_secs(10),
                )
                .await;
        }
    }

    Ok(Json(response))
}
