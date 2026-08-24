//! Integration tests for the swap prepare/submit endpoints (classic single-hop SDEX).

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use common::{sign_envelope_with_keypair, test_keypair, TESTNET_PASSPHRASE, USDC_ISSUER};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use stellarroute_api::{
    broadcast::MockTransactionBroadcaster, kill_switch::KillSwitchState, state::DatabasePools,
    swap::store::InMemorySwapQuoteStore, AppState,
};
use stellarroute_routing::health::policy::OverrideDirective;
use stellarroute_routing::health::scorer::VenueType;
use tower::ServiceExt;

async fn setup_router() -> axum::Router {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .expect("failed to create lazy pool");

    let app_state = AppState::new(DatabasePools::new(pool, None)).with_swap_services(
        Arc::new(InMemorySwapQuoteStore::default()),
        Arc::new(MockTransactionBroadcaster::succeed("")),
    );

    stellarroute_api::routes::create_router(app_state.into_arc())
}

async fn setup_router_with_state(app_state: AppState) -> axum::Router {
    stellarroute_api::routes::create_router(app_state.into_arc())
}

fn valid_prepare_payload(sender: &str) -> Value {
    json!({
        "route": {
            "hops": [{
                "from_asset": { "asset_code": "native", "asset_issuer": null },
                "to_asset": { "asset_code": "USDC", "asset_issuer": USDC_ISSUER },
                "source": "sdex",
                "fee_bps": 30,
                "price": "0.12",
                "venue_ref": "sdex-venue"
            }]
        },
        "amount": "100",
        "sender": sender,
        "slippage_bps": 50
    })
}

async fn post(router: axum::Router, path: &str, payload: &Value) -> (StatusCode, Value) {
    let (status, bytes) = post_raw(router, path, payload).await;
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

async fn post_raw(router: axum::Router, path: &str, payload: &Value) -> (StatusCode, Vec<u8>) {
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
        .unwrap()
        .to_vec();
    (status, bytes)
}

#[tokio::test]
async fn prepare_swap_valid_request_returns_prepared_quote() {
    let (_seed, sender) = test_keypair();
    let router = setup_router().await;
    let (status, body) = post(
        router,
        "/api/v1/swap/prepare",
        &valid_prepare_payload(&sender),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body["data"]["quote_id"].is_string());
    assert!(body["data"]["xdr_envelope"].is_string());
    assert_eq!(body["data"]["expected_output"], "100.0000000");
    assert_eq!(body["data"]["min_output"], "99.5000000");
    assert_eq!(body["data"]["execution_mode"], "classic_path_payment");
    assert_eq!(
        body["data"]["network_passphrase"], TESTNET_PASSPHRASE,
        "{body}"
    );
}

/// SDK/frontend string-hop prepare body must deserialize into the handler
/// (handler validation / success), not fail at Axum serde with a generic 4xx.
#[tokio::test]
async fn prepare_accepts_sdk_string_asset_hops() {
    let (_seed, sender) = test_keypair();
    let payload = json!({
        "route": {
            "hops": [{
                "from_asset": "native",
                "to_asset": format!("USDC:{USDC_ISSUER}"),
                "source": "sdex",
                "fee_bps": 30,
                "price": "0.12",
                "venue_ref": "sdex-venue"
            }]
        },
        "amount": "100",
        "sender": sender,
        "slippage_bps": 50
    });
    let router = setup_router().await;
    let (status, body) = post(router, "/api/v1/swap/prepare", &payload).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["execution_mode"], "classic_path_payment");
    assert!(body["data"]["quote_id"].is_string());
    assert_eq!(body["data"]["network_passphrase"], TESTNET_PASSPHRASE);
}

/// Canonical object-hop prepare body must also reach handler success.
#[tokio::test]
async fn prepare_accepts_canonical_object_asset_hops() {
    let (_seed, sender) = test_keypair();
    let router = setup_router().await;
    let (status, body) = post(
        router,
        "/api/v1/swap/prepare",
        &valid_prepare_payload(&sender),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["execution_mode"], "classic_path_payment");
}

/// Invalid string asset shape fails closed at deserialize (not silently accepted).
#[tokio::test]
async fn prepare_rejects_invalid_string_asset_at_wire() {
    let (_seed, sender) = test_keypair();
    let mut payload = valid_prepare_payload(&sender);
    payload["route"]["hops"][0]["from_asset"] = json!("");
    let router = setup_router().await;
    let (status, bytes) = post_raw(router, "/api/v1/swap/prepare", &payload).await;
    // Axum JSON rejection — must not reach handler success.
    assert!(
        status.is_client_error(),
        "expected client error for empty asset string, got {status}"
    );
    let body_text = String::from_utf8_lossy(&bytes);
    assert!(
        !body_text.contains("classic_path_payment"),
        "empty asset must not produce a prepared quote: {body_text}"
    );
    assert!(
        !body_text.contains("\"quote_id\""),
        "empty asset must not reach handler success: {body_text}"
    );
}

#[tokio::test]
async fn prepare_rejects_amm_venue_unsupported_execution_mode() {
    let (_seed, sender) = test_keypair();
    let mut payload = valid_prepare_payload(&sender);
    payload["route"]["hops"][0]["source"] = json!("amm");
    let router = setup_router().await;
    let (status, body) = post(router, "/api/v1/swap/prepare", &payload).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["data"]["error"], "unsupported_execution_mode");
}

#[tokio::test]
async fn prepare_rejects_multi_hop_unsupported_route() {
    let (_seed, sender) = test_keypair();
    let mut payload = valid_prepare_payload(&sender);
    payload["route"]["hops"] = json!([
        {
            "from_asset": { "asset_code": "native", "asset_issuer": null },
            "to_asset": { "asset_code": "USDC", "asset_issuer": USDC_ISSUER },
            "source": "sdex",
            "venue_ref": "sdex-a"
        },
        {
            "from_asset": { "asset_code": "USDC", "asset_issuer": USDC_ISSUER },
            "to_asset": { "asset_code": "EURC", "asset_issuer": USDC_ISSUER },
            "source": "sdex",
            "venue_ref": "sdex-b"
        }
    ]);
    let router = setup_router().await;
    let (status, body) = post(router, "/api/v1/swap/prepare", &payload).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["data"]["error"], "unsupported_route");
}

#[tokio::test]
async fn prepare_rejects_client_min_below_slippage_floor() {
    let (_seed, sender) = test_keypair();
    let mut payload = valid_prepare_payload(&sender);
    payload["min_output"] = json!("90");
    let router = setup_router().await;
    let (status, body) = post(router, "/api/v1/swap/prepare", &payload).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["data"]["error"], "validation_error");
}

#[tokio::test]
async fn prepare_rejects_concurrent_active_prepare_same_sender() {
    let (_seed, sender) = test_keypair();
    let store = Arc::new(InMemorySwapQuoteStore::default());
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    let app_state = AppState::new(DatabasePools::new(pool, None))
        .with_swap_services(store, Arc::new(MockTransactionBroadcaster::succeed("tx")));
    let router = setup_router_with_state(app_state).await;
    let payload = valid_prepare_payload(&sender);
    let (s1, b1) = post(router.clone(), "/api/v1/swap/prepare", &payload).await;
    assert_eq!(s1, StatusCode::OK, "{b1}");
    let (s2, b2) = post(router, "/api/v1/swap/prepare", &payload).await;
    assert_eq!(s2, StatusCode::CONFLICT, "{b2}");
    assert_eq!(b2["data"]["details"]["status"], "active_prepare_exists");
}

#[tokio::test]
async fn prepare_swap_rejects_paused_venue() {
    let (_seed, sender) = test_keypair();
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    let app_state = AppState::new(DatabasePools::new(pool, None)).with_swap_services(
        Arc::new(InMemorySwapQuoteStore::default()),
        Arc::new(MockTransactionBroadcaster::succeed("tx")),
    );
    let mut venues = std::collections::HashMap::new();
    venues.insert("sdex-venue".to_string(), OverrideDirective::ForceExclude);
    app_state
        .kill_switch
        .update_state(KillSwitchState {
            sources: Default::default(),
            venues,
            providers: Default::default(),
        })
        .await
        .unwrap();

    let router = setup_router_with_state(app_state).await;
    let (status, body) = post(
        router,
        "/api/v1/swap/prepare",
        &valid_prepare_payload(&sender),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["data"]["error"], "not_executable");
}

#[tokio::test]
async fn prepare_swap_rejects_paused_source() {
    let (_seed, sender) = test_keypair();
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    let app_state = AppState::new(DatabasePools::new(pool, None)).with_swap_services(
        Arc::new(InMemorySwapQuoteStore::default()),
        Arc::new(MockTransactionBroadcaster::succeed("tx")),
    );
    let mut sources = std::collections::HashMap::new();
    sources.insert(VenueType::Sdex, OverrideDirective::ForceExclude);
    app_state
        .kill_switch
        .update_state(KillSwitchState {
            sources,
            venues: Default::default(),
            providers: Default::default(),
        })
        .await
        .unwrap();

    let router = setup_router_with_state(app_state).await;
    let (status, body) = post(
        router,
        "/api/v1/swap/prepare",
        &valid_prepare_payload(&sender),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["data"]["error"], "not_executable");
}

#[tokio::test]
async fn prepare_then_submit_success_with_real_signature() {
    let (seed, sender) = test_keypair();
    let store = Arc::new(InMemorySwapQuoteStore::default());
    let broadcaster = Arc::new(MockTransactionBroadcaster::succeed(""));
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    let app_state = AppState::new(DatabasePools::new(pool, None))
        .with_swap_services(store.clone(), broadcaster);
    let router = setup_router_with_state(app_state).await;

    let (prep_status, prep_body) = post(
        router.clone(),
        "/api/v1/swap/prepare",
        &valid_prepare_payload(&sender),
    )
    .await;
    assert_eq!(prep_status, StatusCode::OK, "{prep_body}");
    let quote_id = prep_body["data"]["quote_id"].as_str().unwrap().to_string();
    let unsigned = prep_body["data"]["xdr_envelope"].as_str().unwrap();
    let signed = sign_envelope_with_keypair(unsigned, &seed, TESTNET_PASSPHRASE);

    let (status, body) = post(
        router,
        "/api/v1/swap/submit",
        &json!({ "quote_id": quote_id, "signed_xdr": signed }),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED, "{body}");
    // Deterministic network tx hash from signed envelope, not mock's arbitrary string,
    // when mock returns a different hash — finalize uses expected hash.
    assert!(body["data"]["tx_hash"].is_string());
}
