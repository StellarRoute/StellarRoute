//! Integration tests for the StellarRoute Rust SDK.
//!
//! These tests use a lightweight mock HTTP server (via `wiremock`) to exercise
//! the full client stack — URL construction, request dispatch, response
//! deserialization, and error mapping — without requiring a live API.
//!
//! Run with:
//!   cargo test -p stellarroute-sdk

use stellarroute_sdk::{ApiErrorCode, ClientBuilder, QuoteRequest, QuoteType, SdkError};
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
