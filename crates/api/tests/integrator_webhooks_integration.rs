use axum::{
    body::Body,
    http::{self, Request, StatusCode},
};
use sqlx::PgPool;
use stellarroute_api::{state::DatabasePools, Server, ServerConfig};
use tower::ServiceExt;

#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn test_missing_x_api_key() {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
    });
    let pool = PgPool::connect(&db_url)
        .await
        .expect("Failed to connect to database");

    let router = Server::new(ServerConfig::default(), DatabasePools::new(pool, None))
        .await
        .into_router();

    let response = router
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/api/v1/integrator/webhooks/quote-expiration")
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"webhook_url": "https://example.com/webhook"}"#))
                .unwrap(),
        )
        .await
        .expect("Request failed");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn test_empty_webhook_url() {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
    });
    let pool = PgPool::connect(&db_url)
        .await
        .expect("Failed to connect to database");

    let router = Server::new(ServerConfig::default(), DatabasePools::new(pool, None))
        .await
        .into_router();

    let response = router
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/api/v1/integrator/webhooks/quote-expiration")
                .header(http::header::CONTENT_TYPE, "application/json")
                .header("x-api-key", "test-key")
                .body(Body::from(r#"{"webhook_url": ""}"#))
                .unwrap(),
        )
        .await
        .expect("Request failed");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn test_http_webhook_url() {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
    });
    let pool = PgPool::connect(&db_url)
        .await
        .expect("Failed to connect to database");

    let router = Server::new(ServerConfig::default(), DatabasePools::new(pool, None))
        .await
        .into_router();

    let response = router
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/api/v1/integrator/webhooks/quote-expiration")
                .header(http::header::CONTENT_TYPE, "application/json")
                .header("x-api-key", "test-key")
                .body(Body::from(r#"{"webhook_url": "http://example.com/webhook"}"#))
                .unwrap(),
        )
        .await
        .expect("Request failed");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn test_happy_path() {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
    });
    let pool = PgPool::connect(&db_url)
        .await
        .expect("Failed to connect to database");

    let router = Server::new(ServerConfig::default(), DatabasePools::new(pool, None))
        .await
        .into_router();

    let response = router
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/api/v1/integrator/webhooks/quote-expiration")
                .header(http::header::CONTENT_TYPE, "application/json")
                .header("x-api-key", "test-key")
                .body(Body::from(
                    r#"{"webhook_url": "https://example.com/webhook"}"#,
                ))
                .unwrap(),
        )
        .await
        .expect("Request failed");

    assert_eq!(response.status(), StatusCode::OK);
}
