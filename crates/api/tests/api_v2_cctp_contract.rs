//! Contract tests for Circle CCTP v2 bridge API surface (contract-freeze only).

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    response::IntoResponse,
};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use stellarroute_api::{
    error::ApiError, models::ApiErrorCode, state::DatabasePools, Server, ServerConfig,
};
use tower::ServiceExt;

const TRANSFER_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
const VALID_EVM_RECIPIENT: &str = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0";
const VALID_STELLAR_RECIPIENT: &str = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";
const VALID_EVM_TX_HASH: &str =
    "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
const VALID_STELLAR_TX_HASH: &str =
    "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

async fn setup_test_router() -> axum::Router {
    for key in [
        "CCTP_ENABLED",
        "CCTP_ACCESS_TOKEN_HMAC_KEY",
        "CCTP_STELLAR_RPC_URL",
        "CCTP_SEPOLIA_RPC_URL",
        "SEPOLIA_RPC_URL",
    ] {
        std::env::remove_var(key);
    }

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .expect("Failed to create lazy pool");

    Server::new(ServerConfig::default(), DatabasePools::new(pool, None))
        .await
        .into_router()
}

fn stellar_usdc() -> Value {
    json!({
        "chain_id": "stellar:testnet",
        "asset": "erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA",
        "canonical": "stellar:testnet/erc20:CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA",
        "symbol": "USDC"
    })
}

fn sepolia_usdc() -> Value {
    json!({
        "chain_id": "eip155:11155111",
        "asset": "erc20:0x1c7d4b196cb0c7b01d743fbc6116a902379c7238",
        "canonical": "eip155:11155111/erc20:0x1c7d4b196cb0c7b01d743fbc6116a902379c7238",
        "symbol": "USDC"
    })
}

fn sample_quote_body(direction: &str, finality: &str) -> Value {
    let (source_chain, dest_chain, source_asset, dest_asset, recipient) =
        if direction == "stellar_to_evm" {
            (
                "stellar:testnet",
                "eip155:11155111",
                stellar_usdc(),
                sepolia_usdc(),
                VALID_EVM_RECIPIENT,
            )
        } else {
            (
                "eip155:11155111",
                "stellar:testnet",
                sepolia_usdc(),
                stellar_usdc(),
                VALID_STELLAR_RECIPIENT,
            )
        };

    json!({
        "corridor_id": "circle-cctp:usdc:stellar-testnet:ethereum-sepolia",
        "provider": "circle-cctp",
        "direction": direction,
        "source_chain_id": source_chain,
        "destination_chain_id": dest_chain,
        "source_asset": source_asset,
        "destination_asset": dest_asset,
        "amount": "100.000000",
        "recipient": recipient,
        "finality": finality
    })
}

async fn post_json(router: &axum::Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(json!({}));
    (status, json)
}

async fn get_json(router: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let response = router
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(json!({}));
    (status, json)
}

fn assert_cctp_not_enabled(status: StatusCode, body: &Value) {
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["data"]["error"], "cctp_not_enabled");
}

fn assert_validation_error(status: StatusCode, body: &Value) {
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["data"]["error"], "validation_error");
}

async fn response_parts(err: ApiError) -> (u16, Value) {
    let response = err.into_response();
    let status = response.status().as_u16();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let envelope: Value = serde_json::from_slice(&body).expect("json");
    (status, envelope["data"].clone())
}

#[tokio::test]
async fn api_v2_info_backward_compatible_with_empty_supported_corridors() {
    let router = setup_test_router().await;
    let (status, body) = get_json(&router, "/api/v2").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["bridge_settlement_executable"], false);
    assert_eq!(
        body["data"]["supported_corridors"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
}

#[tokio::test]
async fn cctp_quote_rejects_invalid_sender_before_not_enabled() {
    let router = setup_test_router().await;
    let mut body = sample_quote_body("stellar_to_evm", "standard");
    body["sender"] = json!(VALID_EVM_RECIPIENT);

    let (status, resp) = post_json(&router, "/api/v2/bridge/cctp/quote", body).await;
    assert_validation_error(status, &resp);
}

#[tokio::test]
async fn openapi_registers_all_seven_cctp_paths_and_tag() {
    let router = setup_test_router().await;
    let (status, body) = get_json(&router, "/api-docs/openapi.json").await;
    assert_eq!(status, StatusCode::OK);

    let paths = [
        ("/api/v2/bridge/cctp/quote", "post"),
        ("/api/v2/bridge/cctp/{transfer_id}/prepare-burn", "post"),
        ("/api/v2/bridge/cctp/{transfer_id}/submit-burn", "post"),
        ("/api/v2/bridge/cctp/{transfer_id}", "get"),
        ("/api/v2/bridge/cctp/{transfer_id}/prepare-mint", "post"),
        ("/api/v2/bridge/cctp/{transfer_id}/submit-mint", "post"),
        ("/api/v2/bridge/cctp/{transfer_id}/reattest", "post"),
    ];

    for (path, method) in paths {
        let op = &body["paths"][path][method];
        assert!(
            !op.is_null(),
            "{method} {path} must be registered in OpenAPI"
        );
        let tags = op["tags"].as_array().expect("tags");
        assert!(
            tags.iter().any(|t| t == "cctp"),
            "{method} {path} must use cctp tag"
        );
    }
}

#[tokio::test]
async fn cctp_quote_accepts_stellar_source_fast_finality_then_fail_closed() {
    let router = setup_test_router().await;
    let (status, body) = post_json(
        &router,
        "/api/v2/bridge/cctp/quote",
        sample_quote_body("stellar_to_evm", "fast"),
    )
    .await;

    // Fast is valid for Stellar→EVM; with CCTP disabled the gate is not_enabled.
    assert_cctp_not_enabled(status, &body);
}

#[tokio::test]
async fn cctp_quote_fail_closed_when_valid() {
    let router = setup_test_router().await;
    let (status, body) = post_json(
        &router,
        "/api/v2/bridge/cctp/quote",
        sample_quote_body("stellar_to_evm", "standard"),
    )
    .await;
    assert_cctp_not_enabled(status, &body);
}

#[tokio::test]
async fn cctp_quote_rejects_unsupported_corridor_before_not_enabled() {
    let router = setup_test_router().await;
    let mut body = sample_quote_body("stellar_to_evm", "standard");
    body["corridor_id"] = json!("wrong-corridor");

    let (status, resp) = post_json(&router, "/api/v2/bridge/cctp/quote", body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(resp["data"]["error"], "unsupported_corridor");
}

#[tokio::test]
async fn cctp_quote_rejects_invalid_recipient_before_not_enabled() {
    let router = setup_test_router().await;
    let mut body = sample_quote_body("stellar_to_evm", "standard");
    body["recipient"] = json!(VALID_STELLAR_RECIPIENT);

    let (status, resp) = post_json(&router, "/api/v2/bridge/cctp/quote", body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(resp["data"]["error"], "invalid_recipient");
}

#[tokio::test]
async fn cctp_quote_rejects_invalid_amount_before_not_enabled() {
    let router = setup_test_router().await;
    let mut body = sample_quote_body("evm_to_stellar", "standard");
    body["amount"] = json!("0");

    let (status, resp) = post_json(&router, "/api/v2/bridge/cctp/quote", body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(resp["data"]["error"], "invalid_amount");
}

#[tokio::test]
async fn cctp_quote_rejects_unknown_top_level_fields() {
    let router = setup_test_router().await;
    let mut body = sample_quote_body("evm_to_stellar", "standard");
    body["unexpected_field"] = json!("nope");

    let (status, _) = post_json(&router, "/api/v2/bridge/cctp/quote", body).await;
    assert!(
        status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY,
        "unknown top-level fields must be rejected, got {status}"
    );
}

#[tokio::test]
async fn cctp_quote_rejects_unknown_nested_asset_fields() {
    let router = setup_test_router().await;
    let mut body = sample_quote_body("stellar_to_evm", "standard");
    body["source_asset"]["extra"] = json!(1);

    let (status, _) = post_json(&router, "/api/v2/bridge/cctp/quote", body).await;
    assert!(
        status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY,
        "unknown nested asset fields must be rejected, got {status}"
    );
}

#[tokio::test]
async fn cctp_transfer_endpoints_reject_malformed_transfer_id() {
    let router = setup_test_router().await;
    let bad_id = "not-a-uuid";

    let get_paths = [format!("/api/v2/bridge/cctp/{bad_id}")];
    for path in get_paths {
        let (status, body) = get_json(&router, &path).await;
        assert_validation_error(status, &body);
    }

    let post_paths = [
        (
            format!("/api/v2/bridge/cctp/{bad_id}/prepare-burn"),
            json!({}),
        ),
        (
            format!("/api/v2/bridge/cctp/{bad_id}/submit-burn"),
            json!({ "tx_hash": VALID_EVM_TX_HASH }),
        ),
        (
            format!("/api/v2/bridge/cctp/{bad_id}/prepare-mint"),
            json!({}),
        ),
        (
            format!("/api/v2/bridge/cctp/{bad_id}/submit-mint"),
            json!({ "tx_hash": VALID_STELLAR_TX_HASH }),
        ),
        (format!("/api/v2/bridge/cctp/{bad_id}/reattest"), json!({})),
    ];

    for (path, body) in post_paths {
        let (status, resp) = post_json(&router, &path, body).await;
        assert_validation_error(status, &resp);
    }
}

#[tokio::test]
async fn cctp_transfer_endpoints_fail_closed_with_valid_transfer_id() {
    let router = setup_test_router().await;

    let post_cases = [
        (
            format!("/api/v2/bridge/cctp/{TRANSFER_ID}/prepare-burn"),
            json!({}),
        ),
        (
            format!("/api/v2/bridge/cctp/{TRANSFER_ID}/submit-burn"),
            json!({ "tx_hash": VALID_EVM_TX_HASH }),
        ),
        (
            format!("/api/v2/bridge/cctp/{TRANSFER_ID}/prepare-mint"),
            json!({}),
        ),
        (
            format!("/api/v2/bridge/cctp/{TRANSFER_ID}/submit-mint"),
            json!({ "tx_hash": VALID_STELLAR_TX_HASH }),
        ),
        (
            format!("/api/v2/bridge/cctp/{TRANSFER_ID}/reattest"),
            json!({}),
        ),
    ];

    for (uri, body) in post_cases {
        let (status, json) = post_json(&router, &uri, body).await;
        assert_cctp_not_enabled(status, &json);
    }

    let (status, json) = get_json(&router, &format!("/api/v2/bridge/cctp/{TRANSFER_ID}")).await;
    assert_cctp_not_enabled(status, &json);
}

#[tokio::test]
async fn cctp_submit_rejects_malformed_and_empty_tx_hash() {
    let router = setup_test_router().await;

    for tx_hash in ["", "0xabc", "short", "  "] {
        let (status, body) = post_json(
            &router,
            &format!("/api/v2/bridge/cctp/{TRANSFER_ID}/submit-burn"),
            json!({ "tx_hash": tx_hash }),
        )
        .await;
        assert_validation_error(status, &body);
    }

    let body = json!({ "tx_hash": VALID_EVM_TX_HASH, "extra": true });
    let (status, _) = post_json(
        &router,
        &format!("/api/v2/bridge/cctp/{TRANSFER_ID}/submit-burn"),
        body,
    )
    .await;
    assert!(
        status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY,
        "unknown submit fields must be rejected, got {status}"
    );
}

#[tokio::test]
async fn openapi_documents_cctp_bridge_paths_and_schema_shapes() {
    let router = setup_test_router().await;
    let (status, body) = get_json(&router, "/api-docs/openapi.json").await;
    assert_eq!(status, StatusCode::OK);

    let schemas = &body["components"]["schemas"];

    let direction_enum: Vec<_> = schemas["CctpDirection"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(direction_enum, vec!["stellar_to_evm", "evm_to_stellar"]);

    let finality_enum: Vec<_> = schemas["CctpFinality"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(finality_enum, vec!["standard", "fast"]);

    let status_enum: Vec<_> = schemas["CctpTransferStatus"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(status_enum.contains(&"awaiting_attestation"));
    assert!(status_enum.contains(&"mint_failed_retryable"));

    let payload = &schemas["PreparedWalletPayload"];
    let one_of = payload["oneOf"]
        .as_array()
        .expect("PreparedWalletPayload oneOf");
    assert_eq!(one_of.len(), 2);
    assert!(one_of
        .iter()
        .any(|v| v["properties"]["type"]["enum"][0] == "stellar_xdr"));
    assert!(one_of
        .iter()
        .any(|v| v["properties"]["type"]["enum"][0] == "evm_transaction"));

    let asset_props = &schemas["CctpChainAsset"]["additionalProperties"];
    assert_eq!(asset_props, false);
}

#[test]
fn cctp_wire_models_use_snake_case() {
    use stellarroute_api::models::v2_cctp::{
        CctpDirection, CctpFinality, CctpTransferStatus, PreparedWalletPayload,
    };

    assert_eq!(
        serde_json::to_value(CctpDirection::StellarToEvm).unwrap(),
        "stellar_to_evm"
    );
    assert_eq!(
        serde_json::to_value(CctpFinality::Standard).unwrap(),
        "standard"
    );
    assert_eq!(
        serde_json::to_value(CctpTransferStatus::AwaitingAttestation).unwrap(),
        "awaiting_attestation"
    );

    let payload = PreparedWalletPayload::StellarXdr {
        network_passphrase: "Test SDF Network ; September 2015".into(),
        xdr_envelope: "AAAA".into(),
    };
    let json = serde_json::to_value(&payload).unwrap();
    assert_eq!(json["type"], "stellar_xdr");
}

#[tokio::test]
async fn cctp_api_error_http_mappings() {
    let (status, body) = response_parts(ApiError::CctpNotEnabled("disabled".into())).await;
    assert_eq!(status, 503);
    assert_eq!(body["error"], "cctp_not_enabled");

    let (status, body) = response_parts(ApiError::UnsupportedCorridor).await;
    assert_eq!(status, 400);
    assert_eq!(body["error"], "unsupported_corridor");

    let (status, body) = response_parts(ApiError::InvalidFinality).await;
    assert_eq!(status, 400);
    assert_eq!(body["error"], "invalid_finality");

    let (status, body) = response_parts(ApiError::InvalidRecipient).await;
    assert_eq!(status, 400);
    assert_eq!(body["error"], "invalid_recipient");

    let (status, body) = response_parts(ApiError::InvalidAmount("bad".into())).await;
    assert_eq!(status, 400);
    assert_eq!(body["error"], "invalid_amount");

    let (status, body) = response_parts(ApiError::FeeQuoteUnavailable("no quote".into())).await;
    assert_eq!(status, 503);
    assert_eq!(body["error"], "fee_quote_unavailable");

    let transfer_id = TRANSFER_ID.to_string();
    let (status, body) = response_parts(ApiError::AttestationPending {
        transfer_id: transfer_id.clone(),
    })
    .await;
    assert_eq!(status, 422);
    assert_eq!(body["error"], "attestation_pending");
    assert_eq!(body["details"]["transfer_id"], transfer_id);

    let (status, body) = response_parts(ApiError::AttestationExpired {
        transfer_id: transfer_id.clone(),
    })
    .await;
    assert_eq!(status, 422);
    assert_eq!(body["error"], "attestation_expired");

    let (status, body) = response_parts(ApiError::MintRetryable {
        transfer_id: transfer_id.clone(),
    })
    .await;
    assert_eq!(status, 422);
    assert_eq!(body["error"], "mint_retryable");
    assert_eq!(body["details"]["retryable"], true);

    let (status, body) = response_parts(ApiError::TransferNotFound {
        transfer_id: transfer_id.clone(),
    })
    .await;
    assert_eq!(status, 404);
    assert_eq!(body["error"], "transfer_not_found");

    let (status, body) = response_parts(ApiError::ProviderKilled("killed".into())).await;
    assert_eq!(status, 503);
    assert_eq!(body["error"], "provider_killed");
}

#[test]
fn cctp_error_codes_present_in_taxonomy_source_of_truth() {
    for code in [
        ApiErrorCode::CctpNotEnabled,
        ApiErrorCode::UnsupportedCorridor,
        ApiErrorCode::InvalidFinality,
        ApiErrorCode::InvalidRecipient,
        ApiErrorCode::FeeQuoteUnavailable,
        ApiErrorCode::AttestationPending,
        ApiErrorCode::AttestationExpired,
        ApiErrorCode::MintRetryable,
        ApiErrorCode::TransferNotFound,
        ApiErrorCode::ProviderKilled,
    ] {
        assert!(!code.as_str().is_empty());
    }
}

#[tokio::test]
async fn openapi_cctp_documents_access_and_idempotency_headers() {
    let router = setup_test_router().await;
    let (status, body) = get_json(&router, "/api-docs/openapi.json").await;
    assert_eq!(status, StatusCode::OK);

    let quote = &body["paths"]["/api/v2/bridge/cctp/quote"]["post"];
    let quote_params = quote["parameters"].as_array().expect("quote parameters");
    let idem = quote_params
        .iter()
        .find(|p| p["name"] == "Idempotency-Key")
        .expect("Idempotency-Key header on quote");
    assert_eq!(idem["in"], "header");
    assert_eq!(idem["required"], false);

    let transfer_paths = [
        ("/api/v2/bridge/cctp/{transfer_id}", "get"),
        ("/api/v2/bridge/cctp/{transfer_id}/prepare-burn", "post"),
        ("/api/v2/bridge/cctp/{transfer_id}/submit-burn", "post"),
        ("/api/v2/bridge/cctp/{transfer_id}/prepare-mint", "post"),
        ("/api/v2/bridge/cctp/{transfer_id}/submit-mint", "post"),
        ("/api/v2/bridge/cctp/{transfer_id}/reattest", "post"),
    ];

    for (path, method) in transfer_paths {
        let op = &body["paths"][path][method];
        let params = op["parameters"].as_array().expect("transfer parameters");
        let access = params
            .iter()
            .find(|p| p["name"] == "x-cctp-transfer-access")
            .unwrap_or_else(|| panic!("x-cctp-transfer-access on {method} {path}"));
        assert_eq!(access["in"], "header");
        assert_eq!(access["required"], true);
    }

    let quote_resp = quote["responses"]["409"]["description"]
        .as_str()
        .unwrap_or("");
    assert!(quote_resp.contains("Idempotency") || quote_resp.contains("idempotency"));
    assert!(quote["responses"]["425"].is_object());
    assert!(quote["responses"]["429"].is_object());

    let schemas = &body["components"]["schemas"];
    let openapi_blob = serde_json::to_string(&schemas).unwrap();
    for forbidden in [
        "access_token_hash",
        "response_json",
        "raw_message",
        "attestation",
        "xdr_envelope",
    ] {
        assert!(
            !schemas.as_object().unwrap().contains_key(forbidden),
            "schema must not expose secret field {forbidden}"
        );
    }
    assert!(
        !openapi_blob.contains("\"access_token\"") || openapi_blob.contains("CctpQuoteResponse"),
        "only quote response may document access_token wire field"
    );
}

fn fully_ready_cctp_router(
    kill_switch: std::sync::Arc<stellarroute_api::kill_switch::KillSwitchManager>,
    iris: std::sync::Arc<CountingContractIris>,
) -> axum::Router {
    use std::sync::Arc;
    use stellarroute_api::cctp::{
        access::test_access_token_keyring,
        attestation::FakeAttestationVerifier,
        bootstrap::CctpHttpContext,
        builders::{BuilderError, PreparedMintBundle, StellarCctpMintBuilder},
        config::CctpConfig,
        idempotency::InMemoryCctpQuoteIdempotencyStore,
        prepare_lock::InMemoryCctpPrepareLockStore,
        readiness::CctpRuntime,
        service::CctpService,
        store::{CctpTransferStore, InMemoryCctpTransferStore},
        verifiers::{FakeBurnVerifier, FakeMintVerifier, MintVerifyOutcome, VerifiedMintFacts},
    };
    use stellarroute_api::dependency_health::ExternalDependencyHealth;
    use stellarroute_api::models::v2_cctp::{SEPOLIA_CHAIN_ID, STELLAR_TESTNET_CHAIN_ID};
    use stellarroute_api::state::{AppState, DatabasePools};

    struct ReadyStellarMintBuilder;
    #[async_trait::async_trait]
    impl StellarCctpMintBuilder for ReadyStellarMintBuilder {
        fn is_ready(&self) -> bool {
            true
        }
        async fn prepare_mint(
            &self,
            transfer: &stellarroute_api::cctp::store::CctpTransfer,
            config: &CctpConfig,
        ) -> Result<PreparedMintBundle, BuilderError> {
            Ok(PreparedMintBundle {
                primary: stellarroute_api::models::v2_cctp::PreparedWalletPayload::StellarXdr {
                    network_passphrase: config.stellar_network_passphrase.clone(),
                    xdr_envelope: "AAAA".into(),
                },
                expires_at: transfer.quote_expires_at.timestamp(),
                payload_hash: "contract-test".into(),
            })
        }
    }

    let mut cfg = CctpConfig::default_testnet();
    cfg.enabled = true;
    cfg.sepolia_rpc_url = "https://sepolia.drpc.org".into();
    let mut runtime = CctpRuntime::from_config(&cfg);
    runtime.attestation_verifier = Arc::new(FakeAttestationVerifier { ready: true });
    runtime.stellar_mint_builder = Arc::new(ReadyStellarMintBuilder);
    runtime.stellar_mint_verifier = Arc::new(FakeMintVerifier {
        facts: VerifiedMintFacts {
            tx_hash: "mint".into(),
            destination_chain_id: STELLAR_TESTNET_CHAIN_ID.into(),
            contract_address: "CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA".into(),
            function_selector: "mint".into(),
            message_hash: [0; 32],
            attestation_hash: [0; 32],
            nonce: "nonce".into(),
            payload_hash: "contract-test".into(),
            outcome: MintVerifyOutcome::Pending,
            recipient_evidence: Some(VALID_STELLAR_RECIPIENT.into()),
        },
        completion: MintVerifyOutcome::Pending,
        ready: true,
    });
    struct ReadyEvmBurnBuilder;
    #[async_trait::async_trait]
    impl stellarroute_api::cctp::builders::EvmCctpBurnBuilder for ReadyEvmBurnBuilder {
        fn is_ready(&self) -> bool {
            true
        }
        async fn prepare_burn(
            &self,
            _: &stellarroute_api::cctp::store::CctpTransfer,
            _: &CctpConfig,
        ) -> Result<stellarroute_api::cctp::builders::PreparedBurnBundle, BuilderError> {
            Err(BuilderError::NotReady)
        }
    }
    runtime.evm_burn_builder = Arc::new(ReadyEvmBurnBuilder);
    runtime.evm_burn_verifier = Arc::new(FakeBurnVerifier {
        facts: stellarroute_api::cctp::verifiers::VerifiedBurnFacts {
            tx_hash: "burn".into(),
            source_chain_id: SEPOLIA_CHAIN_ID.into(),
            source_domain: 0,
            destination_domain: 27,
            sender: VALID_EVM_RECIPIENT.into(),
            amount_cctp_subunits: 1,
            burn_token_bytes32: [0; 32],
            mint_recipient_bytes32: [0; 32],
            destination_caller_bytes32: [0; 32],
            min_finality_threshold: 2000,
            hook_data: None,
            token_messenger_bytes32: [0; 32],
            block_or_ledger: None,
        },
        ready: true,
    });
    runtime.evm_approval_verifier =
        Arc::new(stellarroute_api::cctp::approval::FakeApprovalVerifier {
            facts: stellarroute_api::cctp::approval::VerifiedApprovalFacts {
                tx_hash: "approve".into(),
                owner: VALID_EVM_RECIPIENT.into(),
                token_contract: cfg.contracts.sepolia_usdc.clone(),
                spender_contract: cfg.contracts.sepolia_token_messenger.clone(),
                amount: 1,
                chain_id: SEPOLIA_CHAIN_ID.into(),
            },
            ready: true,
        });

    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let idempotency = Arc::new(InMemoryCctpQuoteIdempotencyStore::default());
    idempotency.bind_transfer_store(store.clone());
    let service = Arc::new(CctpService {
        config: cfg.clone(),
        store,
        prepare_lock: Arc::new(InMemoryCctpPrepareLockStore::default()),
        iris,
        kill_switch: kill_switch.clone(),
        runtime: runtime.clone(),
    });
    let ctx = Arc::new(CctpHttpContext {
        config: cfg,
        service,
        runtime,
        idempotency,
        access_token_keys: test_access_token_keyring(),
    });

    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unused")
        .expect("lazy pool");
    let mut state = AppState::new(DatabasePools::new(pool, None)).with_cctp(ctx);
    state.kill_switch = kill_switch;
    state.external_dependency_health = Arc::new(ExternalDependencyHealth::new(vec![], vec![]));
    stellarroute_api::routes::create_router(state.into_arc())
}

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use stellarroute_api::cctp::iris::{IrisClient, IrisFeeQuote, IrisPollOutcome};
use stellarroute_api::kill_switch::{KillSwitchManager, KillSwitchState};
use stellarroute_api::models::v2_cctp::CCTP_PROVIDER_ID;
use stellarroute_routing::health::policy::OverrideDirective;

struct CountingContractIris {
    fee_calls: AtomicUsize,
}

impl CountingContractIris {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            fee_calls: AtomicUsize::new(0),
        })
    }
    fn fee_count(&self) -> usize {
        self.fee_calls.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl IrisClient for CountingContractIris {
    async fn fetch_burn_fees(
        &self,
        _: u32,
        _: u32,
    ) -> Result<IrisFeeQuote, stellarroute_api::cctp::iris::IrisError> {
        self.fee_calls.fetch_add(1, Ordering::SeqCst);
        Ok(IrisFeeQuote {
            standard_fee: "1".into(),
            fast_fee: None,
        })
    }
    async fn poll_messages_by_tx(
        &self,
        _: u32,
        _: &str,
    ) -> Result<IrisPollOutcome, stellarroute_api::cctp::iris::IrisError> {
        Ok(IrisPollOutcome::Pending)
    }
    async fn reattest(&self, _: &str) -> Result<(), stellarroute_api::cctp::iris::IrisError> {
        Ok(())
    }
}

async fn set_circle_provider_kill(kill: &KillSwitchManager) {
    let mut state = kill.get_state().await;
    state
        .providers
        .insert(CCTP_PROVIDER_ID.into(), OverrideDirective::ForceExclude);
    kill.update_state(state).await.unwrap();
}

#[tokio::test]
async fn api_v2_metadata_and_quote_honor_provider_kill_snapshot() {
    let kill = Arc::new(KillSwitchManager::new(None));
    let iris = CountingContractIris::new();
    let router = fully_ready_cctp_router(kill.clone(), iris.clone());
    let mut quote_body = sample_quote_body("evm_to_stellar", "standard");
    quote_body["mint_submitter"] = json!(VALID_STELLAR_RECIPIENT);

    let (status, body) = get_json(&router, "/api/v2").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["bridge_settlement_executable"], true);
    assert!(body["data"]["supported_corridors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["executable"] == true));

    let (quote_status, quote_resp) =
        post_json(&router, "/api/v2/bridge/cctp/quote", quote_body.clone()).await;
    assert_eq!(quote_status, StatusCode::OK);
    assert!(quote_resp["data"]["access_token"].is_string());

    set_circle_provider_kill(&kill).await;
    let router_killed = fully_ready_cctp_router(kill.clone(), iris.clone());
    let fees_before = iris.fee_count();

    let (status2, body2) = get_json(&router_killed, "/api/v2").await;
    assert_eq!(status2, StatusCode::OK);
    assert_eq!(body2["data"]["bridge_settlement_executable"], false);
    assert!(body2["data"]["supported_corridors"]
        .as_array()
        .unwrap()
        .iter()
        .all(|c| c["executable"] == false));

    let (quote_status2, quote_resp2) = post_json(
        &router_killed,
        "/api/v2/bridge/cctp/quote",
        quote_body.clone(),
    )
    .await;
    assert_eq!(quote_status2, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(quote_resp2["data"]["error"], "provider_killed");
    assert_eq!(
        iris.fee_count(),
        fees_before,
        "no Iris fee fetch while killed"
    );

    kill.update_state(KillSwitchState::default()).await.unwrap();
    let router_cleared = fully_ready_cctp_router(kill, iris);
    let (status3, body3) = get_json(&router_cleared, "/api/v2").await;
    assert_eq!(status3, StatusCode::OK);
    assert_eq!(body3["data"]["bridge_settlement_executable"], true);
}
