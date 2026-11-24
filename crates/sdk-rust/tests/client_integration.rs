//! Integration tests for the StellarRoute Rust SDK.
//!
//! These tests use a lightweight mock HTTP server (via `wiremock`) to exercise
//! the full client stack — URL construction, request dispatch, response
//! deserialization, and error mapping — without requiring a live API.
//!
//! Run with:
//!   cargo test -p stellarroute-sdk

use std::time::Duration;
use stellarroute_sdk::{
    ApiErrorCode, ClientBuilder, DryRunHop, QuoteRequest, QuoteType, RoutesRequest, SdkError,
    SimulateRouteRequest,
};
use wiremock::{
    matchers::{method, path, query_param},
    Mock, MockServer, ResponseTemplate,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn mock_server() -> MockServer {
    MockServer::start().await
}

fn client(server: &MockServer) -> stellarroute_sdk::StellarRouteClient {
    ClientBuilder::new(server.uri()).build().unwrap()
}

fn client_with_retries(
    server: &MockServer,
    max_retries: u32,
) -> stellarroute_sdk::StellarRouteClient {
    ClientBuilder::new(server.uri())
        .max_retries(max_retries)
        .base_backoff(Duration::from_millis(10))
        .build()
        .unwrap()
}

// ── Health ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn health_returns_healthy_response() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "healthy",
            "timestamp": "2026-03-25T12:00:00Z",
            "version": "0.1.0",
            "components": { "database": "healthy", "redis": "healthy" }
        })))
        .mount(&server)
        .await;

    let resp = client(&server).health().await.unwrap();
    assert!(resp.is_healthy());
    assert_eq!(resp.version, "0.1.0");
    assert_eq!(
        resp.components.get("database").map(String::as_str),
        Some("healthy")
    );
}

#[tokio::test]
async fn health_unhealthy_still_deserializes() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(503).set_body_json(serde_json::json!({
            "status": "unhealthy",
            "timestamp": "2026-03-25T12:00:00Z",
            "version": "0.1.0",
            "components": { "database": "unhealthy" }
        })))
        .mount(&server)
        .await;

    // 503 is a non-2xx status — the client maps it to SdkError::Api.
    let err = client(&server).health().await.unwrap_err();
    assert!(matches!(err, SdkError::Api { status: 503, .. }));
}

// ── Pairs ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn pairs_returns_typed_list() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/pairs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "pairs": [
                {
                    "base": "XLM",
                    "counter": "USDC",
                    "base_asset": "native",
                    "counter_asset": "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
                    "offer_count": 42,
                    "last_updated": "2026-03-25T11:59:00Z"
                }
            ],
            "total": 1
        })))
        .mount(&server)
        .await;

    let resp = client(&server).pairs().await.unwrap();
    assert_eq!(resp.total, 1);
    assert_eq!(resp.pairs[0].base, "XLM");
    assert_eq!(resp.pairs[0].offer_count, 42);
}

// ── Orderbook ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn orderbook_returns_bids_and_asks() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/orderbook/native/USDC"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "base_asset": { "asset_type": "native", "asset_code": null, "asset_issuer": null },
            "quote_asset": {
                "asset_type": "credit_alphanum4",
                "asset_code": "USDC",
                "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
            },
            "bids": [{ "price": "0.1050000", "amount": "500.0000000", "total": "52.5000000" }],
            "asks": [{ "price": "0.1060000", "amount": "300.0000000", "total": "31.8000000" }],
            "summary": {
                "bid": "0.1050000",
                "ask": "0.1060000",
                "spread_bps": 95,
                "midpoint": "0.1055000"
            },
            "timestamp": 1740312000
        })))
        .mount(&server)
        .await;

    let resp = client(&server).orderbook("native", "USDC").await.unwrap();
    assert!(resp.base_asset.is_native());
    assert_eq!(resp.best_bid(), Some("0.1050000"));
    assert_eq!(resp.best_ask(), Some("0.1060000"));
    assert_eq!(resp.bids.len(), 1);
    assert_eq!(resp.asks.len(), 1);
}

#[tokio::test]
async fn orderbook_not_found_maps_to_typed_error() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/orderbook/native/GHOST"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": "not_found",
            "message": "Asset not found in orderbook"
        })))
        .mount(&server)
        .await;

    let err = client(&server)
        .orderbook("native", "GHOST")
        .await
        .unwrap_err();
    assert!(err.is_not_found());
    assert_eq!(err.status_code(), Some(404));
    match err {
        SdkError::Api {
            code,
            message,
            status,
        } => {
            assert_eq!(code, ApiErrorCode::NotFound);
            assert_eq!(status, 404);
            assert!(!message.is_empty());
        }
        other => panic!("expected Api error, got {other:?}"),
    }
}

// ── Quote ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn quote_sell_sends_correct_query_params() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/quote/native/USDC"))
        .and(query_param("quote_type", "sell"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "base_asset": { "asset_type": "native", "asset_code": null, "asset_issuer": null },
            "quote_asset": {
                "asset_type": "credit_alphanum4",
                "asset_code": "USDC",
                "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
            },
            "amount": "1.0000000",
            "price": "0.1055000",
            "total": "0.1055000",
            "quote_type": "sell",
            "path": [],
            "timestamp": 1740312000
        })))
        .mount(&server)
        .await;

    let resp = client(&server)
        .quote(QuoteRequest::sell("native", "USDC"))
        .await
        .unwrap();

    assert_eq!(resp.price, "0.1055000");
    assert_eq!(resp.quote_type, "sell");
    assert!(resp.base_asset.is_native());
}

#[tokio::test]
async fn quote_buy_with_amount_sends_correct_params() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/quote/native/USDC"))
        .and(query_param("quote_type", "buy"))
        .and(query_param("amount", "100"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "base_asset": { "asset_type": "native", "asset_code": null, "asset_issuer": null },
            "quote_asset": {
                "asset_type": "credit_alphanum4",
                "asset_code": "USDC",
                "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
            },
            "amount": "100.0000000",
            "price": "0.1060000",
            "total": "10.6000000",
            "quote_type": "buy",
            "path": [
                {
                    "from_asset": { "asset_type": "native", "asset_code": null, "asset_issuer": null },
                    "to_asset": {
                        "asset_type": "credit_alphanum4",
                        "asset_code": "USDC",
                        "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
                    },
                    "price": "0.1060000",
                    "source": "sdex"
                }
            ],
            "timestamp": 1740312000
        })))
        .mount(&server)
        .await;

    let resp = client(&server)
        .quote(QuoteRequest {
            base: "native",
            quote: "USDC",
            amount: Some("100"),
            quote_type: QuoteType::Buy,
        })
        .await
        .unwrap();

    assert_eq!(resp.amount, "100.0000000");
    assert_eq!(resp.path.len(), 1);
    assert_eq!(resp.path[0].source, "sdex");
}

#[tokio::test]
async fn quote_validation_error_maps_to_typed_error() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/quote/native/USDC"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": "validation_error",
            "message": "Amount must be greater than zero"
        })))
        .mount(&server)
        .await;

    let err = client(&server)
        .quote(QuoteRequest::sell("native", "USDC"))
        .await
        .unwrap_err();

    assert!(err.is_validation_error());
    assert_eq!(err.status_code(), Some(400));
}

// ── Routes ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn routes_sends_expected_query_params_and_deserializes_response() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/routes/native/USDC"))
        .and(query_param("amount", "1000000"))
        .and(query_param("slippage_bps", "50"))
        .and(query_param("quote_type", "sell"))
        .and(wiremock::matchers::header_regex("user-agent", r"^stellarroute-sdk-rust/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "base_asset": { "asset_type": "native", "asset_code": null, "asset_issuer": null },
            "quote_asset": {
                "asset_type": "credit_alphanum4",
                "asset_code": "USDC",
                "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
            },
            "amount": "1000000",
            "routes": [
                {
                    "estimated_output": "1005000",
                    "impact_bps": 12,
                    "score": 0.98,
                    "policy_used": "best_price",
                    "path": [
                        {
                            "from_asset": { "asset_type": "native", "asset_code": null, "asset_issuer": null },
                            "to_asset": {
                                "asset_type": "credit_alphanum4",
                                "asset_code": "USDC",
                                "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
                            },
                            "price": "1.0050000",
                            "source": "sdex"
                        }
                    ]
                }
            ],
            "timestamp": 1740312000
        })))
        .mount(&server)
        .await;

    let resp = client(&server)
        .routes(RoutesRequest {
            base: "native",
            quote: "USDC",
            amount: 1_000_000,
            slippage_bps: Some(50),
            quote_type: Some(QuoteType::Sell),
        })
        .await
        .unwrap();

    assert_eq!(resp.amount, "1000000");
    assert_eq!(resp.routes[0].estimated_output, "1005000");
    assert_eq!(resp.routes[0].path[0].source, "sdex");
}

#[tokio::test]
async fn routes_no_route_maps_to_no_route_error() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/routes/native/GHOST"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": "no_route",
            "message": "No trading route found for this pair"
        })))
        .mount(&server)
        .await;

    let err = client(&server)
        .routes(RoutesRequest {
            base: "native",
            quote: "GHOST",
            amount: 1_000_000,
            slippage_bps: None,
            quote_type: None,
        })
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        SdkError::Api {
            code: ApiErrorCode::NoRoute,
            ..
        }
    ));
}

#[tokio::test]
#[ignore = "requires live StellarRoute API"]
async fn test_routes_returns_response() {
    let client = stellarroute_sdk::StellarRouteClient::new(
        &std::env::var("STELLARROUTE_API_URL")
            .expect("STELLARROUTE_API_URL must be set to run live tests"),
    )
    .expect("client construction should not fail with a valid URL");

    let response = client
        .routes(RoutesRequest {
            base: "native",
            quote: "USDC",
            amount: 1_000_000,
            slippage_bps: Some(100),
            quote_type: Some(QuoteType::Sell),
        })
        .await
        .expect("routes request should succeed against live API");

    assert!(!response.routes.is_empty(), "expected at least one route");
}

#[tokio::test]
#[ignore = "requires live StellarRoute API — uses a known-unroutable pair"]
async fn test_routes_no_route_returns_correct_error() {
    let client = stellarroute_sdk::StellarRouteClient::new(
        &std::env::var("STELLARROUTE_API_URL").expect("STELLARROUTE_API_URL must be set"),
    )
    .unwrap();

    let err = client
        .routes(RoutesRequest {
            base: "native",
            quote: "NONEXISTENT_ASSET_ABC123",
            amount: 1_000_000,
            slippage_bps: None,
            quote_type: None,
        })
        .await
        .expect_err("should return an error for unroutable pair");

    match err {
        SdkError::Api { code, .. } => assert_eq!(
            code,
            ApiErrorCode::NoRoute,
            "expected NoRoute error code, got: {code:?}"
        ),
        other => panic!("expected API error, got: {other:?}"),
    }
}

// ── Error handling ────────────────────────────────────────────────────────────

#[tokio::test]
async fn rate_limit_response_maps_to_rate_limited_error() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/pairs"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("x-ratelimit-limit", "100")
                .insert_header("x-ratelimit-remaining", "0")
                .insert_header("x-ratelimit-reset", "1740312060")
                .set_body_json(serde_json::json!({
                    "error": "rate_limit_exceeded",
                    "message": "Too many requests"
                })),
        )
        .mount(&server)
        .await;

    let err = client(&server).pairs().await.unwrap_err();
    assert!(err.is_rate_limited());
    assert_eq!(err.status_code(), Some(429));

    match err {
        SdkError::RateLimited { info } => {
            assert_eq!(info.limit, Some(100));
            assert_eq!(info.remaining, Some(0));
            assert_eq!(info.reset, Some(1740312060));
        }
        other => panic!("expected RateLimited, got {other:?}"),
    }
}

#[tokio::test]
async fn unknown_api_error_code_maps_to_other_variant() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/pairs"))
        .respond_with(ResponseTemplate::new(503).set_body_json(serde_json::json!({
            "error": "service_unavailable",
            "message": "Maintenance window"
        })))
        .mount(&server)
        .await;

    let err = client(&server).pairs().await.unwrap_err();
    match err {
        SdkError::Api {
            code: ApiErrorCode::Other(s),
            ..
        } => {
            assert_eq!(s, "service_unavailable");
        }
        other => panic!("expected Api/Other, got {other:?}"),
    }
}

#[tokio::test]
async fn malformed_json_maps_to_deserialization_error() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/pairs"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .mount(&server)
        .await;

    let err = client(&server).pairs().await.unwrap_err();
    assert!(matches!(err, SdkError::Deserialization(_)));
}

// ── Client construction ───────────────────────────────────────────────────────

#[test]
fn invalid_url_returns_config_error() {
    let err = ClientBuilder::new("not a url !!").build().unwrap_err();
    assert!(matches!(err, SdkError::InvalidConfig(_)));
}

#[test]
fn valid_url_builds_successfully() {
    assert!(ClientBuilder::new("http://localhost:3000").build().is_ok());
    assert!(ClientBuilder::new("https://api.stellarroute.io")
        .build()
        .is_ok());
}

// ── Type helpers ──────────────────────────────────────────────────────────────

#[test]
fn asset_info_display_name() {
    use stellarroute_sdk::AssetInfo;

    let native = AssetInfo {
        asset_type: "native".into(),
        asset_code: None,
        asset_issuer: None,
    };
    assert_eq!(native.display_name(), "native");
    assert!(native.is_native());

    let issued = AssetInfo {
        asset_type: "credit_alphanum4".into(),
        asset_code: Some("USDC".into()),
        asset_issuer: Some("GA5Z".into()),
    };
    assert_eq!(issued.display_name(), "USDC:GA5Z");
    assert!(!issued.is_native());
}

#[test]
fn api_error_code_roundtrip() {
    use stellarroute_sdk::ApiErrorCode;

    assert_eq!(
        "not_found".parse::<ApiErrorCode>().unwrap(),
        ApiErrorCode::NotFound
    );
    assert_eq!(
        "rate_limit_exceeded".parse::<ApiErrorCode>().unwrap(),
        ApiErrorCode::RateLimitExceeded
    );
    assert_eq!(
        "validation_error".parse::<ApiErrorCode>().unwrap(),
        ApiErrorCode::ValidationError
    );
    assert_eq!(
        "invalid_asset".parse::<ApiErrorCode>().unwrap(),
        ApiErrorCode::InvalidAsset
    );
    assert_eq!(
        "internal_error".parse::<ApiErrorCode>().unwrap(),
        ApiErrorCode::InternalError
    );
    assert_eq!(
        "stale_market_data".parse::<ApiErrorCode>().unwrap(),
        ApiErrorCode::StaleMarketData
    );
    assert_eq!(
        "overloaded".parse::<ApiErrorCode>().unwrap(),
        ApiErrorCode::Overloaded
    );

    let other = "custom_code".parse::<ApiErrorCode>().unwrap();
    assert_eq!(other.as_str(), "custom_code");
}

#[test]
fn quote_type_display() {
    use stellarroute_sdk::QuoteType;
    assert_eq!(QuoteType::Sell.as_str(), "sell");
    assert_eq!(QuoteType::Buy.as_str(), "buy");
    assert_eq!(QuoteType::Sell.to_string(), "sell");
}

// ── Retry behavior ───────────────────────────────────────────────────────────

#[tokio::test]
async fn retries_on_429_then_succeeds() {
    let server = mock_server().await;

    // First response: 429 with Retry-After
    Mock::given(method("GET"))
        .and(path("/api/v1/pairs"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "1")
                .set_body_json(serde_json::json!({
                    "error": "rate_limit_exceeded",
                    "message": "Too many requests"
                })),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Second response: success
    Mock::given(method("GET"))
        .and(path("/api/v1/pairs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "pairs": [],
            "total": 0
        })))
        .mount(&server)
        .await;

    let resp = client_with_retries(&server, 2).pairs().await.unwrap();
    assert_eq!(resp.total, 0);
}

#[tokio::test]
async fn retries_on_5xx_then_succeeds() {
    let server = mock_server().await;

    // First response: 500
    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "error": "internal_error",
            "message": "Server error"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Second response: success
    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "healthy",
            "timestamp": "2026-03-25T12:00:00Z",
            "version": "0.1.0",
            "components": {}
        })))
        .mount(&server)
        .await;

    let resp = client_with_retries(&server, 2).health().await.unwrap();
    assert!(resp.is_healthy());
}

#[tokio::test]
async fn retries_exhausted_returns_error() {
    let server = mock_server().await;

    // All responses are 429
    Mock::given(method("GET"))
        .and(path("/api/v1/pairs"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "1")
                .set_body_json(serde_json::json!({
                    "error": "rate_limit_exceeded",
                    "message": "Too many requests"
                })),
        )
        .mount(&server)
        .await;

    let err = client_with_retries(&server, 1).pairs().await.unwrap_err();
    assert!(err.is_rate_limited());
    assert_eq!(err.status_code(), Some(429));
}

#[tokio::test]
async fn default_no_retries_returns_error_immediately() {
    let server = mock_server().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/pairs"))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "error": "rate_limit_exceeded",
            "message": "Too many requests"
        })))
        .mount(&server)
        .await;

    // Default client has max_retries = 0
    let err = client(&server).pairs().await.unwrap_err();
    assert!(err.is_rate_limited());
}

#[tokio::test]
async fn retries_respect_retry_after_header() {
    let server = mock_server().await;

    // First: 429 with Retry-After: 1
    Mock::given(method("GET"))
        .and(path("/api/v1/pairs"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "1")
                .set_body_json(serde_json::json!({
                    "error": "rate_limit_exceeded",
                    "message": "Too many requests"
                })),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Second: success
    Mock::given(method("GET"))
        .and(path("/api/v1/pairs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "pairs": [],
            "total": 0
        })))
        .mount(&server)
        .await;

    let start = std::time::Instant::now();
    let resp = client_with_retries(&server, 2).pairs().await.unwrap();
    let elapsed = start.elapsed();

    assert_eq!(resp.total, 0);
    // Should have waited at least ~1 second due to Retry-After: 1
    assert!(elapsed >= Duration::from_millis(900));
}

// ────────────────────────────────────────────────────────────────────────────────
// Additional mock tests for pairs + orderbook (issue #1277)
// These ensure CI stays green without requiring a live API.
// ────────────────────────────────────────────────────────────────────────────────

// ── Pairs: empty response ────────────────────────────────────────────────────────

#[tokio::test]
async fn pairs_empty_response_deserializes() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/pairs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "pairs": [],
            "total": 0
        })))
        .mount(&server)
        .await;

    let resp = client(&server).pairs().await.unwrap();
    assert_eq!(resp.total, 0);
    assert!(resp.pairs.is_empty());
}

// ── Pairs: large page ────────────────────────────────────────────────────────────

#[tokio::test]
async fn pairs_large_page_deserializes() {
    let server = mock_server().await;
    let pairs: Vec<serde_json::Value> = (0..100)
        .map(|i| {
            serde_json::json!({
                "base": "XLM",
                "counter": format!("TOKEN{}", i),
                "base_asset": "native",
                "counter_asset": format!("TOKEN{}:GABCD", i),
                "offer_count": i + 1,
                "last_updated": "2026-03-25T11:59:00Z"
            })
        })
        .collect();

    Mock::given(method("GET"))
        .and(path("/api/v1/pairs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "pairs": pairs,
            "total": 100
        })))
        .mount(&server)
        .await;

    let resp = client(&server).pairs().await.unwrap();
    assert_eq!(resp.total, 100);
    assert_eq!(resp.pairs.len(), 100);
    assert_eq!(resp.pairs[50].counter, "TOKEN50");
}

// ── Orderbook: empty bids and asks ───────────────────────────────────────────────

#[tokio::test]
async fn orderbook_empty_bids_and_asks_deserializes() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/orderbook/native/USDC"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "base_asset": { "asset_type": "native", "asset_code": null, "asset_issuer": null },
            "quote_asset": {
                "asset_type": "credit_alphanum4",
                "asset_code": "USDC",
                "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
            },
            "bids": [],
            "asks": [],
            "summary": {
                "bid": null,
                "ask": null,
                "spread_bps": 0,
                "midpoint": null
            },
            "timestamp": 1740312000
        })))
        .mount(&server)
        .await;

    let resp = client(&server).orderbook("native", "USDC").await.unwrap();
    assert!(resp.base_asset.is_native());
    assert!(resp.bids.is_empty());
    assert!(resp.asks.is_empty());
    assert_eq!(resp.best_bid(), None);
    assert_eq!(resp.best_ask(), None);
}

// ── Orderbook: multiple price levels ──────────────────────────────────────────────

#[tokio::test]
async fn orderbook_multiple_levels_deserializes() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/orderbook/native/USDC"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "base_asset": { "asset_type": "native", "asset_code": null, "asset_issuer": null },
            "quote_asset": {
                "asset_type": "credit_alphanum4",
                "asset_code": "USDC",
                "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
            },
            "bids": [
                { "price": "0.1050000", "amount": "500.0000000", "total": "52.5000000" },
                { "price": "0.1049000", "amount": "1000.0000000", "total": "104.9000000" },
                { "price": "0.1048000", "amount": "2000.0000000", "total": "209.6000000" }
            ],
            "asks": [
                { "price": "0.1060000", "amount": "300.0000000", "total": "31.8000000" },
                { "price": "0.1061000", "amount": "500.0000000", "total": "53.0500000" }
            ],
            "summary": {
                "bid": "0.1050000",
                "ask": "0.1060000",
                "spread_bps": 95,
                "midpoint": "0.1055000"
            },
            "timestamp": 1740312000
        })))
        .mount(&server)
        .await;

    let resp = client(&server).orderbook("native", "USDC").await.unwrap();
    assert_eq!(resp.bids.len(), 3);
    assert_eq!(resp.asks.len(), 2);
    assert_eq!(resp.best_bid(), Some("0.1050000"));
    assert_eq!(resp.best_ask(), Some("0.1060000"));
}

// ── Orderbook: credit_alphanum12 asset ───────────────────────────────────────────

#[tokio::test]
async fn orderbook_alphanum12_asset_deserializes() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/orderbook/native/CUSTOMASSET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "base_asset": { "asset_type": "native", "asset_code": null, "asset_issuer": null },
            "quote_asset": {
                "asset_type": "credit_alphanum12",
                "asset_code": "CUSTOMASSET",
                "asset_issuer": "GABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890"
            },
            "bids": [{ "price": "1.0000000", "amount": "100.0000000", "total": "100.0000000" }],
            "asks": [{ "price": "1.0100000", "amount": "50.0000000", "total": "50.5000000" }],
            "summary": {
                "bid": "1.0000000",
                "ask": "1.0100000",
                "spread_bps": 100,
                "midpoint": "1.0050000"
            },
            "timestamp": 1740312000
        })))
        .mount(&server)
        .await;

    let resp = client(&server)
        .orderbook("native", "CUSTOMASSET")
        .await
        .unwrap();
    assert!(resp.base_asset.is_native());
    assert!(!resp.quote_asset.is_native());
    assert_eq!(resp.quote_asset.asset_code, Some("CUSTOMASSET".to_string()));
    assert_eq!(
        resp.quote_asset.asset_issuer,
        Some("GABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890".to_string())
    );
}

// ── Orderbook: validation error ──────────────────────────────────────────────────

#[tokio::test]
async fn orderbook_validation_error_maps_to_typed_error() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/orderbook/native/USDC"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": "validation_error",
            "message": "Invalid asset code"
        })))
        .mount(&server)
        .await;

    let err = client(&server)
        .orderbook("native", "USDC")
        .await
        .unwrap_err();

    assert!(err.is_validation_error());
    assert_eq!(err.status_code(), Some(400));
}

// ── Orderbook: server error ──────────────────────────────────────────────────────

#[tokio::test]
async fn orderbook_server_error_maps_to_internal_error() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/orderbook/native/USDC"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "error": "internal_error",
            "message": "Orderbook service unavailable"
        })))
        .mount(&server)
        .await;

    let err = client(&server)
        .orderbook("native", "USDC")
        .await
        .unwrap_err();

    assert!(err.is_internal_error());
    assert_eq!(err.status_code(), Some(500));
}

// ── Pairs: server error ──────────────────────────────────────────────────────────

#[tokio::test]
async fn pairs_server_error_maps_to_internal_error() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/pairs"))
        .respond_with(ResponseTemplate::new(503).set_body_json(serde_json::json!({
            "error": "service_unavailable",
            "message": "Pairs service maintenance"
        })))
        .mount(&server)
        .await;

    let err = client(&server).pairs().await.unwrap_err();

    assert!(err.is_internal_error());
    assert_eq!(err.status_code(), Some(503));
}

// ── Pairs: stale market data error ───────────────────────────────────────────────

#[tokio::test]
async fn pairs_stale_market_data_maps_to_typed_error() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/pairs"))
        .respond_with(ResponseTemplate::new(409).set_body_json(serde_json::json!({
            "error": "stale_market_data",
            "message": "Market data is stale, please retry"
        })))
        .mount(&server)
        .await;

    let err = client(&server).pairs().await.unwrap_err();

    match err {
        SdkError::Api {
            code: ApiErrorCode::StaleMarketData,
            status,
            ..
        } => {
            assert_eq!(status, 409);
        }
        other => panic!("expected Api/StaleMarketData, got {other:?}"),
    }
}

// ── Orderbook: overloaded error ──────────────────────────────────────────────────

#[tokio::test]
async fn orderbook_overloaded_maps_to_typed_error() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/orderbook/native/USDC"))
        .respond_with(ResponseTemplate::new(503).set_body_json(serde_json::json!({
            "error": "overloaded",
            "message": "Orderbook service overloaded"
        })))
        .mount(&server)
        .await;

    let err = client(&server)
        .orderbook("native", "USDC")
        .await
        .unwrap_err();

    match err {
        SdkError::Api {
            code: ApiErrorCode::Overloaded,
            status,
            ..
        } => {
            assert_eq!(status, 503);
        }
        other => panic!("expected Api/Overloaded, got {other:?}"),
    }
}

// ── Price History ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn price_history_returns_typed_response() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/price-history/native/USDC"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "base_asset": { "asset_type": "native", "asset_code": null, "asset_issuer": null },
            "quote_asset": {
                "asset_type": "credit_alphanum4",
                "asset_code": "USDC",
                "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
            },
            "window": "24h",
            "source": "orderbook_snapshots",
            "generated_at": 1742908800000_i64,
            "points": [
                { "timestamp": 1742822400000_i64, "price": "0.1040000" },
                { "timestamp": 1742826000000_i64, "price": "0.1050000" },
                { "timestamp": 1742829600000_i64, "price": "0.1060000" }
            ]
        })))
        .mount(&server)
        .await;

    let resp = client(&server)
        .price_history("native", "USDC")
        .await
        .unwrap();
    assert!(resp.base_asset.is_native());
    assert_eq!(resp.points.len(), 3);
    assert_eq!(resp.points[0].price, "0.1040000");
    assert_eq!(resp.window, "24h");
}

#[tokio::test]
async fn price_history_400_maps_to_validation_error() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/price-history/native/USDC"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": "validation_error",
            "message": "Invalid asset identifier"
        })))
        .mount(&server)
        .await;

    let err = client(&server)
        .price_history("native", "USDC")
        .await
        .unwrap_err();
    assert!(err.is_validation_error());
    assert_eq!(err.status_code(), Some(400));
    match err {
        SdkError::Api { code, status, .. } => {
            assert_eq!(code, ApiErrorCode::ValidationError);
            assert_eq!(status, 400);
        }
        other => panic!("expected Api error, got {other:?}"),
    }
}

#[tokio::test]
async fn price_history_404_maps_to_no_route_error() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/price-history/native/GHOST"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": "no_route",
            "message": "No price history found for this pair"
        })))
        .mount(&server)
        .await;

    let err = client(&server)
        .price_history("native", "GHOST")
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        SdkError::Api {
            code: ApiErrorCode::NoRoute,
            ..
        }
    ));
}

#[tokio::test]
async fn price_history_429_exhausted_maps_to_rate_limited() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/price-history/native/USDC"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "1")
                .set_body_json(serde_json::json!({
                    "error": "rate_limit_exceeded",
                    "message": "Too many requests"
                })),
        )
        .mount(&server)
        .await;

    let err = client_with_retries(&server, 1)
        .price_history("native", "USDC")
        .await
        .unwrap_err();
    assert!(err.is_rate_limited());
    assert_eq!(err.status_code(), Some(429));
}

#[tokio::test]
async fn price_history_empty_points_deserializes() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/price-history/native/USDC"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "base_asset": { "asset_type": "native", "asset_code": null, "asset_issuer": null },
            "quote_asset": {
                "asset_type": "credit_alphanum4",
                "asset_code": "USDC",
                "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
            },
            "window": "24h",
            "source": "orderbook_snapshots",
            "generated_at": 1742908800000_i64,
            "points": []
        })))
        .mount(&server)
        .await;

    let resp = client(&server)
        .price_history("native", "USDC")
        .await
        .unwrap();
    assert!(resp.points.is_empty());
    assert_eq!(resp.window, "24h");
}

#[tokio::test]
#[ignore = "requires live StellarRoute API"]
async fn price_history_live_smoke() {
    let client = stellarroute_sdk::StellarRouteClient::new(
        &std::env::var("STELLARROUTE_API_URL")
            .expect("STELLARROUTE_API_URL must be set to run live tests"),
    )
    .expect("client construction should not fail with a valid URL");

    let resp = client
        .price_history("native", "USDC")
        .await
        .expect("price_history should succeed against live API");

    assert_eq!(resp.window, "24h");
    assert!(!resp.source.is_empty());
}

// ── Simulate Route ────────────────────────────────────────────────────────────

/// Build a minimal valid `SimulateRouteRequest` for use in tests.
fn sample_simulate_request() -> SimulateRouteRequest {
    SimulateRouteRequest {
        hops: vec![DryRunHop {
            from_asset: "native".to_string(),
            to_asset: "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string(),
            source: "sdex".to_string(),
            fee_bps: Some(30),
            price: Some("0.1050000".to_string()),
            venue_ref: Some("sdex".to_string()),
        }],
        amount: "100.0".to_string(),
        slippage_bps: Some(50),
        slippage_bps_overrides: vec![],
    }
}

/// Build a valid `ApiResponse<RouteDryRunResponse>` JSON body for mock responses.
fn simulate_route_200_body() -> serde_json::Value {
    serde_json::json!({
        "v": 1,
        "timestamp": 1742908800000_i64,
        "request_id": "test-req-123",
        "data": {
            "quote": {
                "base_asset": { "asset_type": "native", "asset_code": null, "asset_issuer": null },
                "quote_asset": {
                    "asset_type": "credit_alphanum4",
                    "asset_code": "USDC",
                    "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
                },
                "amount": "100.0000000",
                "price": "0.1050000",
                "total": "10.5000000",
                "quote_type": "sell",
                "degraded": false,
                "path": [
                    {
                        "from_asset": { "asset_type": "native", "asset_code": null, "asset_issuer": null },
                        "to_asset": {
                            "asset_type": "credit_alphanum4",
                            "asset_code": "USDC",
                            "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
                        },
                        "price": "0.1050000",
                        "source": "sdex"
                    }
                ],
                "timestamp": 1742908800000_i64
            },
            "exclusion_diagnostics": null,
            "swap_path": {
                "hops": [
                    {
                        "source_asset": "native",
                        "destination_asset": "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
                        "venue_type": "sdex",
                        "venue_ref": "sdex",
                        "price": 0.1050000_f64,
                        "fee_bps": 30
                    }
                ],
                "estimated_output": 987600_i64
            }
        }
    })
}

#[tokio::test]
async fn simulate_route_happy_path() {
    let server = mock_server().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/simulate/route"))
        .respond_with(ResponseTemplate::new(200).set_body_json(simulate_route_200_body()))
        .mount(&server)
        .await;

    let resp = client(&server)
        .simulate_route(sample_simulate_request())
        .await
        .unwrap();

    assert_eq!(resp.quote.price, "0.1050000");
    assert_eq!(resp.quote.total, "10.5000000");
    assert_eq!(resp.swap_path.hops.len(), 1);
    assert_eq!(resp.swap_path.estimated_output, 987600);
    assert_eq!(resp.swap_path.hops[0].venue_type, "sdex");
    assert!(resp.exclusion_diagnostics.is_none());
}

#[tokio::test]
async fn simulate_route_uses_post_method_and_correct_path() {
    let server = mock_server().await;
    // The mock requires POST to exactly /api/v1/simulate/route — any other method
    // or path will result in a 404 from wiremock, causing the assertion to fail.
    Mock::given(method("POST"))
        .and(path("/api/v1/simulate/route"))
        .respond_with(ResponseTemplate::new(200).set_body_json(simulate_route_200_body()))
        .mount(&server)
        .await;

    // If the SDK uses GET or a different path this will unwrap_err instead.
    client(&server)
        .simulate_route(sample_simulate_request())
        .await
        .unwrap();
}

#[tokio::test]
async fn simulate_route_sends_user_agent_header() {
    let server = mock_server().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/simulate/route"))
        .and(wiremock::matchers::header_regex(
            "user-agent",
            r"^stellarroute-sdk-rust/",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(simulate_route_200_body()))
        .mount(&server)
        .await;

    client(&server)
        .simulate_route(sample_simulate_request())
        .await
        .unwrap();
}

#[tokio::test]
async fn simulate_route_400_maps_to_validation_error() {
    let server = mock_server().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/simulate/route"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": "validation_error",
            "message": "route.hops must contain at least one hop"
        })))
        .mount(&server)
        .await;

    let err = client(&server)
        .simulate_route(sample_simulate_request())
        .await
        .unwrap_err();

    assert!(err.is_validation_error());
    assert_eq!(err.status_code(), Some(400));
    match err {
        SdkError::Api { code, status, .. } => {
            assert_eq!(code, ApiErrorCode::ValidationError);
            assert_eq!(status, 400);
        }
        other => panic!("expected Api error, got {other:?}"),
    }
}

#[tokio::test]
async fn simulate_route_404_no_route_maps_to_no_route_error() {
    let server = mock_server().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/simulate/route"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": "no_route",
            "message": "No route found for the given hop chain"
        })))
        .mount(&server)
        .await;

    let err = client(&server)
        .simulate_route(sample_simulate_request())
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        SdkError::Api {
            code: ApiErrorCode::NoRoute,
            ..
        }
    ));
    assert_eq!(err.status_code(), Some(404));
}

#[tokio::test]
async fn simulate_route_500_retries_then_succeeds() {
    let server = mock_server().await;

    // First call: 500
    Mock::given(method("POST"))
        .and(path("/api/v1/simulate/route"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "error": "internal_error",
            "message": "Transient server error"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Second call: success
    Mock::given(method("POST"))
        .and(path("/api/v1/simulate/route"))
        .respond_with(ResponseTemplate::new(200).set_body_json(simulate_route_200_body()))
        .mount(&server)
        .await;

    let resp = client_with_retries(&server, 1)
        .simulate_route(sample_simulate_request())
        .await
        .unwrap();

    assert_eq!(resp.quote.price, "0.1050000");
}

#[tokio::test]
async fn simulate_route_optional_fields_absent() {
    let server = mock_server().await;
    // Response with no optional fields on the quote and no exclusion_diagnostics.
    Mock::given(method("POST"))
        .and(path("/api/v1/simulate/route"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "v": 1,
            "timestamp": 1742908800000_i64,
            "request_id": "test-req-456",
            "data": {
                "quote": {
                    "base_asset": { "asset_type": "native", "asset_code": null, "asset_issuer": null },
                    "quote_asset": {
                        "asset_type": "credit_alphanum4",
                        "asset_code": "USDC",
                        "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
                    },
                    "amount": "100.0000000",
                    "price": "0.1050000",
                    "total": "10.5000000",
                    "quote_type": "sell",
                    "path": [],
                    "timestamp": 1742908800000_i64
                },
                "swap_path": {
                    "hops": [],
                    "estimated_output": 0_i64
                }
            }
        })))
        .mount(&server)
        .await;

    let resp = client(&server)
        .simulate_route(sample_simulate_request())
        .await
        .unwrap();

    assert!(resp.exclusion_diagnostics.is_none());
    assert!(resp.quote.expires_at.is_none());
    assert!(resp.quote.source_timestamp.is_none());
    assert!(resp.quote.ttl_seconds.is_none());
    assert!(resp.quote.rationale.is_none());
    assert!(resp.quote.exclusion_diagnostics.is_none());
    assert!(resp.quote.data_freshness.is_none());
    assert!(resp.quote.midpoint.is_none());
    assert!(resp.quote.spread_bps.is_none());
    assert!(resp.quote.price_impact.is_none());
    assert!(!resp.quote.degraded);
}
