//! Integration tests for the contract registry endpoints.
//!
//! # Structure
//!
//! **Model / serialization tests** (always run, no DB):
//!   Verify that `ContractVersionMetadata` serializes with the exact field
//!   names and types specified in the OpenAPI schema.
//!
//! **Live endpoint tests** (`#[ignore]`, require `DATABASE_URL`):
//!   Cover all three registry routes using a seeded fixture row so the
//!   assertions are deterministic regardless of whatever data the dev DB holds.
//!
//! Run the live tests with:
//!
//! ```text
//! DATABASE_URL=postgres://... cargo test -p stellarroute-api \
//!   --test contract_registry_integration -- --ignored
//! ```
//!
//! # Fixture strategy
//!
//! Each live test function seeds exactly the rows it needs using a unique
//! `contract_name` prefix derived from the test (to avoid cross-test
//! interference when tests run concurrently), and deletes those rows in a
//! `finally` block via a `scopeguard::defer!`-style cleanup implemented
//! with `sqlx::query`. The `contract_registry` table is created by migration
//! `0011_contract_registry.sql` which this file does not re-run; it expects
//! the migration to have been applied already.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use sqlx::PgPool;
use stellarroute_api::{state::DatabasePools, Server, ServerConfig};
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn default_db_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
    })
}

/// Read a response body to bytes and parse as JSON.
async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("failed to read response body");
    serde_json::from_slice(&bytes).expect("response body is not valid JSON")
}

/// Seed one contract registry row and return a cleanup closure that removes it.
///
/// The row uses the migration schema from `0011_contract_registry.sql`:
///   contract_name, version, wasm_hash, network, contract_address,
///   deployed_at, git_commit
async fn seed_contract(
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
    .expect("failed to seed contract registry fixture");
}

/// Remove all rows whose `contract_name` starts with `prefix`.
async fn cleanup_contracts(pool: &PgPool, prefix: &str) {
    sqlx::query("DELETE FROM contract_registry WHERE contract_name LIKE $1")
        .bind(format!("{prefix}%"))
        .execute(pool)
        .await
        .expect("failed to clean up contract registry fixtures");
}

// ---------------------------------------------------------------------------
// Model / serialization tests (no DB required)
// ---------------------------------------------------------------------------

/// ContractVersionMetadata must serialize to the exact field names in the
/// OpenAPI schema: snake_case identifiers for all fields.
#[test]
fn contract_version_metadata_serializes_to_spec_shape() {
    use stellarroute_api::routes::contract_registry::ContractVersionMetadata;

    let meta = ContractVersionMetadata {
        contract_name: "stellar_router".to_string(),
        version: "1.2.3".to_string(),
        wasm_hash: "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
        network: "mainnet".to_string(),
        contract_address: Some(
            "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4".to_string(),
        ),
        deployed_at: Some(1_700_000_000),
        git_commit: Some("abc1234".to_string()),
    };

    let json = serde_json::to_value(&meta).expect("serialization must not fail");

    // Required fields
    assert_eq!(json["contract_name"], "stellar_router");
    assert_eq!(json["version"], "1.2.3");
    assert_eq!(
        json["wasm_hash"],
        "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
    );
    assert_eq!(json["network"], "mainnet");

    // Optional fields present when Some(…)
    assert_eq!(
        json["contract_address"],
        "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4"
    );
    assert_eq!(json["deployed_at"], 1_700_000_000_i64);
    assert_eq!(json["git_commit"], "abc1234");

    // No camelCase leakage
    assert!(
        json.get("contractName").is_none(),
        "field name must be snake_case 'contract_name'"
    );
    assert!(
        json.get("wasmHash").is_none(),
        "field name must be snake_case 'wasm_hash'"
    );
    assert!(
        json.get("contractAddress").is_none(),
        "field name must be snake_case 'contract_address'"
    );
    assert!(
        json.get("deployedAt").is_none(),
        "field name must be snake_case 'deployed_at'"
    );
    assert!(
        json.get("gitCommit").is_none(),
        "field name must be snake_case 'git_commit'"
    );
}

/// Optional fields must serialize as JSON `null` when `None`, not be omitted.
/// The OpenAPI schema marks them as nullable rather than skip_serializing_if.
#[test]
fn contract_version_metadata_optional_fields_serialize_as_null_when_absent() {
    use stellarroute_api::routes::contract_registry::ContractVersionMetadata;

    let meta = ContractVersionMetadata {
        contract_name: "stellar_router".to_string(),
        version: "1.0.0".to_string(),
        wasm_hash: "deadbeef".to_string(),
        network: "testnet".to_string(),
        contract_address: None,
        deployed_at: None,
        git_commit: None,
    };

    let json = serde_json::to_value(&meta).expect("serialization must not fail");

    assert!(
        json.get("contract_address").is_some(),
        "contract_address key must be present even when None"
    );
    assert!(
        json["contract_address"].is_null(),
        "contract_address must serialize as null when None"
    );
    assert!(
        json.get("deployed_at").is_some(),
        "deployed_at key must be present even when None"
    );
    assert!(
        json["deployed_at"].is_null(),
        "deployed_at must serialize as null when None"
    );
    assert!(
        json.get("git_commit").is_some(),
        "git_commit key must be present even when None"
    );
    assert!(
        json["git_commit"].is_null(),
        "git_commit must serialize as null when None"
    );
}

/// wasm_hash is a hex string — verify it round-trips without alteration.
#[test]
fn contract_version_metadata_wasm_hash_is_plain_hex_string() {
    use stellarroute_api::routes::contract_registry::ContractVersionMetadata;

    // 64-character lower-hex SHA-256 style hash (typical Soroban WASM hash)
    let hash = "a3f2c1d4e5b6a7908192a3b4c5d6e7f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6";
    let meta = ContractVersionMetadata {
        contract_name: "router".to_string(),
        version: "2.0.0".to_string(),
        wasm_hash: hash.to_string(),
        network: "futurenet".to_string(),
        contract_address: None,
        deployed_at: None,
        git_commit: None,
    };

    let json = serde_json::to_value(&meta).expect("serialization failed");
    assert_eq!(
        json["wasm_hash"].as_str().unwrap(),
        hash,
        "wasm_hash must round-trip verbatim"
    );
}

/// version is a semver string — verify it round-trips and is a plain string.
#[test]
fn contract_version_metadata_version_is_semver_string() {
    use stellarroute_api::routes::contract_registry::ContractVersionMetadata;

    let meta = ContractVersionMetadata {
        contract_name: "router".to_string(),
        version: "3.14.159".to_string(),
        wasm_hash: "00".to_string(),
        network: "mainnet".to_string(),
        contract_address: None,
        deployed_at: None,
        git_commit: None,
    };

    let json = serde_json::to_value(&meta).expect("serialization failed");
    assert_eq!(
        json["version"].as_str().unwrap(),
        "3.14.159",
        "version must be a plain string"
    );
}

// ---------------------------------------------------------------------------
// Live endpoint tests (require DATABASE_URL and a running PostgreSQL instance)
// ---------------------------------------------------------------------------

/// Helper: build a router backed by a real PgPool, using the same
/// `Server::new` path that production uses.
async fn live_router(pool: PgPool) -> axum::Router {
    Server::new(ServerConfig::default(), DatabasePools::new(pool, None))
        .await
        .into_router()
}

// ---------------------------------------------------------------------------
// Route 1: GET /api/v1/contracts/registry  (list all)
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn list_contract_versions_returns_200_with_seeded_rows() {
    let pool = PgPool::connect(&default_db_url())
        .await
        .expect("failed to connect to database");

    // Unique prefix so parallel test runs don't interfere.
    let prefix = "test_list_";
    let name_a = format!("{prefix}router");
    let name_b = format!("{prefix}amm");

    seed_contract(
        &pool,
        &name_a,
        "1.0.0",
        "aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899",
        "testnet",
        Some("CTEST0000000000000000000000000000000000000000000000000001"),
        Some(1_700_000_100),
        Some("abc1111"),
    )
    .await;
    seed_contract(
        &pool,
        &name_b,
        "2.1.0",
        "ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100",
        "testnet",
        None,
        Some(1_700_000_200),
        None,
    )
    .await;

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

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "list endpoint must return 200"
    );

    let json = body_json(response).await;

    let contracts = json.as_array().expect("response body must be a JSON array");

    // At minimum our two seeded rows must be present.
    let seeded: Vec<&Value> = contracts
        .iter()
        .filter(|c| {
            c["contract_name"]
                .as_str()
                .map(|n| n.starts_with(prefix))
                .unwrap_or(false)
        })
        .collect();

    assert_eq!(
        seeded.len(),
        2,
        "both seeded contracts must appear in the list"
    );

    // Verify spec field shape on the first seeded item.
    let item = seeded[0];
    assert!(
        item.get("contract_name").and_then(|v| v.as_str()).is_some(),
        "contract_name must be a string"
    );
    assert!(
        item.get("version").and_then(|v| v.as_str()).is_some(),
        "version must be a string"
    );
    assert!(
        item.get("wasm_hash").and_then(|v| v.as_str()).is_some(),
        "wasm_hash must be a string"
    );
    assert!(
        item.get("network").and_then(|v| v.as_str()).is_some(),
        "network must be a string"
    );
    assert!(
        item.get("contract_address").is_some(),
        "contract_address key must always be present"
    );
    assert!(
        item.get("deployed_at").is_some(),
        "deployed_at key must always be present"
    );
    assert!(
        item.get("git_commit").is_some(),
        "git_commit key must always be present"
    );

    cleanup_contracts(&pool, prefix).await;
}

/// The list endpoint must return an empty array (not 404 or error) when the
/// table exists but contains no matching rows.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn list_contract_versions_returns_empty_array_when_no_rows() {
    let pool = PgPool::connect(&default_db_url())
        .await
        .expect("failed to connect to database");

    // Ensure no rows with this prefix exist.
    let prefix = "test_empty_list_";
    cleanup_contracts(&pool, prefix).await;

    let router = live_router(pool.clone()).await;

    // We can't assert the global list is empty (other data may exist), but
    // we can verify the response is 200 and an array.
    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/contracts/registry")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "list must return 200 even when table is empty"
    );

    let json = body_json(response).await;
    assert!(
        json.is_array(),
        "response body must be a JSON array, got: {json}"
    );
}

// ---------------------------------------------------------------------------
// Route 2: GET /api/v1/contracts/registry/:contract_name  (get by name)
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn get_contract_version_returns_200_with_correct_fields() {
    let pool = PgPool::connect(&default_db_url())
        .await
        .expect("failed to connect to database");

    let prefix = "test_get_";
    let name = format!("{prefix}router");

    seed_contract(
        &pool,
        &name,
        "1.5.0",
        "deadbeef00112233445566778899aabbccddeeff00112233445566778899aabb",
        "mainnet",
        Some("CMAINNET0000000000000000000000000000000000000000000000002"),
        Some(1_700_001_000),
        Some("def5678"),
    )
    .await;

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

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "get-by-name must return 200 for a known contract"
    );

    let json = body_json(response).await;

    // Required fields
    assert_eq!(
        json["contract_name"].as_str().unwrap(),
        name,
        "contract_name must match the requested name"
    );
    assert_eq!(
        json["version"].as_str().unwrap(),
        "1.5.0",
        "version must match seeded value"
    );
    assert_eq!(
        json["wasm_hash"].as_str().unwrap(),
        "deadbeef00112233445566778899aabbccddeeff00112233445566778899aabb",
        "wasm_hash must match seeded value and be plain hex"
    );
    assert_eq!(
        json["network"].as_str().unwrap(),
        "mainnet",
        "network must match seeded value"
    );

    // Optional fields present
    assert_eq!(
        json["contract_address"].as_str().unwrap(),
        "CMAINNET0000000000000000000000000000000000000000000000002",
        "contract_address must match seeded value"
    );
    assert_eq!(
        json["deployed_at"].as_i64().unwrap(),
        1_700_001_000,
        "deployed_at must be a Unix timestamp integer"
    );
    assert_eq!(
        json["git_commit"].as_str().unwrap(),
        "def5678",
        "git_commit must match seeded value"
    );

    cleanup_contracts(&pool, prefix).await;
}

/// WASM hash must be returned verbatim as a plain hex string with no encoding
/// transformations (e.g., no base64, no 0x prefix).
#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn get_contract_version_wasm_hash_is_plain_hex() {
    let pool = PgPool::connect(&default_db_url())
        .await
        .expect("failed to connect to database");

    let prefix = "test_hash_";
    let name = format!("{prefix}router");
    let hex_hash = "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20";

    seed_contract(
        &pool,
        &name,
        "1.0.0",
        hex_hash,
        "testnet",
        None,
        None,
        None,
    )
    .await;

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
        json["wasm_hash"].as_str().unwrap(),
        hex_hash,
        "wasm_hash must be returned verbatim with no encoding transformation"
    );

    cleanup_contracts(&pool, prefix).await;
}

// ---------------------------------------------------------------------------
// Route 2 — 404 behavior
// ---------------------------------------------------------------------------

/// Requesting an unknown contract name must return 404 with a structured
/// error body matching the API's standard envelope.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn get_contract_version_returns_404_for_unknown_name() {
    let pool = PgPool::connect(&default_db_url())
        .await
        .expect("failed to connect to database");

    let router = live_router(pool).await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/contracts/registry/nonexistent_contract_xyz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "unknown contract name must return 404"
    );

    let json = body_json(response).await;

    // Standard error envelope: { v, timestamp, request_id, data: { error, message } }
    assert_eq!(
        json["v"].as_u64().unwrap_or(0),
        1,
        "envelope version must be 1"
    );
    assert!(
        json["data"]["error"].as_str().is_some(),
        "structured error body must contain data.error"
    );
    assert!(
        json["data"]["message"].as_str().is_some(),
        "structured error body must contain data.message"
    );
    assert_eq!(
        json["data"]["error"].as_str().unwrap(),
        "not_found",
        "error code must be 'not_found'"
    );
}

/// The 404 message must mention the contract name so the caller knows which
/// resource was not found.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn get_contract_version_404_message_names_the_contract() {
    let pool = PgPool::connect(&default_db_url())
        .await
        .expect("failed to connect to database");

    let router = live_router(pool).await;
    let unknown = "totally_unknown_contract";

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
    let message = json["data"]["message"].as_str().unwrap_or("");
    assert!(
        message.contains(unknown),
        "404 message must name the requested contract; got: '{message}'"
    );
}

// ---------------------------------------------------------------------------
// Route 3: GET /api/v1/contracts/registry/:contract_name/network/:network
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn get_contract_version_by_network_returns_correct_row() {
    let pool = PgPool::connect(&default_db_url())
        .await
        .expect("failed to connect to database");

    let prefix = "test_net_";
    let name = format!("{prefix}router");

    // Seed the same contract on two different networks.
    seed_contract(
        &pool,
        &name,
        "1.0.0",
        "aaaa0000000000000000000000000000000000000000000000000000000000aa",
        "mainnet",
        Some("CMAINNET0000000000000000000000000000000000000000000000003"),
        Some(1_700_002_000),
        Some("main111"),
    )
    .await;
    seed_contract(
        &pool,
        &name,
        "1.0.0",
        "bbbb0000000000000000000000000000000000000000000000000000000000bb",
        "testnet",
        Some("CTESTNET0000000000000000000000000000000000000000000000004"),
        Some(1_700_002_100),
        Some("test222"),
    )
    .await;

    let router = live_router(pool.clone()).await;

    // Query specifically for the testnet row.
    let response = router
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/contracts/registry/{name}/network/testnet"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "get-by-network must return 200 for a known contract+network pair"
    );

    let json = body_json(response).await;

    assert_eq!(
        json["contract_name"].as_str().unwrap(),
        name,
        "contract_name must match"
    );
    assert_eq!(
        json["network"].as_str().unwrap(),
        "testnet",
        "network must be the requested network, not mainnet"
    );
    assert_eq!(
        json["wasm_hash"].as_str().unwrap(),
        "bbbb0000000000000000000000000000000000000000000000000000000000bb",
        "wasm_hash must be the testnet-specific hash"
    );
    assert_eq!(
        json["git_commit"].as_str().unwrap(),
        "test222",
        "git_commit must match the testnet row"
    );

    cleanup_contracts(&pool, prefix).await;
}

/// Requesting a valid contract on a network it was never deployed to must
/// return 404 with a structured error body.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn get_contract_version_by_network_returns_404_for_unknown_network() {
    let pool = PgPool::connect(&default_db_url())
        .await
        .expect("failed to connect to database");

    let prefix = "test_netmiss_";
    let name = format!("{prefix}router");

    // Only seed a mainnet row.
    seed_contract(
        &pool,
        &name,
        "1.0.0",
        "cccc0000000000000000000000000000000000000000000000000000000000cc",
        "mainnet",
        None,
        Some(1_700_003_000),
        None,
    )
    .await;

    let router = live_router(pool.clone()).await;

    // Ask for futurenet — which was never seeded.
    let response = router
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/contracts/registry/{name}/network/futurenet"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "contract on wrong network must return 404"
    );

    let json = body_json(response).await;

    assert_eq!(
        json["data"]["error"].as_str().unwrap_or(""),
        "not_found",
        "error code must be 'not_found'"
    );
    assert!(
        json["data"]["message"].as_str().is_some(),
        "error body must include a message"
    );

    cleanup_contracts(&pool, prefix).await;
}

/// Both the contract name and the network are mentioned in the 404 message so
/// the caller can distinguish "wrong network" from "unknown contract".
#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn get_contract_version_by_network_404_message_names_contract_and_network() {
    let pool = PgPool::connect(&default_db_url())
        .await
        .expect("failed to connect to database");

    let router = live_router(pool).await;
    let unknown_name = "totally_unknown_contract_xyz";
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
    let message = json["data"]["message"].as_str().unwrap_or("");

    assert!(
        message.contains(unknown_name),
        "404 message must name the contract; got: '{message}'"
    );
    assert!(
        message.contains(unknown_network),
        "404 message must name the network; got: '{message}'"
    );
}
