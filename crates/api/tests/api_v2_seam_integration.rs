//! `/api/v2` seam: only info + canonicalize are exposed (no v2 quote).

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::Value;
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
async fn api_v2_info_exposes_non_executable_bridge_flag() {
    let router = setup_test_router().await;
    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["v"], 2);
    assert_eq!(json["data"]["version"], 2);
    assert_eq!(json["data"]["bridge_settlement_executable"], false);
    assert_eq!(json["data"]["bridge_venues_metadata_only"], true);
    assert_eq!(
        json["data"]["supported_corridors"]
            .as_array()
            .map(|a| a.len()),
        Some(0)
    );
}

#[tokio::test]
async fn api_v2_canonicalize_accepts_legacy_and_rejects_slip44_native() {
    let router = setup_test_router().await;

    let ok = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v2/assets/canonicalize")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"asset":"XLM"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::OK);
    let body = to_bytes(ok.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        json["data"]["asset"]["canonical"],
        "stellar:pubnet/slip44:148"
    );
    assert_eq!(json["data"]["input_form"], "legacy_stellar");

    let bad = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v2/assets/canonicalize")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"asset":"eip155:1/slip44:native"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn api_v2_has_no_quote_endpoint() {
    let router = setup_test_router().await;
    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v2/quote/native/USDC")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
