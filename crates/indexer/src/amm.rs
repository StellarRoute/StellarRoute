//! AMM Pool State Aggregator
//!
//! This module provides continuous tracking of AMM pool reserves, fees, and lifecycle events.
//! It polls registered pools from the router contract and updates the database with current state.

use crate::db::Database;
use crate::error::Result;
use crate::models::{PoolReserve, PoolState};
use crate::soroban::{SorobanRpc, SorobanRpcClient};
use chrono::Utc;
use serde_json;
use sqlx::Row;
use std::time::Duration;
use tracing::{debug, error, info, warn};

const DISCOVERY_CURSOR_JOB: &str = "soroban_pool_discovery";

/// Configuration for AMM pool indexing
#[derive(Clone, Debug)]
pub struct AmmConfig {
    /// Router contract address to query for registered pools
    pub router_contract: String,
    /// Poll interval for pool state updates
    pub poll_interval_secs: u64,
    /// Stale threshold in seconds (pools not updated within this time are considered stale)
    pub stale_threshold_secs: u64,
    /// Maximum number of pools to process per batch
    pub batch_size: usize,
}

impl Default for AmmConfig {
    fn default() -> Self {
        Self {
            router_contract: String::new(),
            poll_interval_secs: 30,
            stale_threshold_secs: 300, // 5 minutes
            batch_size: 50,
        }
    }
}

/// AMM pool aggregator service
pub struct AmmAggregator {
    config: AmmConfig,
    db: Database,
    soroban: SorobanRpcClient,
}

impl AmmAggregator {
    pub fn new(config: AmmConfig, db: Database, soroban: SorobanRpcClient) -> Self {
        Self {
            config,
            db,
            soroban,
        }
    }

    /// Start the continuous aggregation loop
    pub async fn start_aggregation(&self) -> Result<()> {
        info!("Starting AMM pool aggregation loop");

        let mut interval =
            tokio::time::interval(Duration::from_secs(self.config.poll_interval_secs));

        loop {
            interval.tick().await;

            if let Err(e) = self.aggregate_once().await {
                error!("AMM aggregation cycle failed: {}", e);
                // Continue the loop despite errors
            }
        }
    }

    /// Perform a single aggregation cycle
    pub async fn aggregate_once(&self) -> Result<()> {
        debug!("Starting AMM pool aggregation cycle");

        let current_ledger = self.soroban.get_latest_ledger().await?;
        let cursor_str = self.load_discovery_cursor().await?;
        let start_ledger: u64 = cursor_str.parse().unwrap_or(0);

        if start_ledger >= current_ledger {
            debug!(
                "No new ledgers to process for discovery (start={}, current={})",
                start_ledger, current_ledger
            );
        } else {
            // Discover new pools since last check
            let new_pools = self
                .discover_new_pools(start_ledger, current_ledger)
                .await?;
            if !new_pools.is_empty() {
                info!("Discovered {} new pools", new_pools.len());
                self.process_pool_batch(&new_pools).await?;
            }
        }

        // Always process existing pools to update reserves
        let existing_pools = self.get_tracked_pools().await?;
        debug!("Processing {} existing pools", existing_pools.len());
        for batch in existing_pools.chunks(self.config.batch_size) {
            if let Err(e) = self.process_pool_batch(batch).await {
                warn!("Failed to process pool batch: {}", e);
            }
        }

        // Clean up stale pools
        self.cleanup_stale_pools().await?;

        // Update cursor to current ledger
        self.store_discovery_cursor(
            &current_ledger.to_string(),
            Some(current_ledger as i64),
            "running",
        )
        .await?;

        debug!("Completed AMM pool aggregation cycle");
        Ok(())
    }

    async fn load_discovery_cursor(&self) -> Result<String> {
        let row = sqlx::query("SELECT cursor FROM soroban_sync_cursors WHERE job_name = $1")
            .bind(DISCOVERY_CURSOR_JOB)
            .fetch_optional(self.db.pool())
            .await?;

        if let Some(row) = row {
            return Ok(row.get::<String, _>("cursor"));
        }

        self.store_discovery_cursor("0", Some(0), "initialized")
            .await?;
        Ok("0".to_string())
    }

    async fn store_discovery_cursor(
        &self,
        cursor: &str,
        last_seen_ledger: Option<i64>,
        status: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO soroban_sync_cursors (job_name, cursor, last_seen_ledger, status, updated_at)
            VALUES ($1, $2, $3, $4, now())
            ON CONFLICT (job_name)
            DO UPDATE SET
                cursor = EXCLUDED.cursor,
                last_seen_ledger = EXCLUDED.last_seen_ledger,
                status = EXCLUDED.status,
                updated_at = now()
            "#,
        )
        .bind(DISCOVERY_CURSOR_JOB)
        .bind(cursor)
        .bind(last_seen_ledger)
        .bind(status)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Discover new pools via contract events
    async fn discover_new_pools(&self, start_ledger: u64, end_ledger: u64) -> Result<Vec<String>> {
        use crate::soroban::EventFilter;

        let filters = vec![EventFilter {
            event_type: "contract".to_string(),
            contract_ids: vec![self.config.router_contract.clone()],
            topics: vec![vec!["pool_created".to_string()]], // Standard topic for pool discovery
        }];

        let events = self
            .soroban
            .get_events(start_ledger, Some(end_ledger), filters)
            .await?;
        let mut new_pools = Vec::new();

        for event in events {
            // Topic structure usually: ["pool_created", token_a, token_b, pool_address]
            // Or pool_address is in the value. For this implementation, we assume pool_address is the last topic
            // if it exists, or we'd decode the value XDR.
            if let Some(pool_address) = event.topics.last() {
                new_pools.push(pool_address.clone());
            }
        }

        Ok(new_pools)
    }

    /// Get pools currently tracked in the database
    async fn get_tracked_pools(&self) -> Result<Vec<String>> {
        let rows = sqlx::query("SELECT pool_address FROM amm_pool_reserves")
            .fetch_all(self.db.pool())
            .await?;

        Ok(rows.into_iter().map(|r| r.get("pool_address")).collect())
    }

    /// Process a batch of pools
    async fn process_pool_batch(&self, pool_addresses: &[String]) -> Result<()> {
        for address in pool_addresses {
            if let Err(e) = self.process_pool(address).await {
                warn!("Failed to process pool {}: {}", address, e);
            }
        }
        Ok(())
    }

    /// Process a single pool
    async fn process_pool(&self, pool_address: &str) -> Result<()> {
        // Get pool state from Soroban RPC
        let state = self.get_pool_state(pool_address).await?;

        // Resolve asset IDs
        let selling_asset_id = self.resolve_asset_id(&state.token_a).await?;
        let buying_asset_id = self.resolve_asset_id(&state.token_b).await?;

        // Update database
        self.update_pool_reserve(&PoolReserve {
            pool_address: pool_address.to_string(),
            selling_asset_id,
            buying_asset_id,
            reserve_selling: rust_decimal::Decimal::from_i128_with_scale(state.reserve_a, 0),
            reserve_buying: rust_decimal::Decimal::from_i128_with_scale(state.reserve_b, 0),
            fee_bps: state.fee_bps,
            last_updated_ledger: state.ledger_sequence,
            updated_at: Utc::now(),
        })
        .await?;

        debug!("Updated pool {} reserves", pool_address);
        Ok(())
    }

    /// Get pool state from Soroban RPC
    async fn get_pool_state(&self, pool_address: &str) -> Result<PoolState> {
        // Get contract data
        let contract_data = self.soroban.get_pool_state(pool_address).await?;

        // Parse the XDR data to extract reserves and fee
        // This is a simplified implementation - real implementation would decode XDR
        self.parse_pool_state(&contract_data, pool_address)
    }

    /// Parse pool state from contract data (simplified)
    fn parse_pool_state(
        &self,
        _contract_data: &serde_json::Value,
        pool_address: &str,
    ) -> Result<PoolState> {
        // TODO: Implement proper XDR decoding
        // For now, return mock data
        Ok(PoolState {
            address: pool_address.to_string(),
            token_a: "CDUMMYTOKENA".to_string(),
            token_b: "CDUMMYTOKENB".to_string(),
            reserve_a: 1000000000, // 1000 units
            reserve_b: 2000000000, // 2000 units
            fee_bps: 30,           // 0.3%
            ledger_sequence: 12345,
        })
    }

    /// Resolve asset ID from contract address
    async fn resolve_asset_id(&self, contract_address: &str) -> Result<uuid::Uuid> {
        use sqlx::Row;

        // Check if asset exists in database
        let pool = self.db.pool();
        let row = sqlx::query("SELECT id FROM assets WHERE asset_type = $1 AND asset_issuer = $2")
            .bind("soroban")
            .bind(contract_address)
            .fetch_optional(pool)
            .await?;

        if let Some(row) = row {
            return Ok(row.get("id"));
        }

        // Insert new asset
        let id = uuid::Uuid::new_v4();
        sqlx::query(
            "INSERT INTO assets (id, asset_type, asset_issuer, created_at) VALUES ($1, $2, $3, $4)",
        )
        .bind(id)
        .bind("soroban")
        .bind(contract_address)
        .bind(Utc::now())
        .execute(pool)
        .await?;

        Ok(id)
    }

    /// Update pool reserve in database
    async fn update_pool_reserve(&self, reserve: &PoolReserve) -> Result<()> {
        let pool = self.db.pool();
        sqlx::query("SELECT upsert_amm_pool_reserve($1, $2, $3, $4, $5, $6, $7)")
            .bind(&reserve.pool_address)
            .bind(reserve.selling_asset_id)
            .bind(reserve.buying_asset_id)
            .bind(reserve.reserve_selling.to_string())
            .bind(reserve.reserve_buying.to_string())
            .bind(reserve.fee_bps)
            .bind(reserve.last_updated_ledger)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Clean up stale pools
    async fn cleanup_stale_pools(&self) -> Result<()> {
        let threshold =
            Utc::now() - chrono::Duration::seconds(self.config.stale_threshold_secs as i64);
        let pool = self.db.pool();

        let result = sqlx::query("DELETE FROM amm_pool_reserves WHERE updated_at < $1")
            .bind(threshold)
            .execute(pool)
            .await?;

        if result.rows_affected() > 0 {
            info!("Cleaned up {} stale pool entries", result.rows_affected());
        }

        Ok(())
    }
}
