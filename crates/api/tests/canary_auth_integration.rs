//! Auth regression tests for the canary routing pipeline (issue #1055).
//!
//! `POST /api/v1/system/canary/config` must always require `AdminAuth`.
//! `GET /api/v1/system/canary/report` is public in dev/test but gated the
//! same way in production. Runs fully in-process against a lazily-connected
//! Postgres pool (never actually dials out), so it requires no network
//! access. Tests that touch `STELLARROUTE_ENV` are serialized via
//! `ENV_LOCK` since it's process-global.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use std::sync::Mutex;
use stellarroute_api::{state::DatabasePools, Server, ServerConfig};
use tower::ServiceExt;

const REPORT_PATH: &str = "/api/v1/system/canary/report";
const CONFIG_PATH: &str = "/api/v1/system/canary/config";
const ADMIN_TOKEN: &str = "test-admin-token";

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn reset_env() {
    std::env::remove_var("STELLARROUTE_ENV");
}

fn valid_config_payload() -> serde_json::Value {
    json!({
        "enabled": true,
        "baseline_policy": "production",
        "candidate_policy": "testing",
        "max_latency_drift_ms": 50,
        "max_output_drift_bps": 10,
        "rollback_trigger_threshold": 5,
        "evaluation_rate": 0.25
    })
}

async fn setup_router() -> axum::Router {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .expect("failed to create lazy pool");

    let mut config = ServerConfig::default();
    config.admin_auth_token = Some(ADMIN_TOKEN.to_string());

    Server::new(config, DatabasePools::new(pool, None))
        .await
        .into_router()
}

#[tokio::test]
async fn canary_auth_report_public_outside_production() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_env();
    let router = setup_router().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri(REPORT_PATH)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn canary_auth_report_requires_admin_auth_in_production() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_env();
    std::env::set_var("STELLARROUTE_ENV", "production");

    let router = setup_router().await;

    let denied = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(REPORT_PATH)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);

    let allowed = router
        .oneshot(
            Request::builder()
                .uri(REPORT_PATH)
                .header("x-admin-token", ADMIN_TOKEN)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");
    assert_eq!(allowed.status(), StatusCode::OK);

    reset_env();
}

#[tokio::test]
async fn canary_auth_config_denied_without_admin_token() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_env();
    let router = setup_router().await;

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(CONFIG_PATH)
                .header("content-type", "application/json")
                .body(Body::from(valid_config_payload().to_string()))
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn canary_auth_config_allowed_with_admin_token() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_env();
    let router = setup_router().await;

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(CONFIG_PATH)
                .header("content-type", "application/json")
                .header("x-admin-token", ADMIN_TOKEN)
                .body(Body::from(valid_config_payload().to_string()))
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn canary_auth_config_denied_in_production_without_token() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_env();
    std::env::set_var("STELLARROUTE_ENV", "production");
    let router = setup_router().await;

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(CONFIG_PATH)
                .header("content-type", "application/json")
                .body(Body::from(valid_config_payload().to_string()))
                .unwrap(),
        )
        .await
        .expect("request failed");

    reset_env();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
