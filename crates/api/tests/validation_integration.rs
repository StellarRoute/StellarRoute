//! Integration tests for quote request validation hardening.
//!
//! These tests verify that malformed requests are rejected early with
//! machine-readable error codes and do NOT reach the routing compute path.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use stellarroute_api::{state::DatabasePools, Server, ServerConfig};
use tower::ServiceExt;

async fn setup_test_router() -> axum::Router {
    // We use a lazy pool that won't actually connect unless a query is run.
    // This allows us to test the validation layer (which runs before any DB queries).
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .expect("Failed to create lazy pool");

    Server::new(ServerConfig::default(), DatabasePools::new(pool, None))
        .await
        .into_router()
}

#[tokio::test]
async fn test_validation_rejects_missing_amount() {
    let router = setup_test_router().await;

    // Amount is optional but defaults to 1. If we provide it but it's malformed:
    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/quote/native/USDC?amount=abc")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Request failed");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["data"]["error"], "invalid_amount");
}

#[tokio::test]
async fn test_validation_rejects_zero_amount() {
    let router = setup_test_router().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/quote/native/USDC?amount=0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Request failed");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["data"]["error"], "invalid_amount");
}

#[tokio::test]
async fn test_validation_rejects_negative_amount() {
    let router = setup_test_router().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/quote/native/USDC?amount=-10.5")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Request failed");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["data"]["error"], "invalid_amount");
}

#[tokio::test]
async fn test_validation_rejects_excessive_slippage() {
    let router = setup_test_router().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/quote/native/USDC?slippage_bps=10001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Request failed");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["data"]["error"], "invalid_slippage");
}

#[tokio::test]
async fn test_validation_rejects_malformed_asset() {
    let router = setup_test_router().await;

    // missing issuer for issued asset (colon with nothing after)
    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/quote/native/USDC:")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Request failed");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["data"]["error"], "invalid_asset_format");
}

#[tokio::test]
async fn test_validation_rejects_empty_asset() {
    let router = setup_test_router().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/quote/:/USDC") // empty base (technically matching the route but empty segment)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Request failed");

    // Axum might return 400 or something else depending on routing, but our extractor should catch it if it gets there.
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["data"]["error"], "invalid_asset_format");
}

#[tokio::test]
async fn test_validation_applies_to_route_endpoint() {
    let router = setup_test_router().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/route/native/USDC?amount=0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Request failed");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["data"]["error"], "invalid_amount");
}
