//! API routes

pub mod health;
pub mod kill_switch;
pub mod metrics;
pub mod orderbook;
pub mod pairs;
pub mod prometheus;
pub mod quote;

pub mod replay;
pub mod routes_endpoint;

pub mod ws;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::middleware::legacy_route_deprecation;
use crate::state::AppState;

/// Create the main API router
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Health check
        .route("/health", get(health::health_check))
        .route("/health/deps", get(health::dependency_health))
        .route("/metrics/cache", get(metrics::cache_metrics))
        .route("/metrics", get(prometheus::prometheus_metrics))
        // API v1 routes
        .route("/api/v1/pairs", get(pairs::list_pairs))
        .route("/api/v1/markets", get(pairs::list_markets))
        .route(
            "/api/v1/orderbook/:base/:quote",
            get(orderbook::get_orderbook),
        )
        .route("/api/v1/quote/:base/:quote", get(quote::get_quote))
        .route(
            "/api/v1/route/:base/:quote",
            get(quote::get_route).route_layer(axum::middleware::from_fn(legacy_route_deprecation)),
        )
        .route(
            "/api/v1/batch/quote",
            axum::routing::post(quote::get_batch_quotes),
        )
        // Replay routes
        .route("/api/v1/replay", get(replay::list_artifacts))
        .route("/api/v1/replay/:id", get(replay::get_artifact))
        .route("/api/v1/replay/:id/run", post(replay::run_replay))
        .route("/api/v1/replay/:id/diff", post(replay::diff_replay))
        .route(
            "/api/v1/routes/:base/:quote",
            get(routes_endpoint::get_routes),
        )
        // Admin routes
        .route("/api/v1/admin/kill-switch", get(kill_switch::get_kill_switch))
        .route("/api/v1/admin/kill-switch", post(kill_switch::update_kill_switch))
        .with_state(state)
}
