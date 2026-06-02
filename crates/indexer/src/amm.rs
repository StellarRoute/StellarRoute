//! AMM Pool State Aggregator
//!
//! This module provides continuous tracking of AMM pool reserves, fees, and lifecycle events.
//! It polls registered pools from the router contract and updates the database with current state.

use crate::db::Database;
use crate::error::{IndexerError, Result};
use crate::models::{PoolReserve, PoolState};
use crate::soroban::{SorobanRpc, SorobanRpcClient};
use crate::telemetry::TraceContext;
use stellar_xdr::curr::{Limits, LedgerEntry, LedgerEntryData, ScVal, ReadXdr};
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

        // Run one immediate aggregation at startup to bootstrap configured pools,
        // then continue on the configured interval.
        if let Err(e) = self.aggregate_once().await {
            error!("Initial AMM aggregation failed: {}", e);
        }

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
    #[tracing::instrument(skip(self))]
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
            // Discover new pools since last check via contract events. If none are
            // discovered, fall back to the operator-managed registry or env var list.
            let mut new_pools = self
                .discover_new_pools(start_ledger, current_ledger)
                .await?;

            if new_pools.is_empty() {
                let registry = self.get_registry_pools().await?;
                if !registry.is_empty() {
                    info!("Using {} pools from registry fallback", registry.len());
                    new_pools = registry;
                }
            }

            if !new_pools.is_empty() {
                info!(
                    "Processing {} newly discovered/configured pools",
                    new_pools.len()
                );
                self.process_pool_batch(&new_pools).await?;
            }
        }

        // Always process existing pools to update reserves. Include any
        // operator-registered/configured pools that may not yet have reserves
        // written to `amm_pool_reserves` so they are actively monitored.
        let mut existing_pools = self.get_tracked_pools().await?;
        let configured = self.get_registry_pools().await?;
        for p in configured {
            if !existing_pools.contains(&p) {
                existing_pools.push(p);
            }
        }

        debug!(
            "Processing {} existing/configured pools",
            existing_pools.len()
        );
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

    /// Get operator-managed or env-configured pools to use as bootstrap fallback.
    async fn get_registry_pools(&self) -> Result<Vec<String>> {
        // First, query the `amm_pools` registry table for active pools.
        let rows = sqlx::query("SELECT pool_address FROM amm_pools WHERE active = true")
            .fetch_all(self.db.pool())
            .await?;

        let mut pools: Vec<String> = rows.into_iter().map(|r| r.get("pool_address")).collect();

        // Then append any pools from the AMM_POOLS env var (comma-separated)
        if let Ok(env) = std::env::var("AMM_POOLS") {
            for p in env.split(',') {
                let p = p.trim();
                if p.is_empty() {
                    continue;
                }
                if !pools.contains(&p.to_string()) {
                    pools.push(p.to_string());
                }
            }
        }

        Ok(pools)
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
    #[tracing::instrument(skip(self), fields(pool_address = %pool_address))]
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

    /// Parse pool state from contract data (XDR decoding)
    fn parse_pool_state(
        &self,
        contract_data: &serde_json::Value,
        pool_address: &str,
    ) -> Result<PoolState> {
        parse_soroban_pool_state(contract_data, pool_address)
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
    #[tracing::instrument(skip(self, reserve), fields(pool_address = %reserve.pool_address))]
    async fn update_pool_reserve(&self, reserve: &PoolReserve) -> Result<()> {
        let pool = self.db.pool();
        let trace_context = TraceContext::current();
        sqlx::query("SELECT upsert_amm_pool_reserve($1, $2, $3, $4, $5, $6, $7, $8, $9)")
            .bind(&reserve.pool_address)
            .bind(reserve.selling_asset_id)
            .bind(reserve.buying_asset_id)
            .bind(reserve.reserve_selling.to_string())
            .bind(reserve.reserve_buying.to_string())
            .bind(reserve.fee_bps)
            .bind(reserve.last_updated_ledger)
            .bind(trace_context.trace_id)
            .bind(trace_context.span_id)
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

/// Helper to extract standard signed/unsigned integer values from ScVal
fn parse_scval_integer(val: &ScVal) -> Option<i128> {
    match val {
        ScVal::I128(parts) => Some(((parts.hi as i128) << 64) | (parts.lo as i128)),
        ScVal::U128(parts) => Some(((parts.hi as i128) << 64) | (parts.lo as i128)),
        ScVal::I64(v) => Some(*v as i128),
        ScVal::U64(v) => Some(*v as i128),
        ScVal::I32(v) => Some(*v as i128),
        ScVal::U32(v) => Some(*v as i128),
        _ => None,
    }
}

/// Standalone helper to parse pool state from contract data XDR
pub fn parse_soroban_pool_state(
    contract_data: &serde_json::Value,
    pool_address: &str,
) -> Result<PoolState> {
    let xdr_base64 = contract_data
        .get("xdr")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            let err_msg = "missing `xdr` field in contract data response".to_string();
            warn!("XDR decode fallback: pool {} - {}", pool_address, err_msg);
            IndexerError::SorobanRpc(err_msg)
        })?;

    // 1. Decode base64 XDR as LedgerEntry or LedgerEntryData
    let ledger_entry_data = match LedgerEntry::from_xdr_base64(xdr_base64, Limits::none()) {
        Ok(entry) => entry.data,
        Err(_) => {
            match LedgerEntryData::from_xdr_base64(xdr_base64, Limits::none()) {
                Ok(data) => data,
                Err(e) => {
                    let err_msg = format!("failed to parse XDR as LedgerEntry or LedgerEntryData: {}", e);
                    warn!("XDR decode fallback: pool {} - {}", pool_address, err_msg);
                    return Err(IndexerError::SorobanRpc(err_msg));
                }
            }
        }
    };

    // 2. Extract ContractDataEntry
    let contract_data_entry = match ledger_entry_data {
        LedgerEntryData::ContractData(entry) => entry,
        _ => {
            let err_msg = "ledger entry is not ContractData".to_string();
            warn!("XDR decode fallback: pool {} - {}", pool_address, err_msg);
            return Err(IndexerError::SorobanRpc(err_msg));
        }
    };

    // 3. Extract ContractInstance
    let instance = match contract_data_entry.val {
        ScVal::ContractInstance(instance) => instance,
        _ => {
            let err_msg = "contract data val is not ContractInstance".to_string();
            warn!("XDR decode fallback: pool {} - {}", pool_address, err_msg);
            return Err(IndexerError::SorobanRpc(err_msg));
        }
    };

    // 4. Parse the instance storage map
    let storage = instance.storage.ok_or_else(|| {
        let err_msg = "instance storage map is empty".to_string();
        warn!("XDR decode fallback: pool {} - {}", pool_address, err_msg);
        IndexerError::SorobanRpc(err_msg)
    })?;

    let mut token_a: Option<String> = None;
    let mut token_b: Option<String> = None;
    let mut reserve_a: Option<i128> = None;
    let mut reserve_b: Option<i128> = None;
    let mut fee_bps: Option<i32> = None;

    for entry in storage.iter() {
        let key_str = match &entry.key {
            ScVal::Symbol(sym) => sym.to_string(),
            _ => continue,
        };

        match key_str.as_str() {
            "token_a" | "token_x" | "asset_a" | "token_0" => {
                if let ScVal::Address(addr) = &entry.val {
                    token_a = Some(addr.to_string());
                }
            }
            "token_b" | "token_y" | "asset_b" | "token_1" => {
                if let ScVal::Address(addr) = &entry.val {
                    token_b = Some(addr.to_string());
                }
            }
            "reserve_a" | "res_a" | "reserve_x" | "res_0" => {
                reserve_a = parse_scval_integer(&entry.val);
            }
            "reserve_b" | "res_b" | "reserve_y" | "res_1" => {
                reserve_b = parse_scval_integer(&entry.val);
            }
            "fee_bps" | "fee" | "fee_rate" => {
                fee_bps = parse_scval_integer(&entry.val).map(|v| v as i32);
            }
            _ => {}
        }
    }

    let token_a = token_a.ok_or_else(|| {
        let err_msg = "failed to find token_a in pool storage".to_string();
        warn!("XDR decode fallback: pool {} - {}", pool_address, err_msg);
        IndexerError::SorobanRpc(err_msg)
    })?;

    let token_b = token_b.ok_or_else(|| {
        let err_msg = "failed to find token_b in pool storage".to_string();
        warn!("XDR decode fallback: pool {} - {}", pool_address, err_msg);
        IndexerError::SorobanRpc(err_msg)
    })?;

    let reserve_a = reserve_a.ok_or_else(|| {
        let err_msg = "failed to find reserve_a in pool storage".to_string();
        warn!("XDR decode fallback: pool {} - {}", pool_address, err_msg);
        IndexerError::SorobanRpc(err_msg)
    })?;

    let reserve_b = reserve_b.ok_or_else(|| {
        let err_msg = "failed to find reserve_b in pool storage".to_string();
        warn!("XDR decode fallback: pool {} - {}", pool_address, err_msg);
        IndexerError::SorobanRpc(err_msg)
    })?;

    let ledger_sequence = contract_data
        .get("lastModifiedLedgerSeq")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let fee_bps = fee_bps.unwrap_or(30);

    Ok(PoolState {
        address: pool_address.to_string(),
        token_a,
        token_b,
        reserve_a,
        reserve_b,
        fee_bps,
        ledger_sequence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use stellar_xdr::curr::{
        ContractDataEntry, ContractDataDurability, ContractExecutable, ScContractInstance,
        ExtensionPoint, LedgerEntryData, ScAddress, ScMap, ScMapEntry, ScSymbol, ScVal,
        Int128Parts, Hash, WriteXdr,
    };
    use serde_json::json;

    #[test]
    fn test_parse_soroban_pool_state_success() {
        let token_a_addr = ScAddress::Contract(Hash([11; 32]));
        let token_b_addr = ScAddress::Contract(Hash([22; 32]));

        let storage_entries = vec![
            ScMapEntry {
                key: ScVal::Symbol(ScSymbol("token_a".try_into().unwrap())),
                val: ScVal::Address(token_a_addr.clone()),
            },
            ScMapEntry {
                key: ScVal::Symbol(ScSymbol("token_b".try_into().unwrap())),
                val: ScVal::Address(token_b_addr.clone()),
            },
            ScMapEntry {
                key: ScVal::Symbol(ScSymbol("reserve_a".try_into().unwrap())),
                val: ScVal::I128(Int128Parts {
                    hi: 0,
                    lo: 123456789,
                }),
            },
            ScMapEntry {
                key: ScVal::Symbol(ScSymbol("reserve_b".try_into().unwrap())),
                val: ScVal::I128(Int128Parts {
                    hi: 0,
                    lo: 987654321,
                }),
            },
            ScMapEntry {
                key: ScVal::Symbol(ScSymbol("fee_bps".try_into().unwrap())),
                val: ScVal::U32(25),
            },
        ];

        let storage_map = ScMap::try_from(storage_entries).unwrap();

        let instance = ScContractInstance {
            executable: ContractExecutable::Wasm(Hash([0; 32])),
            storage: Some(storage_map),
        };

        let entry = LedgerEntryData::ContractData(ContractDataEntry {
            ext: ExtensionPoint::V0,
            contract: ScAddress::Contract(Hash([33; 32])),
            key: ScVal::LedgerKeyContractInstance,
            durability: ContractDataDurability::Persistent,
            val: ScVal::ContractInstance(instance),
        });

        let base64_xdr = entry.to_xdr_base64(Limits::none()).unwrap();

        let contract_data = json!({
            "xdr": base64_xdr,
            "lastModifiedLedgerSeq": 99999
        });

        let parsed = parse_soroban_pool_state(&contract_data, "CCONTRACTADDRESS").unwrap();

        assert_eq!(parsed.address, "CCONTRACTADDRESS");
        assert_eq!(parsed.token_a, token_a_addr.to_string());
        assert_eq!(parsed.token_b, token_b_addr.to_string());
        assert_eq!(parsed.reserve_a, 123456789);
        assert_eq!(parsed.reserve_b, 987654321);
        assert_eq!(parsed.fee_bps, 25);
        assert_eq!(parsed.ledger_sequence, 99999);
    }

    #[test]
    fn test_parse_soroban_pool_state_missing_xdr() {
        let contract_data = json!({
            "lastModifiedLedgerSeq": 99999
        });

        let res = parse_soroban_pool_state(&contract_data, "CPOOLADDR");
        assert!(res.is_err());
        let err_str = format!("{}", res.unwrap_err());
        assert!(err_str.contains("missing `xdr`"));
    }

    #[test]
    fn test_parse_soroban_pool_state_invalid_xdr() {
        let contract_data = json!({
            "xdr": "INVALIDXDRDATABASE64STRING!!!",
            "lastModifiedLedgerSeq": 99999
        });

        let res = parse_soroban_pool_state(&contract_data, "CPOOLADDR");
        assert!(res.is_err());
        let err_str = format!("{}", res.unwrap_err());
        assert!(err_str.contains("failed to parse XDR"));
    }
}
