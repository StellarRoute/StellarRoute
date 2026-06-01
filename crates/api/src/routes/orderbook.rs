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
    models::{request::AssetPath, AssetInfo, OrderbookLevel, OrderbookResponse, OrderbookSummary},
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
        (
            status = 400,
            description = "Invalid asset",
            body = crate::models::ErrorResponse,
            example = json!({
                "v": 1,
                "timestamp": 1740312000000_i64,
                "request_id": "req_01hyxk6bzv4n9p8m8j1f4c0a2r",
                "data": {
                    "error": "invalid_asset",
                    "message": "Invalid base asset: unknown asset format"
                }
            })
        ),
        (
            status = 404,
            description = "Asset not found",
            body = crate::models::ErrorResponse,
            example = json!({
                "v": 1,
                "timestamp": 1740312000000_i64,
                "request_id": "req_01hyxk6bzv4n9p8m8j1f4c0a2r",
                "data": {
                    "error": "not_found",
                    "message": "Asset not found in orderbook"
                }
            })
        ),
        (
            status = 500,
            description = "Internal server error",
            body = crate::models::ErrorResponse,
            example = json!({
                "v": 1,
                "timestamp": 1740312000000_i64,
                "request_id": "req_01hyxk6bzv4n9p8m8j1f4c0a2r",
                "data": {
                    "error": "internal_error",
                    "message": "An internal error occurred"
                }
            })
        ),
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
                state.liquidity_thinness_alerts.maybe_alert(&cached);
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

    let summary = compute_orderbook_summary(&bids, &asks);

    let response = OrderbookResponse {
        base_asset: base_info,
        quote_asset: quote_info,
        asks,
        bids,
        summary,
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

    state.liquidity_thinness_alerts.maybe_alert(&response);

    Ok(Json(response))
}

/// Compute summary fields for an orderbook snapshot.
fn compute_orderbook_summary(bids: &Vec<OrderbookLevel>, asks: &Vec<OrderbookLevel>) -> OrderbookSummary {
    let best_bid = bids.first().map(|l| l.price.clone());
    let best_ask = asks.first().map(|l| l.price.clone());

    if let (Some(bid_s), Some(ask_s)) = (&best_bid, &best_ask) {
        match (bid_s.parse::<f64>(), ask_s.parse::<f64>()) {
            (Ok(b), Ok(a)) if a > 0.0 && b > 0.0 => {
                let mid = (a + b) / 2.0;
                let spread = ((a - b) / mid) * 10000.0;
                OrderbookSummary {
                    bid: best_bid,
                    ask: best_ask,
                    spread_bps: Some(spread.round() as i64),
                    midpoint: Some(format!("{:.7}", mid)),
                }
            }
            _ => OrderbookSummary {
                bid: best_bid,
                ask: best_ask,
                spread_bps: None,
                midpoint: None,
            },
        }
    } else {
        OrderbookSummary {
            bid: best_bid,
            ask: best_ask,
            spread_bps: None,
            midpoint: None,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn lvl(price: &str, amount: &str, total: &str) -> OrderbookLevel {
        OrderbookLevel {
            price: price.to_string(),
            amount: amount.to_string(),
            total: total.to_string(),
        }
    }

    #[test]
    fn summary_empty_book() {
        let bids: Vec<OrderbookLevel> = vec![];
        let asks: Vec<OrderbookLevel> = vec![];

        let s = compute_orderbook_summary(&bids, &asks);
        assert!(s.bid.is_none());
        assert!(s.ask.is_none());
        assert!(s.midpoint.is_none());
        assert!(s.spread_bps.is_none());
    }

    #[test]
    fn summary_only_bids() {
        let bids = vec![lvl("0.1050000", "100.0", "10.5")];
        let asks: Vec<OrderbookLevel> = vec![];

        let s = compute_orderbook_summary(&bids, &asks);
        assert_eq!(s.bid.as_deref(), Some("0.1050000"));
        assert!(s.ask.is_none());
        assert!(s.midpoint.is_none());
        assert!(s.spread_bps.is_none());
    }

    #[test]
    fn summary_only_asks() {
        let bids: Vec<OrderbookLevel> = vec![];
        let asks = vec![lvl("0.1060000", "50.0", "5.3")];

        let s = compute_orderbook_summary(&bids, &asks);
        assert!(s.bid.is_none());
        assert_eq!(s.ask.as_deref(), Some("0.1060000"));
        assert!(s.midpoint.is_none());
        assert!(s.spread_bps.is_none());
    }

    #[test]
    fn summary_both_sides() {
        let bids = vec![lvl("0.1050000", "100.0", "10.5")];
        let asks = vec![lvl("0.1060000", "50.0", "5.3")];

        let s = compute_orderbook_summary(&bids, &asks);
        assert_eq!(s.bid.as_deref(), Some("0.1050000"));
        assert_eq!(s.ask.as_deref(), Some("0.1060000"));
        assert_eq!(s.midpoint.as_deref(), Some("0.1055000"));
        assert_eq!(s.spread_bps, Some(95));
    }
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
    .fetch_all(state.db.read_pool())
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
