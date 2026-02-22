//! Orderbook endpoint

use axum::{
    extract::{Path, State},
    Json,
};
use sqlx::Row;
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tracing::{debug, warn};

use crate::{
    cache,
    error::{ApiError, Result},
    models::{request::AssetPath, AssetInfo, OrderbookLevel, OrderbookResponse},
    state::AppState,
};

/// Get orderbook for a trading pair
///
/// Returns bids and asks for the specified base/quote pair
#[utoipa::path(
    get,
    path = "/api/v1/orderbook/{base}/{quote}",
    tag = "trading",
    params(
        ("base" = String, Path, description = "Base asset (e.g., 'native', 'USDC', or 'USDC:ISSUER')"),
        ("quote" = String, Path, description = "Quote asset (e.g., 'native', 'USDC', or 'USDC:ISSUER')"),
    ),
    responses(
        (status = 200, description = "Orderbook data", body = OrderbookResponse),
        (status = 400, description = "Invalid asset", body = ErrorResponse),
        (status = 404, description = "Asset not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
)]
pub async fn get_orderbook(
    State(state): State<Arc<AppState>>,
    Path((base, quote)): Path<(String, String)>,
) -> Result<Json<OrderbookResponse>> {
    debug!("Fetching orderbook for {}/{}", base, quote);

    // Try to get from cache first
    if let Some(cache) = &state.cache {
        if let Ok(mut cache) = cache.try_lock() {
            if let Some(cached) = cache
                .get::<OrderbookResponse>(&cache::keys::orderbook(&base, &quote))
                .await
            {
                debug!("Returning cached orderbook for {}/{}", base, quote);
                return Ok(Json(cached));
            }
        }
    }

    // Parse asset identifiers
    let base_asset = AssetPath::parse(&base)
        .map_err(|e| ApiError::InvalidAsset(format!("Invalid base asset: {}", e)))?;
    let quote_asset = AssetPath::parse(&quote)
        .map_err(|e| ApiError::InvalidAsset(format!("Invalid quote asset: {}", e)))?;

    // Get asset IDs from database
    let base_id = find_asset_id(&state, &base_asset).await?;
    let quote_id = find_asset_id(&state, &quote_asset).await?;

    // Fetch asks (selling base for quote)
    let asks = fetch_orderbook_side(&state, base_id, quote_id, true).await?;

    // Fetch bids (buying base with quote - reverse pair)
    let bids = fetch_orderbook_side(&state, quote_id, base_id, false).await?;

    let timestamp = chrono::Utc::now().timestamp();

    let base_info = asset_path_to_info(&base_asset);
    let quote_info = asset_path_to_info(&quote_asset);

    debug!(
        "Orderbook for {}/{}: {} asks, {} bids",
        base,
        quote,
        asks.len(),
        bids.len()
    );

    let response = OrderbookResponse {
        base_asset: base_info,
        quote_asset: quote_info,
        asks,
        bids,
        timestamp,
    };

    // Cache the response (TTL: 5 seconds for orderbook data)
    if let Some(cache) = &state.cache {
        if let Ok(mut cache) = cache.try_lock() {
            let _ = cache
                .set(
                    &cache::keys::orderbook(&base, &quote),
                    &response,
                    Duration::from_secs(5),
                )
                .await;
        }
    }

    Ok(Json(response))
}

/// Find asset ID in database
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
        .fetch_optional(&state.db)
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
        .fetch_optional(&state.db)
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

/// Fetch one side of the orderbook
async fn fetch_orderbook_side(
    state: &AppState,
    selling_id: uuid::Uuid,
    buying_id: uuid::Uuid,
    is_asks: bool,
) -> Result<Vec<OrderbookLevel>> {
    let rows = sqlx::query(
        r#"
        select price::text as price, amount::text as amount
        from sdex_offers
        where selling_asset_id = $1
          and buying_asset_id = $2
        order by price asc
        limit 50
        "#,
    )
    .bind(selling_id)
    .bind(buying_id)
    .fetch_all(&state.db)
    .await?;

    // Aggregate by price level
    let mut levels: BTreeMap<String, (f64, f64)> = BTreeMap::new();

    for row in rows {
        let price_str: String = row.get("price");
        let amount_str: String = row.get("amount");

        let price_f64: f64 = price_str.parse().unwrap_or(0.0);
        let amount_f64: f64 = amount_str.parse().unwrap_or(0.0);

        levels
            .entry(price_str.clone())
            .and_modify(|(_, total_amount)| *total_amount += amount_f64)
            .or_insert((price_f64, amount_f64));
    }

    // Convert to response format with cumulative totals
    let mut cumulative = 0.0;
    let mut result: Vec<OrderbookLevel> = levels
        .into_iter()
        .map(|(price_str, (price_f64, amount))| {
            cumulative += amount * price_f64;
            OrderbookLevel {
                price: price_str,
                amount: format!("{:.7}", amount),
                total: format!("{:.7}", cumulative),
            }
        })
        .collect();

    // For bids, reverse the order (highest price first)
    if !is_asks {
        result.reverse();
    }

    Ok(result)
}

/// Convert AssetPath to AssetInfo
fn asset_path_to_info(asset: &AssetPath) -> AssetInfo {
    if asset.asset_code == "native" {
        AssetInfo::native()
    } else {
        AssetInfo::credit(asset.asset_code.clone(), asset.asset_issuer.clone())
    }
}
