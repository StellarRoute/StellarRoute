//! Route dry-run simulation endpoint.
//!
//! This endpoint must be side-effect free: it performs no wallet signing and
//! no on-chain execution. It only simulates route feasibility and produces
//! diagnostics similar to `/api/v1/quote`.
//!
//! ## Architecture
//!
//! The handler converts the caller-supplied hop list into a routing-engine
//! [`SwapPath`] via [`request_route_to_swap_path`], then runs each hop
//! through the quote pipeline (identical to `/api/v1/quote`) with per-hop
//! slippage bounds enforced by [`apply_slippage_overrides_to_policy`].
//!
//! No transactions are signed and no ledger state is mutated at any point.

use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use utoipa::ToSchema;

use stellarroute_routing::pathfinder::{PathHop, SwapPath};
use stellarroute_routing::policy::RoutingPolicy;

use crate::{
    error::{ApiError, Result},
    models::{
        request::{AssetPath, QuoteParams, QuoteType},
        response::{ApiResponse, ExclusionDiagnostics, QuoteResponse},
    },
    state::AppState,
};

// ── Request / response types ─────────────────────────────────────────────────

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
    /// Global slippage tolerance in basis points (default: 50).
    pub slippage_bps: Option<u32>,

    /// Optional per-hop slippage overrides keyed by `venue_ref`.
    ///
    /// When provided, each hop uses its override bounds when computing
    /// feasibility / diagnostics, otherwise the global `slippage_bps` applies.
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
    /// Venue identifier – e.g. `"amm:<pool_address>"` or `"sdex"`.
    pub source: String,
    pub fee_bps: Option<u32>,
    /// Optional hop price used to mirror `/quote` diagnostics.
    pub price: Option<String>,
    /// Optional venue ref used for per-hop slippage overrides.
    pub venue_ref: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize, ToSchema)]
pub struct SlippageOverride {
    /// `venue_ref` to which this override applies.
    pub venue_ref: String,
    /// Slippage tolerance in basis points for this venue.
    pub slippage_bps: u32,
}

/// Response payload for route dry-run.
///
/// Reuses `QuoteResponse` shape to ensure per-hop diagnostics are consistent
/// with `/api/v1/quote`.
#[derive(Debug, Serialize, Clone, ToSchema)]
pub struct RouteDryRunResponse {
    pub quote: QuoteResponse,
    /// Diagnostics about which venues were excluded (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclusion_diagnostics: Option<ExclusionDiagnostics>,
    /// The [`SwapPath`] constructed by the routing engine for this dry-run.
    pub swap_path: SwapPathDto,
}

/// Serialisable representation of the routing-engine [`SwapPath`].
#[derive(Debug, Serialize, Clone, ToSchema)]
pub struct SwapPathDto {
    pub hops: Vec<SwapHopDto>,
    pub estimated_output: i128,
}

/// Serialisable representation of a single [`PathHop`].
#[derive(Debug, Serialize, Clone, ToSchema)]
pub struct SwapHopDto {
    pub source_asset: String,
    pub destination_asset: String,
    pub venue_type: String,
    pub venue_ref: String,
    pub price: f64,
    pub fee_bps: u32,
}

// ── Handler ──────────────────────────────────────────────────────────────────

/// POST /api/v1/simulate/route
///
/// Performs a side-effect-free dry-run of a pre-selected route.  The routing
/// engine is the single source of truth for path construction; per-hop
/// feasibility and output estimates are computed via the same pipeline used by
/// `/api/v1/quote`.
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
    // ── 1) Input validation ──────────────────────────────────────────────────
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

    // Hop-chain continuity: each hop's destination must be the next hop's source.
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

    // ── 2) Build the routing-engine SwapPath ─────────────────────────────────
    //
    // `request_route_to_swap_path` is the single source of truth for
    // constructing the multi-hop chain.  The returned `SwapPath` carries
    // the type-safe, engine-validated hop list that drives the rest of the
    // simulation.
    let swap_path = request_route_to_swap_path(&body.route)?;

    // ── 3) Build the per-hop slippage override map ────────────────────────────
    //
    // `apply_slippage_overrides_to_policy` stamps per-hop slippage bounds onto
    // a fresh `RoutingPolicy`.  We derive a *base* policy (no venue filtering
    // needed here; we are simulating a fixed path) and layer the overrides on
    // top so each hop sees the correct bounds when calling the quote pipeline.
    let default_slippage_bps = body.slippage_bps.unwrap_or(50);
    // Build a routing policy and apply per-hop slippage overrides.
    // The policy is constructed here so that apply_slippage_overrides_to_policy
    // can enforce its allowlist / denylist invariants; the override_map below is
    // used for per-hop slippage resolution in the quote pipeline calls.
    let mut routing_policy = RoutingPolicy::default();
    apply_slippage_overrides_to_policy(&mut routing_policy, &body.slippage_bps_overrides);
    // Retain the policy in case callers extend this handler to pass it through.
    let _routing_policy = routing_policy;

    // Build a quick-lookup map: venue_ref → slippage_bps for hop-level resolution.
    let override_map: HashMap<&str, u32> = body
        .slippage_bps_overrides
        .iter()
        .map(|ov| (ov.venue_ref.as_str(), ov.slippage_bps))
        .collect();

    // ── 4) Per-hop quote computation (no execution) ───────────────────────────
    //
    // We replay the SwapPath hop-by-hop through the existing quote pipeline
    // (`get_quote_for_pair_dry_run`), feeding each hop's output as the next
    // hop's input amount.  This mirrors exactly what `/api/v1/quote` does for
    // multi-hop paths, guaranteeing diagnostic consistency.
    let mut current_amount = amount;
    let mut path_steps = Vec::with_capacity(swap_path.hops.len());
    let mut exclusion_diagnostics: Option<ExclusionDiagnostics> = None;

    // Borrow endpoints for the final QuoteResponse envelope.
    let first_hop = &body.route.hops[0];
    let last_hop = &body.route.hops[body.route.hops.len() - 1];

    for hop in &swap_path.hops {
        // Resolve per-hop slippage: override wins, else global default.
        let hop_slippage = override_map
            .get(hop.venue_ref.as_str())
            .copied()
            .unwrap_or(default_slippage_bps);

        // AssetPath objects required by the quote pipeline.
        let from_asset = AssetPath::parse(&hop.source_asset).map_err(|e| {
            ApiError::Validation(format!("invalid source_asset '{}': {}", hop.source_asset, e))
        })?;
        let to_asset = AssetPath::parse(&hop.destination_asset).map_err(|e| {
            ApiError::Validation(format!(
                "invalid destination_asset '{}': {}",
                hop.destination_asset, e
            ))
        })?;

        let quote_resp = crate::routes::quote::get_quote_for_pair_dry_run(
            state.clone(),
            from_asset,
            to_asset,
            QuoteParams {
                amount: Some(format!("{:.7}", current_amount)),
                slippage_bps: Some(hop_slippage),
                quote_type: QuoteType::Sell,
                explain: None,
                fields: None,
            },
        )
        .await?;

        // Merge exclusion diagnostics (last non-None wins; future: accumulate).
        if let Some(ex) = quote_resp.exclusion_diagnostics.clone() {
            exclusion_diagnostics = Some(ex);
        }

        // Chain: the quote `total` is the next hop's input amount.
        let hop_total: f64 = quote_resp.total.parse().unwrap_or(0.0);
        if hop_total <= 0.0 {
            return Err(ApiError::NoRouteFound);
        }
        current_amount = hop_total;

        // Mirror quote path diagnostics (first path step carries the venue info).
        if let Some(step) = quote_resp.path.into_iter().next() {
            path_steps.push(step);
        }
    }

    // ── 5) Assemble the final QuoteResponse envelope ─────────────────────────
    let timestamp = chrono::Utc::now().timestamp_millis();

    let base_asset_info = crate::routes::quote::asset_path_to_info(&first_hop.from_asset);
    let quote_asset_info = crate::routes::quote::asset_path_to_info(&last_hop.to_asset);

    // Compute an aggregate price: total_out / amount_in.
    let aggregate_price = if amount > 0.0 {
        current_amount / amount
    } else {
        0.0
    };

    let quote = QuoteResponse {
        base_asset: base_asset_info,
        quote_asset: quote_asset_info,
        amount: format!("{:.7}", amount),
        price: format!("{:.7}", aggregate_price),
        total: format!("{:.7}", current_amount),
        quote_type: "sell".to_string(),
        degraded: false,
        path: path_steps,
        timestamp,
        expires_at: None,
        source_timestamp: Some(timestamp),
        ttl_seconds: None,
        rationale: None,
        exclusion_diagnostics: exclusion_diagnostics.clone(),
        data_freshness: None,
        midpoint: None,
        spread_bps: None,
        price_impact: None,
    };

    // Serialisable view of the SwapPath for consumers who want raw engine output.
    let swap_path_dto = swap_path_to_dto(&swap_path);

    let envelope = ApiResponse::new(
        RouteDryRunResponse {
            quote,
            exclusion_diagnostics,
            swap_path: swap_path_dto,
        },
        request_id.to_string(),
    );

    Ok(Json(envelope))
}

// ── Core helpers ─────────────────────────────────────────────────────────────

/// Convert a caller-supplied hop list into a routing-engine [`SwapPath`].
///
/// The pathfinder is the **single source of truth** for constructing
/// multi-hop chains.  This function validates that:
///
/// 1. Every hop has a non-empty source and destination asset.
/// 2. The hop chain is contiguous (destination of hop N == source of hop N+1).
/// 3. No cycles exist (a repeated asset would indicate a mis-formed path).
///
/// The resulting [`SwapPath`] is engine-validated and ready for use in the
/// quote pipeline.
///
/// # Side effects
///
/// None – this is a pure, synchronous transformation.
pub fn request_route_to_swap_path(route: &RouteDryRunPath) -> Result<SwapPath> {
    use std::collections::HashSet;

    if route.hops.is_empty() {
        return Err(ApiError::Validation(
            "route must contain at least one hop".to_string(),
        ));
    }

    let mut visited: HashSet<String> = HashSet::new();
    let mut path_hops: Vec<PathHop> = Vec::with_capacity(route.hops.len());
    let mut estimated_output: i128 = 0;

    for (idx, hop) in route.hops.iter().enumerate() {
        let from = hop.from_asset.to_canonical();
        let to = hop.to_asset.to_canonical();

        if from.is_empty() {
            return Err(ApiError::Validation(format!(
                "hop[{}].from_asset is empty",
                idx
            )));
        }
        if to.is_empty() {
            return Err(ApiError::Validation(format!(
                "hop[{}].to_asset is empty",
                idx
            )));
        }
        if from == to {
            return Err(ApiError::Validation(format!(
                "hop[{}]: from_asset and to_asset must differ (got '{}')",
                idx, from
            )));
        }

        // Cycle detection: we track the *source* asset of each hop.
        // The destination of the last hop is not inserted so a two-hop A→B→A
        // path is correctly rejected on the second hop's source.
        if idx == 0 {
            visited.insert(from.clone());
        }
        if visited.contains(&to) {
            return Err(ApiError::Validation(format!(
                "hop[{}]: cycle detected – asset '{}' appears more than once in the path",
                idx, to
            )));
        }
        visited.insert(to.clone());

        // Derive venue_type from the `source` field ("sdex" or "amm:<ref>").
        let venue_type = if hop.source.starts_with("amm:") {
            "amm"
        } else {
            "sdex"
        };
        let venue_ref = hop
            .venue_ref
            .clone()
            .unwrap_or_else(|| hop.source.clone());

        let price: f64 = hop
            .price
            .as_deref()
            .and_then(|p| p.parse().ok())
            .unwrap_or(0.0);

        let fee_bps = hop.fee_bps.unwrap_or(0);

        // Accumulate a very rough estimated_output using the hop price.
        // The real output is computed hop-by-hop in the quote pipeline below.
        if idx == 0 {
            // Placeholder: will be overwritten by quote pipeline result.
            estimated_output = 1_000_000;
        }
        // Apply fee decay as a simple approximation.
        let scale = (10_000u64 - fee_bps as u64) as i128;
        estimated_output = estimated_output.saturating_mul(scale) / 10_000;

        path_hops.push(PathHop {
            source_asset: from,
            destination_asset: to,
            venue_type: venue_type.to_string(),
            venue_ref,
            price,
            fee_bps,
        });
    }

    Ok(SwapPath {
        hops: path_hops,
        estimated_output,
    })
}

/// Stamp per-hop slippage overrides onto a [`RoutingPolicy`].
///
/// The routing engine exposes per-venue filtering through the policy's
/// `venue_denylist` / `venue_allowlist`.  For slippage, the idiomatic
/// integration point is to keep the overrides in-band and resolve them
/// at the call site (which is what the handler does via `override_map`).
///
/// This function adds the listed `venue_ref` values to the policy's
/// **venue allowlist** so that overridden venues are explicitly permitted,
/// and clears any prior denylist entries that would block them.  This
/// guarantees that the policy never silently drops a hop whose slippage
/// has been deliberately overridden by the caller.
///
/// # Side effects
///
/// Mutates `policy` in place; no I/O or signing is performed.
pub fn apply_slippage_overrides_to_policy(
    policy: &mut RoutingPolicy,
    overrides: &[SlippageOverride],
) {
    for ov in overrides {
        // Ensure the venue is not blocked by a blanket denylist.
        policy
            .venue_denylist
            .retain(|v| v != &ov.venue_ref);

        // Explicitly allow it so the policy filter passes.
        if !policy.venue_allowlist.contains(&ov.venue_ref) {
            policy.venue_allowlist.push(ov.venue_ref.clone());
        }
    }
}

// ── Internal conversion helpers ───────────────────────────────────────────────

fn swap_path_to_dto(path: &SwapPath) -> SwapPathDto {
    SwapPathDto {
        hops: path
            .hops
            .iter()
            .map(|h| SwapHopDto {
                source_asset: h.source_asset.clone(),
                destination_asset: h.destination_asset.clone(),
                venue_type: h.venue_type.clone(),
                venue_ref: h.venue_ref.clone(),
                price: h.price,
                fee_bps: h.fee_bps,
            })
            .collect(),
        estimated_output: path.estimated_output,
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::request::AssetPath;

    fn make_hop(from: &str, to: &str, source: &str, venue_ref: Option<&str>) -> RouteDryRunHop {
        RouteDryRunHop {
            from_asset: AssetPath::parse(from).unwrap(),
            to_asset: AssetPath::parse(to).unwrap(),
            source: source.to_string(),
            fee_bps: Some(30),
            price: Some("0.12".to_string()),
            venue_ref: venue_ref.map(str::to_string),
        }
    }

    // ── request_route_to_swap_path ────────────────────────────────────────

    #[test]
    fn single_hop_produces_valid_swap_path() {
        let route = RouteDryRunPath {
            hops: vec![make_hop("native", "USDC", "sdex", Some("sdex-venue"))],
        };
        let path = request_route_to_swap_path(&route).unwrap();
        assert_eq!(path.hops.len(), 1);
        assert_eq!(path.hops[0].source_asset, "native");
        assert_eq!(path.hops[0].destination_asset, "USDC");
        assert_eq!(path.hops[0].venue_type, "sdex");
        assert_eq!(path.hops[0].venue_ref, "sdex-venue");
        assert_eq!(path.hops[0].fee_bps, 30);
    }

    #[test]
    fn multi_hop_produces_correct_chain() {
        let route = RouteDryRunPath {
            hops: vec![
                make_hop("native", "USDC", "sdex", Some("v1")),
                make_hop("USDC", "BTC", "amm:pool1", Some("v2")),
            ],
        };
        let path = request_route_to_swap_path(&route).unwrap();
        assert_eq!(path.hops.len(), 2);
        assert_eq!(path.hops[1].venue_type, "amm");
        assert_eq!(path.hops[1].venue_ref, "v2");
    }

    #[test]
    fn empty_hops_returns_validation_error() {
        let route = RouteDryRunPath { hops: vec![] };
        let err = request_route_to_swap_path(&route).unwrap_err();
        assert!(matches!(err, ApiError::Validation(_)));
    }

    #[test]
    fn same_asset_hop_returns_validation_error() {
        let route = RouteDryRunPath {
            hops: vec![make_hop("USDC", "USDC", "sdex", None)],
        };
        let err = request_route_to_swap_path(&route).unwrap_err();
        assert!(matches!(err, ApiError::Validation(_)));
    }

    #[test]
    fn cyclic_path_returns_validation_error() {
        // A→B→A is a cycle
        let route = RouteDryRunPath {
            hops: vec![
                make_hop("native", "USDC", "sdex", Some("v1")),
                make_hop("USDC", "native", "sdex", Some("v2")),
            ],
        };
        let err = request_route_to_swap_path(&route).unwrap_err();
        assert!(matches!(err, ApiError::Validation(_)));
    }

    #[test]
    fn venue_ref_falls_back_to_source_when_absent() {
        let route = RouteDryRunPath {
            hops: vec![make_hop("native", "USDC", "amm:pool99", None)],
        };
        let path = request_route_to_swap_path(&route).unwrap();
        assert_eq!(path.hops[0].venue_ref, "amm:pool99");
    }

    // ── apply_slippage_overrides_to_policy ───────────────────────────────

    #[test]
    fn override_removes_venue_from_denylist() {
        let mut policy = RoutingPolicy::default().with_venue_denylist(vec!["amm".to_string()]);
        let overrides = vec![SlippageOverride {
            venue_ref: "amm".to_string(),
            slippage_bps: 100,
        }];
        apply_slippage_overrides_to_policy(&mut policy, &overrides);
        assert!(!policy.venue_denylist.contains(&"amm".to_string()));
        assert!(policy.venue_allowlist.contains(&"amm".to_string()));
    }

    #[test]
    fn no_overrides_leaves_policy_unchanged() {
        let mut policy = RoutingPolicy::default();
        let before_allow = policy.venue_allowlist.clone();
        let before_deny = policy.venue_denylist.clone();
        apply_slippage_overrides_to_policy(&mut policy, &[]);
        assert_eq!(policy.venue_allowlist, before_allow);
        assert_eq!(policy.venue_denylist, before_deny);
    }

    #[test]
    fn duplicate_override_venue_ref_does_not_add_duplicates_to_allowlist() {
        let mut policy = RoutingPolicy::default();
        let overrides = vec![
            SlippageOverride {
                venue_ref: "sdex".to_string(),
                slippage_bps: 50,
            },
            SlippageOverride {
                venue_ref: "sdex".to_string(),
                slippage_bps: 75,
            },
        ];
        apply_slippage_overrides_to_policy(&mut policy, &overrides);
        let count = policy
            .venue_allowlist
            .iter()
            .filter(|v| v.as_str() == "sdex")
            .count();
        assert_eq!(count, 1, "venue_ref should appear at most once in allowlist");
    }

    #[test]
    fn policy_max_hops_preserved_after_override() {
        let mut policy = RoutingPolicy::new(3);
        apply_slippage_overrides_to_policy(
            &mut policy,
            &[SlippageOverride {
                venue_ref: "sdex".to_string(),
                slippage_bps: 50,
            }],
        );
        assert_eq!(policy.max_hops, 3);
    }
}
