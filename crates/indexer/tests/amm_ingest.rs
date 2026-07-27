//! Proof that the AMM loop actually ingests reserves for registered testnet pools.
//!
//! This is an operator/CI verification test, not a unit test: it drives the real
//! [`AmmAggregator`] against a live Soroban **testnet** RPC and a real Postgres
//! database, then asserts that at least one row in `amm_pool_reserves` has
//! positive reserves with an `updated_at` newer than the moment the test started.
//!
//! It is `#[ignore]`d by default because it needs network + DB. Run it manually or
//! from a nightly CI job:
//!
//! ```bash
//! export DATABASE_URL=postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute
//! export ROUTER_CONTRACT_ADDRESS=C...        # required
//! export SOROBAN_RPC_URL=https://soroban-testnet.stellar.org
//! cargo test -p stellarroute-indexer amm_ingest -- --ignored --nocapture
//! ```
//!
//! Tunables (all optional):
//!
//! | Env var | Default | Meaning |
//! | --- | --- | --- |
//! | `AMM_INGEST_TIMEOUT_SECS` | `600` (10 min) | Give up after this long |
//! | `AMM_INGEST_POLL_SECS` | `15` | Delay between aggregation cycles |
//!
//! Mainnet is refused outright — this test must never be pointed at production.

use std::time::{Duration, Instant};

use sqlx::Row;
use stellarroute_indexer::amm::{AmmAggregator, AmmConfig};
use stellarroute_indexer::config::{HorizonMode, IndexerConfig};
use stellarroute_indexer::db::Database;
use stellarroute_indexer::soroban::{SorobanRpcClient, SorobanRpcConfig, StellarNetwork};

const DEFAULT_TIMEOUT_SECS: u64 = 600;
const DEFAULT_POLL_SECS: u64 = 15;

fn env_or(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

/// Reject anything that looks like mainnet. The acceptance criteria are explicit
/// that this proof must not require — or touch — production.
fn assert_not_mainnet(rpc_url: &str) {
    let lowered = rpc_url.to_lowercase();
    let looks_like_mainnet = lowered.contains("mainnet")
        || lowered.contains("soroban-rpc.stellar.org")
        || (lowered.contains("public") && !lowered.contains("testnet"));

    assert!(
        !looks_like_mainnet,
        "refusing to run AMM ingestion proof against what looks like mainnet: {rpc_url}\n\
         Point SOROBAN_RPC_URL at testnet (https://soroban-testnet.stellar.org) or a local RPC."
    );
}

fn test_config(database_url: String, soroban_rpc_url: String, router: String) -> IndexerConfig {
    IndexerConfig {
        stellar_horizon_url: "https://horizon-testnet.stellar.org".to_string(),
        horizon_mode: HorizonMode::Poll,
        soroban_rpc_url,
        router_contract_address: router,
        database_url,
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
        partition_count: 4,
        hot_pair_allowlist: String::new(),
        hot_pair_volume_threshold: 1_000_000_000,
        hot_pair_window_secs: 300,
        partition_id: 0,
    }
}

/// Every pool currently tracked in `amm_pool_reserves`, for failure diagnostics.
async fn tracked_pools(db: &Database) -> Vec<String> {
    sqlx::query("SELECT pool_address FROM amm_pool_reserves ORDER BY pool_address")
        .fetch_all(db.pool())
        .await
        .map(|rows| {
            rows.iter()
                .map(|row| row.get::<String, _>("pool_address"))
                .collect()
        })
        .unwrap_or_default()
}

/// Returns `(pool_address, updated_at)` for the first pool with positive reserves
/// refreshed after `started_at`.
async fn freshly_ingested_pool(
    db: &Database,
    started_at: chrono::DateTime<chrono::Utc>,
) -> Option<(String, chrono::DateTime<chrono::Utc>)> {
    sqlx::query(
        "SELECT pool_address, updated_at
           FROM amm_pool_reserves
          WHERE reserve_selling > 0
            AND reserve_buying > 0
            AND updated_at > $1
          ORDER BY updated_at DESC
          LIMIT 1",
    )
    .bind(started_at)
    .fetch_optional(db.pool())
    .await
    .ok()
    .flatten()
    .map(|row| {
        (
            row.get::<String, _>("pool_address"),
            row.get::<chrono::DateTime<chrono::Utc>, _>("updated_at"),
        )
    })
}

#[tokio::test]
#[ignore = "requires testnet Soroban RPC, a router contract address, and a live database"]
async fn amm_ingest_populates_reserves_for_registered_pools() {
    let database_url = env_or(
        "DATABASE_URL",
        "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute",
    );
    let soroban_rpc_url = env_or("SOROBAN_RPC_URL", "https://soroban-testnet.stellar.org");
    let router = std::env::var("ROUTER_CONTRACT_ADDRESS").expect(
        "ROUTER_CONTRACT_ADDRESS must be set — this test proves ingestion for a *registered* router",
    );

    assert_not_mainnet(&soroban_rpc_url);

    let timeout = Duration::from_secs(env_u64("AMM_INGEST_TIMEOUT_SECS", DEFAULT_TIMEOUT_SECS));
    let poll = Duration::from_secs(env_u64("AMM_INGEST_POLL_SECS", DEFAULT_POLL_SECS));

    let config = test_config(database_url, soroban_rpc_url.clone(), router.clone());
    let db = Database::new(&config).await.expect("database connect");
    db.migrate().await.expect("migrations applied");

    // Anchor on the database clock, not the test host's, so skew cannot produce a
    // false pass.
    let started_at: chrono::DateTime<chrono::Utc> = sqlx::query("SELECT now() AS now")
        .fetch_one(db.pool())
        .await
        .expect("read database clock")
        .get("now");

    let soroban = SorobanRpcClient::new(SorobanRpcConfig {
        base_url: soroban_rpc_url.clone(),
        ..SorobanRpcConfig::for_network(StellarNetwork::Testnet)
    })
    .expect("soroban rpc client");

    let aggregator = AmmAggregator::new(
        AmmConfig {
            router_contract: router.clone(),
            ..Default::default()
        },
        db.clone(),
        soroban,
    );

    println!(
        "amm_ingest: router={router} rpc={soroban_rpc_url} timeout={}s poll={}s start={started_at}",
        timeout.as_secs(),
        poll.as_secs(),
    );

    let deadline = Instant::now() + timeout;
    let mut last_rpc_error: Option<String> = None;
    let mut cycles = 0u32;

    loop {
        cycles += 1;
        match aggregator.aggregate_once().await {
            Ok(()) => println!("amm_ingest: cycle {cycles} completed"),
            Err(e) => {
                println!("amm_ingest: cycle {cycles} failed: {e}");
                last_rpc_error = Some(e.to_string());
            }
        }

        if let Some((pool, updated_at)) = freshly_ingested_pool(&db, started_at).await {
            println!(
                "amm_ingest: PASS — pool {pool} has fresh reserves (updated_at={updated_at}) after {cycles} cycle(s)"
            );
            return;
        }

        if Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(poll).await;
    }

    let pools = tracked_pools(&db).await;
    let pools_checked = if pools.is_empty() {
        "<none — no rows in amm_pool_reserves>".to_string()
    } else {
        pools.join(", ")
    };

    panic!(
        "AMM ingestion proof FAILED after {timeout_secs}s ({cycles} cycles).\n\
         No pool in `amm_pool_reserves` has positive reserves with updated_at > {started_at}.\n\
         \n\
         router contract: {router}\n\
         soroban rpc:     {soroban_rpc_url}\n\
         pools checked:   {pools_checked}\n\
         last RPC error:  {last_error}\n\
         \n\
         Likely causes: the router has no registered pools, discovery never reached the \
         pool-registration ledger, or the RPC rejected the getEvents/simulateTransaction calls.",
        timeout_secs = timeout.as_secs(),
        last_error =
            last_rpc_error.unwrap_or_else(|| "<none — all RPC calls succeeded>".to_string()),
    );
}
