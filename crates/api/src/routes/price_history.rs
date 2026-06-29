//! Historical price endpoint for charting selected pairs.

use axum::{
    extract::{Path, State},
    Json,
};
use sqlx::Row;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tracing::{debug, warn};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    cache,
    error::{ApiError, Result},
    models::{request::AssetPath, AssetInfo},
    state::AppState,
};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PriceHistoryPoint {
    pub timestamp: i64,
    pub price: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PriceHistoryResponse {
    pub base_asset: AssetInfo,
    pub quote_asset: AssetInfo,
    pub window: String,
    pub source: String,
    pub generated_at: i64,
    pub points: Vec<PriceHistoryPoint>,
}

/// Return a 24h historical price series for a selected trading pair.
#[utoipa::path(
    get,
    path = "/api/v1/price-history/{base}/{quote}",
    tag = "trading",
    params(
        ("base" = String, Path, description = "Base asset (e.g. 'native', 'USDC', or 'USDC:ISSUER')"),
        ("quote" = String, Path, description = "Quote asset (e.g. 'native', 'USDC', or 'USDC:ISSUER')"),
    ),
    responses(
        (status = 200, description = "24h price history", body = PriceHistoryResponse),
        (
            status = 400,
            description = "Invalid asset",
            body = crate::models::ErrorResponse,
        ),
        (
            status = 404,
            description = "Trading pair not found",
            body = crate::models::ErrorResponse,
        ),
        (
            status = 500,
            description = "Internal server error",
            body = crate::models::ErrorResponse,
        ),
    )
)]
pub async fn get_price_history(
    State(state): State<Arc<AppState>>,
    Path((base, quote)): Path<(String, String)>,
) -> Result<Json<PriceHistoryResponse>> {
    debug!("Fetching price history for {}/{}", base, quote);

    if let Some(cache) = &state.cache {
        if let Ok(mut cache) = cache.try_lock() {
            if let Some(cached) = cache
                .get::<PriceHistoryResponse>(&cache::keys::price_history(&base, &quote))
                .await
            {
                debug!("Returning cached price history for {}/{}", base, quote);
                return Ok(Json(cached));
            }
        }
    }

    let base_asset = AssetPath::parse(&base)
        .map_err(|e| ApiError::InvalidAsset(format!("Invalid base asset: {}", e)))?;
    let quote_asset = AssetPath::parse(&quote)
        .map_err(|e| ApiError::InvalidAsset(format!("Invalid quote asset: {}", e)))?;

    let base_id = find_asset_id(&state, &base_asset).await?;
    let quote_id = find_asset_id(&state, &quote_asset).await?;
    let trading_pair_id = find_trading_pair_id(&state, base_id, quote_id).await?;

    let sdex_rows = sqlx::query(
        r#"
        select
            (extract(epoch from date_trunc('hour', snapshot_time)) * 1000)::bigint as timestamp_ms,
            avg(mid_price)::text as price
        from orderbook_snapshots
        where trading_pair_id = $1
          and snapshot_time >= now() - interval '24 hours'
          and mid_price is not null
        group by date_trunc('hour', snapshot_time)
        order by date_trunc('hour', snapshot_time) asc
        limit 24
        "#,
    )
    .bind(trading_pair_id)
    .fetch_all(state.db.read_pool())
    .await
    .map_err(|e| ApiError::Database(Arc::new(e)))?;

    let amm_rows = sqlx::query(
        r#"
        select
            (extract(epoch from date_trunc('hour', updated_at)) * 1000)::bigint as timestamp_ms,
            avg((reserve_buying / nullif(reserve_selling, 0)))::text as price
        from amm_pool_reserves
        where selling_asset_id = $1
          and buying_asset_id = $2
          and updated_at >= now() - interval '24 hours'
        group by date_trunc('hour', updated_at)
        order by date_trunc('hour', updated_at) asc
        limit 24
        "#,
    )
    .bind(base_id)
    .bind(quote_id)
    .fetch_all(state.db.read_pool())
    .await
    .map_err(|e| ApiError::Database(Arc::new(e)))?;

    let sdex_points = sdex_rows
        .into_iter()
        .filter_map(|row| {
            let timestamp = row.get::<i64, _>("timestamp_ms");
            let price = row.get::<Option<String>, _>("price");
            price.map(|price| PriceHistoryPoint { timestamp, price })
        })
        .collect::<Vec<_>>();

    let amm_points = amm_rows
        .into_iter()
        .filter_map(|row| {
            let timestamp = row.get::<i64, _>("timestamp_ms");
            let price = row.get::<Option<String>, _>("price");
            price.map(|price| PriceHistoryPoint { timestamp, price })
        })
        .collect::<Vec<_>>();

    let has_sdex = !sdex_points.is_empty();
    let has_amm = !amm_points.is_empty();
    let points = merge_price_history_points(sdex_points, amm_points);

    let response = PriceHistoryResponse {
        base_asset: asset_path_to_info(&base_asset),
        quote_asset: asset_path_to_info(&quote_asset),
        window: "24h".to_string(),
        source: price_history_source(has_sdex, has_amm),
        generated_at: chrono::Utc::now().timestamp_millis(),
        points,
    };

    if let Some(cache) = &state.cache {
        if let Ok(mut cache) = cache.try_lock() {
            let _ = cache
                .set(
                    &cache::keys::price_history(&base, &quote),
                    &response,
                    Duration::from_secs(30),
                )
                .await;
        }
    }

    Ok(Json(response))
}

async fn find_asset_id(state: &AppState, asset: &AssetPath) -> Result<uuid::Uuid> {
    let asset_type = asset.to_asset_type();

    let row = if asset.asset_code == "native" {
        sqlx::query(
            r#"
            select id from assets
            where asset_type = $1
            limit 1
            "#,
        )
        .bind(&asset_type)
        .fetch_optional(state.db.read_pool())
        .await?
    } else {
        sqlx::query(
            r#"
            select id from assets
            where asset_type = $1
              and asset_code = $2
              and ($3::text is null or asset_issuer = $3)
            limit 1
            "#,
        )
        .bind(&asset_type)
        .bind(&asset.asset_code)
        .bind(&asset.asset_issuer)
        .fetch_optional(state.db.read_pool())
        .await?
    };

    match row {
        Some(row) => Ok(row.get("id")),
        None => {
            warn!("Asset not found: {:?}", asset);
            Err(ApiError::NotFound(format!(
                "Asset not found: {}",
                asset.asset_code
            )))
        }
    }
}

async fn find_trading_pair_id(
    state: &AppState,
    base_id: uuid::Uuid,
    quote_id: uuid::Uuid,
) -> Result<uuid::Uuid> {
    let row = sqlx::query(
        r#"
        select id
        from trading_pairs
        where base_asset_id = $1
          and counter_asset_id = $2
        limit 1
        "#,
    )
    .bind(base_id)
    .bind(quote_id)
    .fetch_optional(state.db.read_pool())
    .await?;

    match row {
        Some(row) => Ok(row.get("id")),
        None => {
            warn!(
                "Trading pair not found: base_id={:?}, quote_id={:?}",
                base_id, quote_id
            );
            Err(ApiError::NotFound("Trading pair not found".to_string()))
        }
    }
}

fn asset_path_to_info(asset: &AssetPath) -> AssetInfo {
    if asset.asset_code == "native" {
        AssetInfo::native()
    } else {
        AssetInfo::credit(asset.asset_code.clone(), asset.asset_issuer.clone())
    }
}

fn merge_price_history_points(
    sdex_points: Vec<PriceHistoryPoint>,
    amm_points: Vec<PriceHistoryPoint>,
) -> Vec<PriceHistoryPoint> {
    let mut points_by_timestamp: HashMap<i64, (Option<String>, Option<String>)> = HashMap::new();

    for point in sdex_points {
        points_by_timestamp
            .entry(point.timestamp)
            .and_modify(|(sdex_price, _)| {
                *sdex_price = Some(point.price.clone());
            })
            .or_insert((Some(point.price), None));
    }

    for point in amm_points {
        points_by_timestamp
            .entry(point.timestamp)
            .and_modify(|(_, amm_price)| {
                *amm_price = Some(point.price.clone());
            })
            .or_insert((None, Some(point.price)));
    }

    let mut points = points_by_timestamp
        .into_iter()
        .filter_map(|(timestamp, (sdex_price, amm_price))| {
            let price = match (sdex_price, amm_price) {
                (Some(_), Some(amm_price)) => Some(amm_price),
                (Some(sdex_price), None) => Some(sdex_price),
                (None, Some(amm_price)) => Some(amm_price),
                (None, None) => None,
            }?;

            Some(PriceHistoryPoint { timestamp, price })
        })
        .collect::<Vec<_>>();

    points.sort_by_key(|point| point.timestamp);
    points
}

fn price_history_source(has_sdex: bool, has_amm: bool) -> String {
    match (has_sdex, has_amm) {
        (true, true) => {
            "blended:orderbook_snapshots.mid_price+amm_pool_reserves.reserve_ratio".to_string()
        }
        (true, false) => "orderbook_snapshots.mid_price".to_string(),
        (false, true) => "amm_pool_reserves.reserve_ratio".to_string(),
        (false, false) => "none".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{merge_price_history_points, price_history_source, PriceHistoryPoint};

    #[test]
    fn merges_sdex_and_amm_points_for_the_same_hour() {
        let sdex = vec![
            PriceHistoryPoint {
                timestamp: 1_700_000_000_000,
                price: "10.0".to_string(),
            },
            PriceHistoryPoint {
                timestamp: 1_700_000_360_000,
                price: "11.0".to_string(),
            },
        ];
        let amm = vec![PriceHistoryPoint {
            timestamp: 1_700_000_000_000,
            price: "12.0".to_string(),
        }];

        let points = merge_price_history_points(sdex, amm);

        assert_eq!(points.len(), 2);
        assert_eq!(points[0].price, "12.0");
        assert_eq!(points[1].price, "11.0");
    }

    #[test]
    fn returns_blended_source_when_both_sources_contribute() {
        let source = price_history_source(true, true);
        assert!(source.contains("blended"));
    }
}
