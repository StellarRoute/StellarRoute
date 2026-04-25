//! Integration tests for deprecation headers on legacy API routes.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::postgres::PgPoolOptions;
use stellarroute_api::{state::DatabasePools, Server, ServerConfig};
use tower::ServiceExt;

async fn setup_test_router() -> axum::Router {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .expect("Failed to create lazy pool");

    Server::new(ServerConfig::default(), DatabasePools::new(pool, None))
        .await
        .into_router()
}

#[tokio::test]
async fn legacy_route_emits_deprecation_headers() {
    let router = setup_test_router().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/route/native/USDC?amount=0&slippage_bps=25")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Request failed");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let headers = response.headers();
    assert_eq!(
        headers
            .get("deprecation")
            .and_then(|value| value.to_str().ok()),
        Some("true")
    );
    assert_eq!(
        headers.get("sunset").and_then(|value| value.to_str().ok()),
        Some("Wed, 01 Jul 2026 00:00:00 GMT")
    );

    let link = headers
        .get("link")
        .and_then(|value| value.to_str().ok())
        .expect("legacy route should emit Link header");

    assert!(
        link.contains("/api/v1/routes/native/USDC?amount=0&slippage_bps=25"),
        "successor link should point to the replacement endpoint: {link}"
    );
    assert!(
        link.contains("rel=\"successor-version\""),
        "link header should mark the replacement route: {link}"
    );
    assert!(
        link.contains("docs/api/versioning.md"),
        "link header should advertise the migration guide: {link}"
    );
}
