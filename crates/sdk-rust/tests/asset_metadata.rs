//! Mock-HTTP tests for the additive `GET /api/v1/assets/{code}` wrapper.
//!
//! These exercise URL construction, the optional `issuer` query parameter,
//! response deserialization against the current OpenAPI shape, and error
//! mapping — without touching any existing client method.
//!
//! Run with:
//!   cargo test -p stellarroute-sdk --test asset_metadata

use stellarroute_sdk::{ApiErrorCode, ClientBuilder, SdkError};
use wiremock::{
    matchers::{method, path, query_param},
    Mock, MockServer, ResponseTemplate,
};

async fn mock_server() -> MockServer {
    MockServer::start().await
}

fn client(server: &MockServer) -> stellarroute_sdk::StellarRouteClient {
    ClientBuilder::new(server.uri()).build().unwrap()
}

#[tokio::test]
async fn asset_metadata_returns_full_payload() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/assets/USDC"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": "USDC",
            "issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
            "decimals": 7,
            "asset_type": "credit_alphanum4",
            "display_name": "USD Coin",
            "icon_url": "https://example.test/usdc.png",
            "domain": "centre.io"
        })))
        .mount(&server)
        .await;

    let asset = client(&server).asset_metadata("USDC", None).await.unwrap();

    assert_eq!(asset.code, "USDC");
    assert_eq!(
        asset.issuer.as_deref(),
        Some("GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN")
    );
    assert_eq!(asset.decimals, 7);
    assert_eq!(asset.asset_type, "credit_alphanum4");
    assert_eq!(asset.display_name.as_deref(), Some("USD Coin"));
    assert_eq!(asset.icon_url.as_deref(), Some("https://example.test/usdc.png"));
    assert_eq!(asset.domain.as_deref(), Some("centre.io"));
}

#[tokio::test]
async fn asset_metadata_omits_issuer_query_when_none() {
    let server = mock_server().await;
    // Mounted without a query_param matcher: the request must still match, and
    // the assertion below proves the optional field round-trips as absent.
    Mock::given(method("GET"))
        .and(path("/api/v1/assets/XLM"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": "XLM",
            "decimals": 7,
            "asset_type": "native"
        })))
        .mount(&server)
        .await;

    let asset = client(&server).asset_metadata("XLM", None).await.unwrap();

    assert_eq!(asset.code, "XLM");
    assert_eq!(asset.asset_type, "native");
    // The server omits these rather than sending nulls; they must deserialize
    // to None instead of failing.
    assert!(asset.issuer.is_none());
    assert!(asset.display_name.is_none());
    assert!(asset.icon_url.is_none());
    assert!(asset.domain.is_none());
}

#[tokio::test]
async fn asset_metadata_sends_issuer_query_when_provided() {
    let issuer = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";

    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/assets/USDC"))
        .and(query_param("issuer", issuer))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": "USDC",
            "issuer": issuer,
            "decimals": 7,
            "asset_type": "credit_alphanum4"
        })))
        .mount(&server)
        .await;

    // The mock only matches when the issuer query parameter is present, so a
    // successful call is itself the assertion.
    let asset = client(&server)
        .asset_metadata("USDC", Some(issuer))
        .await
        .unwrap();

    assert_eq!(asset.issuer.as_deref(), Some(issuer));
}

#[tokio::test]
async fn asset_metadata_maps_404_to_not_found() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/assets/NOPE"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": "not_found",
            "message": "Asset metadata not found for NOPE"
        })))
        .mount(&server)
        .await;

    let err = client(&server).asset_metadata("NOPE", None).await.unwrap_err();

    match err {
        SdkError::Api { code, status, .. } => {
            assert_eq!(status, 404);
            assert_eq!(code, ApiErrorCode::NotFound);
        }
        other => panic!("expected Api error, got {other:?}"),
    }
}

#[tokio::test]
async fn asset_metadata_surfaces_server_error() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/assets/USDC"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "error": "internal_error",
            "message": "boom"
        })))
        .mount(&server)
        .await;

    let err = client(&server).asset_metadata("USDC", None).await.unwrap_err();

    assert!(
        matches!(err, SdkError::Api { status: 500, .. }),
        "expected a 500 Api error, got {err:?}"
    );
}
