//! Integration tests for GET /health
//!
//! Unit tests run without any external dependencies.
//! Live endpoint tests require DATABASE_URL and are `#[ignore]`:
//!   DATABASE_URL=postgres://... cargo test -p stellarroute-api --test health_integration -- --ignored

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use sqlx::PgPool;
use stellarroute_api::{Server, ServerConfig};
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// Model / serialization tests (no external deps)
// ---------------------------------------------------------------------------

#[test]
fn health_response_serializes_to_spec_shape() {
    use std::collections::HashMap;
    use stellarroute_api::models::HealthResponse;

    let mut components = HashMap::new();
    components.insert("database".to_string(), "healthy".to_string());
    components.insert("redis".to_string(), "not_configured".to_string());

    let response = HealthResponse {
        status: "healthy".to_string(),
        timestamp: "2026-01-20T12:00:00+00:00".to_string(),
        version: "0.1.0".to_string(),
        components,
    };

    let json = serde_json::to_value(&response).expect("serialization failed");

    assert_eq!(json["status"], "healthy");
    assert!(
        json["timestamp"].as_str().is_some(),
        "timestamp must be a string"
    );
    assert_eq!(json["version"], "0.1.0");
    assert!(
        json.get("components").and_then(|v| v.as_object()).is_some(),
        "components must be an object"
    );
    assert_eq!(json["components"]["database"], "healthy");

    // The old shape must not appear
    assert!(
        json.get("timestamp").and_then(|v| v.as_i64()).is_none(),
        "timestamp must be a string, not an integer"
    );
}

// ---------------------------------------------------------------------------
// Live endpoint tests (require DATABASE_URL)
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn health_returns_200_when_db_is_up() {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
    });

    let pool = PgPool::connect(&db_url)
        .await
        .expect("Failed to connect to database");

    let router = Server::new(ServerConfig::default(), pool)
        .await
        .into_router();

    let response = router
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "healthy");
    assert!(json["timestamp"].as_str().is_some());
    assert!(json["version"].as_str().is_some());

    let components = json["components"].as_object().expect("components missing");
    assert_eq!(
        components.get("database").and_then(|v| v.as_str()),
        Some("healthy")
    );
    // redis will be "not_configured" in default config
    assert!(
        components.contains_key("redis"),
        "redis key must be present"
    );
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn health_has_json_content_type() {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
    });

    let pool = PgPool::connect(&db_url)
        .await
        .expect("Failed to connect to database");

    let router = Server::new(ServerConfig::default(), pool)
        .await
        .into_router();

    let response = router
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Request failed");

    let ct = response
        .headers()
        .get("content-type")
        .and_then(|v: &axum::http::HeaderValue| v.to_str().ok())
        .unwrap_or("");

    assert!(ct.contains("application/json"), "got: {ct}");
}
