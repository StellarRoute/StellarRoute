//! Swap prepare/submit endpoints (issue #1051).
//!
//! These endpoints define the durable OpenAPI contract for the live swap
//! path: `prepare` builds an unsigned transaction envelope from a
//! pre-selected route, and `submit` accepts a signed envelope and submits it
//! on-chain. Neither transaction construction nor on-chain submission is
//! implemented yet (tracked under milestone M4 — Live swap path; see
//! `docs/readiness/live-swap-testnet-checklist.md`). Both handlers validate
//! their input using the same engine validation as `/api/v1/simulate/route`,
//! then return a documented `501 not_implemented` error so the SDK and
//! frontend can integrate against a stable, versioned contract today and
//! detect the moment real execution ships (see
//! `sdk-js/src/client.ts`'s `executeSwap`, which already expects this code).

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use crate::{
    error::{ApiError, Result},
    models::ApiResponse,
    routes::simulation_route::{request_route_to_swap_path, RouteDryRunPath},
    state::AppState,
};

// ── Request / response types ─────────────────────────────────────────────────

/// Request body for `POST /api/v1/swap/prepare`.
#[derive(Debug, Deserialize, Clone, ToSchema)]
pub struct SwapPrepareRequest {
    /// Pre-selected route path (as produced by `/api/v1/routes`).
    pub route: RouteDryRunPath,
    /// Input amount.
    pub amount: String,
    /// Stellar account (G...) that will sign and submit the transaction.
    pub sender: String,
    /// Minimum acceptable output amount; the caller should abort signing if
    /// the prepared transaction quotes less than this.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_output: Option<String>,
    /// Slippage tolerance in basis points (default: 50).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slippage_bps: Option<u32>,
}

/// Response body for `POST /api/v1/swap/prepare`.
///
/// Mirrors `ExecuteSwapResult` in `sdk-js/src/types.ts`.
#[derive(Debug, Serialize, Clone, ToSchema)]
pub struct SwapPrepareResponse {
    /// Base64-encoded unsigned transaction envelope (XDR) ready for signing.
    pub xdr_envelope: String,
    /// Expected output amount at the time of preparation.
    pub expected_output: String,
    /// Unix timestamp (ms) after which the prepared envelope should be
    /// considered stale and re-prepared.
    pub expires_at: i64,
}

/// Request body for `POST /api/v1/swap/submit`.
#[derive(Debug, Deserialize, Clone, ToSchema)]
pub struct SwapSubmitRequest {
    /// Base64-encoded signed transaction envelope (XDR).
    pub xdr_envelope: String,
    /// Idempotency key correlating this submission with a prior `prepare`
    /// call; duplicate submissions for the same key should be rejected once
    /// submission is implemented.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote_id: Option<String>,
}

/// Response body for `POST /api/v1/swap/submit`.
#[derive(Debug, Serialize, Clone, ToSchema)]
pub struct SwapSubmitResponse {
    /// On-chain transaction hash.
    pub tx_hash: String,
    /// Submission status (e.g. `"pending"`, `"success"`, `"failed"`).
    pub status: String,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// POST /api/v1/swap/prepare
///
/// Validates a pre-selected route and amount, then builds an unsigned
/// transaction envelope for the caller to sign. Transaction construction is
/// not implemented yet; a valid request currently returns `501
/// not_implemented` after passing validation.
#[utoipa::path(
    post,
    path = "/api/v1/swap/prepare",
    tag = "swap",
    request_body(
        content = SwapPrepareRequest,
        description = "Pre-selected route and amount to build an unsigned swap transaction for"
    ),
    responses(
        (status = 200, description = "Unsigned transaction envelope ready for signing", body = ApiResponse<SwapPrepareResponse>),
        (status = 400, description = "Invalid parameters", body = crate::models::ErrorResponse),
        (status = 501, description = "Swap transaction building is not yet available", body = crate::models::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::models::ErrorResponse),
    )
)]
pub async fn prepare_swap(
    State(_state): State<Arc<AppState>>,
    _request_id: crate::middleware::RequestId,
    Json(body): Json<SwapPrepareRequest>,
) -> Result<Json<ApiResponse<SwapPrepareResponse>>> {
    validate_prepare_request(&body)?;

    Err(ApiError::NotImplemented(
        "swap prepare (transaction building) is not yet available; see \
         docs/readiness/live-swap-testnet-checklist.md"
            .to_string(),
    ))
}

/// POST /api/v1/swap/submit
///
/// Accepts a signed transaction envelope and submits it on-chain.
/// Submission is not implemented yet; a valid request currently returns
/// `501 not_implemented` after passing validation.
#[utoipa::path(
    post,
    path = "/api/v1/swap/submit",
    tag = "swap",
    request_body(
        content = SwapSubmitRequest,
        description = "Signed transaction envelope to submit on-chain"
    ),
    responses(
        (status = 200, description = "Submission accepted", body = ApiResponse<SwapSubmitResponse>),
        (status = 400, description = "Invalid parameters", body = crate::models::ErrorResponse),
        (status = 501, description = "Swap submission is not yet available", body = crate::models::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::models::ErrorResponse),
    )
)]
pub async fn submit_swap(
    State(_state): State<Arc<AppState>>,
    _request_id: crate::middleware::RequestId,
    Json(body): Json<SwapSubmitRequest>,
) -> Result<Json<ApiResponse<SwapSubmitResponse>>> {
    if body.xdr_envelope.trim().is_empty() {
        return Err(ApiError::Validation(
            "xdr_envelope must not be empty".to_string(),
        ));
    }

    Err(ApiError::NotImplemented(
        "swap submit (on-chain submission) is not yet available; see \
         docs/readiness/live-swap-testnet-checklist.md"
            .to_string(),
    ))
}

// ── Validation ───────────────────────────────────────────────────────────────

fn validate_prepare_request(body: &SwapPrepareRequest) -> Result<()> {
    if body.amount.trim().is_empty() {
        return Err(ApiError::Validation("amount must be non-empty".to_string()));
    }

    let amount: f64 = body
        .amount
        .parse()
        .map_err(|_| ApiError::Validation("amount must be a valid number".to_string()))?;
    if !amount.is_finite() || amount <= 0.0 {
        return Err(ApiError::Validation(
            "amount must be greater than zero".to_string(),
        ));
    }

    if body.sender.trim().is_empty() {
        return Err(ApiError::Validation("sender must be non-empty".to_string()));
    }

    // Reuses the routing engine's own validation (contiguity, cycle
    // detection, non-empty hops) so `prepare` and `/api/v1/simulate/route`
    // never disagree about what counts as a valid route.
    request_route_to_swap_path(&body.route)?;

    Ok(())
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::request::AssetPath;

    fn valid_request() -> SwapPrepareRequest {
        SwapPrepareRequest {
            route: RouteDryRunPath {
                hops: vec![crate::routes::simulation_route::RouteDryRunHop {
                    from_asset: AssetPath::parse("native").unwrap(),
                    to_asset: AssetPath::parse("USDC").unwrap(),
                    source: "sdex".to_string(),
                    fee_bps: Some(30),
                    price: Some("0.12".to_string()),
                    venue_ref: Some("sdex-venue".to_string()),
                }],
            },
            amount: "100".to_string(),
            sender: "GABCDEF".to_string(),
            min_output: None,
            slippage_bps: None,
        }
    }

    #[test]
    fn valid_request_passes_validation() {
        assert!(validate_prepare_request(&valid_request()).is_ok());
    }

    #[test]
    fn empty_amount_is_rejected() {
        let mut req = valid_request();
        req.amount = "".to_string();
        assert!(matches!(
            validate_prepare_request(&req),
            Err(ApiError::Validation(_))
        ));
    }

    #[test]
    fn zero_amount_is_rejected() {
        let mut req = valid_request();
        req.amount = "0".to_string();
        assert!(matches!(
            validate_prepare_request(&req),
            Err(ApiError::Validation(_))
        ));
    }

    #[test]
    fn empty_sender_is_rejected() {
        let mut req = valid_request();
        req.sender = "".to_string();
        assert!(matches!(
            validate_prepare_request(&req),
            Err(ApiError::Validation(_))
        ));
    }

    #[test]
    fn empty_route_is_rejected() {
        let mut req = valid_request();
        req.route = RouteDryRunPath { hops: vec![] };
        assert!(matches!(
            validate_prepare_request(&req),
            Err(ApiError::Validation(_))
        ));
    }

}
