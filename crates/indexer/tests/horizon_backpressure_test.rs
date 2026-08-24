//! Integration tests for Horizon 429 backpressure handling.
//!
//! These tests use `wiremock` to simulate Horizon returning 429 responses
//! and verify that:
//! - The `Retry-After` header is respected when present.
//! - Cursor progress is preserved (the same URL is retried, not advanced).
//! - Throttle metrics are incremented correctly.
//! - After the backoff window the client succeeds on the next attempt.

use std::time::Duration;
use stellarroute_indexer::horizon::backpressure::{
    parse_retry_after, BackoffConfig, ThrottleState,
};
use stellarroute_indexer::horizon::HorizonClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn offers_page_json(records: serde_json::Value) -> String {
    serde_json::json!({
        "_links": {
            "next": { "href": "https://horizon-testnet.stellar.org/offers?cursor=123&limit=200" }
        },
        "_embedded": { "records": records }
    })
    .to_string()
}

fn sample_offer_json() -> serde_json::Value {
    serde_json::json!({
        "id": "42",
        "paging_token": "42",
        "seller": "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN",
        "selling": { "asset_type": "native" },
        "buying": {
            "asset_type": "credit_alphanum4",
            "asset_code": "USDC",
            "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
        },
        "amount": "100.0000000",
        "price": "0.1000000",
        "price_r": { "n": 1, "d": 10 },
        "last_modified_ledger": 40_000_000_i64,
        "last_modified_time": "2024-01-01T00:00:00Z",
        "sponsor": null
    })
}

// ---------------------------------------------------------------------------
// parse_retry_after unit tests
// ---------------------------------------------------------------------------

#[test]
fn test_parse_retry_after_integer_seconds() {
    assert_eq!(parse_retry_after(Some("30")), Some(30));
    assert_eq!(parse_retry_after(Some("0")), Some(0));
    assert_eq!(parse_retry_after(Some("  60  ")), Some(60));
}

#[test]
fn test_parse_retry_after_none_when_absent() {
    assert_eq!(parse_retry_after(None), None);
}

#[test]
fn test_parse_retry_after_none_for_garbage() {
    assert_eq!(parse_retry_after(Some("not-a-number")), None);
    assert_eq!(parse_retry_after(Some("")), None);
}

// ---------------------------------------------------------------------------
// ThrottleState unit tests
// ---------------------------------------------------------------------------

#[test]
fn test_throttle_state_respects_retry_after_header() {
    let state = ThrottleState::new();
    let cfg = BackoffConfig::default();
    let delay = state.record_rate_limit(Some(15), &cfg);
    assert_eq!(delay, Duration::from_secs(15));
    assert_eq!(state.throttle_events(), 1);
}

#[test]
fn test_throttle_state_jitter_within_bounds() {
    let state = ThrottleState::new();
    let cfg = BackoffConfig {
        min_delay_ms: 100,
        base_delay_ms: 200,
        max_delay_ms: 5_000,
    };
    for _ in 0..100 {
        let delay = state.record_rate_limit(None, &cfg);
        assert!(
            delay.as_millis() >= 100,
            "delay {} ms below minimum",
            delay.as_millis()
        );
        assert!(
            delay.as_millis() <= 5_000,
            "delay {} ms above maximum",
            delay.as_millis()
        );
    }
}

#[test]
fn test_throttle_state_success_resets_consecutive() {
    let state = ThrottleState::new();
    let cfg = BackoffConfig::default();
    state.record_rate_limit(Some(1), &cfg);
    state.record_rate_limit(Some(1), &cfg);
    assert_eq!(state.consecutive_429s(), 2);
    state.record_success();
    assert_eq!(state.consecutive_429s(), 0);
}

#[test]
fn test_throttle_wait_ms_accumulates() {
    let state = ThrottleState::new();
    let cfg = BackoffConfig::default();
    state.record_rate_limit(Some(2), &cfg); // 2000 ms
    state.record_rate_limit(Some(3), &cfg); // 3000 ms
    assert_eq!(state.throttle_wait_ms(), 5_000);
}

// ---------------------------------------------------------------------------
// Integration: mocked 429 → success
// ---------------------------------------------------------------------------

/// Horizon returns 429 once (with Retry-After: 0 so the test is instant),
/// then succeeds on the second attempt.
#[tokio::test]
async fn test_get_offers_recovers_after_single_429() {
    let mock_server = MockServer::start().await;

    // First call: 429 with Retry-After: 0 (instant retry)
    Mock::given(method("GET"))
        .and(path("/offers"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "0")
                .set_body_string("Too Many Requests"),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Second call: 200 with one offer
    Mock::given(method("GET"))
        .and(path("/offers"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(offers_page_json(serde_json::json!([sample_offer_json()]))),
        )
        .mount(&mock_server)
        .await;

    let client = HorizonClient::new(mock_server.uri());
    let offers = client.get_offers(Some(10), None, None).await.unwrap();

    assert_eq!(
        offers.len(),
        1,
        "Should have recovered and returned 1 offer"
    );
    // After success the consecutive counter resets
    assert_eq!(client.throttle.consecutive_429s(), 0);
    // One throttle event was recorded
    assert_eq!(client.throttle.throttle_events(), 1);
}

/// Horizon returns 429 with a Retry-After header; the client must honour it.
#[tokio::test]
async fn test_get_offers_respects_retry_after_header() {
    let mock_server = MockServer::start().await;

    // 429 with Retry-After: 0 (instant, so the test doesn't block)
    Mock::given(method("GET"))
        .and(path("/offers"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "0")
                .set_body_string("Too Many Requests"),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/offers"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(offers_page_json(serde_json::json!([]))),
        )
        .mount(&mock_server)
        .await;

    let client = HorizonClient::new(mock_server.uri());
    let result = client.get_offers(None, None, None).await;
    assert!(result.is_ok());
    assert_eq!(client.throttle.throttle_events(), 1);
}

/// When Horizon returns 429 repeatedly (beyond the retry limit) the client
/// must surface a `RateLimitExceeded` error rather than looping forever.
#[tokio::test]
async fn test_get_offers_exhausts_rate_limit_retries() {
    use stellarroute_indexer::error::IndexerError;

    let mock_server = MockServer::start().await;

    // Always 429 — the client should give up after max retries
    Mock::given(method("GET"))
        .and(path("/offers"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "0")
                .set_body_string("Too Many Requests"),
        )
        .mount(&mock_server)
        .await;

    // Use a custom backoff config with very small delays so the test is fast
    use stellarroute_indexer::horizon::backpressure::BackoffConfig;
    use stellarroute_indexer::horizon::client::RetryConfig;
    let cfg = RetryConfig {
        max_retries: 0,
        initial_delay_ms: 0,
        max_delay_ms: 0,
        backoff_multiplier: 1.0,
    };
    let client = HorizonClient::with_retry_config_and_backoff(
        mock_server.uri(),
        cfg,
        BackoffConfig {
            min_delay_ms: 0,
            base_delay_ms: 0,
            max_delay_ms: 0,
        },
    );
    let err = client.get_offers(None, None, None).await.unwrap_err();

    assert!(
        matches!(err, IndexerError::RateLimitExceeded { .. }),
        "Expected RateLimitExceeded, got {:?}",
        err
    );
    // Multiple throttle events should have been recorded
    assert!(client.throttle.throttle_events() > 0);
}

/// Cursor is preserved across 429 responses — the same URL is retried.
/// We verify this by checking that the mock server received the same
/// query parameters on both the 429 and the success response.
#[tokio::test]
async fn test_cursor_preserved_on_rate_limit() {
    use wiremock::matchers::query_param;

    let mock_server = MockServer::start().await;

    // 429 on the cursor=99 request
    Mock::given(method("GET"))
        .and(path("/offers"))
        .and(query_param("cursor", "99"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "0")
                .set_body_string("Too Many Requests"),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Success on the same cursor=99 request
    Mock::given(method("GET"))
        .and(path("/offers"))
        .and(query_param("cursor", "99"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(offers_page_json(serde_json::json!([sample_offer_json()]))),
        )
        .mount(&mock_server)
        .await;

    let client = HorizonClient::new(mock_server.uri());
    // Pass cursor="99" — it must be retried with the same cursor after 429
    let offers = client.get_offers(None, Some("99"), None).await.unwrap();
    assert_eq!(offers.len(), 1);
    assert_eq!(client.throttle.throttle_events(), 1);
}
