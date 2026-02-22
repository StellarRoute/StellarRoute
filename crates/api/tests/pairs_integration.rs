//! Integration tests for GET /api/v1/pairs
//!
//! Unit tests (no DB) validate the response model shape and serialization.
//! The live endpoint tests are `#[ignore]` — run with:
//!   DATABASE_URL=postgres://... cargo test -p stellarroute-api -- --ignored

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use sqlx::PgPool;
use stellarroute_api::{Server, ServerConfig};
use tower::ServiceExt; // for `oneshot`

// ---------------------------------------------------------------------------
// Model / serialization tests (no DB required)
// ---------------------------------------------------------------------------

#[test]
fn trading_pair_serializes_to_spec_shape() {
    use stellarroute_api::models::{PairsResponse, TradingPair};

    let pair = TradingPair {
        base: "XLM".to_string(),
        counter: "USDC".to_string(),
        base_asset: "native".to_string(),
        counter_asset: "USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5".to_string(),
        offer_count: 42,
        last_updated: None,
    };

    let response = PairsResponse {
        pairs: vec![pair],
        total: 1,
    };

    let json = serde_json::to_value(&response).expect("serialization failed");

    // Top-level shape
    assert!(json.get("pairs").is_some(), "missing 'pairs' key");
    assert_eq!(json["total"], 1);

    let first = &json["pairs"][0];
    // Spec-required fields
    assert_eq!(first["base"], "XLM");
    assert_eq!(first["counter"], "USDC");
    assert_eq!(first["base_asset"], "native");
    assert!(
        first["counter_asset"]
            .as_str()
            .unwrap_or("")
            .starts_with("USDC:"),
        "counter_asset should be CODE:ISSUER"
    );

    // No stray 'quote_asset' from the old shape
    assert!(
        first.get("quote_asset").is_none(),
        "legacy 'quote_asset' key must not appear"
    );
}

#[test]
fn asset_info_to_canonical_native() {
    use stellarroute_api::models::AssetInfo;

    let info = AssetInfo::native();
    assert_eq!(info.to_canonical(), "native");
    assert_eq!(info.display_name(), "XLM");
}

#[test]
fn asset_info_to_canonical_credit_with_issuer() {
    use stellarroute_api::models::AssetInfo;

    let issuer = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5".to_string();
    let info = AssetInfo::credit("USDC".to_string(), Some(issuer.clone()));
    assert_eq!(info.to_canonical(), format!("USDC:{}", issuer));
    assert_eq!(info.display_name(), "USDC");
}

#[test]
fn asset_info_to_canonical_credit_without_issuer() {
    use stellarroute_api::models::AssetInfo;

    let info = AssetInfo::credit("USDC".to_string(), None);
    assert_eq!(info.to_canonical(), "USDC");
}

// ---------------------------------------------------------------------------
// Live endpoint tests (require DATABASE_URL)
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires a running PostgreSQL database with SDEX data (set DATABASE_URL)"]
async fn get_pairs_returns_200_and_valid_json() {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
    });

    let pool = PgPool::connect(&db_url)
        .await
        .expect("Failed to connect to database");

    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        redis_url: None,
    };

    let router = Server::new(config, pool).await.into_router();

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/pairs")
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

    assert!(
        json.get("pairs").and_then(|v| v.as_array()).is_some(),
        "Response must contain a 'pairs' array"
    );
    assert!(json.get("total").is_some(), "Response must contain 'total'");

    // Validate spec shape on the first pair (if any)
    if let Some(pair) = json["pairs"].as_array().and_then(|a| a.first()) {
        assert!(pair.get("base").is_some(), "pair missing 'base'");
        assert!(pair.get("counter").is_some(), "pair missing 'counter'");
        assert!(
            pair.get("base_asset").is_some(),
            "pair missing 'base_asset'"
        );
        assert!(
            pair.get("counter_asset").is_some(),
            "pair missing 'counter_asset'"
        );
        assert!(
            pair.get("offer_count").is_some(),
            "pair missing 'offer_count'"
        );
    }
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn get_pairs_returns_correct_content_type() {
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
                .uri("/api/v1/pairs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v: &axum::http::HeaderValue| v.to_str().ok())
        .unwrap_or("");

    assert!(
        content_type.contains("application/json"),
        "Content-Type must be application/json, got: {content_type}"
    );
}
