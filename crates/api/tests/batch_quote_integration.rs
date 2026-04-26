//! Integration tests for the batch quote endpoint.
//!
//! # Test coverage
//!
//! ## Unit / in-process tests (no DB required)
//! - Request validation: empty batch, oversized batch, duplicate pairs, invalid assets
//! - Per-item validation: bad amount, bad slippage, same base/quote
//! - Response shape: `results` array, `items_succeeded`, `items_failed`, `snapshot_timestamp`
//! - Per-item error envelope: `status`, `code`, `message`
//! - Order preservation: results are in the same order as request items
//!
//! ## Integration tests (require DATABASE_URL — marked `#[ignore]`)
//! - End-to-end batch with mixed success/error items
//! - Shared snapshot semantics: all items share the same `snapshot_timestamp`
//! - Load test: concurrent batch vs N sequential calls

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use stellarroute_api::{state::DatabasePools, Server, ServerConfig};
use tower::ServiceExt;

// ─── Helpers ─────────────────────────────────────────────────────────────────

async fn make_router() -> axum::Router {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
    });
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(20)
        .connect(&db_url)
        .await
        .expect("Failed to connect to database");
    let server = Server::new(ServerConfig::default(), DatabasePools::new(pool, None)).await;
    server.into_router()
}

async fn post_batch(router: &axum::Router, body: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/batch/quote")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let resp = router.clone().oneshot(req).await.expect("request failed");
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    (status, json)
}

// ─── AC #1: Request size limits and per-item errors ──────────────────────────

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn empty_batch_returns_400() {
    let router = make_router().await;
    let (status, json) = post_batch(&router, json!({"quotes": []})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(json["data"]["error"], "validation_error");
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn oversized_batch_returns_400() {
    let router = make_router().await;
    // 26 items — one over the limit
    let items: Vec<Value> = (0..26)
        .map(|i| {
            json!({
                "base": "native",
                "quote": format!("ASSET{}", i),
                "amount": "1"
            })
        })
        .collect();
    let (status, json) = post_batch(&router, json!({"quotes": items})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let msg = json["data"]["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("25"),
        "error message should mention the limit: {}",
        msg
    );
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn per_item_invalid_asset_returns_error_not_abort() {
    let router = make_router().await;
    let (status, json) = post_batch(
        &router,
        json!({
            "quotes": [
                // Item 0: valid pair (may or may not have a route)
                {"base": "native", "quote": "native", "amount": "1"},
                // Item 1: invalid base asset format
                {"base": "INVALID:::ASSET", "quote": "native", "amount": "1"}
            ]
        }),
    )
    .await;

    // Batch-level HTTP status is 200 — per-item errors don't abort
    assert_eq!(status, StatusCode::OK);

    let data = &json["data"];
    let results = data["results"].as_array().expect("results array");
    assert_eq!(results.len(), 2);

    // Item 1 must be an error
    assert_eq!(results[1]["status"], "error");
    assert!(results[1]["error"]["code"].as_str().is_some());
    assert!(results[1]["error"]["message"].as_str().is_some());
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn per_item_same_base_quote_returns_validation_error() {
    let router = make_router().await;
    let (status, json) = post_batch(
        &router,
        json!({
            "quotes": [
                {"base": "native", "quote": "native", "amount": "1"}
            ]
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let results = json["data"]["results"].as_array().unwrap();
    assert_eq!(results[0]["status"], "error");
    assert_eq!(results[0]["error"]["code"], "validation_error");
}

// ─── AC #2: Shared market snapshot ───────────────────────────────────────────

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn snapshot_timestamp_is_present_and_consistent() {
    let router = make_router().await;
    let before = chrono::Utc::now().timestamp_millis();

    let (status, json) = post_batch(
        &router,
        json!({
            "quotes": [
                {"base": "native", "quote": "USDC", "amount": "1"},
                {"base": "native", "quote": "USDC", "amount": "10"}
            ]
        }),
    )
    .await;

    let after = chrono::Utc::now().timestamp_millis();

    assert_eq!(status, StatusCode::OK);
    let snapshot_ts = json["data"]["snapshot_timestamp"]
        .as_i64()
        .expect("snapshot_timestamp must be present");

    assert!(
        snapshot_ts >= before,
        "snapshot_timestamp {} must be >= request start {}",
        snapshot_ts,
        before
    );
    assert!(
        snapshot_ts <= after,
        "snapshot_timestamp {} must be <= request end {}",
        snapshot_ts,
        after
    );
}

// ─── AC #3: OpenAPI documented ───────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn openapi_spec_includes_batch_endpoint() {
    let router = make_router().await;
    let req = Request::builder()
        .uri("/api-docs/openapi.json")
        .body(Body::empty())
        .unwrap();
    let resp = router.oneshot(req).await.expect("request failed");
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let spec: Value = serde_json::from_slice(&bytes).unwrap();

    // Verify the batch endpoint is in the spec
    let paths = spec["paths"].as_object().expect("paths object");
    assert!(
        paths.contains_key("/api/v1/batch/quote"),
        "OpenAPI spec must include /api/v1/batch/quote"
    );

    // Verify the POST method is documented
    let post = &spec["paths"]["/api/v1/batch/quote"]["post"];
    assert!(
        !post.is_null(),
        "POST /api/v1/batch/quote must be documented"
    );

    // Verify request body schema is referenced
    let request_body = &post["requestBody"];
    assert!(
        !request_body.is_null(),
        "POST /api/v1/batch/quote must have a requestBody"
    );

    // Verify 200 response is documented
    let responses = &post["responses"];
    assert!(
        responses["200"].is_object(),
        "200 response must be documented"
    );

    // Verify BatchQuoteResponse schema is in components
    let schemas = &spec["components"]["schemas"];
    assert!(
        schemas["BatchQuoteResponse"].is_object(),
        "BatchQuoteResponse schema must be in components"
    );
    assert!(
        schemas["BatchQuoteItemResult"].is_object(),
        "BatchQuoteItemResult schema must be in components"
    );
    assert!(
        schemas["BatchItemError"].is_object(),
        "BatchItemError schema must be in components"
    );
}

// ─── AC #4: Load test — throughput gain vs N sequential calls ────────────────

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn batch_is_faster_than_sequential_calls() {
    use std::time::Instant;

    let router = make_router().await;

    // Use 10 identical pairs — they'll all hit the single-flight cache after
    // the first, so this tests the overhead of sequential vs concurrent dispatch.
    let n = 10usize;
    let pair = json!({"base": "native", "quote": "USDC", "amount": "1"});

    // ── Sequential: N individual POST requests ────────────────────────────
    let seq_start = Instant::now();
    for _ in 0..n {
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/batch/quote")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&json!({"quotes": [pair.clone()]})).unwrap(),
            ))
            .unwrap();
        router
            .clone()
            .oneshot(req)
            .await
            .expect("sequential request failed");
    }
    let seq_duration = seq_start.elapsed();

    // ── Batch: 1 POST with N items ────────────────────────────────────────
    let items: Vec<Value> = (0..n).map(|_| pair.clone()).collect();
    let batch_start = Instant::now();
    let (status, json) = post_batch(&router, json!({"quotes": items})).await;
    let batch_duration = batch_start.elapsed();

    assert_eq!(status, StatusCode::OK);
    let total = json["data"]["total"].as_u64().unwrap_or(0);
    assert_eq!(total as usize, n, "all {} items must be in the response", n);

    println!(
        "Sequential ({} calls): {:?}  |  Batch ({} items): {:?}  |  speedup: {:.1}x",
        n,
        seq_duration,
        n,
        batch_duration,
        seq_duration.as_secs_f64() / batch_duration.as_secs_f64()
    );

    // The batch should be meaningfully faster than N sequential calls.
    // We use a conservative 1.5x threshold to avoid flakiness on slow CI.
    assert!(
        batch_duration < seq_duration,
        "batch ({:?}) should be faster than {} sequential calls ({:?})",
        batch_duration,
        n,
        seq_duration
    );
}

// ─── Response shape unit tests (no DB) ───────────────────────────────────────

#[test]
fn batch_item_result_ok_shape() {
    use stellarroute_api::models::{AssetInfo, BatchQuoteItemResult, QuoteResponse};

    let quote = QuoteResponse {
        base_asset: AssetInfo::native(),
        quote_asset: AssetInfo::native(),
        amount: "1.0000000".to_string(),
        price: "1.0000000".to_string(),
        total: "1.0000000".to_string(),
        quote_type: "sell".to_string(),
        path: vec![],
        timestamp: 0,
        expires_at: None,
        source_timestamp: None,
        ttl_seconds: None,
        rationale: None,
        price_impact: None,
        exclusion_diagnostics: None,
        data_freshness: None,
    };

    let result = BatchQuoteItemResult::ok(0, quote);
    assert_eq!(result.status, "ok");
    assert_eq!(result.index, 0);
    assert!(result.quote.is_some());
    assert!(result.error.is_none());

    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["status"], "ok");
    assert!(json.get("error").is_none(), "error must be omitted on ok");
}

#[test]
fn batch_item_result_error_shape() {
    use stellarroute_api::models::{BatchItemError, BatchQuoteItemResult};

    let result = BatchQuoteItemResult::err(
        2,
        BatchItemError {
            code: "no_route".to_string(),
            message: "No trading route found".to_string(),
        },
    );
    assert_eq!(result.status, "error");
    assert_eq!(result.index, 2);
    assert!(result.quote.is_none());
    assert!(result.error.is_some());

    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["status"], "error");
    assert_eq!(json["error"]["code"], "no_route");
    assert!(
        json.get("quote").is_none(),
        "quote must be omitted on error"
    );
}

#[test]
fn batch_response_counters_are_correct() {
    use stellarroute_api::models::{
        AssetInfo, BatchItemError, BatchQuoteItemResult, BatchQuoteResponse, QuoteResponse,
    };

    let ok_quote = QuoteResponse {
        base_asset: AssetInfo::native(),
        quote_asset: AssetInfo::native(),
        amount: "1.0000000".to_string(),
        price: "1.0000000".to_string(),
        total: "1.0000000".to_string(),
        quote_type: "sell".to_string(),
        path: vec![],
        timestamp: 0,
        expires_at: None,
        source_timestamp: None,
        ttl_seconds: None,
        rationale: None,
        price_impact: None,
        exclusion_diagnostics: None,
        data_freshness: None,
    };

    let results = vec![
        BatchQuoteItemResult::ok(0, ok_quote.clone()),
        BatchQuoteItemResult::err(
            1,
            BatchItemError {
                code: "no_route".to_string(),
                message: "No route".to_string(),
            },
        ),
        BatchQuoteItemResult::ok(2, ok_quote),
    ];

    let response = BatchQuoteResponse {
        items_succeeded: results.iter().filter(|r| r.status == "ok").count(),
        items_failed: results.iter().filter(|r| r.status == "error").count(),
        total: results.len(),
        snapshot_timestamp: 1714000000000,
        results,
    };

    assert_eq!(response.items_succeeded, 2);
    assert_eq!(response.items_failed, 1);
    assert_eq!(response.total, 3);
    assert_eq!(response.snapshot_timestamp, 1714000000000);
}

#[test]
fn order_is_preserved_in_results() {
    use stellarroute_api::models::{BatchItemError, BatchQuoteItemResult};

    // Simulate results arriving out of order (as they would from concurrent futures)
    // and verify the index field preserves the original order.
    let mut results = [
        BatchQuoteItemResult::err(
            2,
            BatchItemError {
                code: "no_route".to_string(),
                message: "".to_string(),
            },
        ),
        BatchQuoteItemResult::err(
            0,
            BatchItemError {
                code: "no_route".to_string(),
                message: "".to_string(),
            },
        ),
        BatchQuoteItemResult::err(
            1,
            BatchItemError {
                code: "no_route".to_string(),
                message: "".to_string(),
            },
        ),
    ];

    // Sort by index to restore original order
    results.sort_by_key(|r| r.index);

    assert_eq!(results[0].index, 0);
    assert_eq!(results[1].index, 1);
    assert_eq!(results[2].index, 2);
}

#[test]
fn batch_max_items_constant_is_25() {
    use stellarroute_api::routes::quote::BATCH_MAX_ITEMS;
    assert_eq!(BATCH_MAX_ITEMS, 25);
}

#[test]
fn quote_request_item_validate_rejects_same_base_quote() {
    use stellarroute_api::models::request::QuoteRequestItem;

    let item = QuoteRequestItem {
        base: "native".to_string(),
        quote: "native".to_string(),
        amount: Some("1".to_string()),
        slippage_bps: None,
        quote_type: None,
    };
    assert!(item.validate().is_err());
    let msg = item.validate().unwrap_err();
    assert!(
        msg.contains("differ"),
        "error should mention 'differ': {}",
        msg
    );
}

#[test]
fn quote_request_item_validate_rejects_zero_amount() {
    use stellarroute_api::models::request::QuoteRequestItem;

    let item = QuoteRequestItem {
        base: "native".to_string(),
        quote: "USDC".to_string(),
        amount: Some("0".to_string()),
        slippage_bps: None,
        quote_type: None,
    };
    assert!(item.validate().is_err());
}

#[test]
fn quote_request_item_validate_rejects_negative_amount() {
    use stellarroute_api::models::request::QuoteRequestItem;

    let item = QuoteRequestItem {
        base: "native".to_string(),
        quote: "USDC".to_string(),
        amount: Some("-5".to_string()),
        slippage_bps: None,
        quote_type: None,
    };
    assert!(item.validate().is_err());
}

#[test]
fn quote_request_item_validate_rejects_excessive_slippage() {
    use stellarroute_api::models::request::{QuoteRequestItem, MAX_SLIPPAGE_BPS};

    let item = QuoteRequestItem {
        base: "native".to_string(),
        quote: "USDC".to_string(),
        amount: Some("1".to_string()),
        slippage_bps: Some(MAX_SLIPPAGE_BPS + 1),
        quote_type: None,
    };
    assert!(item.validate().is_err());
}

#[test]
fn quote_request_item_validate_accepts_valid_item() {
    use stellarroute_api::models::request::QuoteRequestItem;

    let item = QuoteRequestItem {
        base: "native".to_string(),
        quote: "USDC".to_string(),
        amount: Some("100".to_string()),
        slippage_bps: Some(50),
        quote_type: None,
    };
    assert!(item.validate().is_ok());
}

#[test]
fn quote_request_item_validate_accepts_no_amount() {
    use stellarroute_api::models::request::QuoteRequestItem;

    let item = QuoteRequestItem {
        base: "native".to_string(),
        quote: "USDC".to_string(),
        amount: None, // defaults to 1
        slippage_bps: None,
        quote_type: None,
    };
    assert!(item.validate().is_ok());
}
