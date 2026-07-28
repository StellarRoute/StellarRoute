//! Integration tests for the swap prepare/submit stub endpoints (issue
//! #1051). Transaction building/submission aren't implemented yet — these
//! confirm validation runs first and that valid requests get a documented
//! `501 not_implemented` rather than a silent failure or wrong status.
//!
//! Runs fully in-process against a lazily-connected Postgres pool (never
//! actually dials out), so it requires no network access.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use stellarroute_api::{state::DatabasePools, Server, ServerConfig};
use tower::ServiceExt;

async fn setup_router() -> axum::Router {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .expect("failed to create lazy pool");

    Server::new(ServerConfig::default(), DatabasePools::new(pool, None))
        .await
        .into_router()
}

fn valid_prepare_payload() -> Value {
    json!({
        "route": {
            "hops": [{
                "from_asset": { "asset_code": "native", "asset_issuer": null },
                "to_asset": { "asset_code": "USDC", "asset_issuer": null },
                "source": "sdex",
                "fee_bps": 30,
                "price": "0.12",
                "venue_ref": "sdex-venue"
            }]
        },
        "amount": "100",
        "sender": "GABCDEF",
    })
}

async fn post(router: axum::Router, path: &str, payload: &Value) -> (StatusCode, Value) {
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .expect("request failed");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    (status, json)
}

#[tokio::test]
async fn prepare_swap_valid_request_returns_not_implemented() {
    let router = setup_router().await;
    let (status, body) = post(router, "/api/v1/swap/prepare", &valid_prepare_payload()).await;

    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
    assert_eq!(body["data"]["error"], "not_implemented");
}

#[tokio::test]
async fn prepare_swap_rejects_zero_amount_before_not_implemented() {
    let mut payload = valid_prepare_payload();
    payload["amount"] = json!("0");
    let router = setup_router().await;
    let (status, body) = post(router, "/api/v1/swap/prepare", &payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["data"]["error"], "validation_error");
}

#[tokio::test]
async fn prepare_swap_rejects_empty_route() {
    let mut payload = valid_prepare_payload();
    payload["route"]["hops"] = json!([]);
    let router = setup_router().await;
    let (status, body) = post(router, "/api/v1/swap/prepare", &payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["data"]["error"], "validation_error");
}

#[tokio::test]
async fn submit_swap_valid_request_returns_not_implemented() {
    let payload = json!({ "xdr_envelope": "AAAAAgAAAAA=" });
    let router = setup_router().await;
    let (status, body) = post(router, "/api/v1/swap/submit", &payload).await;

    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
    assert_eq!(body["data"]["error"], "not_implemented");
}

#[tokio::test]
async fn submit_swap_rejects_empty_xdr_envelope() {
    let payload = json!({ "xdr_envelope": "" });
    let router = setup_router().await;
    let (status, body) = post(router, "/api/v1/swap/submit", &payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["data"]["error"], "validation_error");
}
