//! Integration tests for GET /api/v1/activity/swaps pagination and response shape.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::PgPool;
use stellarroute_api::{
    routes::activity::{SwapActivityItem, SwapActivityResponse},
    state::DatabasePools,
    Server, ServerConfig,
};
use tower::ServiceExt;

const TEST_EVENT_PREFIX: &str = "test-838-";

// ─── Helpers ─────────────────────────────────────────────────────────────────

async fn make_router(pool: PgPool) -> axum::Router {
    Server::new(ServerConfig::default(), DatabasePools::new(pool, None))
        .await
        .into_router()
}

async fn connect_pool() -> PgPool {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
    });
    PgPool::connect(&db_url)
        .await
        .expect("Failed to connect to database")
}

async fn ensure_swap_activity_table(pool: &PgPool) {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS contract_swap_activity (
            event_id TEXT PRIMARY KEY,
            contract_id TEXT NOT NULL,
            ledger BIGINT NOT NULL,
            ledger_closed_at TIMESTAMPTZ,
            paging_token TEXT NOT NULL,
            sender TEXT NOT NULL,
            amount_in NUMERIC NOT NULL,
            amount_out NUMERIC NOT NULL,
            fee_amount NUMERIC NOT NULL,
            route JSONB NOT NULL DEFAULT '{}'::jsonb,
            source_asset TEXT,
            destination_asset TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT now()
        )
        "#,
    )
    .execute(pool)
    .await
    .expect("contract_swap_activity table must exist");
}

async fn cleanup_test_rows(pool: &PgPool) {
    sqlx::query("DELETE FROM contract_swap_activity WHERE event_id LIKE $1")
        .bind(format!("{TEST_EVENT_PREFIX}%"))
        .execute(pool)
        .await
        .expect("failed to clean up test rows");
}

async fn seed_swap_rows(pool: &PgPool) {
    let now = Utc::now();
    let rows = [
        (format!("{TEST_EVENT_PREFIX}a"), 100_i64, "token-a"),
        (format!("{TEST_EVENT_PREFIX}b"), 100_i64, "token-b"),
        (format!("{TEST_EVENT_PREFIX}c"), 90_i64, "token-c"),
        (format!("{TEST_EVENT_PREFIX}d"), 80_i64, "token-d"),
        (format!("{TEST_EVENT_PREFIX}e"), 70_i64, "token-e"),
    ];

    for (event_id, ledger, paging_token) in rows {
        sqlx::query(
            r#"
            INSERT INTO contract_swap_activity (
                event_id, contract_id, ledger, ledger_closed_at, paging_token,
                sender, amount_in, amount_out, fee_amount, route,
                source_asset, destination_asset
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (event_id) DO NOTHING
            "#,
        )
        .bind(event_id)
        .bind("CROUTER")
        .bind(ledger)
        .bind(now)
        .bind(paging_token)
        .bind("GTEST")
        .bind("1000000")
        .bind("990000")
        .bind("1000")
        .bind(json!([{"hop": 1}]))
        .bind("native")
        .bind("USDC")
        .execute(pool)
        .await
        .expect("failed to seed swap row");
    }
}

async fn get_swaps(router: &axum::Router, query: &str) -> (StatusCode, Value) {
    let uri = format!("/api/v1/activity/swaps{query}");
    let req = Request::builder()
        .method("GET")
        .uri(&uri)
        .body(Body::empty())
        .unwrap();
    let resp = router.clone().oneshot(req).await.expect("request failed");
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    (status, json)
}

fn swap_ledgers(json: &Value) -> Vec<i64> {
    json["data"]["swaps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["ledger"].as_i64().unwrap())
        .collect()
}

fn swap_event_ids(json: &Value) -> Vec<String> {
    json["data"]["swaps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["event_id"].as_str().unwrap().to_string())
        .collect()
}

// ─── Unit tests (no database) ────────────────────────────────────────────────

#[test]
fn swap_activity_item_serializes_to_openapi_shape() {
    let item = SwapActivityItem {
        event_id: "evt-1".to_string(),
        contract_id: "CROUTER".to_string(),
        ledger: 42,
        ledger_closed_at: Some(Utc::now()),
        paging_token: "tok".to_string(),
        sender: "GABC".to_string(),
        amount_in: "100".to_string(),
        amount_out: "99".to_string(),
        fee_amount: "1".to_string(),
        route: json!({"hops": []}),
        source_asset: Some("native".to_string()),
        destination_asset: Some("USDC".to_string()),
    };

    let json = serde_json::to_value(&item).expect("serialization failed");
    assert_eq!(json["event_id"], "evt-1");
    assert_eq!(json["contract_id"], "CROUTER");
    assert_eq!(json["ledger"], 42);
    assert!(json["ledger_closed_at"].is_string());
    assert_eq!(json["paging_token"], "tok");
    assert_eq!(json["sender"], "GABC");
    assert!(json["amount_in"].is_string());
    assert!(json["amount_out"].is_string());
    assert!(json["fee_amount"].is_string());
    assert!(json["route"].is_object());
    assert_eq!(json["source_asset"], "native");
    assert_eq!(json["destination_asset"], "USDC");
}

#[test]
fn swap_activity_response_empty_array_serializes() {
    let response = SwapActivityResponse { swaps: vec![] };
    let json = serde_json::to_value(&response).expect("serialization failed");
    assert!(json["swaps"].as_array().unwrap().is_empty());
}

// ─── HTTP integration tests (require PostgreSQL) ─────────────────────────────

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn default_limit_returns_up_to_fifty_rows_in_descending_order() {
    let pool = connect_pool().await;
    ensure_swap_activity_table(&pool).await;
    cleanup_test_rows(&pool).await;
    seed_swap_rows(&pool).await;
    let router = make_router(pool.clone()).await;

    let (status, json) = get_swaps(&router, "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["v"], 1);
    assert!(json["request_id"].is_string());
    assert_eq!(json["data"]["swaps"].as_array().unwrap().len(), 5);

    let ledgers = swap_ledgers(&json);
    assert_eq!(ledgers, vec![100, 100, 90, 80, 70]);
    let event_ids = swap_event_ids(&json);
    assert_eq!(event_ids[0], format!("{TEST_EVENT_PREFIX}b"));
    assert_eq!(event_ids[1], format!("{TEST_EVENT_PREFIX}a"));

    cleanup_test_rows(&pool).await;
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn explicit_limit_is_honored() {
    let pool = connect_pool().await;
    ensure_swap_activity_table(&pool).await;
    cleanup_test_rows(&pool).await;
    seed_swap_rows(&pool).await;
    let router = make_router(pool.clone()).await;

    let (status, json) = get_swaps(&router, "?limit=2").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["data"]["swaps"].as_array().unwrap().len(), 2);

    cleanup_test_rows(&pool).await;
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn limit_is_capped_at_one_hundred() {
    let pool = connect_pool().await;
    ensure_swap_activity_table(&pool).await;
    cleanup_test_rows(&pool).await;
    seed_swap_rows(&pool).await;
    let router = make_router(pool.clone()).await;

    let (status, json) = get_swaps(&router, "?limit=500").await;
    assert_eq!(status, StatusCode::OK);
    assert!(json["data"]["swaps"].as_array().unwrap().len() <= 100);

    cleanup_test_rows(&pool).await;
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn zero_or_negative_limit_clamps_to_one() {
    let pool = connect_pool().await;
    ensure_swap_activity_table(&pool).await;
    cleanup_test_rows(&pool).await;
    seed_swap_rows(&pool).await;
    let router = make_router(pool.clone()).await;

    let (status, json) = get_swaps(&router, "?limit=0").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["data"]["swaps"].as_array().unwrap().len(), 1);

    cleanup_test_rows(&pool).await;
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn before_ledger_filters_older_entries() {
    let pool = connect_pool().await;
    ensure_swap_activity_table(&pool).await;
    cleanup_test_rows(&pool).await;
    seed_swap_rows(&pool).await;
    let router = make_router(pool.clone()).await;

    let (status, json) = get_swaps(&router, "?before_ledger=90").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(swap_ledgers(&json), vec![80, 70]);

    cleanup_test_rows(&pool).await;
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn cursor_pagination_returns_non_overlapping_pages() {
    let pool = connect_pool().await;
    ensure_swap_activity_table(&pool).await;
    cleanup_test_rows(&pool).await;
    seed_swap_rows(&pool).await;
    let router = make_router(pool.clone()).await;

    let (_, page1) = get_swaps(&router, "?limit=2").await;
    let page1_ids = swap_event_ids(&page1);
    let min_ledger = swap_ledgers(&page1).last().copied().unwrap();

    let (_, page2) = get_swaps(&router, &format!("?limit=2&before_ledger={min_ledger}")).await;
    let page2_ids = swap_event_ids(&page2);

    for id in &page2_ids {
        assert!(!page1_ids.contains(id), "pages must not overlap");
    }

    cleanup_test_rows(&pool).await;
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn empty_result_set_returns_200_with_empty_array() {
    let pool = connect_pool().await;
    ensure_swap_activity_table(&pool).await;
    cleanup_test_rows(&pool).await;
    let router = make_router(pool.clone()).await;

    let (status, json) = get_swaps(&router, "?before_ledger=1").await;
    assert_eq!(status, StatusCode::OK);
    assert!(json["data"]["swaps"].as_array().unwrap().is_empty());

    cleanup_test_rows(&pool).await;
}
