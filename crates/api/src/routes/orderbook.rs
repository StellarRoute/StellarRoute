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
    // Parse asset identifiers
    let base_asset = AssetPath::parse(&base)
        .map_err(|e| ApiError::InvalidAsset(format!("Invalid base asset: {}", e)))?;
    let quote_asset = AssetPath::parse(&quote)
        .map_err(|e| ApiError::InvalidAsset(format!("Invalid quote asset: {}", e)))?;

    let response = get_orderbook_inner(state, base_asset, quote_asset).await?;
    Ok(Json(response))
}

pub(crate) async fn get_orderbook_inner(
    state: Arc<AppState>,
    base_asset: AssetPath,
    quote_asset: AssetPath,
) -> Result<OrderbookResponse> {
    let base = base_asset.to_canonical();
    let quote = quote_asset.to_canonical();

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
                return Ok(cached);
            }
        }
    }

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

    Ok(response)
}

/// Maximum number of items allowed in a single batch request.
pub const BATCH_MAX_ITEMS: usize = 25;

/// Map an [`ApiError`] to a `(code, message)` pair for per-item batch errors.
fn batch_error_from_api_error(e: &ApiError) -> (String, String) {
    match e {
        ApiError::NotFound(msg) => ("not_found".to_string(), msg.clone()),
        ApiError::InvalidAsset(msg) => ("invalid_asset".to_string(), msg.clone()),
        ApiError::Validation(msg) => ("validation_error".to_string(), msg.clone()),
        _ => (
            "internal_error".to_string(),
            "An internal error occurred".to_string(),
        ),
    }
}

/// POST /api/v1/batch/orderbook
///
/// Evaluate up to 25 orderbooks in a single request.
///
/// All items are executed concurrently. Per-item failures (e.g. invalid asset)
/// do not abort the batch — each item carries its own `status` field.
///
/// # Request size limits
///
/// | Limit                  | Value |
/// |------------------------|-------|
/// | Maximum items per call | 25    |
/// | Minimum items per call | 1     |
///
/// # Rate Limit Policy
///
/// Batch endpoints consume rate limits per-request, not per-item. For example,
/// a batch of 25 pairs counts as 1 request against the IP rate limit bucket.
#[utoipa::path(
    post,
    path = "/api/v1/batch/orderbook",
    tag = "trading",
    request_body(
        content = crate::models::request::BatchOrderbookRequest,
        description = "Up to 25 orderbook items to evaluate concurrently",
        example = json!({
            "requests": [
                {
                    "base": "native",
                    "quote": "USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5"
                },
                {
                    "base": "native",
                    "quote": "yXLM:GARDNV3Q7YGT4AKSDF25LT32YSCCW4EV22Y2TV3I2PU2MMXJTEDL5T55"
                }
            ]
        })
    ),
    responses(
        (
            status = 200,
            description = "Batch orderbook results (individual items may have status=error)",
            body = crate::models::response::BatchOrderbookResponse,
            example = json!({
                "v": 1,
                "timestamp": 1714000000000_i64,
                "request_id": "req-abc123",
                "data": {
                    "results": [
                        {
                            "index": 0,
                            "status": "ok",
                            "orderbook": {
                                "base_asset": {"asset_type": "native"},
                                "quote_asset": {
                                    "asset_type": "credit_alphanum4",
                                    "asset_code": "USDC",
                                    "asset_issuer": "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5"
                                },
                                "asks": [],
                                "bids": [],
                                "summary": {},
                                "timestamp": 1714000000000_i64
                            }
                        },
                        {
                            "index": 1,
                            "status": "error",
                            "error": {
                                "code": "not_found",
                                "message": "Asset not found in orderbook"
                            }
                        }
                    ],
                    "items_succeeded": 1,
                    "items_failed": 1,
                    "total": 2
                }
            })
        ),
        (
            status = 400,
            description = "Invalid batch request (empty, too large, or malformed items)",
            body = crate::models::ErrorResponse
        ),
        (
            status = 429,
            description = "Rate limit exceeded",
            body = crate::models::ErrorResponse
        ),
    )
)]
pub async fn get_batch_orderbooks(
    State(state): State<Arc<AppState>>,
    request_id: crate::middleware::RequestId,
    Json(payload): Json<crate::models::request::BatchOrderbookRequest>,
) -> Result<Json<crate::models::ApiResponse<crate::models::response::BatchOrderbookResponse>>> {
    use crate::models::response::{BatchItemError, BatchOrderbookItemResult, BatchOrderbookResponse};
    use futures_util::future::join_all;

    // ── 1. Batch-level validation ─────────────────────────────────────────
    if payload.requests.is_empty() {
        return Err(ApiError::Validation(
            "Batch request must contain at least 1 item".to_string(),
        ));
    }
    if payload.requests.len() > BATCH_MAX_ITEMS {
        return Err(ApiError::Validation(format!(
            "Batch size {} exceeds maximum of {} items",
            payload.requests.len(),
            BATCH_MAX_ITEMS
        )));
    }

    // ── 2. Per-item pre-validation (fail fast on obviously bad inputs) ────
    let mut pre_errors: Vec<Option<BatchItemError>> = vec![None; payload.requests.len()];
    for (i, item) in payload.requests.iter().enumerate() {
        if let Err(msg) = item.validate() {
            pre_errors[i] = Some(BatchItemError {
                code: "validation_error".to_string(),
                message: msg,
            });
        }
    }

    // ── 3. Concurrent execution ───────────────────────────────────────────
    let futures: Vec<_> = payload
        .requests
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, item)| {
            let state = state.clone();
            let pre_err = pre_errors[i].take();
            async move {
                if let Some(err) = pre_err {
                    return BatchOrderbookItemResult::err(i, err);
                }

                let base_asset = match AssetPath::parse(&item.base) {
                    Ok(a) => a,
                    Err(e) => {
                        return BatchOrderbookItemResult::err(
                            i,
                            BatchItemError {
                                code: "invalid_asset".to_string(),
                                message: format!("Invalid base asset: {}", e),
                            },
                        )
                    }
                };
                let quote_asset = match AssetPath::parse(&item.quote) {
                    Ok(a) => a,
                    Err(e) => {
                        return BatchOrderbookItemResult::err(
                            i,
                            BatchItemError {
                                code: "invalid_asset".to_string(),
                                message: format!("Invalid quote asset: {}", e),
                            },
                        )
                    }
                };

                match get_orderbook_inner(state, base_asset, quote_asset).await {
                    Ok(orderbook) => BatchOrderbookItemResult::ok(i, orderbook),
                    Err(e) => {
                        let (code, message) = batch_error_from_api_error(&e);
                        BatchOrderbookItemResult::err(i, BatchItemError { code, message })
                    }
                }
            }
        })
        .collect();

    let results: Vec<BatchOrderbookItemResult> = join_all(futures).await;

    // ── 4. Aggregate counters ─────────────────────────────────────────────
    let items_succeeded = results.iter().filter(|r| r.status == "ok").count();
    let items_failed = results.len() - items_succeeded;
    let total = results.len();

    let response = BatchOrderbookResponse {
        results,
        items_succeeded,
        items_failed,
        total,
    };

    let envelope = crate::models::ApiResponse::new(response, request_id.to_string());
    Ok(Json(envelope))
}

/// Compute summary fields for an orderbook snapshot.
fn compute_orderbook_summary(
    bids: &Vec<OrderbookLevel>,
    asks: &Vec<OrderbookLevel>,
) -> OrderbookSummary {
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
