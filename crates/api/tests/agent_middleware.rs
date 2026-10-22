//! Middleware contract tests for the AI agent routes (AI-42, issue #1452).
//!
//! Asserts that agent routes share the existing public rate limiter, that the
//! admin kill-switch still requires admin auth in production, and that the
//! flag-off 404 happens without touching the database. Runs fully in-process
//! against a lazily-connected pool - no Postgres, Redis, or network required.
//! Env mutations are serialized via `ENV_LOCK`.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use std::sync::Mutex;
use stellarroute_api::{middleware::EndpointConfig, state::DatabasePools, Server, ServerConfig};
use tower::ServiceExt;

static ENV_LOCK: Mutex<()> = Mutex::new(());
const ADMIN_TOKEN: &str = "test-admin-token";

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

fn reset_env() {
    std::env::remove_var("AI_AGENT_ENABLED");
    std::env::remove_var("STELLARROUTE_ENV");
    std::env::remove_var("ADMIN_AUTH_TOKEN");
}

/// One agent request pinned to a fixed client bucket via `x-forwarded-for`,
/// so the assertions do not depend on how the limiter derives the client
/// address from an in-process `oneshot` request.
fn agent_health_request() -> Request<Body> {
    Request::builder()
        .uri("/api/v1/agent/health")
        .header("x-forwarded-for", "203.0.113.42")
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn agent_burst_past_public_limit_is_rejected_with_429() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_env();
    std::env::set_var("AI_AGENT_ENABLED", "true");

    let router = setup_router(None).await;
    let limit = EndpointConfig::default()
        .for_path("/api/v1/agent/health", None)
        .max_requests as usize;

    // First request must pass and advertise the existing public limit.
    let first = router
        .clone()
        .oneshot(agent_health_request())
        .await
        .expect("request failed");
    assert_eq!(first.status(), StatusCode::OK);
    let advertised: u64 = first
        .headers()
        .get("x-ratelimit-limit")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
        .expect("X-RateLimit-Limit must be present");
    assert_eq!(
        advertised, limit as u64,
        "agent routes must use the existing public default rate limit"
    );

    // Everything inside the limit passes.
    for i in 0..limit {
        let status = router
            .clone()
            .oneshot(agent_health_request())
            .await
            .expect("request failed")
            .status();
        assert_eq!(
            status,
            StatusCode::OK,
            "request {} of {} must pass",
            i + 2,
            limit + 1
        );
    }

    // The next request crosses the limit and is rejected with the existing
    // rate-limit status, headers, and error code.
    let denied = router
        .clone()
        .oneshot(agent_health_request())
        .await
        .expect("request failed");
    assert_eq!(denied.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(
        denied.headers().contains_key("retry-after"),
        "429 must carry Retry-After"
    );

    let bytes = axum::body::to_bytes(denied.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&bytes).expect("429 body must be JSON");
    assert_eq!(
        json["error"], "rate_limit_exceeded",
        "429 body must use the existing rate_limit_exceeded error code"
    );

    reset_env();
}

#[tokio::test]
async fn admin_kill_switch_requires_admin_auth_in_production() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_env();
    std::env::set_var("STELLARROUTE_ENV", "production");

    let router = setup_router(Some(ADMIN_TOKEN)).await;

    let unauthorized = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/kill-switch")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");
    assert_eq!(
        unauthorized.status(),
        StatusCode::UNAUTHORIZED,
        "kill-switch must still require admin auth in production"
    );

    let authorized = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/kill-switch")
                .header("x-admin-token", ADMIN_TOKEN)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");
    assert_eq!(
        authorized.status(),
        StatusCode::OK,
        "a valid admin token must still reach the kill-switch handler"
    );

    reset_env();
}

#[tokio::test]
async fn agent_flag_off_answers_404_without_database() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_env(); // AI_AGENT_ENABLED unset

    // The lazy pool never dials out: a handler that reached the database
    // would fail the connection instead of answering 404.
    let router = setup_router(None).await;

    for path in ["/api/v1/agent/health", "/api/v1/agent/tools"] {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request failed");
        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "{path} must return 404 while AI_AGENT_ENABLED is unset"
        );
    }

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agent/intents/validate")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"type":"convert","amount":"10"}"#))
                .unwrap(),
        )
        .await
        .expect("request failed");
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "POST /api/v1/agent/intents/validate must 404 before any handler work"
    );

    reset_env();
}
