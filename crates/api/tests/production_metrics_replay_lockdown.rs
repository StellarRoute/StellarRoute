//! Regression tests (issue #1059) for locking down operator-only surfaces
//! (`/metrics`, `/metrics/cache`, `/metrics/pool`, `/api/v1/replay/*`) in
//! production while preserving the permissive local-dev default.
//!
//! Runs fully in-process against a lazily-connected Postgres pool (never
//! actually dials out), so it requires no network access. Tests mutate the
//! process-global `STELLARROUTE_ENV` / `ADMIN_AUTH_TOKEN` env vars and are
//! serialized via `ENV_LOCK` to avoid racing each other within this binary.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::postgres::PgPoolOptions;
use std::sync::Mutex;
use stellarroute_api::{state::DatabasePools, Server, ServerConfig};
use tower::ServiceExt;

static ENV_LOCK: Mutex<()> = Mutex::new(());
const ADMIN_TOKEN: &str = "test-admin-token";

fn reset_env() {
    std::env::remove_var("STELLARROUTE_ENV");
    std::env::remove_var("ADMIN_AUTH_TOKEN");
}

async fn setup_router(admin_auth_token: Option<&str>) -> axum::Router {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .expect("failed to create lazy pool");

    let mut config = ServerConfig::default();
    config.admin_auth_token = admin_auth_token.map(|s| s.to_string());

    Server::new(config, DatabasePools::new(pool, None))
        .await
        .into_router()
}

async fn get(router: &axum::Router, path: &str) -> StatusCode {
    router
        .clone()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .expect("request failed")
        .status()
}

#[tokio::test]
async fn metrics_and_replay_stay_public_outside_production() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_env();

    let router = setup_router(None).await;

    for path in ["/metrics", "/metrics/cache", "/metrics/pool"] {
        let status = get(&router, path).await;
        assert_ne!(
            status,
            StatusCode::UNAUTHORIZED,
            "{path} must stay public in dev/test, got {status}"
        );
    }

    reset_env();
}

#[tokio::test]
async fn metrics_require_admin_auth_in_production() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_env();
    std::env::set_var("STELLARROUTE_ENV", "production");

    let router = setup_router(Some(ADMIN_TOKEN)).await;

    for path in ["/metrics", "/metrics/cache", "/metrics/pool"] {
        let status = get(&router, path).await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "{path} must require admin auth in production, got {status}"
        );
    }

    reset_env();
}

#[tokio::test]
async fn metrics_accessible_with_valid_admin_token_in_production() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_env();
    std::env::set_var("STELLARROUTE_ENV", "production");

    let router = setup_router(Some(ADMIN_TOKEN)).await;

    for path in ["/metrics", "/metrics/cache", "/metrics/pool"] {
        let status = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .header("x-admin-token", ADMIN_TOKEN)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request failed")
            .status();
        assert_eq!(
            status,
            StatusCode::OK,
            "{path} with a valid admin token must succeed in production, got {status}"
        );
    }

    reset_env();
}

#[tokio::test]
async fn replay_list_requires_admin_auth_in_production() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_env();
    std::env::set_var("STELLARROUTE_ENV", "production");

    let router = setup_router(Some(ADMIN_TOKEN)).await;

    let status = get(&router, "/api/v1/replay").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    reset_env();
}

#[tokio::test]
async fn replay_run_requires_admin_auth_in_production() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_env();
    std::env::set_var("STELLARROUTE_ENV", "production");

    let router = setup_router(Some(ADMIN_TOKEN)).await;

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/replay/00000000-0000-0000-0000-000000000000/run")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    reset_env();
}

#[tokio::test]
async fn metrics_deny_when_admin_token_not_configured_in_production() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_env();
    std::env::set_var("STELLARROUTE_ENV", "production");

    // No ADMIN_AUTH_TOKEN configured at all: production must still deny,
    // not fail open.
    let router = setup_router(None).await;

    let status = get(&router, "/metrics").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    reset_env();
}
