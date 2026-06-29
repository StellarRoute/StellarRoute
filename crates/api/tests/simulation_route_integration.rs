//! Integration tests for POST /api/v1/simulate/route
//!
//! Unit tests (no DB) validate request/response model shapes and the handler's
//! input-validation logic by driving the axum router directly via
//! `tower::ServiceExt::oneshot`.
//!
//! The live endpoint tests depend on a live PostgreSQL instance (and a fully
//! migrated schema) and are marked `#[ignore]`.  Run them with:
//!
//! ```sh
//! DATABASE_URL=postgres://stellarroute:stellarroute_dev@localhost:5432/stellarroute \
//!   cargo test -p stellarroute-api --test simulation_route_integration -- --ignored
//! ```

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use stellarroute_api::{state::DatabasePools, Server, ServerConfig};
use tower::ServiceExt; // for `oneshot`

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a minimal, dependency-free router backed by a null DB pool.
///
/// This is safe for the four unit-style tests below because those tests only
/// reach the handler's early-return validation branches — no SQL queries are
/// ever executed.
async fn build_router_no_db() -> axum::Router {
    // We need *a* pool object to satisfy `DatabasePools::new`, but no query
    // will actually be sent for the validation-only paths exercised here.
    // Provide a deliberately invalid URL; the pool is lazy and the connection
    // is never opened unless a query runs.
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        // Avoid blocking: if the pool is accidentally hit the test will
        // receive an internal error rather than hang.
        .acquire_timeout(std::time::Duration::from_millis(100))
        .connect_lazy("postgres://invalid:invalid@localhost:0/invalid")
        .expect("lazy pool construction must not fail");

    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        redis_url: None,
        admin_auth_token: None,
        quote_cache_ttl_seconds: 2,
    };

    Server::new(config, DatabasePools::new(pool, None))
        .await
        .into_router()
}

/// POST a JSON body to `/api/v1/simulate/route` and return (status, body).
async fn post_simulate(router: axum::Router, body: Value) -> (StatusCode, Value) {
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/simulate/route")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .expect("oneshot request failed");

    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("failed to read response body");
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);

    (status, json)
}

// ---------------------------------------------------------------------------
// Test 1 — Empty hops array is rejected with 400 (no DB required)
// ---------------------------------------------------------------------------

/// The endpoint must return HTTP 400 with a `validation_error` code when the
/// caller supplies an empty `route.hops` array.
///
/// This exercises the guard:
///   `if body.route.hops.is_empty() { return Err(ApiError::Validation(…)) }`
#[tokio::test]
async fn empty_hops_returns_400_validation_error() {
    let router = build_router_no_db().await;

    let body = json!({
        "route": { "hops": [] },
        "amount": "100"
    });

    let (status, json) = post_simulate(router, body).await;

    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "empty hops must yield 400; body: {json}"
    );

    // The response is wrapped in ApiResponse: { data: { error, message } }
    let data = &json["data"];
    assert_eq!(
        data["error"], "validation_error",
        "error code must be validation_error; got: {data}"
    );
    assert!(
        data["message"]
            .as_str()
            .map(|m| m.contains("hops"))
            .unwrap_or(false),
        "error message must mention 'hops'; got: {data}"
    );
}

// ---------------------------------------------------------------------------
// Test 2 — Non-contiguous hop chain is rejected with 400 (no DB required)
// ---------------------------------------------------------------------------

/// When the hop chain has a gap (hop[i].to_asset ≠ hop[i+1].from_asset) the
/// endpoint must return HTTP 400 with a `validation_error` code explaining
/// which hop pair is broken.
///
/// This exercises the continuity guard loop inside the handler.
#[tokio::test]
async fn non_contiguous_hops_returns_400_with_descriptive_error() {
    let router = build_router_no_db().await;

    // hop[0]: XLM → USDC
    // hop[1]: BTC  → EUR   (gap: from_asset BTC ≠ previous to_asset USDC)
    let body = json!({
        "route": {
            "hops": [
                {
                    "from_asset": { "asset_code": "native" },
                    "to_asset":   { "asset_code": "USDC" },
                    "source":     "sdex"
                },
                {
                    "from_asset": { "asset_code": "BTC" },
                    "to_asset":   { "asset_code": "EUR" },
                    "source":     "sdex"
                }
            ]
        },
        "amount": "50"
    });

    let (status, json) = post_simulate(router, body).await;

    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "non-contiguous hops must yield 400; body: {json}"
    );

    let data = &json["data"];
    assert_eq!(
        data["error"], "validation_error",
        "error code must be validation_error; got: {data}"
    );

    let message = data["message"].as_str().unwrap_or("");
    assert!(
        message.contains("contiguous") || message.contains("hop"),
        "error message must explain the hop gap; got: '{message}'"
    );
}

// ---------------------------------------------------------------------------
// Test 3 — Happy-path 2-hop fixture produces diagnostic parity (live DB)
// ---------------------------------------------------------------------------

/// Sends a valid 2-hop route (XLM → USDC → BTC) to the endpoint and verifies
/// that the response shape mirrors what the quote pipeline returns:
///   - HTTP 200
///   - `data.quote.base_asset` / `data.quote.quote_asset` present
///   - `data.quote.amount` echoes the request amount
///   - `data.quote.path` contains exactly 2 hops
///   - `data.quote.total` is a positive-numeric string
///   - Response is wrapped in the standard `ApiResponse` envelope (v, timestamp, request_id)
///
/// This test requires a running PostgreSQL instance with fully migrated schema.
/// Run with: `cargo test -p stellarroute-api --test simulation_route_integration -- --ignored`
#[tokio::test]
#[ignore = "requires a running PostgreSQL instance and fully migrated schema (set DATABASE_URL)"]
async fn two_hop_happy_path_returns_200_with_quote_diagnostics() {
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
        admin_auth_token: None,
        quote_cache_ttl_seconds: 2,
    };

    let router = Server::new(config, DatabasePools::new(pool, None))
        .await
        .into_router();

    // A valid contiguous 2-hop chain: XLM → USDC → BTC
    let body = json!({
        "route": {
            "hops": [
                {
                    "from_asset": { "asset_code": "native" },
                    "to_asset":   { "asset_code": "USDC" },
                    "source":     "sdex",
                    "fee_bps":    30,
                    "venue_ref":  "sdex-xlm-usdc"
                },
                {
                    "from_asset": { "asset_code": "USDC" },
                    "to_asset":   { "asset_code": "BTC" },
                    "source":     "amm:pool-usdc-btc",
                    "fee_bps":    30,
                    "venue_ref":  "amm-usdc-btc"
                }
            ]
        },
        "amount": "100",
        "slippage_bps": 50
    });

    let (status, json) = post_simulate(router, body).await;

    assert_eq!(
        status,
        StatusCode::OK,
        "valid 2-hop fixture must return 200; body: {json}"
    );

    // ── Envelope shape (ApiResponse) ────────────────────────────────────
    assert_eq!(json["v"], 1, "response must carry version field v=1");
    assert!(
        json["timestamp"].as_i64().is_some(),
        "response must carry a numeric timestamp"
    );
    assert!(
        json["request_id"].as_str().is_some(),
        "response must carry a request_id"
    );

    let data = &json["data"];

    // ── RouteDryRunResponse.quote shape ──────────────────────────────────
    let quote = &data["quote"];
    assert!(
        quote.get("base_asset").is_some(),
        "quote must expose base_asset"
    );
    assert!(
        quote.get("quote_asset").is_some(),
        "quote must expose quote_asset"
    );
    assert_eq!(
        quote["amount"].as_str().unwrap_or(""),
        "100.0000000",
        "quote amount must echo the request amount"
    );
    assert_eq!(
        quote["quote_type"].as_str().unwrap_or(""),
        "sell",
        "dry-run must default to sell direction"
    );

    // path must contain both hops
    let path = quote["path"].as_array().expect("quote.path must be an array");
    assert_eq!(
        path.len(),
        2,
        "2-hop route must produce exactly 2 path steps; got: {path:?}"
    );

    // total must be a parseable positive number
    let total: f64 = quote["total"]
        .as_str()
        .unwrap_or("0")
        .parse()
        .expect("quote.total must be a numeric string");
    assert!(
        total > 0.0,
        "quote.total must be > 0 for a valid 2-hop route"
    );

    // ── Diagnostic parity check ───────────────────────────────────────────
    // Each path step must carry the same fields the quote pipeline exposes.
    for (i, step) in path.iter().enumerate() {
        assert!(
            step.get("from_asset").is_some(),
            "path[{i}] must have from_asset"
        );
        assert!(
            step.get("to_asset").is_some(),
            "path[{i}] must have to_asset"
        );
        assert!(
            step.get("price").is_some(),
            "path[{i}] must have price (diagnostic parity with quote pipeline)"
        );
        assert!(
            step.get("source").is_some(),
            "path[{i}] must have source"
        );
    }
}

// ---------------------------------------------------------------------------
// Test 4 — Per-hop slippage override map is accepted and reflected (live DB)
// ---------------------------------------------------------------------------

/// Verifies that a request with `slippage_bps_overrides` for each hop is
/// accepted and that the response is structurally sound, confirming that the
/// override map does not corrupt the hop chain or produce unexpected errors.
///
/// Specific per-hop override semantics are validated at the unit level in
/// `simulation_route.rs`; this test validates the HTTP boundary behaviour.
///
/// Requires a running PostgreSQL instance with fully migrated schema.
/// Run with: `cargo test -p stellarroute-api --test simulation_route_integration -- --ignored`
#[tokio::test]
#[ignore = "requires a running PostgreSQL instance and fully migrated schema (set DATABASE_URL)"]
async fn slippage_override_map_is_applied_and_accepted() {
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
        admin_auth_token: None,
        quote_cache_ttl_seconds: 2,
    };

    let router = Server::new(config, DatabasePools::new(pool, None))
        .await
        .into_router();

    // Single-hop XLM → USDC with a tight per-hop slippage override (10 bps).
    // The override map must be applied without the handler returning a
    // validation error; response must be 200 or 404 (no route), never 400/500.
    let body = json!({
        "route": {
            "hops": [
                {
                    "from_asset": { "asset_code": "native" },
                    "to_asset":   { "asset_code": "USDC" },
                    "source":     "sdex",
                    "fee_bps":    30,
                    "venue_ref":  "sdex-xlm-usdc"
                }
            ]
        },
        "amount": "10",
        "slippage_bps": 50,
        // Per-hop override: tighten slippage to 10 bps for this venue
        "slippage_bps_overrides": [
            { "venue_ref": "sdex-xlm-usdc", "slippage_bps": 10 }
        ]
    });

    let (status, json) = post_simulate(router, body).await;

    // 200 (route found) or 404 (no live liquidity for this pair in CI) are
    // both valid outcomes.  What must never happen is a 400 or 5xx — those
    // would indicate the override map itself caused a fault.
    assert!(
        status == StatusCode::OK || status == StatusCode::NOT_FOUND,
        "slippage override must not cause a 400 or 5xx error; got {status}; body: {json}"
    );

    if status == StatusCode::OK {
        let data = &json["data"];
        let quote = &data["quote"];

        // The response envelope must be complete.
        assert_eq!(json["v"], 1, "envelope v must be 1");
        assert!(
            quote.get("amount").is_some(),
            "quote must expose amount even with overrides"
        );
        assert!(
            quote.get("total").is_some(),
            "quote must expose total even with overrides"
        );

        // Verify amount round-trips correctly.
        assert_eq!(
            quote["amount"].as_str().unwrap_or(""),
            "10.0000000",
            "override must not alter the echoed amount"
        );
    }
}

// ---------------------------------------------------------------------------
// Additional model-level tests (no DB required)
// ---------------------------------------------------------------------------

/// Ensures a missing `amount` field (malformed JSON body) returns 422 or 400,
/// not a 500 — i.e., the handler defends against incomplete payloads.
#[tokio::test]
async fn missing_amount_field_is_rejected() {
    let router = build_router_no_db().await;

    let body = json!({
        "route": {
            "hops": [
                {
                    "from_asset": { "asset_code": "native" },
                    "to_asset":   { "asset_code": "USDC" },
                    "source":     "sdex"
                }
            ]
        }
        // "amount" intentionally omitted
    });

    let (status, _) = post_simulate(router, body).await;

    // axum's built-in JSON extractor returns 422 for missing required fields.
    // Either 422 or 400 is acceptable — what must not happen is a 500.
    assert!(
        status == StatusCode::UNPROCESSABLE_ENTITY || status == StatusCode::BAD_REQUEST,
        "missing amount field must yield 422 or 400, not 500; got {status}"
    );
}

/// Ensures a zero amount is rejected before any DB access.
#[tokio::test]
async fn zero_amount_returns_400_validation_error() {
    let router = build_router_no_db().await;

    let body = json!({
        "route": {
            "hops": [
                {
                    "from_asset": { "asset_code": "native" },
                    "to_asset":   { "asset_code": "USDC" },
                    "source":     "sdex"
                }
            ]
        },
        "amount": "0"
    });

    let (status, json) = post_simulate(router, body).await;

    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "zero amount must yield 400; body: {json}"
    );
    assert_eq!(
        json["data"]["error"], "validation_error",
        "error code must be validation_error"
    );
}

/// Verifies that a non-numeric amount string (e.g. `"abc"`) is rejected.
#[tokio::test]
async fn non_numeric_amount_returns_400_validation_error() {
    let router = build_router_no_db().await;

    let body = json!({
        "route": {
            "hops": [
                {
                    "from_asset": { "asset_code": "native" },
                    "to_asset":   { "asset_code": "USDC" },
                    "source":     "sdex"
                }
            ]
        },
        "amount": "abc"
    });

    let (status, json) = post_simulate(router, body).await;

    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "non-numeric amount must yield 400; body: {json}"
    );
    assert_eq!(
        json["data"]["error"], "validation_error",
        "error code must be validation_error"
    );
}
