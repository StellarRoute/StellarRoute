//! Integration tests for the indexer

use stellarroute_indexer::amm::{AmmAggregator, AmmConfig};
use stellarroute_indexer::config::IndexerConfig;
use stellarroute_indexer::db::Database;
use stellarroute_indexer::models::asset::Asset;
use stellarroute_indexer::soroban::{SorobanRpc, SorobanRpcClient};
use tracing::debug;

#[tokio::test]
#[ignore] // Requires database and Horizon API
async fn test_database_connection() {
    let config = IndexerConfig {
        stellar_horizon_url: "https://horizon-testnet.stellar.org".to_string(),
        soroban_rpc_url: "https://soroban-testnet.stellar.org".to_string(),
        router_contract_address: "CDUMMYROUTER".to_string(),
        database_url: std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
        }),
        poll_interval_secs: 5,
        amm_poll_interval_secs: 30,
        stale_threshold_secs: 300,
        horizon_limit: 200,
        max_connections: 5,
        min_connections: 1,
        connection_timeout_secs: 30,
        idle_timeout_secs: 600,
        max_lifetime_secs: 1800,
        maintenance_interval_mins: 60,
        snapshot_retention_days: 90,
        snapshot_compaction_hours: 24,
    };

    let db = Database::new(&config)
        .await
        .expect("Failed to connect to database");
    db.health_check().await.expect("Health check failed");
}

#[tokio::test]
#[ignore] // Requires Soroban RPC
async fn test_soroban_client_get_latest_ledger() {
    let client =
        SorobanRpcClient::for_network(stellarroute_indexer::soroban::StellarNetwork::Testnet)
            .expect("Failed to create Soroban client");
    let ledger: Result<u64, _> = client.get_latest_ledger().await;

    assert!(ledger.is_ok());
    if let Ok(ledger) = ledger {
        debug!(ledger, "Latest ledger");
        assert!(ledger > 0);
    }
}

#[tokio::test]
#[ignore] // Requires database and Soroban RPC
async fn test_amm_aggregator_initialization() {
    let config = IndexerConfig {
        stellar_horizon_url: "https://horizon-testnet.stellar.org".to_string(),
        soroban_rpc_url: "https://soroban-testnet.stellar.org".to_string(),
        router_contract_address: "CDUMMYROUTER".to_string(),
        database_url: std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
        }),
        poll_interval_secs: 5,
        amm_poll_interval_secs: 30,
        stale_threshold_secs: 300,
        horizon_limit: 200,
        max_connections: 5,
        min_connections: 1,
        connection_timeout_secs: 30,
        idle_timeout_secs: 600,
        max_lifetime_secs: 1800,
        maintenance_interval_mins: 60,
        snapshot_retention_days: 90,
        snapshot_compaction_hours: 24,
    };

    let db = Database::new(&config)
        .await
        .expect("Failed to connect to database");

    let soroban =
        SorobanRpcClient::for_network(stellarroute_indexer::soroban::StellarNetwork::Testnet)
            .expect("Failed to create Soroban client");

    let amm_config = AmmConfig {
        router_contract: config.router_contract_address,
        poll_interval_secs: config.amm_poll_interval_secs,
        stale_threshold_secs: config.stale_threshold_secs,
        batch_size: 10,
    };

    let aggregator = AmmAggregator::new(amm_config, db, soroban);

    // Test a single aggregation cycle (should not fail even with dummy data)
    let result = aggregator.aggregate_once().await;
    // We expect this to succeed or fail gracefully
    debug!("AMM aggregation result: {:?}", result);
}

#[tokio::test]
#[ignore] // Integration: seeds registry and verifies indexer picks up pools
async fn test_registry_seed_and_indexer_bootstrap() {
    let config = IndexerConfig {
        stellar_horizon_url: "https://horizon-testnet.stellar.org".to_string(),
        soroban_rpc_url: "https://soroban-testnet.stellar.org".to_string(),
        router_contract_address: "CDUMMYROUTER".to_string(),
        database_url: std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
        }),
        poll_interval_secs: 5,
        amm_poll_interval_secs: 30,
        stale_threshold_secs: 300,
        horizon_limit: 200,
        max_connections: 5,
        min_connections: 1,
        connection_timeout_secs: 30,
        idle_timeout_secs: 600,
        max_lifetime_secs: 1800,
        maintenance_interval_mins: 60,
        snapshot_retention_days: 90,
        snapshot_compaction_hours: 24,
    };

    let db = Database::new(&config)
        .await
        .expect("Failed to connect to database");

    // Seed the operator registry with a test pool
    let pool_addr = "CSEEDTESTPOOL";
    sqlx::query("INSERT INTO amm_pools (pool_address, network, active, metadata) VALUES ($1, $2, true, '{}'::jsonb) ON CONFLICT (pool_address) DO UPDATE SET active = true, updated_at = now();")
        .bind(pool_addr)
        .bind("testnet")
        .execute(db.pool())
        .await
        .expect("Failed to seed amm_pools registry");

    let soroban =
        SorobanRpcClient::for_network(stellarroute_indexer::soroban::StellarNetwork::Testnet)
            .expect("Failed to create Soroban client");

    let amm_config = AmmConfig {
        router_contract: config.router_contract_address,
        poll_interval_secs: config.amm_poll_interval_secs,
        stale_threshold_secs: config.stale_threshold_secs,
        batch_size: 10,
    };

    let aggregator = AmmAggregator::new(amm_config, db.clone(), soroban);

    // Run a single aggregation cycle which should pick up the seeded registry pool
    let _ = aggregator.aggregate_once().await;

    // Verify that the pool was processed and an entry exists in amm_pool_reserves
    let row = sqlx::query("SELECT pool_address FROM amm_pool_reserves WHERE pool_address = $1")
        .bind(pool_addr)
        .fetch_optional(db.pool())
        .await
        .expect("Query failed");

    assert!(
        row.is_some(),
        "Seeded registry pool was not processed into amm_pool_reserves"
    );
}

#[test]
fn test_asset_key_generation() {
    let native = Asset::Native;
    let (asset_type, code, issuer) = native.key();
    assert_eq!(asset_type, "native");
    assert_eq!(code, None);
    assert_eq!(issuer, None);

    let usdc = Asset::CreditAlphanum4 {
        asset_code: "USDC".to_string(),
        asset_issuer: "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string(),
    };
    let (asset_type, code, issuer) = usdc.key();
    assert_eq!(asset_type, "credit_alphanum4");
    assert_eq!(code, Some("USDC".to_string()));
    assert_eq!(
        issuer,
        Some("GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string())
    );
}
