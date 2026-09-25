//! Integration tests for POST /api/v1/swap/submit (crypto verify + reconcile).

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration, Utc};
use common::{sign_envelope_with_keypair, test_keypair, TESTNET_PASSPHRASE, USDC_ISSUER};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use stellarroute_api::models::request::AssetPath;
use stellarroute_api::routes::simulation_route::{RouteDryRunHop, RouteDryRunPath};
use stellarroute_api::{
    audit::AuditRedactor,
    broadcast::{BroadcastError, BroadcastResult, MockTransactionBroadcaster},
    state::DatabasePools,
    swap::route::validate_classic_route,
    swap::store::{InMemorySwapQuoteStore, PreparedSwapQuote, SubmissionStatus, SwapQuoteStore},
    swap::tx::{build_unsigned_swap_tx, PrepareTxInput, DEFAULT_BASE_FEE},
    AppState,
};
use tower::ServiceExt;

async fn make_submit_router(
    store: Arc<InMemorySwapQuoteStore>,
    broadcaster: Arc<MockTransactionBroadcaster>,
) -> axum::Router {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .expect("Failed to create lazy pool");

    let app_state =
        AppState::new(DatabasePools::new(pool, None)).with_swap_services(store, broadcaster);

    stellarroute_api::routes::create_router(app_state.into_arc())
}

async fn post_submit(router: &axum::Router, body: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/swap/submit")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let resp = router.clone().oneshot(req).await.expect("request failed");
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(json!({}));
    (status, json)
}

fn sdex_route() -> RouteDryRunPath {
    RouteDryRunPath {
        hops: vec![RouteDryRunHop {
            from_asset: AssetPath {
                asset_code: "native".into(),
                asset_issuer: None,
            },
            to_asset: AssetPath {
                asset_code: "USDC".into(),
                asset_issuer: Some(USDC_ISSUER.to_string()),
            },
            source: "sdex".into(),
            fee_bps: Some(30),
            price: Some("0.12".into()),
            venue_ref: Some("sdex-venue".into()),
        }],
    }
}

fn build_prepared_pair(id: &str, expired: bool) -> (PreparedSwapQuote, String, [u8; 32], String) {
    let (seed, sender) = test_keypair();
    let route = sdex_route();
    let validated = validate_classic_route(&route).unwrap();
    let built = build_unsigned_swap_tx(PrepareTxInput {
        sender: &sender,
        validated: &validated,
        amount: 10.0,
        min_output: 1.0,
        sequence: 100,
        timeout_secs: 30,
        base_fee: DEFAULT_BASE_FEE,
        network_passphrase: TESTNET_PASSPHRASE,
    })
    .unwrap();
    let signed = sign_envelope_with_keypair(&built.xdr_envelope, &seed, TESTNET_PASSPHRASE);
    let expires_at = if expired {
        Utc::now() - Duration::minutes(5)
    } else {
        Utc::now() + Duration::minutes(5)
    };
    let quote = PreparedSwapQuote {
        quote_id: id.to_string(),
        sender_account: sender.clone(),
        sender_account_hash: AuditRedactor::redact_account(&sender),
        unsigned_xdr_hash: built.unsigned_xdr_hash,
        expires_at,
        estimated_output: "1.0000000".to_string(),
        min_output: "1.0000000".to_string(),
        amount_in: "10.0000000".into(),
        execution_mode: "classic_path_payment".into(),
        network_passphrase: TESTNET_PASSPHRASE.to_string(),
        route_digest: validated.route_digest,
        price_digest: "pd".into(),
        source_sequence: Some(built.source_sequence),
        timebounds_max: Some(built.timebounds_max as i64),
        base_fee: Some(DEFAULT_BASE_FEE as i32),
        valid_until_ledger: None,
        submission_status: SubmissionStatus::Prepared,
        tx_hash: None,
    };
    (quote, signed, seed, sender)
}

#[tokio::test]
async fn submit_swap_success() {
    let store = Arc::new(InMemorySwapQuoteStore::default());
    let (quote, signed, _, _) = build_prepared_pair("q-success", false);
    store.insert_prepared(&quote).await.unwrap();

    // Empty horizon hash → server keeps the cryptographically bound hash.
    let broadcaster = Arc::new(MockTransactionBroadcaster::succeed(""));
    let router = make_submit_router(store.clone(), broadcaster).await;

    let (status, json) = post_submit(
        &router,
        json!({ "quote_id": "q-success", "signed_xdr": signed }),
    )
    .await;

    assert_eq!(status, StatusCode::ACCEPTED, "{json}");
    assert!(!json["data"]["tx_hash"].as_str().unwrap().is_empty());
    assert_eq!(json["data"]["status"], "pending");
    let after = store.get("q-success").await.unwrap().unwrap();
    assert_eq!(after.submission_status, SubmissionStatus::Submitted);
    assert!(after.tx_hash.is_some());
}

#[tokio::test]
async fn submit_swap_duplicate_conflict() {
    let store = Arc::new(InMemorySwapQuoteStore::default());
    let (mut quote, signed, _, _) = build_prepared_pair("q-dup", false);
    quote.submission_status = SubmissionStatus::Submitted;
    quote.tx_hash = Some("existing-hash".into());
    store.insert_prepared(&quote).await.unwrap();

    let broadcaster = Arc::new(MockTransactionBroadcaster::succeed(""));
    let router = make_submit_router(store.clone(), broadcaster).await;

    let (status, json) = post_submit(
        &router,
        json!({ "quote_id": "q-dup", "signed_xdr": signed }),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(json["data"]["error"], "duplicate_quote");
}

#[tokio::test]
async fn submit_swap_in_progress_without_hash_is_integrity_conflict() {
    let store = Arc::new(InMemorySwapQuoteStore::default());
    let (mut quote, signed, _, _) = build_prepared_pair("q-progress", false);
    quote.submission_status = SubmissionStatus::Submitting;
    quote.tx_hash = None;
    store.insert_prepared(&quote).await.unwrap();

    let broadcaster = Arc::new(MockTransactionBroadcaster::succeed(""));
    let router = make_submit_router(store, broadcaster).await;

    let (status, json) = post_submit(
        &router,
        json!({ "quote_id": "q-progress", "signed_xdr": signed }),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT, "{json}");
    assert_eq!(json["data"]["details"]["status"], "submitting_without_hash");
}

#[tokio::test]
async fn submit_swap_permanently_failed_not_revived() {
    let store = Arc::new(InMemorySwapQuoteStore::default());
    let (mut quote, signed, _, _) = build_prepared_pair("q-perm", false);
    quote.submission_status = SubmissionStatus::Failed;
    store.insert_prepared(&quote).await.unwrap();

    let broadcaster = Arc::new(MockTransactionBroadcaster::succeed(""));
    let router = make_submit_router(store, broadcaster).await;
    let (status, json) = post_submit(
        &router,
        json!({ "quote_id": "q-perm", "signed_xdr": signed }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{json}");
    assert_eq!(json["data"]["details"]["status"], "permanently_failed");
}

#[tokio::test]
async fn submit_legacy_empty_passphrase_fails_closed() {
    let store = Arc::new(InMemorySwapQuoteStore::default());
    let (mut quote, signed, _, _) = build_prepared_pair("q-legacy", false);
    quote.network_passphrase = String::new();
    store.insert_prepared(&quote).await.unwrap();

    let broadcaster = Arc::new(MockTransactionBroadcaster::succeed(""));
    let router = make_submit_router(store, broadcaster).await;
    let (status, json) = post_submit(
        &router,
        json!({ "quote_id": "q-legacy", "signed_xdr": signed }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{json}");
    assert_eq!(
        json["data"]["details"]["status"],
        "missing_network_passphrase"
    );
}

#[tokio::test]
async fn submit_swap_unknown_quote() {
    let store = Arc::new(InMemorySwapQuoteStore::default());
    let broadcaster = Arc::new(MockTransactionBroadcaster::succeed(""));
    let router = make_submit_router(store.clone(), broadcaster).await;

    let (status, json) = post_submit(
        &router,
        json!({ "quote_id": "q-unknown", "signed_xdr": "c2lnbmVk" }),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(json["data"]["error"], "quote_not_found");
}

#[tokio::test]
async fn submit_swap_expired_prepared_quote() {
    let store = Arc::new(InMemorySwapQuoteStore::default());
    let (quote, signed, _, _) = build_prepared_pair("q-expired", true);
    store.insert_prepared(&quote).await.unwrap();

    let broadcaster = Arc::new(MockTransactionBroadcaster::succeed(""));
    let router = make_submit_router(store.clone(), broadcaster).await;

    let (status, json) = post_submit(
        &router,
        json!({ "quote_id": "q-expired", "signed_xdr": signed }),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(json["data"]["error"], "quote_expired");
}

#[tokio::test]
async fn submit_submitting_past_ttl_remains_reconcilable() {
    let store = Arc::new(InMemorySwapQuoteStore::default());
    let (quote, signed, _, _) = build_prepared_pair("q-ttl", false);
    store.insert_prepared(&quote).await.unwrap();

    // Claim with hash via first timed-out submit.
    let broadcaster =
        Arc::new(MockTransactionBroadcaster::fail(BroadcastError::Timeout).with_lookup(None));
    let router = make_submit_router(store.clone(), broadcaster).await;
    let (status, _) = post_submit(
        &router,
        json!({ "quote_id": "q-ttl", "signed_xdr": signed.clone() }),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);

    // Backdate expiry while still submitting.
    store.set_expires_at_for_tests("q-ttl", Utc::now() - Duration::minutes(30));
    let quote = store.get("q-ttl").await.unwrap().unwrap();
    assert_eq!(quote.submission_status, SubmissionStatus::Submitting);
    assert!(quote.tx_hash.is_some());

    let bound = quote.tx_hash.clone().unwrap();
    let found = BroadcastResult {
        tx_hash: bound.clone(),
        status: "pending".into(),
        ledger: Some(9),
    };
    let broadcaster2 = Arc::new(MockTransactionBroadcaster::succeed("").with_lookup(Some(found)));
    let router2 = make_submit_router(store.clone(), broadcaster2.clone()).await;
    let (status2, json2) = post_submit(
        &router2,
        json!({ "quote_id": "q-ttl", "signed_xdr": signed }),
    )
    .await;
    assert_eq!(status2, StatusCode::ACCEPTED, "{json2}");
    assert_eq!(json2["data"]["tx_hash"], bound);
    // Lookup path must not rebroadcast.
    assert_eq!(broadcaster2.call_count(), 0);
}

#[tokio::test]
async fn submit_rejects_different_signed_envelope_on_retry_zero_broadcast() {
    let store = Arc::new(InMemorySwapQuoteStore::default());
    let (quote, signed_a, seed, sender) = build_prepared_pair("q-attack", false);
    store.insert_prepared(&quote).await.unwrap();

    let broadcaster =
        Arc::new(MockTransactionBroadcaster::fail(BroadcastError::Timeout).with_lookup(None));
    let router = make_submit_router(store.clone(), broadcaster.clone()).await;
    let (status, _) = post_submit(
        &router,
        json!({ "quote_id": "q-attack", "signed_xdr": signed_a }),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(broadcaster.call_count(), 1);

    // Different body, validly signed with the same key.
    let route = sdex_route();
    let validated = validate_classic_route(&route).unwrap();
    let other = build_unsigned_swap_tx(PrepareTxInput {
        sender: &sender,
        validated: &validated,
        amount: 99.0,
        min_output: 1.0,
        sequence: 100,
        timeout_secs: 30,
        base_fee: DEFAULT_BASE_FEE,
        network_passphrase: TESTNET_PASSPHRASE,
    })
    .unwrap();
    let signed_b = sign_envelope_with_keypair(&other.xdr_envelope, &seed, TESTNET_PASSPHRASE);

    let broadcaster2 = Arc::new(MockTransactionBroadcaster::succeed(""));
    let router2 = make_submit_router(store.clone(), broadcaster2.clone()).await;
    let (status2, json2) = post_submit(
        &router2,
        json!({ "quote_id": "q-attack", "signed_xdr": signed_b }),
    )
    .await;
    assert_eq!(status2, StatusCode::BAD_REQUEST, "{json2}");
    assert_eq!(broadcaster2.call_count(), 0);
    let after = store.get("q-attack").await.unwrap().unwrap();
    assert_eq!(after.submission_status, SubmissionStatus::Submitting);
    assert_ne!(after.submission_status, SubmissionStatus::Submitted);
}

#[tokio::test]
async fn submit_lookup_found_does_not_rebroadcast() {
    let store = Arc::new(InMemorySwapQuoteStore::default());
    let (quote, signed, _, _) = build_prepared_pair("q-found", false);
    store.insert_prepared(&quote).await.unwrap();

    let broadcaster =
        Arc::new(MockTransactionBroadcaster::fail(BroadcastError::Timeout).with_lookup(None));
    let router = make_submit_router(store.clone(), broadcaster).await;
    let _ = post_submit(
        &router,
        json!({ "quote_id": "q-found", "signed_xdr": signed.clone() }),
    )
    .await;

    let bound = store
        .get("q-found")
        .await
        .unwrap()
        .unwrap()
        .tx_hash
        .unwrap();
    let found = BroadcastResult {
        tx_hash: bound.clone(),
        status: "success".into(),
        ledger: Some(42),
    };
    let broadcaster2 =
        Arc::new(MockTransactionBroadcaster::succeed("should-not-submit").with_lookup(Some(found)));
    let router2 = make_submit_router(store.clone(), broadcaster2.clone()).await;
    let (status, json) = post_submit(
        &router2,
        json!({ "quote_id": "q-found", "signed_xdr": signed }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["data"]["tx_hash"], bound);
    assert_eq!(broadcaster2.call_count(), 0);
}

#[tokio::test]
async fn submit_swap_rejects_wrong_key_signature() {
    let store = Arc::new(InMemorySwapQuoteStore::default());
    let (quote, _signed, _seed, _) = build_prepared_pair("q-wrongkey", false);
    store.insert_prepared(&quote).await.unwrap();

    let wrong_seed = [9u8; 32];
    let route = sdex_route();
    let validated = validate_classic_route(&route).unwrap();
    let built = build_unsigned_swap_tx(PrepareTxInput {
        sender: &quote.sender_account,
        validated: &validated,
        amount: 10.0,
        min_output: 1.0,
        sequence: 100,
        timeout_secs: 30,
        base_fee: DEFAULT_BASE_FEE,
        network_passphrase: TESTNET_PASSPHRASE,
    })
    .unwrap();
    assert_eq!(built.unsigned_xdr_hash, quote.unsigned_xdr_hash);
    let wrong_signed =
        sign_envelope_with_keypair(&built.xdr_envelope, &wrong_seed, TESTNET_PASSPHRASE);

    let broadcaster = Arc::new(MockTransactionBroadcaster::succeed(""));
    let router = make_submit_router(store, broadcaster).await;
    let (status, json) = post_submit(
        &router,
        json!({ "quote_id": "q-wrongkey", "signed_xdr": wrong_signed }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{json}");
}

#[tokio::test]
async fn submit_swap_rejects_tampered_xdr() {
    let store = Arc::new(InMemorySwapQuoteStore::default());
    let (quote, _signed, seed, sender) = build_prepared_pair("q-tamper", false);
    store.insert_prepared(&quote).await.unwrap();

    let route = sdex_route();
    let validated = validate_classic_route(&route).unwrap();
    let other = build_unsigned_swap_tx(PrepareTxInput {
        sender: &sender,
        validated: &validated,
        amount: 99.0,
        min_output: 1.0,
        sequence: 100,
        timeout_secs: 30,
        base_fee: DEFAULT_BASE_FEE,
        network_passphrase: TESTNET_PASSPHRASE,
    })
    .unwrap();
    let tampered = sign_envelope_with_keypair(&other.xdr_envelope, &seed, TESTNET_PASSPHRASE);

    let broadcaster = Arc::new(MockTransactionBroadcaster::succeed(""));
    let router = make_submit_router(store, broadcaster).await;
    let (status, json) = post_submit(
        &router,
        json!({ "quote_id": "q-tamper", "signed_xdr": tampered }),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{json}");
    assert!(json["data"]["message"]
        .as_str()
        .unwrap()
        .contains("does not match the prepared quote"));
}

#[tokio::test]
async fn submit_swap_broadcast_failure_marks_failed() {
    let store = Arc::new(InMemorySwapQuoteStore::default());
    let (quote, signed, _, _) = build_prepared_pair("q-fail", false);
    store.insert_prepared(&quote).await.unwrap();

    let broadcaster = Arc::new(MockTransactionBroadcaster::fail(
        BroadcastError::InsufficientFee,
    ));
    let router = make_submit_router(store.clone(), broadcaster).await;

    let (status, _json) = post_submit(
        &router,
        json!({ "quote_id": "q-fail", "signed_xdr": signed }),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    let after = store.get("q-fail").await.unwrap().unwrap();
    assert_eq!(after.submission_status, SubmissionStatus::Failed);
    assert!(
        after.tx_hash.is_some(),
        "hash bound at claim before broadcast"
    );
}

#[tokio::test]
async fn submit_submitting_past_timebounds_absent_horizon_marks_failed() {
    let store = Arc::new(InMemorySwapQuoteStore::default());
    let (quote, signed, _, _) = build_prepared_pair("q-tb", false);
    store.insert_prepared(&quote).await.unwrap();

    let broadcaster =
        Arc::new(MockTransactionBroadcaster::fail(BroadcastError::Timeout).with_lookup(None));
    let router = make_submit_router(store.clone(), broadcaster).await;
    let (status, _) = post_submit(
        &router,
        json!({ "quote_id": "q-tb", "signed_xdr": signed.clone() }),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);

    // Close the Stellar timebounds window while Horizon still has no tx.
    store.set_timebounds_max_for_tests("q-tb", Some(Utc::now().timestamp() - 10));

    let broadcaster2 = Arc::new(MockTransactionBroadcaster::succeed("").with_lookup(None));
    let router2 = make_submit_router(store.clone(), broadcaster2.clone()).await;
    let (status2, json2) = post_submit(
        &router2,
        json!({ "quote_id": "q-tb", "signed_xdr": signed }),
    )
    .await;
    assert_eq!(status2, StatusCode::CONFLICT, "{json2}");
    assert_eq!(json2["data"]["details"]["status"], "permanently_failed");
    assert_eq!(broadcaster2.call_count(), 0);

    let after = store.get("q-tb").await.unwrap().unwrap();
    assert_eq!(after.submission_status, SubmissionStatus::Failed);
}

#[tokio::test]
async fn submit_timeout_keeps_submitting_then_allows_reconcile_retry() {
    let store = Arc::new(InMemorySwapQuoteStore::default());
    let (quote, signed, _, _) = build_prepared_pair("q-retry", false);
    store.insert_prepared(&quote).await.unwrap();

    let broadcaster =
        Arc::new(MockTransactionBroadcaster::fail(BroadcastError::Timeout).with_lookup(None));
    let router = make_submit_router(store.clone(), broadcaster).await;

    let (status, json) = post_submit(
        &router,
        json!({ "quote_id": "q-retry", "signed_xdr": signed.clone() }),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{json}");

    let after = store.get("q-retry").await.unwrap().unwrap();
    assert_eq!(after.submission_status, SubmissionStatus::Submitting);
    assert!(after.tx_hash.is_some());

    let broadcaster2 = Arc::new(MockTransactionBroadcaster::succeed(""));
    let router2 = make_submit_router(store.clone(), broadcaster2).await;
    let (status2, json2) = post_submit(
        &router2,
        json!({ "quote_id": "q-retry", "signed_xdr": signed }),
    )
    .await;
    assert_eq!(status2, StatusCode::ACCEPTED, "{json2}");
    assert_eq!(
        json2["data"]["tx_hash"],
        after.tx_hash.as_ref().unwrap().as_str()
    );
}
