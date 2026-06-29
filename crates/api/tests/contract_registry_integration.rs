//! Integration tests for the contract registry endpoints.
//!
//! # Structure
//!
//! **Serialization tests** (no DB, always run):
//!   Verify `ContractVersionMetadata` serializes with the exact OpenAPI field
//!   names and that wasm_hash / version are plain strings.
//!
//! **Live endpoint tests** (`#[ignore]`, require `DATABASE_URL`):
//!   Seed deterministic fixture rows, exercise all three routes, and clean up.
//!
//! Run the live tests:
//!
//! ```text
//! DATABASE_URL=postgres://... cargo test -p stellarroute-api \
//!   --test contract_registry_integration -- --ignored
//! ```
//!
//! # Fixture strategy
//!
//! Each live test seeds rows with a unique `contract_name` prefix and removes
//! them via `DELETE WHERE contract_name LIKE '{prefix}%'` at the end.
//! `ON CONFLICT DO UPDATE` makes seeds idempotent so re-runs never fail on the
//! unique constraint from `0011_contract_registry.sql`.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use sqlx::PgPool;
use stellarroute_api::{state::DatabasePools, Server, ServerConfig};
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn default_db_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
    })
}

/// Build a router backed by a real pool using the same `Server::new` path
/// that production uses.
async fn live_router(pool: PgPool) -> axum::Router {
    Server::new(ServerConfig::default(), DatabasePools::new(pool, None))
        .await
        .into_router()
}

/// Drain a response body and parse it as JSON.
async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("failed to read response body");
    serde_json::from_slice(&bytes).expect("response body is not valid JSON")
}

/// Seed one row into `contract_registry`.
/// Uses `ON CONFLICT DO UPDATE` so repeated runs are idempotent.
async fn seed(
    pool: &PgPool,
    contract_name: &str,
    version: &str,
    wasm_hash: &str,
    network: &str,
    contract_address: Option<&str>,
    deployed_at: Option<i64>,
    git_commit: Option<&str>,
) {
    sqlx::query(
        r#"
        INSERT INTO contract_registry
            (contract_name, version, wasm_hash, network,
             contract_address, deployed_at, git_commit)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (contract_name, version, network) DO UPDATE
            SET wasm_hash        = EXCLUDED.wasm_hash,
                contract_address = EXCLUDED.contract_address,
                deployed_at      = EXCLUDED.deployed_at,
                git_commit       = EXCLUDED.git_commit,
                updated_at       = NOW()
        "#,
    )
    .bind(contract_name)
    .bind(version)
    .bind(wasm_hash)
    .bind(network)
    .bind(contract_address)
    .bind(deployed_at)
    .bind(git_commit)
    .execute(pool)
    .await
    .expect("seed failed");
}

/// Remove all rows whose `contract_name` begins with `prefix`.
async fn cleanup(pool: &PgPool, prefix: &str) {
    sqlx::query("DELETE FROM contract_registry WHERE contract_name LIKE $1")
        .bind(format!("{prefix}%"))
        .execute(pool)
        .await
        .expect("cleanup failed");
}

// ---------------------------------------------------------------------------
// Serialization tests (no DB required)
// ---------------------------------------------------------------------------

/// All field names must be snake_case matching the OpenAPI schema.
/// No camelCase variants are permitted.
#[test]
fn contract_version_metadata_field_names_are_snake_case() {
    use stellarroute_api::routes::contract_registry::ContractVersionMetadata;

    let meta = ContractVersionMetadata {
        contract_name: "stellar_router".to_string(),
        version: "1.2.3".to_string(),
        wasm_hash: "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
        network: "mainnet".to_string(),
        contract_address: Some("CADDR00000000000000000000000000000000000000000000000000001".to_string()),
        deployed_at: Some(1_700_000_000),
        git_commit: Some("abc1234".to_string()),
    };

    let json = serde_json::to_value(&meta).expect("serialization must not fail");

    // Required fields present with correct names
    assert_eq!(json["contract_name"], "stellar_router");
    assert_eq!(json["version"], "1.2.3");
    assert_eq!(json["wasm_hash"], "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890");
    assert_eq!(json["network"], "mainnet");

    // No camelCase leakage
    assert!(json.get("contractName").is_none(), "must not have camelCase 'contractName'");
    assert!(json.get("wasmHash").is_none(),     "must not have camelCase 'wasmHash'");
    assert!(json.get("contractAddress").is_none(), "must not have camelCase 'contractAddress'");
    assert!(json.get("deployedAt").is_none(),   "must not have camelCase 'deployedAt'");
    assert!(json.get("gitCommit").is_none(),    "must not have camelCase 'gitCommit'");
}

/// Optional fields serialize as JSON `null` when `None` (keys present, value null).
#[test]
fn contract_version_metadata_none_fields_serialize_as_null() {
    use stellarroute_api::routes::contract_registry::ContractVersionMetadata;

    let meta = ContractVersionMetadata {
        contract_name: "router".to_string(),
        version: "1.0.0".to_string(),
        wasm_hash: "00".to_string(),
        network: "testnet".to_string(),
        contract_address: None,
        deployed_at: None,
        git_commit: None,
    };

    let json = serde_json::to_value(&meta).expect("serialization must not fail");

    // Keys must be present
    assert!(json.get("contract_address").is_some(), "contract_address key must be present");
    assert!(json.get("deployed_at").is_some(),      "deployed_at key must be present");
    assert!(json.get("git_commit").is_some(),       "git_commit key must be present");

    // Values must be null (not omitted)
    assert!(json["contract_address"].is_null(), "contract_address must be null when None");
    assert!(json["deployed_at"].is_null(),      "deployed_at must be null when None");
    assert!(json["git_commit"].is_null(),       "git_commit must be null when None");
}

/// wasm_hash must round-trip verbatim as a plain hex string.
#[test]
fn wasm_hash_round_trips_as_plain_hex_string() {
    use stellarroute_api::routes::contract_registry::ContractVersionMetadata;

    let hash = "a3f2c1d4e5b6a7908192a3b4c5d6e7f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6";
    let meta = ContractVersionMetadata {
        contract_name: "r".to_string(),
        version: "1.0.0".to_string(),
        wasm_hash: hash.to_string(),
        network: "testnet".to_string(),
        contract_address: None,
        deployed_at: None,
        git_commit: None,
    };

    let json = serde_json::to_value(&meta).expect("serialization failed");
    assert_eq!(
        json["wasm_hash"].as_str().unwrap(),
        hash,
        "wasm_hash must round-trip verbatim with no encoding change"
    );
}

/// version is a plain semver string — no numeric conversion.
#[test]
fn version_field_is_a_plain_string() {
    use stellarroute_api::routes::contract_registry::ContractVersionMetadata;

    let meta = ContractVersionMetadata {
        contract_name: "r".to_string(),
        version: "3.14.159".to_string(),
        wasm_hash: "00".to_string(),
        network: "mainnet".to_string(),
        contract_address: None,
        deployed_at: None,
        git_commit: None,
    };

    let json = serde_json::to_value(&meta).expect("serialization failed");
    assert_eq!(json["version"].as_str().unwrap(), "3.14.159");
}

// ---------------------------------------------------------------------------
// Route 1 — GET /api/v1/contracts/registry
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn list_returns_200_and_array() {
    let pool = PgPool::connect(&default_db_url()).await.expect("connect");
    let router = live_router(pool).await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/contracts/registry")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    assert!(json.is_array(), "response must be a JSON array");
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn list_includes_seeded_rows_with_correct_field_shape() {
    let pool = PgPool::connect(&default_db_url()).await.expect("connect");
    let prefix = "t835_list_";
    let name_a = format!("{prefix}router");
    let name_b = format!("{prefix}amm");

    seed(
        &pool, &name_a, "1.0.0",
        "aabb0000000000000000000000000000000000000000000000000000000000aa",
        "testnet",
        Some("CTESTADDR000000000000000000000000000000000000000000000001"),
        Some(1_700_000_100),
        Some("abc111"),
    ).await;
    seed(
        &pool, &name_b, "2.1.0",
        "ccdd0000000000000000000000000000000000000000000000000000000000cc",
        "testnet",
        None, Some(1_700_000_200), None,
    ).await;

    let router = live_router(pool.clone()).await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/contracts/registry")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let json = body_json(response).await;
    let contracts = json.as_array().expect("must be an array");

    let seeded: Vec<&Value> = contracts
        .iter()
        .filter(|c| {
            c["contract_name"]
                .as_str()
                .map(|n| n.starts_with(prefix))
                .unwrap_or(false)
        })
        .collect();

    assert_eq!(seeded.len(), 2, "both seeded rows must appear in list");

    // Verify spec field shape on one item
    let item = seeded[0];
    assert!(item["contract_name"].is_string(), "contract_name must be a string");
    assert!(item["version"].is_string(),       "version must be a string");
    assert!(item["wasm_hash"].is_string(),     "wasm_hash must be a string");
    assert!(item["network"].is_string(),       "network must be a string");
    assert!(item.get("contract_address").is_some(), "contract_address key must be present");
    assert!(item.get("deployed_at").is_some(),      "deployed_at key must be present");
    assert!(item.get("git_commit").is_some(),       "git_commit key must be present");

    cleanup(&pool, prefix).await;
}

// ---------------------------------------------------------------------------
// Route 2 — GET /api/v1/contracts/registry/:contract_name
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn get_by_name_returns_correct_fields() {
    let pool = PgPool::connect(&default_db_url()).await.expect("connect");
    let prefix = "t835_get_";
    let name = format!("{prefix}router");

    seed(
        &pool, &name, "1.5.0",
        "deadbeef00112233445566778899aabbccddeeff00112233445566778899aabb",
        "mainnet",
        Some("CMAINNET000000000000000000000000000000000000000000000002"),
        Some(1_700_001_000),
        Some("def5678"),
    ).await;

    let router = live_router(pool.clone()).await;

    let response = router
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/contracts/registry/{name}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK, "known contract must return 200");

    let json = body_json(response).await;

    assert_eq!(json["contract_name"].as_str().unwrap(), name);
    assert_eq!(json["version"].as_str().unwrap(), "1.5.0");
    assert_eq!(
        json["wasm_hash"].as_str().unwrap(),
        "deadbeef00112233445566778899aabbccddeeff00112233445566778899aabb",
        "wasm_hash must match seeded value verbatim"
    );
    assert_eq!(json["network"].as_str().unwrap(), "mainnet");
    assert_eq!(
        json["contract_address"].as_str().unwrap(),
        "CMAINNET000000000000000000000000000000000000000000000002"
    );
    assert_eq!(json["deployed_at"].as_i64().unwrap(), 1_700_001_000_i64);
    assert_eq!(json["git_commit"].as_str().unwrap(), "def5678");

    cleanup(&pool, prefix).await;
}

/// wasm_hash must be returned as plain hex with no base64 or 0x prefix.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn get_by_name_wasm_hash_is_verbatim_hex() {
    let pool = PgPool::connect(&default_db_url()).await.expect("connect");
    let prefix = "t835_hash_";
    let name = format!("{prefix}router");
    let hex = "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20";

    seed(&pool, &name, "1.0.0", hex, "testnet", None, None, None).await;

    let router = live_router(pool.clone()).await;

    let response = router
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/contracts/registry/{name}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    assert_eq!(
        json["wasm_hash"].as_str().unwrap(), hex,
        "wasm_hash must be returned verbatim without encoding transformation"
    );

    cleanup(&pool, prefix).await;
}

// ---------------------------------------------------------------------------
// Route 2 — 404 behavior
// ---------------------------------------------------------------------------

/// Unknown contract name must return 404 with the standard error envelope.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn get_by_name_returns_404_for_unknown_contract() {
    let pool = PgPool::connect(&default_db_url()).await.expect("connect");
    let router = live_router(pool).await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/contracts/registry/nonexistent_contract_xyz_835")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::NOT_FOUND, "unknown contract must return 404");

    let json = body_json(response).await;

    // Standard error envelope: { v, timestamp, request_id, data: { error, message } }
    assert_eq!(json["v"].as_u64().unwrap_or(0), 1, "envelope version must be 1");
    assert_eq!(
        json["data"]["error"].as_str().unwrap_or(""),
        "not_found",
        "error code must be 'not_found'"
    );
    assert!(
        json["data"]["message"].as_str().is_some(),
        "error body must include a message"
    );
}

/// The 404 message must contain the requested contract name.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn get_by_name_404_message_names_the_contract() {
    let pool = PgPool::connect(&default_db_url()).await.expect("connect");
    let router = live_router(pool).await;
    let unknown = "totally_unknown_835_xyz";

    let response = router
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/contracts/registry/{unknown}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let json = body_json(response).await;
    let msg = json["data"]["message"].as_str().unwrap_or("");
    assert!(
        msg.contains(unknown),
        "404 message must name the requested contract; got: '{msg}'"
    );
}

// ---------------------------------------------------------------------------
// Route 3 — GET /api/v1/contracts/registry/:contract_name/network/:network
// ---------------------------------------------------------------------------

/// Querying a specific network returns only the row for that network.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn get_by_network_returns_network_specific_row() {
    let pool = PgPool::connect(&default_db_url()).await.expect("connect");
    let prefix = "t835_net_";
    let name = format!("{prefix}router");

    // Same contract, two networks, different hashes
    seed(
        &pool, &name, "1.0.0",
        "aaaa0000000000000000000000000000000000000000000000000000000000aa",
        "mainnet",
        Some("CMAINNET000000000000000000000000000000000000000000000003"),
        Some(1_700_002_000),
        Some("main111"),
    ).await;
    seed(
        &pool, &name, "1.0.0",
        "bbbb0000000000000000000000000000000000000000000000000000000000bb",
        "testnet",
        Some("CTESTNET000000000000000000000000000000000000000000000004"),
        Some(1_700_002_100),
        Some("test222"),
    ).await;

    let router = live_router(pool.clone()).await;

    let response = router
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/contracts/registry/{name}/network/testnet"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK, "known contract+network must return 200");

    let json = body_json(response).await;
    assert_eq!(json["contract_name"].as_str().unwrap(), name);
    assert_eq!(json["network"].as_str().unwrap(), "testnet", "must return the requested network");
    assert_eq!(
        json["wasm_hash"].as_str().unwrap(),
        "bbbb0000000000000000000000000000000000000000000000000000000000bb",
        "must return the testnet-specific wasm_hash"
    );
    assert_eq!(json["git_commit"].as_str().unwrap(), "test222");

    cleanup(&pool, prefix).await;
}

/// Valid contract on an undeployed network returns 404 with structured body.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn get_by_network_returns_404_for_undeployed_network() {
    let pool = PgPool::connect(&default_db_url()).await.expect("connect");
    let prefix = "t835_netmiss_";
    let name = format!("{prefix}router");

    // Only mainnet seeded
    seed(
        &pool, &name, "1.0.0",
        "cccc0000000000000000000000000000000000000000000000000000000000cc",
        "mainnet", None, Some(1_700_003_000), None,
    ).await;

    let router = live_router(pool.clone()).await;

    let response = router
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/contracts/registry/{name}/network/futurenet"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(
        response.status(), StatusCode::NOT_FOUND,
        "contract on undeployed network must return 404"
    );

    let json = body_json(response).await;
    assert_eq!(
        json["data"]["error"].as_str().unwrap_or(""),
        "not_found",
        "error code must be 'not_found'"
    );
    assert!(json["data"]["message"].as_str().is_some(), "error must include a message");

    cleanup(&pool, prefix).await;
}

/// The 404 message for the network route must name both the contract and the network.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn get_by_network_404_message_names_contract_and_network() {
    let pool = PgPool::connect(&default_db_url()).await.expect("connect");
    let router = live_router(pool).await;

    let unknown_name    = "totally_unknown_835_xyz";
    let unknown_network = "futurenet";

    let response = router
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/contracts/registry/{unknown_name}/network/{unknown_network}"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let json = body_json(response).await;
    let msg = json["data"]["message"].as_str().unwrap_or("");
    assert!(msg.contains(unknown_name),    "404 message must name the contract; got: '{msg}'");
    assert!(msg.contains(unknown_network), "404 message must name the network; got: '{msg}'");
}
