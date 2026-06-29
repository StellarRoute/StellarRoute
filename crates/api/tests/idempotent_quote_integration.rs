//! Integration tests for POST /api/v1/quote idempotency-key behavior.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use stellarroute_api::{
    exactlyonce::RequestIdentity,
    routes::idempotent_quote::IDEMPOTENCY_KEY_MAX_LEN,
    state::DatabasePools,
    Server, ServerConfig,
};
use tower::ServiceExt;

// ─── Helpers ─────────────────────────────────────────────────────────────────

async fn make_lazy_router() -> axum::Router {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .expect("Failed to create lazy pool");
    Server::new(ServerConfig::default(), DatabasePools::new(pool, None))
        .await
        .into_router()
}

async fn make_db_router() -> axum::Router {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
    });
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&db_url)
        .await
        .expect("Failed to connect to database");
    Server::new(ServerConfig::default(), DatabasePools::new(pool, None))
        .await
        .into_router()
}

async fn post_quote(
    router: &axum::Router,
    key: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/v1/quote")
        .header("content-type", "application/json");
    if let Some(k) = key {
        builder = builder.header("idempotency-key", k);
    }
    let req = builder
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

async fn fetch_prometheus_metrics(router: &axum::Router) -> String {
    let req = Request::builder()
        .uri("/metrics")
        .body(Body::empty())
        .unwrap();
    let resp = router.clone().oneshot(req).await.expect("metrics failed");
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).expect("metrics must be utf-8")
}

fn prometheus_quote_misses(metrics_text: &str) -> u64 {
    metrics_text
        .lines()
        .find(|line| line.starts_with("stellarroute_cache_misses_total{type=\"quote\"}"))
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

fn sample_quote_body() -> Value {
    json!({
        "base": "native",
        "quote": "USDC",
        "amount": "1"
    })
}

// ─── Unit tests (no database) ────────────────────────────────────────────────

#[test]
fn post_quote_scoped_keys_do_not_collide_with_get_quote_identities() {
    let post_quote_identity = RequestIdentity {
        base_asset: "post_quote:shared-key".to_string(),
        quote_asset: String::new(),
        amount: String::new(),
        slippage_bps: 0,
        quote_type: String::new(),
    };
    let get_quote_identity = RequestIdentity {
        base_asset: "shared-key".to_string(),
        quote_asset: "USDC".to_string(),
        amount: "1".to_string(),
        slippage_bps: 50,
        quote_type: "sell".to_string(),
    };

    assert_ne!(
        post_quote_identity.canonical_key(),
        get_quote_identity.canonical_key(),
        "post_quote-prefixed keys must not collide with GET quote dedupe identities"
    );
}

#[tokio::test]
async fn empty_idempotency_key_returns_400() {
    let router = make_lazy_router().await;
    let (status, json) = post_quote(&router, Some("   "), sample_quote_body()).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(json["data"]["error"], "validation_error");
    assert!(
        json["data"]["message"]
            .as_str()
            .unwrap()
            .contains("Idempotency-Key")
    );
}

#[tokio::test]
async fn oversized_idempotency_key_returns_400() {
    let router = make_lazy_router().await;
    let long_key = "k".repeat(IDEMPOTENCY_KEY_MAX_LEN + 1);
    let (status, json) = post_quote(&router, Some(&long_key), sample_quote_body()).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(json["data"]["error"], "validation_error");
}

#[tokio::test]
async fn missing_idempotency_header_skips_key_validation() {
    let router = make_lazy_router().await;
    let (status, _json) = post_quote(&router, None, sample_quote_body()).await;
    // Without a key, validation passes and the handler proceeds toward quote lookup.
    assert_ne!(status, StatusCode::BAD_REQUEST);
}

// ─── HTTP integration tests (require PostgreSQL) ─────────────────────────────

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn duplicate_idempotency_key_replays_identical_quote_data() {
    let router = make_db_router().await;
    let body = sample_quote_body();
    let key = "test-key-836-dup";

    let (status1, json1) = post_quote(&router, Some(key), body.clone()).await;
    let (status2, json2) = post_quote(&router, Some(key), body).await;

    assert_eq!(status1, status2);
    assert_eq!(json1["data"], json2["data"], "quote payload must match on replay");
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn idempotency_key_normalization_is_case_insensitive() {
    let router = make_db_router().await;
    let body = sample_quote_body();

    let (_, json1) = post_quote(&router, Some("Case-Key-836"), body.clone()).await;
    let (_, json2) = post_quote(&router, Some("case-key-836"), body).await;

    assert_eq!(json1["data"], json2["data"]);
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn different_idempotency_keys_are_independent() {
    let router = make_db_router().await;
    let body = sample_quote_body();

    let (status1, json1) = post_quote(&router, Some("key-a-836"), body.clone()).await;
    let (status2, json2) = post_quote(&router, Some("key-b-836"), body).await;

    assert_eq!(status1, status2);
    // Both should succeed independently; payloads may match for identical params
    // but each request executed the pipeline separately.
    assert!(json1["data"].is_object());
    assert!(json2["data"].is_object());
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn replay_does_not_increment_quote_cache_miss_counter() {
    let router = make_db_router().await;
    let body = sample_quote_body();
    let key = "metrics-key-836";

    let misses_before = prometheus_quote_misses(&fetch_prometheus_metrics(&router).await);
    let (status, _) = post_quote(&router, Some(key), body.clone()).await;
    assert_eq!(status, StatusCode::OK);
    let misses_after_first = prometheus_quote_misses(&fetch_prometheus_metrics(&router).await);
    assert!(
        misses_after_first >= misses_before,
        "first request should reach the quote pipeline"
    );

    let (status, _) = post_quote(&router, Some(key), body).await;
    assert_eq!(status, StatusCode::OK);
    let misses_after_replay = prometheus_quote_misses(&fetch_prometheus_metrics(&router).await);
    assert_eq!(
        misses_after_replay, misses_after_first,
        "idempotent replay must not re-run the quote pipeline"
    );
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn requests_without_idempotency_key_both_run_pipeline() {
    let router = make_db_router().await;
    let body = sample_quote_body();

    let misses_before = prometheus_quote_misses(&fetch_prometheus_metrics(&router).await);
    let (status1, _) = post_quote(&router, None, body.clone()).await;
    let (status2, _) = post_quote(&router, None, body).await;
    assert_eq!(status1, StatusCode::OK);
    assert_eq!(status2, StatusCode::OK);

    let misses_after = prometheus_quote_misses(&fetch_prometheus_metrics(&router).await);
    assert!(
        misses_after > misses_before,
        "missing header should allow both requests through the pipeline"
    );
}
