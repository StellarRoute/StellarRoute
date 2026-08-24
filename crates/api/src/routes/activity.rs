//! Swap activity history endpoints.

use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::{error::Result, models::ApiResponse, state::AppState};

#[derive(Debug, Deserialize)]
pub struct SwapActivityQuery {
    pub limit: Option<i64>,
    pub before_ledger: Option<i64>,
}

/// Resolve effective pagination parameters for swap activity listing.
pub(crate) fn resolve_swap_activity_params(params: &SwapActivityQuery) -> (i64, i64) {
    let limit = params.limit.unwrap_or(50).clamp(1, 100);
    let before_ledger = params.before_ledger.unwrap_or(i64::MAX);
    (limit, before_ledger)
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SwapActivityItem {
    pub event_id: String,
    pub contract_id: String,
    pub ledger: i64,
    pub ledger_closed_at: Option<DateTime<Utc>>,
    pub paging_token: String,
    pub sender: String,
    pub amount_in: String,
    pub amount_out: String,
    pub fee_amount: String,
    pub route: serde_json::Value,
    pub source_asset: Option<String>,
    pub destination_asset: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SwapActivityResponse {
    pub swaps: Vec<SwapActivityItem>,
}

#[utoipa::path(
    get,
    path = "/api/v1/activity/swaps",
    tag = "activity",
    params(
        ("limit" = Option<i64>, Query, description = "Maximum number of swaps to return, capped at 100."),
        ("before_ledger" = Option<i64>, Query, description = "Return swaps before this ledger.")
    ),
    responses(
        (status = 200, description = "Recent indexed contract swaps", body = SwapActivityResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn list_swap_activity(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SwapActivityQuery>,
    request_id: crate::middleware::RequestId,
) -> Result<Json<ApiResponse<SwapActivityResponse>>> {
    let (limit, before_ledger) = resolve_swap_activity_params(&params);

    let rows = sqlx::query(
        r#"
        SELECT
            event_id,
            contract_id,
            ledger,
            ledger_closed_at,
            paging_token,
            sender,
            amount_in::text AS amount_in,
            amount_out::text AS amount_out,
            fee_amount::text AS fee_amount,
            route,
            source_asset,
            destination_asset
        FROM contract_swap_activity
        WHERE ledger < $1
        ORDER BY ledger DESC, event_id DESC
        LIMIT $2
        "#,
    )
    .bind(before_ledger)
    .bind(limit)
    .fetch_all(state.db.read_pool())
    .await?;

    let swaps = rows
        .into_iter()
        .map(|row| SwapActivityItem {
            event_id: row.get("event_id"),
            contract_id: row.get("contract_id"),
            ledger: row.get("ledger"),
            ledger_closed_at: row.get("ledger_closed_at"),
            paging_token: row.get("paging_token"),
            sender: row.get("sender"),
            amount_in: row.get("amount_in"),
            amount_out: row.get("amount_out"),
            fee_amount: row.get("fee_amount"),
            route: row.get("route"),
            source_asset: row.get("source_asset"),
            destination_asset: row.get("destination_asset"),
        })
        .collect();

    Ok(Json(ApiResponse::new(
        SwapActivityResponse { swaps },
        request_id.to_string(),
    )))
}

#[cfg(test)]
mod tests {
    use super::SwapActivityQuery;

    #[test]
    fn resolve_swap_activity_params_defaults_and_clamps_limit() {
        use super::resolve_swap_activity_params;

        assert_eq!(
            resolve_swap_activity_params(&SwapActivityQuery {
                limit: None,
                before_ledger: None,
            }),
            (50, i64::MAX)
        );
        assert_eq!(
            resolve_swap_activity_params(&SwapActivityQuery {
                limit: Some(2),
                before_ledger: None,
            }),
            (2, i64::MAX)
        );
        assert_eq!(
            resolve_swap_activity_params(&SwapActivityQuery {
                limit: Some(500),
                before_ledger: None,
            }),
            (100, i64::MAX)
        );
        assert_eq!(
            resolve_swap_activity_params(&SwapActivityQuery {
                limit: Some(0),
                before_ledger: None,
            }),
            (1, i64::MAX)
        );
        assert_eq!(
            resolve_swap_activity_params(&SwapActivityQuery {
                limit: Some(10),
                before_ledger: Some(90),
            }),
            (10, 90)
        );
    }
}
