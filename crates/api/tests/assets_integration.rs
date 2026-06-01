//! Integration tests for asset metadata endpoints

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use sqlx::PgPool;
use stellarroute_api::{state::DatabasePools, Server, ServerConfig};
use tower::ServiceExt;

#[test]
fn asset_metadata_serializes_to_spec_shape() {
    use stellarroute_api::models::AssetMetadataResponse;

    let meta = AssetMetadataResponse {
        code: "USDC".to_string(),
        issuer: Some("GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string()),
        decimals: 7,
        asset_type: "credit_alphanum4".to_string(),
        display_name: Some("USDC".to_string()),
        icon_url: Some("https://example.com/icon.png".to_string()),
        domain: Some("centre.io".to_string()),
    };

    let json = serde_json::to_value(&meta).expect("serialization failed");

    assert_eq!(json["code"], "USDC");
    assert!(json["issuer"].is_string());
    assert_eq!(json["decimals"], 7);
    assert_eq!(json["asset_type"], "credit_alphanum4");
    assert_eq!(json["display_name"], "USDC");
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn get_native_asset_metadata_returns_200() {
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
                .uri("/api/v1/assets/XLM")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read body");

    let json: Value = serde_json::from_slice(&body).expect("Body is not valid JSON");

    assert_eq!(json["data"]["code"], "XLM");
    assert_eq!(json["data"]["asset_type"], "native");
    assert_eq!(json["data"]["decimals"], 7);
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn get_bulk_asset_metadata_returns_200() {
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
                .uri("/api/v1/assets?codes=XLM,native")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read body");

    let json: Value = serde_json::from_slice(&body).expect("Body is not valid JSON");

    let assets = json["data"]["assets"]
        .as_array()
        .expect("Expected 'assets' array");
    assert!(assets.len() >= 1);
}
