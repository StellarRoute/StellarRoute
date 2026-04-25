//! Integration tests for request ID propagation.

use axum::{
    body::Body,
    http::{header::HeaderName, HeaderValue, Request, StatusCode},
};
use sqlx::postgres::PgPoolOptions;
use stellarroute_api::{state::DatabasePools, Server, ServerConfig};
use tower::ServiceExt;

async fn build_test_server() -> Server {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute")
        .expect("lazy pool should build");

    Server::new(ServerConfig::default(), DatabasePools::new(pool, None)).await
}

#[tokio::test]
async fn generated_request_id_is_returned_in_response_header() {
    let router = build_test_server().await.into_router();

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api-docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response.headers().contains_key("x-request-id"),
        "response should include x-request-id header"
    );
}

#[tokio::test]
async fn incoming_request_id_is_echoed_back_to_the_client() {
    let router = build_test_server().await.into_router();
    let request_id = HeaderValue::from_static("wave-test-request-id");

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api-docs/openapi.json")
                .header(HeaderName::from_static("x-request-id"), request_id.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("x-request-id"),
        Some(&request_id),
        "response should preserve the caller-provided x-request-id"
    );
}
