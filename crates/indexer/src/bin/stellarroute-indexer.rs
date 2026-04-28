//! StellarRoute Indexer Binary
//!
//! Main entry point for the SDEX orderbook indexer service.

use std::process;
use tracing::{error, info};

use std::time::Duration;
use stellarroute_indexer::amm::{AmmAggregator, AmmConfig};
use stellarroute_indexer::asset_metadata::{MetadataEnrichmentJob, MetadataJobConfig};
use stellarroute_indexer::config::IndexerConfig;
use stellarroute_indexer::db::{archival::ArchivalManager, Database};
use stellarroute_indexer::horizon::HorizonClient;
use stellarroute_indexer::sdex::SdexIndexer;
use stellarroute_indexer::shutdown::IndexerShutdown;
use stellarroute_indexer::soroban::{RetryPolicy, SorobanRpc, SorobanRpcClient, SorobanRpcConfig};

fn parse_bool_env(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| {
            let v = value.trim().to_ascii_lowercase();
            matches!(v.as_str(), "1" | "true" | "yes" | "on")
        })
        .unwrap_or(false)
}

async fn run_startup_reachability_checks(
    config: &IndexerConfig,
    soroban: &SorobanRpcClient,
) -> std::result::Result<(), String> {
    let db_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await
        .map_err(|_| "Startup check failed: DATABASE_URL is not reachable".to_string())?;

    sqlx::query("SELECT 1")
        .execute(&db_pool)
        .await
        .map_err(|_| "Startup check failed: DATABASE_URL query failed".to_string())?;

    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|_| "Startup check failed: unable to create HTTP client".to_string())?;

    let horizon = format!("{}/", config.stellar_horizon_url.trim_end_matches('/'));
    let horizon_status =
        http.get(&horizon).send().await.map_err(|_| {
            "Startup check failed: STELLAR_HORIZON_URL is not reachable".to_string()
        })?;
    if !horizon_status.status().is_success() {
        return Err(
            "Startup check failed: STELLAR_HORIZON_URL returned non-success status".to_string(),
        );
    }

    soroban
        .get_latest_ledger()
        .await
        .map_err(|_| "Startup check failed: SOROBAN_RPC_URL is not reachable".to_string())?;

    Ok(())
}

#[tokio::main]
async fn main() {
    // Initialize structured logging (reads RUST_LOG and LOG_FORMAT env vars)
    stellarroute_indexer::telemetry::init();

    info!("Starting StellarRoute Indexer");

    // Load configuration
    let config = match IndexerConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            process::exit(1);
        }
    };

    // Initialize database
    let db = match Database::new(&config).await {
        Ok(db) => db,
        Err(e) => {
            error!("Failed to connect to database: {}", e);
            process::exit(1);
        }
    };

    // Run migrations
    if let Err(e) = db.migrate().await {
        error!("Failed to run migrations: {}", e);
        process::exit(1);
    }

    // Initialize Horizon client
    let horizon = HorizonClient::new(&config.stellar_horizon_url);

    // Initialize Soroban RPC client
    let soroban = match SorobanRpcClient::new(SorobanRpcConfig {
        base_url: config.soroban_rpc_url.clone(),
        timeout_secs: 30,
        retry: RetryPolicy::default(),
    }) {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to create Soroban RPC client: {}", e);
            process::exit(1);
        }
    };

    if parse_bool_env("STARTUP_CREDENTIAL_CHECK") {
        info!("Running startup dependency reachability checks");
        if let Err(message) = run_startup_reachability_checks(&config, &soroban).await {
            error!("{}", message);
            process::exit(1);
        }
    }

    // ── Graceful shutdown token ───────────────────────────────────────────
    let shutdown = IndexerShutdown::new();
    info!(
        drain_timeout_secs = shutdown.drain_timeout.as_secs(),
        "Graceful shutdown configured"
    );

    // Create SDEX indexer
    let sdex_indexer = SdexIndexer::new(horizon, db.clone());

    // Create AMM aggregator
    let amm_config = AmmConfig {
        router_contract: config.router_contract_address.clone(),
        poll_interval_secs: config.amm_poll_interval_secs,
        stale_threshold_secs: config.stale_threshold_secs,
        batch_size: 50,
    };
    let amm_aggregator = AmmAggregator::new(amm_config, db.clone(), soroban);

    // ── Spawn worker tasks ────────────────────────────────────────────────

    let sdex_shutdown = shutdown.clone();
    let sdex_handle = tokio::spawn(async move {
        info!("Starting SDEX indexing loop");
        if let Err(e) = sdex_indexer.start_indexing().await {
            if !sdex_shutdown.is_stopping() {
                error!("SDEX indexer error: {}", e);
            }
        }
    });

    let amm_shutdown = shutdown.clone();
    let amm_handle = tokio::spawn(async move {
        info!("Starting AMM aggregation loop");
        if let Err(e) = amm_aggregator.start_aggregation().await {
            if !amm_shutdown.is_stopping() {
                error!("AMM aggregator error: {}", e);
            }
        }
    });

    // Asset metadata enrichment job
    let metadata_db = db.clone();
    let metadata_shutdown = shutdown.clone();
    let metadata_handle = tokio::spawn(async move {
        info!("Starting asset metadata enrichment job");
        let job = MetadataEnrichmentJob::new(MetadataJobConfig::default(), metadata_db);
        if let Err(e) = job.start().await {
            if !metadata_shutdown.is_stopping() {
                error!("Asset metadata enrichment job error: {}", e);
            }
        }
    });

    // Maintenance loop
    let archival_manager = ArchivalManager::new(db.pool().clone());
    let maintenance_config = config.clone();
    let maintenance_shutdown = shutdown.clone();
    let maintenance_handle = tokio::spawn(async move {
        let interval = Duration::from_secs(maintenance_config.maintenance_interval_mins * 60);
        info!(
            "Starting maintenance loop with interval of {} minutes",
            maintenance_config.maintenance_interval_mins
        );

        loop {
            tokio::time::sleep(interval).await;

            if maintenance_shutdown.is_stopping() {
                info!("Maintenance loop stopping due to shutdown signal");
                break;
            }

            info!("Triggering scheduled maintenance tasks");

            if let Err(e) = archival_manager
                .compact_snapshots(
                    maintenance_config.snapshot_compaction_hours,
                    maintenance_config.snapshot_retention_days,
                )
                .await
            {
                error!("Maintenance error during snapshot compaction: {}", e);
            }

            if let Err(e) = archival_manager.run_retention_cleanup().await {
                error!("Maintenance error during retention cleanup: {}", e);
            }

            if let Err(e) = archival_manager.refresh_orderbook_summary().await {
                error!("Maintenance error during orderbook summary refresh: {}", e);
            }
        }
    });

    // ── Wait for shutdown signal ──────────────────────────────────────────
    // This blocks until SIGTERM/SIGINT is received, then waits for the drain
    // window before returning.
    shutdown.wait_for_signal().await;

    info!("Shutdown signal received — aborting worker tasks");

    // Abort long-running loops so they don't block process exit.
    sdex_handle.abort();
    amm_handle.abort();
    metadata_handle.abort();
    maintenance_handle.abort();

    info!("StellarRoute Indexer stopped cleanly");
    process::exit(0);
}
