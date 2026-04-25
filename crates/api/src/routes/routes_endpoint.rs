//! Multiple Routes Endpoint

use axum::{
    extract::{Path, Query, State},
    Json,
};
use std::sync::Arc;
use tracing::debug;

use stellarroute_routing::health::policy::ExclusionPolicy;
use stellarroute_routing::optimizer::HybridOptimizer;
use stellarroute_routing::policy::RoutingPolicy;

use crate::{
    error::{ApiError, Result},
    middleware::RequestId,
    models::{
        request::{AssetPath, RoutesParams},
        ApiResponse, AssetInfo, RouteCandidate, RouteHop, RoutesResponse,
    },
    state::AppState,
};

/// Convert canonical string identifiers into API AssetInfo
fn parse_asset_to_info(s: &str) -> AssetInfo {
    AssetPath::parse(s)
        .map(|p| {
            if p.asset_code == "native" {
                AssetInfo::native()
            } else {
                AssetInfo::credit(p.asset_code, p.asset_issuer)
            }
        })
        .unwrap_or_else(|_| AssetInfo::native())
}

/// Convert AssetPath into an AssetInfo DTO
fn asset_path_to_info(asset: &AssetPath) -> AssetInfo {
    if asset.asset_code == "native" {
        AssetInfo::native()
    } else {
        AssetInfo::credit(asset.asset_code.clone(), asset.asset_issuer.clone())
    }
}

/// GET /api/v1/routes/:base/:quote
///
/// Returns multiple ranked execution route candidates for a trading pair.
/// Routes are scored by the HybridOptimizer and may include multi-hop paths.
///
/// # Query Parameters
/// - `amount`: Trade amount (default: "1")
/// - `limit`: Max routes to return (default: 5)
/// - `max_hops`: Max hops per route (default: 3)
/// - `environment`: Optimizer policy environment (default: "production")
#[utoipa::path(
    get,
    path = "/api/v1/routes/{base}/{quote}",
    tag = "trading",
    params(
        ("base" = String, Path, description = "Base asset ('native', 'USDC', or 'USDC:ISSUER')"),
        ("quote" = String, Path, description = "Quote asset"),
        ("amount" = Option<String>, Query, description = "Amount to trade (default: 1)"),
        ("limit" = Option<usize>, Query, description = "Maximum number of routes to return (default: 5)"),
        ("max_hops" = Option<usize>, Query, description = "Maximum number of hops (default: 3)"),
        ("environment" = Option<String>, Query, description = "Optimizer policy environment"),
    ),
    responses(
        (status = 200, description = "Ranked route candidates", body = RoutesResponse),
        (status = 400, description = "Invalid request parameters"),
        (status = 404, description = "No routes found"),
        (status = 500, description = "Internal server error"),
    )
)]
pub async fn get_routes(
    State(state): State<Arc<AppState>>,
    request_id: RequestId,
    Path((base, quote)): Path<(String, String)>,
    Query(params): Query<RoutesParams>,
) -> Result<Json<ApiResponse<RoutesResponse>>> {
    debug!("get_routes: {}/{} params={:?}", base, quote, params);

    // ── Input validation ────────────────────────────────────────────────────
    let base_asset = AssetPath::parse(&base)
        .map_err(|e| ApiError::InvalidAsset(format!("Invalid base asset: {e}")))?;
    let quote_asset = AssetPath::parse(&quote)
        .map_err(|e| ApiError::InvalidAsset(format!("Invalid quote asset: {e}")))?;

    let amount: f64 = params
        .amount
        .as_deref()
        .unwrap_or("1")
        .parse()
        .map_err(|_| ApiError::Validation("amount must be a valid number".into()))?;

    if amount <= 0.0 || !amount.is_finite() {
        return Err(ApiError::Validation(
            "amount must be a positive, finite number".into(),
        ));
    }

    let limit_param = params.limit.unwrap_or(5).min(20); // cap at 20
    let max_hops_param = params.max_hops.unwrap_or(3).min(6); // cap at 6
    let env_param = params
        .environment
        .clone()
        .unwrap_or_else(|| "production".into());

    // ── Single-flight deduplication key ────────────────────────────────────
    let sf_key = format!(
        "routes:{}:{}:{:.7}:{}:{}:{}",
        base, quote, amount, limit_param, max_hops_param, env_param
    );

    // Clone everything needed to move into the async closure
    let base_c = base_asset.clone();
    let quote_c = quote_asset.clone();
    let state_c = state.clone();
    let env_c = env_param.clone();

    // ── Execute via SingleFlight (collapses burst identical requests) ───────
    let result_arc = state
        .routes_single_flight
        .execute(&sf_key, || async move {
            // Read the pre-built in-memory liquidity graph — zero DB hit
            let compacted_graph = state_c.graph_manager.get_edges();

            if compacted_graph.asset_count() == 0 {
                return Arc::new(Err(ApiError::NoRouteFound));
            }

            // Apply kill switches via a logic that works with compacted indices?
            // For now, we perform filtering during BFS in the pathfinder which is safer.
            // However, we need to pass the exclusion policy down.
            let overrides = state_c.kill_switch.get_override_registry().await;
            let _exclusion_policy = ExclusionPolicy {
                thresholds: Default::default(),
                overrides,
                circuit_breaker: Some(state_c.circuit_breaker.clone()),
            };

            let amount_e7 = (amount * 1e7) as i128;

            let base_canary = base_c.clone();
            let quote_canary = quote_c.clone();
            let graph_canary = compacted_graph.clone();

            // Offload CPU-bound BFS to blocking thread pool to prevent async starvation
            let spawn_result = tokio::task::spawn_blocking(move || {
                let mut optimizer = HybridOptimizer::default();
                let _ = optimizer.set_active_policy(&env_c);

                let routing_policy = RoutingPolicy {
                    max_hops: max_hops_param,
                    ..Default::default()
                };

                let base_canonical = asset_path_to_info(&base_c).to_canonical();
                let quote_canonical = asset_path_to_info(&quote_c).to_canonical();

                optimizer.find_optimal_routes_compacted(
                    &base_canonical,
                    &quote_canonical,
                    &compacted_graph,
                    amount_e7,
                    &routing_policy,
                )
            })
            .await;

            // Handle task-join error (thread panic)
            let join_result = match spawn_result {
                Ok(r) => r,
                Err(e) => {
                    return Arc::new(Err(ApiError::Validation(format!(
                        "Route computation panicked: {e}"
                    ))))
                }
            };

            // Handle routing error (no path found)
            let diag = match join_result {
                Ok(d) => d,
                Err(_) => return Arc::new(Err(ApiError::NoRouteFound)),
            };

            // ── Canary Pipeline ──────────────────────────────────────────────────
            let state_canary = state_c.clone();
            let diag_baseline = diag.clone();
            let amount_e7_canary = amount_e7;

            tokio::spawn(async move {
                let config = state_canary.canary_config.read().await.clone();
                if !config.enabled {
                    return;
                }

                // Pseudo-random sampling
                let rate = (chrono::Utc::now().timestamp_micros() % 1000) as f64 / 1000.0;
                if rate > config.evaluation_rate {
                    return;
                }

                let rp = RoutingPolicy {
                    max_hops: max_hops_param,
                    ..Default::default()
                };

                let candidate_policy = config.candidate_policy.clone();
                let base_str = asset_path_to_info(&base_canary).to_canonical();
                let quote_str = asset_path_to_info(&quote_canary).to_canonical();
                let base_str_c = base_str.clone();
                let quote_str_c = quote_str.clone();

                let candidate_result = tokio::task::spawn_blocking(move || {
                    let mut optimizer = HybridOptimizer::default();
                    if optimizer.set_active_policy(&candidate_policy).is_err() {
                        return None;
                    }
                    optimizer
                        .find_optimal_routes_compacted(
                            &base_str,
                            &quote_str,
                            &graph_canary,
                            amount_e7_canary,
                            &rp,
                        )
                        .ok()
                })
                .await;

                if let Ok(Some(candidate_diag)) = candidate_result {
                    let evaluation = stellarroute_routing::canary::CanaryEvaluator::evaluate(
                        &config,
                        &diag_baseline,
                        &candidate_diag,
                        &base_str_c,
                        &quote_str_c,
                        amount_e7_canary,
                    );

                    let mut history = state_canary.canary_history.write().await;
                    if history.len() >= 1000 {
                        history.pop_front();
                    }
                    let is_violation = evaluation.is_violation;
                    history.push_back(evaluation);

                    if is_violation {
                        let recent_violations = history
                            .iter()
                            .rev()
                            .take(config.rollback_trigger_threshold as usize)
                            .filter(|e| e.is_violation)
                            .count();

                        if recent_violations >= config.rollback_trigger_threshold as usize {
                            tracing::warn!(
                                "Canary trigger threshold reached! Disabling canary pipeline."
                            );
                            let mut cfg = state_canary.canary_config.write().await;
                            cfg.enabled = false;
                        }
                    }
                }
            });
            // ───────────────────────────────────────────────────────────────────

            // Record route compute time metric
            crate::metrics::record_route_compute_time(
                std::time::Duration::from_millis(diag.total_compute_time_ms),
                &diag.policy.environment,
            );

            // ── Map diagnostics → response DTO ─────────────────────────────
            let build_candidate = |path: &stellarroute_routing::pathfinder::SwapPath,
                                   metrics: &stellarroute_routing::optimizer::RouteMetrics|
             -> RouteCandidate {
                let mut hops = Vec::new();
                let mut active = amount_e7;

                for h in &path.hops {
                    let after_fee = (active * (10000 - h.fee_bps as i128)) / 10000;
                    let out = (after_fee as f64 * h.price) as i128;

                    hops.push(RouteHop {
                        from_asset: parse_asset_to_info(&h.source_asset),
                        to_asset: parse_asset_to_info(&h.destination_asset),
                        price: format!("{:.7}", h.price),
                        amount_out_of_hop: format!("{:.7}", out as f64 / 1e7),
                        fee_bps: h.fee_bps,
                        source: if h.venue_type == "amm" {
                            format!("amm:{}", h.venue_ref)
                        } else {
                            "sdex".into()
                        },
                    });
                    active = out;
                }

                RouteCandidate {
                    estimated_output: format!("{:.7}", metrics.output_amount as f64 / 1e7),
                    impact_bps: metrics.impact_bps,
                    score: metrics.score,
                    policy_used: diag.policy.environment.clone(),
                    path: hops,
                }
            };

            let mut routes = Vec::with_capacity(limit_param);
            routes.push(build_candidate(&diag.selected_path, &diag.metrics));

            for (path, metric) in diag.alternatives.iter().take(limit_param - 1) {
                routes.push(build_candidate(path, metric));
            }

            Arc::new(Ok(RoutesResponse {
                base_asset: asset_path_to_info(&base_asset),
                quote_asset: asset_path_to_info(&quote_asset),
                amount: format!("{:.7}", amount),
                routes,
                timestamp: chrono::Utc::now().timestamp_millis(),
            }))
        })
        .await;

    // ── Unwrap Arc (shared by single-flight callers) ────────────────────────
    match Arc::try_unwrap(result_arc) {
        Ok(res) => res.map(|r| Json(ApiResponse::new(r, request_id.to_string()))),
        Err(arc) => (*arc)
            .clone()
            .map(|r| Json(ApiResponse::new(r, request_id.to_string()))),
    }
}
