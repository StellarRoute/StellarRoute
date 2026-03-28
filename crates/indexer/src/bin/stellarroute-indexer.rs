//! StellarRoute Indexer Binary
//!
//! Main entry point for the SDEX orderbook indexer service.

use std::process;
use tracing::{error, info};

use std::time::Duration;
use stellarroute_indexer::amm::{AmmAggregator, AmmConfig};
use stellarroute_indexer::config::IndexerConfig;
use stellarroute_indexer::db::{archival::ArchivalManager, Database};
use stellarroute_indexer::horizon::HorizonClient;
use stellarroute_indexer::sdex::SdexIndexer;
use stellarroute_indexer::soroban::{SorobanRpcClient, StellarNetwork};

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
    let network = if config.soroban_rpc_url.contains("testnet") {
        StellarNetwork::Testnet
    } else {
        StellarNetwork::Pubnet
    };
    let soroban = match SorobanRpcClient::for_network(network) {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to create Soroban RPC client: {}", e);
            process::exit(1);
        }
    };

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

    // Start both indexers concurrently
    let sdex_handle = tokio::spawn(async move {
        info!("Starting SDEX indexing loop");
        if let Err(e) = sdex_indexer.start_indexing().await {
            error!("SDEX indexer error: {}", e);
        }
    });

    let amm_handle = tokio::spawn(async move {
        info!("Starting AMM aggregation loop");
        if let Err(e) = amm_aggregator.start_aggregation().await {
            error!("AMM aggregator error: {}", e);
        }
    });

    // Create archival manager for maintenance tasks
    let archival_manager = ArchivalManager::new(db.pool().clone());
    let maintenance_config = config.clone();

    let maintenance_handle = tokio::spawn(async move {
        let interval = Duration::from_secs(maintenance_config.maintenance_interval_mins * 60);
        info!(
            "Starting maintenance loop with interval of {} minutes",
            maintenance_config.maintenance_interval_mins
        );

        loop {
            // Wait first, or run immediately? Usually wait first to avoid thundering herd on startup
            tokio::time::sleep(interval).await;

            info!("Triggering scheduled maintenance tasks");

            // 1. Snapshot compaction
            if let Err(e) = archival_manager
                .compact_snapshots(
                    maintenance_config.snapshot_compaction_hours,
                    maintenance_config.snapshot_retention_days,
                )
                .await
            {
                error!("Maintenance error during snapshot compaction: {}", e);
            }

            // 2. Retention policy cleanup
            if let Err(e) = archival_manager.run_retention_cleanup().await {
                error!("Maintenance error during retention cleanup: {}", e);
            }

            // 3. Refresh materialized views
            if let Err(e) = archival_manager.refresh_orderbook_summary().await {
                error!("Maintenance error during orderbook summary refresh: {}", e);
            }
        }
    });

    // Wait for indexers and maintenance task
    let (sdex_result, amm_result, maintenance_result) =
        tokio::join!(sdex_handle, amm_handle, maintenance_handle);

    if let Err(e) = sdex_result {
        error!("SDEX indexer task failed: {}", e);
    }

    if let Err(e) = amm_result {
        error!("AMM aggregator task failed: {}", e);
    }

    if let Err(e) = maintenance_result {
        error!("Maintenance task failed: {}", e);
    }

    process::exit(1);
}
