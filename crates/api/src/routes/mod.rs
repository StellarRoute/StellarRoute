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
/// stats, replay artifacts and replay mutations) in production.
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

/// Fail-fast guard for quote/swap traffic.
///
/// When a dependency circuit breaker (Postgres, Soroban RPC, Horizon) is open,
/// reject with `503` immediately instead of letting every request queue behind a
/// dependency that is already known to be down. Health endpoints stay reachable so
/// operators can still see why, and so probes can close the breaker again.
async fn dependency_breaker_guard(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    match state.external_dependency_health.guard_live_path() {
        Ok(()) => next.run(request).await,
        Err(err) => err.into_response(),
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
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            production_admin_guard,
        ));

    // Quote/swap surfaces. These are the paths that price real trades against
    // live dependencies, so they fail fast when a dependency breaker is open.
    let live_path_routes = Router::new()
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
            "/api/v1/routes/:base/:quote",
            get(routes_endpoint::get_routes),
        )
        .route(
            "/api/v1/simulate/route",
            post(simulation_route::simulate_route_dry_run),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            dependency_breaker_guard,
        ));

    Router::new()
        // Health check
        .route("/health", get(health::health_check))
        .route("/health/deps", get(health::dependency_health))
        .merge(operator_routes)
        .merge(live_path_routes)
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
        .route(
            "/api/v1/batch/orderbook",
            axum::routing::post(orderbook::get_batch_orderbooks),
        )
        .route(
            "/api/v1/integrator/webhooks/quote-expiration",
            post(integrator_webhooks::upsert_quote_expiration_webhook),
        )
        // Replay routes are registered above via `operator_routes`.
        // Admin routes
        .route(
            "/api/v1/admin/cache/flush/:base/:quote",
            axum::routing::post(admin::flush_cache),
        )
        .route("/api/v1/admin/cache/flush", post(admin_cache::flush_cache))
        .route(
            "/api/v1/admin/kill-switch",
            get(kill_switch::get_kill_switch),
        )
        .route(
            "/api/v1/admin/kill-switch",
            post(kill_switch::update_kill_switch),
        )
        // Canary routes
        .route("/api/v1/system/canary/report", get(canary::get_report))
        .route("/api/v1/system/canary/config", post(canary::update_config))
        // `/api/v1/simulate/route` is registered above via `live_path_routes`.
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
