//! Unit tests for the swap prepare/submit helpers.
//!
//! HTTP is mocked with `wiremock` — no live API or network access required.
//!
//! Run with:
//!   cargo test -p stellarroute-sdk swap

use stellarroute_sdk::{
    ApiErrorCode, ClientBuilder, Route, RouteHop, SdkError, SwapPrepareRequest, SwapSubmitRequest,
};
use wiremock::{
    matchers::{body_partial_json, method, path},
    Mock, MockServer, ResponseTemplate,
};

fn hop() -> RouteHop {
    RouteHop {
        from_asset: None,
        to_asset: None,
        price: "0.12".to_string(),
        fee_bps: Some(30),
        amount_out_of_hop: "98".to_string(),
        source: "sdex".to_string(),
    }
}

fn route() -> Route {
    Route {
        estimated_output: "98".to_string(),
        impact_bps: 12,
        score: 0.94,
        policy_used: "best_output".to_string(),
        path: vec![hop()],
    }
}

#[tokio::test]
async fn prepare_swap_returns_unsigned_envelope() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/swap/prepare"))
        .and(body_partial_json(serde_json::json!({
            "amount": "100",
            "sender": "GABC",
            "slippage_bps": 50,
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "xdr_envelope": "AAAAAgAAAAB...",
            "estimated_output": "98.5",
            "min_output": "98.0",
            "valid_until_ledger": 1_234_567u64,
        })))
        .mount(&server)
        .await;

    let client = ClientBuilder::new(server.uri()).build().unwrap();
    let prepared = client
        .prepare_swap(SwapPrepareRequest::from_route(&route(), "100", "GABC").slippage_bps(50))
        .await
        .expect("prepare succeeds");

    assert_eq!(prepared.xdr_envelope, "AAAAAgAAAAB...");
    assert_eq!(prepared.estimated_output, "98.5");
    assert_eq!(prepared.valid_until_ledger, Some(1_234_567));
}

#[tokio::test]
async fn prepare_swap_maps_no_route_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/swap/prepare"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": "no_route",
            "message": "Route is no longer executable",
            "details": null,
        })))
        .mount(&server)
        .await;

    let client = ClientBuilder::new(server.uri()).build().unwrap();
    let err = client
        .prepare_swap(SwapPrepareRequest::from_route(&route(), "100", "GABC"))
        .await
        .expect_err("prepare fails");

    match err {
        SdkError::Api { code, status, .. } => {
            assert_eq!(code, ApiErrorCode::NoRoute);
            assert_eq!(status, 404);
        }
        other => panic!("expected API error, got {other:?}"),
    }
}

#[tokio::test]
async fn submit_swap_returns_tx_receipt() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/swap/submit"))
        .and(body_partial_json(serde_json::json!({
            "signed_xdr": "c2lnbmVk",
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "tx_hash": "abc123",
            "status": "success",
            "output_amount": "98.4",
            "ledger": 42u64,
        })))
        .mount(&server)
        .await;

    let client = ClientBuilder::new(server.uri()).build().unwrap();
    let receipt = client
        .submit_swap(SwapSubmitRequest::new("c2lnbmVk"))
        .await
        .expect("submit succeeds");

    assert!(receipt.is_success());
    assert_eq!(receipt.tx_hash, "abc123");
    assert_eq!(receipt.output_amount.as_deref(), Some("98.4"));
    assert_eq!(receipt.ledger, Some(42));
}

#[tokio::test]
async fn submit_swap_maps_validation_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/swap/submit"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": "validation_error",
            "message": "Envelope is not signed",
            "details": null,
        })))
        .mount(&server)
        .await;

    let client = ClientBuilder::new(server.uri()).build().unwrap();
    let err = client
        .submit_swap(SwapSubmitRequest::new("unsigned"))
        .await
        .expect_err("submit fails");

    match err {
        SdkError::Api { code, message, .. } => {
            assert_eq!(code, ApiErrorCode::ValidationError);
            assert_eq!(message, "Envelope is not signed");
        }
        other => panic!("expected API error, got {other:?}"),
    }
}

#[tokio::test]
async fn submit_swap_reports_pending_status() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/swap/submit"))
        .respond_with(ResponseTemplate::new(202).set_body_json(serde_json::json!({
            "tx_hash": "pending123",
            "status": "pending",
        })))
        .mount(&server)
        .await;

    let client = ClientBuilder::new(server.uri()).build().unwrap();
    let receipt = client
        .submit_swap(SwapSubmitRequest::new("c2lnbmVk"))
        .await
        .expect("submit accepted");

    assert!(!receipt.is_success());
    assert_eq!(receipt.status, "pending");
    assert!(receipt.output_amount.is_none());
}
