//! API routes

pub mod health;
pub mod metrics;
pub mod orderbook;
pub mod pairs;
pub mod quote;

pub mod replay;
pub mod routes_endpoint;

pub mod ws;


use axum::{routing::{get, post}, Router};
use std::sync::Arc;

use crate::state::AppState;

/// Create the main API router
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Health check
        .route("/health", get(health::health_check))
        .route("/metrics/cache", get(metrics::cache_metrics))
        // API v1 routes
        .route("/api/v1/pairs", get(pairs::list_pairs))
        .route(
            "/api/v1/orderbook/:base/:quote",
            get(orderbook::get_orderbook),
        )
        .route("/api/v1/quote/:base/:quote", get(quote::get_quote))
        .route("/api/v1/route/:base/:quote", get(quote::get_route))
        .route("/api/v1/batch/quote", axum::routing::post(quote::get_batch_quotes))

        // Replay routes
        .route("/api/v1/replay", get(replay::list_artifacts))
        .route("/api/v1/replay/:id", get(replay::get_artifact))
        .route("/api/v1/replay/:id/run", post(replay::run_replay))
        .route("/api/v1/replay/:id/diff", post(replay::diff_replay))

        .route("/api/v1/routes/:base/:quote", get(routes_endpoint::get_routes))

        .with_state(state)
}
