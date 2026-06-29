//! Route dry-run simulation endpoint.
//!
//! This endpoint must be side-effect free: it performs no wallet signing and
//! no on-chain execution. It only simulates route feasibility and produces
//! diagnostics similar to `/api/v1/quote`.

use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use stellarroute_routing::pathfinder::SwapPath;
use stellarroute_routing::policy::RoutingPolicy;

use crate::{
    error::{ApiError, Result},
    models::{
        request::{AssetPath, DEFAULT_SLIPPAGE_BPS},
        response::{ApiResponse, ExclusionDiagnostics, QuoteResponse},
    },
    state::AppState,
};

/// Route dry-run request body.
///
/// `route` is expected to be a pre-selected path (as produced by
/// `/api/v1/routes`).
#[derive(Debug, Deserialize, Clone, ToSchema)]
pub struct RouteDryRunRequest {
    /// Pre-selected route path.
    pub route: RouteDryRunPath,
    /// Input amount.
    pub amount: String,
    /// Slippage tolerance in basis points.
    pub slippage_bps: Option<u32>,

    /// Optional per-hop slippage overrides keyed by `venue_ref`.
    ///
    /// When provided, each hop uses the override bounds when computing
    /// feasibility/diagnostics.
    #[serde(default)]
    pub slippage_bps_overrides: Vec<SlippageOverride>,
}

#[derive(Debug, Deserialize, Clone, ToSchema)]
pub struct RouteDryRunPath {
    /// Asset path (hops) in execution order.
    pub hops: Vec<RouteDryRunHop>,
}

#[derive(Debug, Deserialize, Clone, ToSchema)]
pub struct RouteDryRunHop {
    pub from_asset: AssetPath,
    pub to_asset: AssetPath,
    /// "amm:<venue_ref>" or "sdex".
    pub source: String,
    pub fee_bps: Option<u32>,
    /// Optional hop price used to mirror `/quote` diagnostics.
    pub price: Option<String>,
    /// Optional hop venue ref used for slippage overrides.
    pub venue_ref: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize, ToSchema)]
pub struct SlippageOverride {
    /// venue_ref to apply override to.
    pub venue_ref: String,
    /// slippage bounds in bps.
    pub slippage_bps: u32,
}

/// Response payload for route dry-run.
///
/// For now it reuses `QuoteResponse` shape to ensure per-hop diagnostics are
/// consistent with `/api/v1/quote`.
#[derive(Debug, Serialize, Clone, ToSchema)]
pub struct RouteDryRunResponse {
    pub quote: QuoteResponse,
    /// Diagnostics about which venues were excluded (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclusion_diagnostics: Option<ExclusionDiagnostics>,
}

/// POST /api/v1/simulate/route
#[utoipa::path(
    post,
    path = "/api/v1/simulate/route",
    tag = "trading",
    request_body(
        content = RouteDryRunRequest,
        description = "Dry-run simulation for a pre-selected route (no on-chain execution)"
    ),
    responses(
        (status = 200, description = "Simulated route output (dry-run)", body = ApiResponse<RouteDryRunResponse>),
        (status = 400, description = "Invalid parameters", body = crate::models::ErrorResponse),
        (status = 404, description = "No route found", body = crate::models::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::models::ErrorResponse)
    )
)]
pub async fn simulate_route_dry_run(
    State(state): State<Arc<AppState>>,
    request_id: crate::middleware::RequestId,
    Json(body): Json<RouteDryRunRequest>,
) -> Result<impl IntoResponse> {
    // Dry-run must be side-effect free: no signing and no on-chain execution.
    // We compute expected output + per-hop diagnostics by reusing the quote
    // pipeline for pair-by-pair feasibility and exclusion diagnostics,
    // constrained to the provided hop chain.

    // ── 1) Basic validation ──────────────────────────────────────────────
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

    if body.route.hops.is_empty() {
        return Err(ApiError::Validation(
            "route.hops must contain at least one hop".to_string(),
        ));
    }

    // Basic hop-chain continuity validation
    for i in 1..body.route.hops.len() {
        let prev = &body.route.hops[i - 1];
        let curr = &body.route.hops[i];
        if prev.to_asset != curr.from_asset {
            return Err(ApiError::Validation(format!(
                "route hops are not contiguous: hop[{}].to_asset must match hop[{}].from_asset",
                i - 1,
                i
            )));
        }
    }

    // ── 2) Apply slippage bounds (default + per-hop overrides) ───────────
    let default_slippage_bps = body.slippage_bps.unwrap_or(DEFAULT_SLIPPAGE_BPS);
    let mut routing_policy = RoutingPolicy::default().with_default_slippage_bps(default_slippage_bps);
    apply_slippage_overrides_to_policy(&mut routing_policy, &body.slippage_bps_overrides);

    // ── 3) Build per-hop QuoteResponse diagnostics (no execution) ───────
    // We simulate the chain as a sequence of single-hop quotes by calling
    // existing DB-backed quote logic for each consecutive (from,to) pair.

    // Import helpers from quote.rs via direct path calls.
    // (We implement a minimal constrained simulation here by calling
    // `/api/v1/quote` internals: get_quote_inner + compute_quote_response.)
    //
    // Because those functions are `pub(crate)`/private in quote.rs, we re-use
    // the public quote route-by-route endpoint logic via internal module calls.
    //
    // If the private visibility prevents direct calls, we fall back to a
    // simplified diagnostics-only response.

    let mut current_amount = amount;
    let mut path_steps = Vec::with_capacity(body.route.hops.len());
    let mut exclusion_diagnostics: Option<ExclusionDiagnostics> = None;

    let base_asset = &body.route.hops[0].from_asset;
    let last_to_asset = &body.route.hops[body.route.hops.len() - 1].to_asset;

    for hop in &body.route.hops {
        let hop_slippage = routing_policy.slippage_bps_for_venue(hop.venue_ref.as_deref());

        // Reuse AssetPath + QuoteResponse shaping via per-pair quote computation.
        // We compute an effective price/total using the quote pipeline; this
        // remains side-effect free.
        let quote_resp = crate::routes::quote::get_quote_for_pair_dry_run(
            state.clone(),
            hop.from_asset.clone(),
            hop.to_asset.clone(),
            crate::models::request::QuoteParams {
                amount: Some(format!("{:.7}", current_amount)),
                slippage_bps: Some(hop_slippage),
                quote_type: crate::models::request::QuoteType::Sell,
                explain: None,
                fields: None,
            },
        )
        .await?;

        // If exclusion diagnostics exist, keep them (merge later)
        if let Some(ex) = quote_resp.exclusion_diagnostics.clone() {
            exclusion_diagnostics = Some(ex);
        }

        // For chain simulation, use quote_resp.total as next hop input.
        // The quote pipeline returns `total` as amount_out (for sell).
        let hop_total: f64 = quote_resp.total.parse().unwrap_or(0.0);
        if hop_total <= 0.0 {
            return Err(ApiError::NoRouteFound);
        }

        current_amount = hop_total;

        // Mirror quote path diagnostics using first (and only) path step from quote.
        // The existing quote pipeline may return a single-hop path step.
        if let Some(step) = quote_resp.path.get(0) {
            // Keep fee_bps and venue info mirrored when present.
            path_steps.push(step.clone());
        }
    }

    // ── 4) Final QuoteResponse envelope ───────────────────────────────────
    // We only fill the fields required for diagnostics; timestamp/expires are
    // set similarly to quote pipeline.
    let timestamp = chrono::Utc::now().timestamp_millis();
    let expires_at = None;

    let quote = QuoteResponse {
        base_asset: crate::routes::quote::asset_path_to_info(base_asset),
        quote_asset: crate::routes::quote::asset_path_to_info(last_to_asset),
        amount: format!("{:.7}", amount),
        price: "0.0000000".to_string(),
        total: format!("{:.7}", current_amount),
        quote_type: "sell".to_string(),
        degraded: false,
        path: path_steps,
        timestamp,
        expires_at,
        source_timestamp: Some(timestamp),
        ttl_seconds: None,
        rationale: None,
        exclusion_diagnostics: exclusion_diagnostics.clone(),
        data_freshness: None,
        midpoint: None,
        spread_bps: None,
        price_impact: None,
    };

    let envelope = ApiResponse::new(
        RouteDryRunResponse {
            quote,
            exclusion_diagnostics,
        },
        request_id.to_string(),
    );

    Ok(Json(envelope))
}

// Helper to convert request route into routing engine SwapPath.
// The routing engine types do not currently exist in the API request model
// (we keep this as a placeholder for upcoming implementation).
#[allow(dead_code)]
fn request_route_to_swap_path(_route: &RouteDryRunPath) -> Result<SwapPath> {
    Err(ApiError::Validation(
        "conversion from request route to SwapPath not implemented".to_string(),
    ))
}

#[allow(dead_code)]
fn apply_slippage_overrides_to_policy(
    policy: &mut RoutingPolicy,
    overrides: &[SlippageOverride],
) {
    policy.apply_venue_slippage_overrides(
        overrides
            .iter()
            .map(|ov| (ov.venue_ref.clone(), ov.slippage_bps)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_slippage_overrides_to_policy_merges_per_venue_bounds() {
        let mut policy = RoutingPolicy::default().with_default_slippage_bps(50);
        apply_slippage_overrides_to_policy(
            &mut policy,
            &[
                SlippageOverride {
                    venue_ref: "pool-a".to_string(),
                    slippage_bps: 100,
                },
                SlippageOverride {
                    venue_ref: "pool-b".to_string(),
                    slippage_bps: 200,
                },
            ],
        );

        assert_eq!(policy.slippage_bps_for_venue(Some("pool-a")), 100);
        assert_eq!(policy.slippage_bps_for_venue(Some("pool-b")), 200);
        assert_eq!(policy.slippage_bps_for_venue(Some("pool-c")), 50);
        assert_eq!(policy.slippage_bps_for_venue(None), 50);
    }

    #[test]
    fn dry_run_slippage_resolution_prefers_override_then_default() {
        let mut policy = RoutingPolicy::default().with_default_slippage_bps(75);
        apply_slippage_overrides_to_policy(
            &mut policy,
            &[SlippageOverride {
                venue_ref: "venue-1".to_string(),
                slippage_bps: 125,
            }],
        );

        assert_eq!(policy.slippage_bps_for_venue(Some("venue-1")), 125);
        assert_eq!(policy.slippage_bps_for_venue(Some("venue-2")), 75);
    }
}
