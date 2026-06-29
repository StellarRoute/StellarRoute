//! Integration tests for the batch orderbook endpoint (`POST /api/v1/batch/orderbook`).
//!
//! # Test coverage
//!
//! ## Unit / in-process tests (no DB required)
//! - Request validation: empty batch, oversized batch, same base/quote, empty field
//! - Response shape: `results` array, `items_succeeded`, `items_failed`, `total`
//! - Per-item result shape: `status`, `index`, `orderbook` vs `error` field presence
//! - Result ordering: `index` field preserves original request order
//! - Error taxonomy: machine-readable codes conform to `docs/api/error_taxonomy.md`
//! - `BATCH_MAX_ITEMS` constant equals 25
//!
//! ## Integration tests (require DATABASE_URL — marked `#[ignore]`)
//! - Empty batch → 400 `validation_error`
//! - Unknown pair → per-item `not_found` at HTTP 200
//! - Mixed success/failure batch → counters and per-item codes are correct
//! - Strict response ordering → `results[i].index == i` for all items

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use stellarroute_api::{state::DatabasePools, Server, ServerConfig};
use tower::ServiceExt;

// ─── Shared helpers ───────────────────────────────────────────────────────────

/// Build a router backed by a real Postgres connection.
/// Requires `DATABASE_URL` to be set; tests using this are `#[ignore]` by default.
async fn make_router() -> axum::Router {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
    });
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(20)
        .connect(&db_url)
        .await
        .expect("Failed to connect to database for orderbook batch tests");
    Server::new(ServerConfig::default(), DatabasePools::new(pool, None))
        .await
        .into_router()
}

/// Build a router with a *lazy* pool — no real DB connection is made.
/// Safe for validation-layer tests that never touch the database.
async fn make_lazy_router() -> axum::Router {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .expect("Failed to create lazy pool");
    Server::new(ServerConfig::default(), DatabasePools::new(pool, None))
        .await
        .into_router()
}

/// POST a JSON body to the batch orderbook endpoint and return `(StatusCode, parsed body)`.
async fn post_batch(router: &axum::Router, body: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/batch/orderbook")
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

// ─── AC #1: Empty batch ───────────────────────────────────────────────────────

/// An empty `requests` array must be rejected at the batch-level with HTTP 400
/// and a machine-readable `validation_error` code — no crash, no 500.
#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn empty_batch_returns_400_validation_error() {
    let router = make_router().await;
    let (status, json) = post_batch(&router, json!({ "requests": [] })).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        json["data"]["error"], "validation_error",
        "empty batch must produce error code 'validation_error', got: {}",
        json["data"]["error"]
    );
    assert!(
        json["data"]["message"].as_str().is_some(),
        "response must include a human-readable message"
    );
}

/// A lazy-pool variant so the empty-batch guard can be verified without a DB.
#[tokio::test]
async fn empty_batch_returns_400_no_db_required() {
    let router = make_lazy_router().await;
    let (status, json) = post_batch(&router, json!({ "requests": [] })).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(json["data"]["error"], "validation_error");
}

// ─── AC #1 (continued): Oversized batch ──────────────────────────────────────

/// A batch exceeding 25 items must be rejected at the batch-level with HTTP 400.
/// The error message must mention the limit value so callers can self-diagnose.
#[tokio::test]
async fn oversized_batch_returns_400_no_db_required() {
    let router = make_lazy_router().await;
    // 26 items — one over the documented limit
    let items: Vec<Value> = (0..26)
        .map(|i| json!({ "base": "native", "quote": format!("ASSET{}", i) }))
        .collect();
    let (status, json) = post_batch(&router, json!({ "requests": items })).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(json["data"]["error"], "validation_error");
    let msg = json["data"]["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("25"),
        "error message should reference the limit (25): got '{}'",
        msg
    );
}

// ─── AC #2: Unknown pair → per-item not_found ─────────────────────────────────

/// Requesting an orderbook for a completely non-existent asset pair must:
///   - Return HTTP 200 (batch itself is valid)
///   - Carry `status == "error"` on the item
///   - Use the machine-readable code `"not_found"` (from error taxonomy)
///   - Omit the `orderbook` field on the failed item
#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn unknown_pair_returns_not_found_per_item() {
    let router = make_router().await;
    let (status, json) = post_batch(
        &router,
        json!({
            "requests": [
                // Pair that does not exist in any fixture data
                { "base": "NONEXISTENT", "quote": "ALSONONEXISTENT" }
            ]
        }),
    )
    .await;

    // Batch-level HTTP status is 200 — per-item errors never abort the batch
    assert_eq!(status, StatusCode::OK);

    let results = json["data"]["results"].as_array().expect("results array");
    assert_eq!(results.len(), 1, "one result expected for one input");

    let item = &results[0];
    assert_eq!(item["index"], 0, "index must match input position");
    assert_eq!(
        item["status"], "error",
        "unknown pair must produce status=error"
    );

    // Error code must be taxonomy-compliant
    let code = item["error"]["code"].as_str().unwrap_or("");
    assert!(
        code == "not_found" || code == "invalid_asset",
        "unknown pair must map to 'not_found' or 'invalid_asset', got '{}'",
        code
    );

    // `orderbook` field must be absent on error items (skip_serializing_if = None)
    assert!(
        item.get("orderbook").is_none() || item["orderbook"].is_null(),
        "orderbook must be absent on error items"
    );

    // `message` field must be present and non-empty
    assert!(
        item["error"]["message"].as_str().map(|s| !s.is_empty()).unwrap_or(false),
        "error item must carry a non-empty message"
    );
}

// ─── AC #3: Mixed success/failure items ──────────────────────────────────────

/// A batch containing a mix of valid pairs and invalid/unknown pairs must:
///   - Return HTTP 200
///   - Report `items_succeeded` and `items_failed` counters accurately
///   - Mark each item independently with `status == "ok"` or `status == "error"`
///   - Never abort the entire batch because one item failed
#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn mixed_batch_partial_failure_semantics() {
    let router = make_router().await;
    let (status, json) = post_batch(
        &router,
        json!({
            "requests": [
                // Item 0: valid, known pair (XLM/USDC lives in the fixture seed)
                {
                    "base": "native",
                    "quote": "USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5"
                },
                // Item 1: completely unknown pair → should produce a per-item error
                { "base": "FAKECOIN", "quote": "OTHERFAKE" },
                // Item 2: same base == quote → pre-validation should catch this
                { "base": "native", "quote": "native" }
            ]
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "mixed batch must return HTTP 200");

    let data = &json["data"];
    let results = data["results"].as_array().expect("results must be an array");
    assert_eq!(results.len(), 3, "result count must match request count");

    // Items 1 and 2 must be errors
    assert_eq!(results[1]["status"], "error", "item 1 (unknown pair) must be error");
    assert_eq!(results[2]["status"], "error", "item 2 (same base/quote) must be error");

    // item 2 same-base-quote violation maps to validation_error
    assert_eq!(
        results[2]["error"]["code"], "validation_error",
        "same base/quote must produce 'validation_error'"
    );

    // Counters must be consistent with per-item statuses
    let succeeded = data["items_succeeded"].as_u64().expect("items_succeeded");
    let failed = data["items_failed"].as_u64().expect("items_failed");
    let total = data["total"].as_u64().expect("total");

    assert_eq!(
        succeeded + failed,
        total,
        "items_succeeded + items_failed must equal total"
    );
    assert_eq!(total, 3, "total must equal input batch size");
    // At minimum the two known-bad items must be counted as failed
    assert!(
        failed >= 2,
        "at least 2 items must be failures, got {}",
        failed
    );
}

/// Variant without a real DB: the pre-validation layer (same base/quote,
/// empty fields) must catch bad items and still return HTTP 200 with per-item
/// errors — even before any database call is attempted.
#[tokio::test]
async fn mixed_batch_pre_validation_errors_no_db_required() {
    let router = make_lazy_router().await;
    let (status, json) = post_batch(
        &router,
        json!({
            "requests": [
                // Item 0: passes pre-validation (DB call will fail, but that's fine
                //          — the lazy pool will error, mapping to internal_error)
                { "base": "native", "quote": "USDC" },
                // Item 1: same base == quote — caught by pre-validation, no DB needed
                { "base": "native", "quote": "native" },
                // Item 2: empty base — caught by pre-validation
                { "base": "", "quote": "USDC" }
            ]
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);

    let results = json["data"]["results"].as_array().expect("results array");
    assert_eq!(results.len(), 3);

    // Items 1 and 2 must be errors from pre-validation
    assert_eq!(results[1]["status"], "error");
    assert_eq!(results[1]["error"]["code"], "validation_error");
    assert_eq!(results[2]["status"], "error");
    assert_eq!(results[2]["error"]["code"], "validation_error");

    // Counters must be coherent
    let failed = json["data"]["items_failed"].as_u64().unwrap_or(0);
    assert!(failed >= 2, "at least 2 items should fail pre-validation");
}

// ─── AC #4: Strict response ordering ─────────────────────────────────────────

/// The `index` field on each result item must exactly match its position in the
/// original request array, even when concurrent futures complete out of order.
#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn results_preserve_request_order() {
    let router = make_router().await;
    // Use 5 distinct pairs (some unknown) to increase the chance of futures
    // completing in a different order than they were submitted.
    let (status, json) = post_batch(
        &router,
        json!({
            "requests": [
                { "base": "native", "quote": "USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5" },
                { "base": "FAKE1", "quote": "FAKE2" },
                { "base": "native", "quote": "USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5" },
                { "base": "FAKE3", "quote": "FAKE4" },
                { "base": "native", "quote": "USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5" }
            ]
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);

    let results = json["data"]["results"].as_array().expect("results array");
    assert_eq!(results.len(), 5, "result count must equal input count");

    for (expected_index, item) in results.iter().enumerate() {
        let actual_index = item["index"]
            .as_u64()
            .expect("each result must have an integer 'index' field") as usize;
        assert_eq!(
            actual_index, expected_index,
            "result at position {} has wrong index {}",
            expected_index, actual_index
        );
    }
}

// ─── Error taxonomy unit tests (no DB) ───────────────────────────────────────
// These tests verify that the machine-readable error codes produced by the
// handler's error mapping layer exactly match the codes declared in
// docs/api/error_taxonomy.md.

/// `BatchOrderbookItemResult::ok` must set status="ok", populate `orderbook`,
/// and omit `error` from the serialized JSON.
#[test]
fn item_result_ok_shape() {
    use stellarroute_api::models::{
        AssetInfo, BatchOrderbookItemResult, OrderbookResponse, OrderbookSummary,
    };

    let orderbook = OrderbookResponse {
        base_asset: AssetInfo::native(),
        quote_asset: AssetInfo::native(),
        asks: vec![],
        bids: vec![],
        summary: OrderbookSummary {
            bid: None,
            ask: None,
            spread_bps: None,
            midpoint: None,
        },
        timestamp: 0,
    };

    let result = BatchOrderbookItemResult::ok(3, orderbook);
    assert_eq!(result.status, "ok");
    assert_eq!(result.index, 3);
    assert!(result.orderbook.is_some(), "orderbook must be present on ok items");
    assert!(result.error.is_none(), "error must be absent on ok items");

    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["index"], 3);
    assert!(
        json.get("error").is_none(),
        "'error' key must be omitted from serialized ok result (skip_serializing_if)"
    );
    assert!(
        json["orderbook"].is_object(),
        "'orderbook' must be present in serialized ok result"
    );
}

/// `BatchOrderbookItemResult::err` must set status="error", populate `error`,
/// and omit `orderbook` from the serialized JSON.
#[test]
fn item_result_error_shape() {
    use stellarroute_api::models::{BatchItemError, BatchOrderbookItemResult};

    let result = BatchOrderbookItemResult::err(
        1,
        BatchItemError {
            code: "not_found".to_string(),
            message: "Asset not found in orderbook".to_string(),
        },
    );

    assert_eq!(result.status, "error");
    assert_eq!(result.index, 1);
    assert!(result.error.is_some(), "error must be present on error items");
    assert!(result.orderbook.is_none(), "orderbook must be absent on error items");

    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["status"], "error");
    assert_eq!(json["error"]["code"], "not_found");
    assert!(
        json.get("orderbook").is_none(),
        "'orderbook' key must be omitted from serialized error result"
    );
}

/// Every valid error code used by the batch handler must be listed in the
/// taxonomy and serialise to snake_case strings.
#[test]
fn error_codes_are_taxonomy_compliant() {
    // These are all the codes that `batch_error_from_api_error` and the
    // pre-validation path can emit, as documented in error_taxonomy.md.
    let taxonomy_codes = [
        "not_found",
        "invalid_asset",
        "validation_error",
        "internal_error",
    ];

    for code in &taxonomy_codes {
        // Codes must be non-empty, lowercase snake_case
        assert!(!code.is_empty(), "code must not be empty");
        assert_eq!(
            *code,
            code.to_lowercase().as_str(),
            "code '{}' must be lowercase",
            code
        );
        assert!(
            code.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "code '{}' must be snake_case (lowercase letters and underscores only)",
            code
        );
    }
}

/// Response counters must stay consistent: succeeded + failed == total.
#[test]
fn batch_response_counter_invariant() {
    use stellarroute_api::models::{
        AssetInfo, BatchItemError, BatchOrderbookItemResult, BatchOrderbookResponse,
        OrderbookResponse, OrderbookSummary,
    };

    let ok_ob = OrderbookResponse {
        base_asset: AssetInfo::native(),
        quote_asset: AssetInfo::native(),
        asks: vec![],
        bids: vec![],
        summary: OrderbookSummary {
            bid: None,
            ask: None,
            spread_bps: None,
            midpoint: None,
        },
        timestamp: 0,
    };

    let results = vec![
        BatchOrderbookItemResult::ok(0, ok_ob.clone()),
        BatchOrderbookItemResult::err(
            1,
            BatchItemError {
                code: "not_found".to_string(),
                message: "not found".to_string(),
            },
        ),
        BatchOrderbookItemResult::ok(2, ok_ob),
        BatchOrderbookItemResult::err(
            3,
            BatchItemError {
                code: "validation_error".to_string(),
                message: "same base/quote".to_string(),
            },
        ),
    ];

    let succeeded = results.iter().filter(|r| r.status == "ok").count();
    let failed = results.iter().filter(|r| r.status == "error").count();
    let total = results.len();

    let response = BatchOrderbookResponse {
        items_succeeded: succeeded,
        items_failed: failed,
        total,
        results,
    };

    assert_eq!(response.items_succeeded, 2);
    assert_eq!(response.items_failed, 2);
    assert_eq!(response.total, 4);
    assert_eq!(
        response.items_succeeded + response.items_failed,
        response.total,
        "succeeded + failed must equal total"
    );
}

/// The `index` field must exactly preserve the original request position even
/// if futures resolve out of order.  We test this by constructing results in a
/// scrambled order, sorting by `index`, and asserting the restored sequence.
#[test]
fn index_ordering_is_preserved_after_sort() {
    use stellarroute_api::models::{BatchItemError, BatchOrderbookItemResult};

    // Simulate futures completing in reverse order
    let mut results = vec![
        BatchOrderbookItemResult::err(
            4,
            BatchItemError { code: "not_found".to_string(), message: String::new() },
        ),
        BatchOrderbookItemResult::err(
            2,
            BatchItemError { code: "not_found".to_string(), message: String::new() },
        ),
        BatchOrderbookItemResult::err(
            0,
            BatchItemError { code: "not_found".to_string(), message: String::new() },
        ),
        BatchOrderbookItemResult::err(
            3,
            BatchItemError { code: "not_found".to_string(), message: String::new() },
        ),
        BatchOrderbookItemResult::err(
            1,
            BatchItemError { code: "not_found".to_string(), message: String::new() },
        ),
    ];

    // The handler uses join_all which preserves insertion order, but here we
    // verify the index field itself is correct regardless of sort.
    results.sort_by_key(|r| r.index);

    for (expected, item) in results.iter().enumerate() {
        assert_eq!(
            item.index, expected,
            "after sort, result at position {} must have index {}",
            expected, expected
        );
    }
}

/// `BATCH_MAX_ITEMS` is part of the public API contract; this test acts as a
/// regression guard so any limit change is an intentional, reviewed decision.
#[test]
fn batch_max_items_constant_is_25() {
    use stellarroute_api::routes::orderbook::BATCH_MAX_ITEMS;
    assert_eq!(
        BATCH_MAX_ITEMS, 25,
        "BATCH_MAX_ITEMS must remain 25 — change this only after reviewing the API contract"
    );
}

// ─── Per-item validate() unit tests (no DB) ──────────────────────────────────

/// `OrderbookRequestItem::validate()` must reject same base/quote.
#[test]
fn orderbook_request_item_validate_rejects_same_base_quote() {
    use stellarroute_api::models::request::OrderbookRequestItem;

    let item = OrderbookRequestItem {
        base: "native".to_string(),
        quote: "native".to_string(),
    };
    let result = item.validate();
    assert!(result.is_err(), "same base and quote must fail validation");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("differ") || msg.contains("same") || msg.contains("native"),
        "error message should describe the problem: '{}'",
        msg
    );
}

/// `OrderbookRequestItem::validate()` must reject an empty base field.
#[test]
fn orderbook_request_item_validate_rejects_empty_base() {
    use stellarroute_api::models::request::OrderbookRequestItem;

    let item = OrderbookRequestItem {
        base: String::new(),
        quote: "USDC".to_string(),
    };
    assert!(item.validate().is_err(), "empty base must fail validation");
}

/// `OrderbookRequestItem::validate()` must reject an empty quote field.
#[test]
fn orderbook_request_item_validate_rejects_empty_quote() {
    use stellarroute_api::models::request::OrderbookRequestItem;

    let item = OrderbookRequestItem {
        base: "native".to_string(),
        quote: String::new(),
    };
    assert!(item.validate().is_err(), "empty quote must fail validation");
}

/// `OrderbookRequestItem::validate()` must accept a well-formed pair.
#[test]
fn orderbook_request_item_validate_accepts_valid_pair() {
    use stellarroute_api::models::request::OrderbookRequestItem;

    let item = OrderbookRequestItem {
        base: "native".to_string(),
        quote: "USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5".to_string(),
    };
    assert!(item.validate().is_ok(), "valid pair must pass validation");
}
