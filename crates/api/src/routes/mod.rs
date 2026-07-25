//! API routes

pub mod activity;
pub mod admin;
pub mod admin_cache;
pub mod assets;
pub mod canary;
pub mod contract_registry;
pub mod health;
pub mod idempotent_quote;
pub mod integrator_webhooks;
pub mod kill_switch;
pub mod metrics;
pub mod orderbook;
pub mod pairs;
pub mod price_history;
pub mod prometheus;
pub mod quote;
pub mod replay;
pub mod routes_endpoint;
pub mod simulation_route;
pub mod swap;
pub mod ws;

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

use crate::middleware::legacy_route_deprecation;
use crate::models::{ApiErrorCode, ErrorResponse};
use crate::state::AppState;

/// Middleware guarding operator-only surfaces (Prometheus metrics, pool/cache
/// stats, replay artifacts and replay mutations, and the kill-switch /
/// canary state reads) in production.
///
/// Outside of production these stay open for local Prometheus scraping and
/// demos. When `STELLARROUTE_ENV=production`, the same `ADMIN_AUTH_TOKEN`
/// used for `/api/v1/admin/*` is required (`x-admin-token` header, or
/// `Authorization: Bearer <token>`). See
/// `docs/api/production-exposure.md` for the full endpoint inventory.
async fn production_admin_guard(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    if !crate::env_profile::is_production() {
        return next.run(request).await;
    }

    let unauthorized = |message: &str| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(ApiErrorCode::Unauthorized, message)),
        )
            .into_response()
    };

    let Some(expected_token) = state.admin_auth_token.as_ref() else {
        return unauthorized("Admin auth is not configured");
    };

    match crate::middleware::admin::extract_admin_token(request.headers()) {
        Some(token) if token == *expected_token => next.run(request).await,
        _ => unauthorized("Missing or invalid admin credentials"),
    }
}

/// Create the main API router
pub fn create_router(state: Arc<AppState>) -> Router {
    // Operator-only surfaces: publicly reachable in dev/test for local
    // Prometheus scraping and demos, but gated behind admin auth in
    // production. See `production_admin_guard` and
    // `docs/api/production-exposure.md`.
    let operator_routes = Router::new()
        .route("/metrics/cache", get(metrics::cache_metrics))
        .route("/metrics/pool", get(metrics::pool_stats))
        .route("/metrics", get(prometheus::prometheus_metrics))
        .route("/api/v1/replay", get(replay::list_artifacts))
        .route("/api/v1/replay/:id", get(replay::get_artifact))
        .route("/api/v1/replay/:id/run", post(replay::run_replay))
        .route("/api/v1/replay/:id/diff", post(replay::diff_replay))
        // Read-only state for the kill switch and canary config: open in
        // dev/test, admin-token-gated in production (issues #1053, #1055).
        // The mutating counterparts (POST kill-switch, POST canary/config)
        // always require AdminAuth regardless of environment — see
        // `routes::kill_switch::update_kill_switch` and
        // `routes::canary::update_config`.
        .route(
            "/api/v1/admin/kill-switch",
            get(kill_switch::get_kill_switch),
        )
        .route("/api/v1/system/canary/report", get(canary::get_report))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            production_admin_guard,
        ));

    Router::new()
        // Health check
        .route("/health", get(health::health_check))
        .route("/health/deps", get(health::dependency_health))
        .merge(operator_routes)
        // API v1 routes
        .route("/api/v1/assets", get(assets::list_assets_metadata))
        .route("/api/v1/assets/:code", get(assets::get_asset_metadata))
        .route("/api/v1/activity/swaps", get(activity::list_swap_activity))
        .route("/api/v1/pairs", get(pairs::list_pairs))
        .route("/api/v1/markets", get(pairs::list_markets))
        .route(
            "/api/v1/price-history/:base/:quote",
            get(price_history::get_price_history),
        )
        .route(
            "/api/v1/orderbook/:base/:quote",
            get(orderbook::get_orderbook),
        )
        .route("/api/v1/quote/:base/:quote", get(quote::get_quote))
        .route("/api/v1/quote", post(idempotent_quote::post_quote))
        .route(
            "/api/v1/route/:base/:quote",
            get(quote::get_route).route_layer(axum::middleware::from_fn(legacy_route_deprecation)),
        )
        .route(
            "/api/v1/batch/quote",
            axum::routing::post(quote::get_batch_quotes),
        )
        .route(
            "/api/v1/batch/orderbook",
            axum::routing::post(orderbook::get_batch_orderbooks),
        )
        .route(
            "/api/v1/integrator/webhooks/quote-expiration",
            post(integrator_webhooks::upsert_quote_expiration_webhook),
        )
        // Swap prepare/submit (issue #1051): OpenAPI contract for the live
        // swap path. Transaction building/submission are not implemented
        // yet — see `routes::swap` module docs.
        .route("/api/v1/swap/prepare", post(swap::prepare_swap))
        .route("/api/v1/swap/submit", post(swap::submit_swap))
        // Replay routes are registered above via `operator_routes`.
        .route(
            "/api/v1/routes/:base/:quote",
            get(routes_endpoint::get_routes),
        )
        // Admin routes
        .route(
            "/api/v1/admin/cache/flush/:base/:quote",
            axum::routing::post(admin::flush_cache),
        )
        .route("/api/v1/admin/cache/flush", post(admin_cache::flush_cache))
        // GET /api/v1/admin/kill-switch is registered above via
        // `operator_routes`.
        .route(
            "/api/v1/admin/kill-switch",
            post(kill_switch::update_kill_switch),
        )
        // GET /api/v1/system/canary/report is registered above via
        // `operator_routes`.
        .route("/api/v1/system/canary/config", post(canary::update_config))
        .route(
            "/api/v1/simulate/route",
            post(simulation_route::simulate_route_dry_run),
        )
        // Contract registry routes
        .route(
            "/api/v1/contracts/registry",
            get(contract_registry::list_contract_versions),
        )
        .route(
            "/api/v1/contracts/registry/:contract_name",
            get(contract_registry::get_contract_version),
        )
        .route(
            "/api/v1/contracts/registry/:contract_name/network/:network",
            get(contract_registry::get_contract_version_by_network),
        )
        // WebSocket quote stream (real-time quotes)
        .route("/ws", get(ws::ws_handler))
        .with_state(state)
}
