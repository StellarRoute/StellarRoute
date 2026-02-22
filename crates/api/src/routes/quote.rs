//! Quote endpoint

use axum::{
    extract::{Path, Query, State},
    Json,
};
use sqlx::Row;
use std::{sync::Arc, time::Duration};
use tracing::debug;

use crate::{
    cache,
    error::{ApiError, Result},
    models::{
        request::{AssetPath, QuoteParams},
        AssetInfo, PathStep, QuoteResponse,
    },
    state::AppState,
};

/// Get price quote for a trading pair
///
/// Returns the best available price for trading the specified amount
#[utoipa::path(
    get,
    path = "/api/v1/quote/{base}/{quote}",
    tag = "trading",
    params(
        ("base" = String, Path, description = "Base asset (e.g., 'native', 'USDC', or 'USDC:ISSUER')"),
        ("quote" = String, Path, description = "Quote asset (e.g., 'native', 'USDC', or 'USDC:ISSUER')"),
        ("amount" = Option<String>, Query, description = "Amount to trade (default: 1)"),
        ("quote_type" = Option<String>, Query, description = "Type of quote: 'sell' or 'buy' (default: sell)"),
    ),
    responses(
        (status = 200, description = "Price quote", body = QuoteResponse),
        (status = 400, description = "Invalid parameters", body = ErrorResponse),
        (status = 404, description = "No route found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
)]
pub async fn get_quote(
    State(state): State<Arc<AppState>>,
    Path((base, quote)): Path<(String, String)>,
    Query(params): Query<QuoteParams>,
) -> Result<Json<QuoteResponse>> {
    debug!(
        "Getting quote for {}/{} with params: {:?}",
        base, quote, params
    );

    // Parse asset identifiers
    let base_asset = AssetPath::parse(&base)
        .map_err(|e| ApiError::InvalidAsset(format!("Invalid base asset: {}", e)))?;
    let quote_asset = AssetPath::parse(&quote)
        .map_err(|e| ApiError::InvalidAsset(format!("Invalid quote asset: {}", e)))?;

    // Parse amount (default to 1)
    let amount: f64 = params
        .amount
        .as_deref()
        .unwrap_or("1")
        .parse()
        .map_err(|_| ApiError::Validation("Invalid amount".to_string()))?;

    if amount <= 0.0 {
        return Err(ApiError::Validation(
            "Amount must be greater than zero".to_string(),
        ));
    }

    // Try to get from cache first
    let amount_str = format!("{:.7}", amount);
    if let Some(cache) = &state.cache {
        if let Ok(mut cache) = cache.try_lock() {
            if let Some(cached) = cache
                .get::<QuoteResponse>(&cache::keys::quote(&base, &quote, &amount_str))
                .await
            {
                debug!("Returning cached quote for {}/{}", base, quote);
                return Ok(Json(cached));
            }
        }
    }

    // For now, implement simple direct path (SDEX only)
    // TODO: Implement multi-hop routing in Phase 2
    let (price, path) = find_best_price(&state, &base_asset, &quote_asset, amount).await?;

    let total = amount * price;
    let timestamp = chrono::Utc::now().timestamp();

    let quote_type = match params.quote_type {
        crate::models::request::QuoteType::Sell => "sell",
        crate::models::request::QuoteType::Buy => "buy",
    };

    let response = QuoteResponse {
        base_asset: asset_path_to_info(&base_asset),
        quote_asset: asset_path_to_info(&quote_asset),
        amount: format!("{:.7}", amount),
        price: format!("{:.7}", price),
        total: format!("{:.7}", total),
        quote_type: quote_type.to_string(),
        path,
        timestamp,
    };

    // Cache the response (TTL: 2 seconds for quote data)
    if let Some(cache) = &state.cache {
        if let Ok(mut cache) = cache.try_lock() {
            let _ = cache
                .set(
                    &cache::keys::quote(&base, &quote, &amount_str),
                    &response,
                    Duration::from_secs(2),
                )
                .await;
        }
    }

    Ok(Json(response))
}

/// Find best price for a trading pair
async fn find_best_price(
    state: &AppState,
    base: &AssetPath,
    quote: &AssetPath,
    _amount: f64,
) -> Result<(f64, Vec<PathStep>)> {
    // Get asset IDs
    let base_id = find_asset_id(state, base).await?;
    let quote_id = find_asset_id(state, quote).await?;

    // Find best offer
    let row = sqlx::query(
        r#"
        select price::text as price
        from sdex_offers
        where selling_asset_id = $1
          and buying_asset_id = $2
        order by price asc
        limit 1
        "#,
    )
    .bind(base_id)
    .bind(quote_id)
    .fetch_optional(&state.db)
    .await?;

    match row {
        Some(row) => {
            let price_str: String = row.get("price");
            let price_f64: f64 = price_str.parse().unwrap_or(0.0);

            // Create simple path
            let path = vec![PathStep {
                from_asset: asset_path_to_info(base),
                to_asset: asset_path_to_info(quote),
                price: format!("{:.7}", price_f64),
                source: "sdex".to_string(),
            }];

            Ok((price_f64, path))
        }
        None => Err(ApiError::NoRouteFound),
    }
}

/// Find asset ID in database
async fn find_asset_id(state: &AppState, asset: &AssetPath) -> Result<uuid::Uuid> {
    use sqlx::Row;

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
        None => Err(ApiError::NotFound(format!(
            "Asset not found: {}",
            asset.asset_code
        ))),
    }
}

/// Convert AssetPath to AssetInfo
fn asset_path_to_info(asset: &AssetPath) -> AssetInfo {
    if asset.asset_code == "native" {
        AssetInfo::native()
    } else {
        AssetInfo::credit(asset.asset_code.clone(), asset.asset_issuer.clone())
    }
}
