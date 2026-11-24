//! CCTP v2 bridge integration tests (`/api/v2/bridge/cctp/*`).
//!
//! Uses `wiremock`; no live API required and no CCTP execution is implied —
//! these only verify the SDK's wire shapes and header plumbing against the
//! documented contract (`docs/api/cctp-v2-contract.md`). Execution stays
//! fail-closed behind `CCTP_ENABLED` on the server; this SDK does not
//! change that default.

use stellarroute_sdk::{
    CctpDirection, CctpFinality, CctpQuoteRequest, CctpSubmitBurnRequest, CctpSubmitMintRequest,
    ChainAsset, ClientBuilder, CCTP_IDEMPOTENCY_HEADER, CCTP_TRANSFER_ACCESS_HEADER,
};
use wiremock::{
    matchers::{body_json, header, method, path},
    Mock, MockServer, ResponseTemplate,
};

async fn mock_server() -> MockServer {
    MockServer::start().await
}

fn client(server: &MockServer) -> stellarroute_sdk::StellarRouteClient {
    ClientBuilder::new(server.uri()).build().unwrap()
}

fn stellar_testnet_usdc() -> ChainAsset {
    ChainAsset {
        chain_id: "stellar:testnet".to_string(),
        asset: "erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA".to_string(),
        canonical: "stellar:testnet/erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA"
            .to_string(),
        symbol: Some("USDC".to_string()),
    }
}

fn sepolia_usdc() -> ChainAsset {
    ChainAsset {
        chain_id: "eip155:11155111".to_string(),
        asset: "erc20:0x1c7d4b196cb0c7b01d743fbc6116a902379c7238".to_string(),
        canonical: "eip155:11155111/erc20:0x1c7d4b196cb0c7b01d743fbc6116a902379c7238".to_string(),
        symbol: Some("USDC".to_string()),
    }
}

fn sample_quote_request() -> CctpQuoteRequest {
    CctpQuoteRequest {
        corridor_id: "circle-cctp:usdc:stellar-testnet:ethereum-sepolia".to_string(),
        provider: "circle-cctp".to_string(),
        direction: CctpDirection::StellarToEvm,
        source_chain_id: "stellar:testnet".to_string(),
        destination_chain_id: "eip155:11155111".to_string(),
        source_asset: stellar_testnet_usdc(),
        destination_asset: sepolia_usdc(),
        amount: "100".to_string(),
        recipient: "0x1c7d4b196cb0c7b01d743fbc6116a902379c7238".to_string(),
        sender: None,
        mint_submitter: None,
        finality: CctpFinality::Standard,
    }
}

#[tokio::test]
async fn cctp_quote_sends_idempotency_header_and_deserializes_response() {
    let server = mock_server().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/bridge/cctp/quote"))
        .and(header(CCTP_IDEMPOTENCY_HEADER, "idem-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "transfer_id": "transfer-1",
                "corridor_id": "circle-cctp:usdc:stellar-testnet:ethereum-sepolia",
                "provider": "circle-cctp",
                "direction": "stellar_to_evm",
                "source_amount": "100",
                "destination_amount": "99.5",
                "fee_quote": {},
                "expires_at": 1_740_312_000,
                "finality": "standard",
                "access_token": "token-abc"
            }
        })))
        .mount(&server)
        .await;

    let resp = client(&server)
        .cctp_quote(sample_quote_request(), Some("idem-1"))
        .await
        .unwrap();

    assert_eq!(resp.transfer_id, "transfer-1");
    assert_eq!(resp.access_token, "token-abc");
    assert_eq!(resp.direction, CctpDirection::StellarToEvm);
    assert_eq!(resp.finality, CctpFinality::Standard);
}

#[tokio::test]
async fn cctp_quote_without_idempotency_key_omits_header() {
    let server = mock_server().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/bridge/cctp/quote"))
        .respond_with(ResponseTemplate::new(503).set_body_json(serde_json::json!({
            "error": "cctp_not_enabled",
            "message": "CCTP is not enabled"
        })))
        .mount(&server)
        .await;

    let err = client(&server)
        .cctp_quote(sample_quote_request(), None)
        .await
        .unwrap_err();

    assert_eq!(err.status_code(), Some(503));

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert!(
        !requests[0].headers.contains_key(CCTP_IDEMPOTENCY_HEADER),
        "idempotency header must be omitted when no key is given"
    );
}

#[tokio::test]
async fn cctp_get_transfer_sends_access_header() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/bridge/cctp/transfer-1"))
        .and(header(CCTP_TRANSFER_ACCESS_HEADER, "token-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "transfer_id": "transfer-1",
                "corridor_id": "circle-cctp:usdc:stellar-testnet:ethereum-sepolia",
                "provider": "circle-cctp",
                "direction": "stellar_to_evm",
                "status": "awaiting_attestation",
                "retryable": false
            }
        })))
        .mount(&server)
        .await;

    let resp = client(&server)
        .cctp_get_transfer("transfer-1", "token-abc")
        .await
        .unwrap();

    assert_eq!(
        resp.status,
        stellarroute_sdk::CctpTransferStatus::AwaitingAttestation
    );
    assert!(!resp.retryable);
    assert!(resp.source_tx_hash.is_none());
}

#[tokio::test]
async fn cctp_prepare_burn_returns_stellar_xdr_payload() {
    let server = mock_server().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/bridge/cctp/transfer-1/prepare-burn"))
        .and(header(CCTP_TRANSFER_ACCESS_HEADER, "token-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "transfer_id": "transfer-1",
                "status": "burn_prepared",
                "payload": {
                    "type": "stellar_xdr",
                    "network_passphrase": "Test SDF Network ; September 2015",
                    "xdr_envelope": "opaque-xdr"
                },
                "expires_at": 1_740_312_000,
                "approval_required": false
            }
        })))
        .mount(&server)
        .await;

    let resp = client(&server)
        .cctp_prepare_burn("transfer-1", "token-abc")
        .await
        .unwrap();

    match resp.payload {
        stellarroute_sdk::PreparedWalletPayload::StellarXdr { xdr_envelope, .. } => {
            assert_eq!(xdr_envelope, "opaque-xdr");
        }
        other => panic!("expected StellarXdr payload, got {other:?}"),
    }
    assert!(!resp.approval_required);
}

#[tokio::test]
async fn cctp_submit_burn_posts_tx_hash_with_access_header() {
    let server = mock_server().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/bridge/cctp/transfer-1/submit-burn"))
        .and(header(CCTP_TRANSFER_ACCESS_HEADER, "token-abc"))
        .and(body_json(serde_json::json!({ "tx_hash": "a".repeat(64) })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "transfer_id": "transfer-1",
                "status": "burn_submitted",
                "source_tx_hash": "a".repeat(64)
            }
        })))
        .mount(&server)
        .await;

    let resp = client(&server)
        .cctp_submit_burn(
            "transfer-1",
            "token-abc",
            CctpSubmitBurnRequest {
                tx_hash: "a".repeat(64),
            },
        )
        .await
        .unwrap();

    assert_eq!(
        resp.status,
        stellarroute_sdk::CctpTransferStatus::BurnSubmitted
    );
    assert_eq!(resp.source_tx_hash, "a".repeat(64));
}

#[tokio::test]
async fn cctp_prepare_mint_reports_trustline_required() {
    let server = mock_server().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/bridge/cctp/transfer-1/prepare-mint"))
        .and(header(CCTP_TRANSFER_ACCESS_HEADER, "token-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "transfer_id": "transfer-1",
                "status": "mint_prepared",
                "payload": {
                    "type": "stellar_xdr",
                    "network_passphrase": "Test SDF Network ; September 2015",
                    "xdr_envelope": "opaque-changetrust-xdr",
                    "source": "GSENDER"
                },
                "expires_at": 1_740_312_000,
                "trustline_required": true
            }
        })))
        .mount(&server)
        .await;

    let resp = client(&server)
        .cctp_prepare_mint("transfer-1", "token-abc")
        .await
        .unwrap();

    assert!(resp.trustline_required);
}

#[tokio::test]
async fn cctp_submit_mint_posts_tx_hash() {
    let server = mock_server().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/bridge/cctp/transfer-1/submit-mint"))
        .and(body_json(
            serde_json::json!({ "tx_hash": format!("0x{}", "b".repeat(64)) }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "transfer_id": "transfer-1",
                "status": "mint_submitted",
                "destination_tx_hash": format!("0x{}", "b".repeat(64))
            }
        })))
        .mount(&server)
        .await;

    let resp = client(&server)
        .cctp_submit_mint(
            "transfer-1",
            "token-abc",
            CctpSubmitMintRequest {
                tx_hash: format!("0x{}", "b".repeat(64)),
            },
        )
        .await
        .unwrap();

    assert_eq!(
        resp.status,
        stellarroute_sdk::CctpTransferStatus::MintSubmitted
    );
}

#[tokio::test]
async fn cctp_reattest_returns_retryable_flag() {
    let server = mock_server().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/bridge/cctp/transfer-1/reattest"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "transfer_id": "transfer-1",
                "status": "awaiting_attestation",
                "retryable": true
            }
        })))
        .mount(&server)
        .await;

    let resp = client(&server)
        .cctp_reattest("transfer-1", "token-abc")
        .await
        .unwrap();

    assert!(resp.retryable);
}
