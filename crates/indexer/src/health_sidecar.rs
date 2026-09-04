use crate::config::IndexerConfig;
use chrono::Utc;
use serde::Serialize;
use sqlx::Row;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Serialize, Clone)]
pub struct HealthSnapshot {
    pub ok: bool,
    pub sdex_lag: u64,
    pub amm_lag: u64,
    pub ts: String,
}

#[derive(Clone)]
pub struct HealthSidecar {
    path: String,
    interval: Duration,
}

impl HealthSidecar {
    pub fn from_config(config: &IndexerConfig) -> Option<Self> {
        let path = config
            .indexer_health_file
            .as_deref()
            .filter(|value| !value.trim().is_empty())?;

        Some(Self::new(path.to_string(), Duration::from_secs(30)))
    }

    pub fn new(path: String, interval: Duration) -> Self {
        Self { path, interval }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn write_snapshot(&self, snapshot: &HealthSnapshot) -> Result<(), String> {
        let bytes = serde_json::to_vec(snapshot)
            .map_err(|e| format!("serialize indexer health sidecar JSON: {e}"))?;

        let path = Path::new(&self.path);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("create parent directory for {}: {e}", self.path))?;
            }
        }

        std::fs::write(path, bytes)
            .map_err(|e| format!("write health file {}: {e}", self.path))?;

        Ok(())
    }

    pub fn start<F, Fut>(self, mut poll: F)
    where
        F: FnMut() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<HealthSnapshot, String>> + Send,
    {
        let path = self.path.clone();
        let interval = self.interval;
        let _ = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.tick().await;
            loop {
                match poll().await {
                    Ok(snapshot) => {
                        let sidecar = HealthSidecar::new(path.clone(), interval);
                        if let Err(e) = sidecar.write_snapshot(&snapshot) {
                            tracing::warn!(path = %path, error = %e, "Failed to update indexer health sidecar snapshot");
                        }
                    }
                    Err(e) => {
                        tracing::warn!(path = %path, error = %e, "Failed to compute indexer health sidecar snapshot");
                    }
                }
                ticker.tick().await;
            }
        });
    }
}

pub async fn compute_health_snapshot(sdex_lag: u64, amm_lag: u64) -> HealthSnapshot {
    HealthSnapshot {
        ok: sdex_lag <= 10 && amm_lag <= 10,
        sdex_lag,
        amm_lag,
        ts: Utc::now().to_rfc3339(),
    }
}

pub async fn snapshot_from_db(db: &sqlx::PgPool, horizon_url: &str) -> Result<HealthSnapshot, String> {
    let sdex_lag = fetch_sdex_lag(db, horizon_url).await?;
    let amm_lag = fetch_amm_lag(db, horizon_url).await?;
    Ok(compute_health_snapshot(sdex_lag, amm_lag).await)
}

async fn fetch_horizon_ledger(horizon_url: &str) -> Result<u64, String> {
    let url = format!("{}/ledgers?order=desc&limit=1", horizon_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("create HTTP client: {e}"))?;
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("HTTP error fetching Horizon ledger: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Horizon returned {} for ledger probe", resp.status()));
    }
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("parse Horizon ledger JSON: {e}"))?;
    body["_embedded"]["records"][0]["sequence"]
        .as_u64()
        .ok_or_else(|| "missing sequence field in Horizon ledger response".to_string())
}

async fn fetch_sdex_lag(db: &sqlx::PgPool, horizon_url: &str) -> Result<u64, String> {
    let horizon_ledger = fetch_horizon_ledger(horizon_url).await?;
    let row = sqlx::query(
        r#"
        SELECT GREATEST(
            COALESCE((SELECT NULLIF(value, '')::BIGINT FROM ingestion_state WHERE key = 'sdex_last_horizon_ledger'), 0),
            COALESCE((SELECT MAX(last_modified_ledger) FROM sdex_offers), 0)
        )::BIGINT AS seq
        "#,
    )
    .fetch_optional(db)
    .await
    .map_err(|e| format!("query SDEX ledger state: {e}"))?;
    let indexed_ledger = row.map(|r| r.get::<i64, _>("seq") as u64).unwrap_or(0);
    Ok(horizon_ledger.saturating_sub(indexed_ledger))
}

async fn fetch_amm_lag(db: &sqlx::PgPool, horizon_url: &str) -> Result<u64, String> {
    let horizon_ledger = fetch_horizon_ledger(horizon_url).await?;
    let row = sqlx::query(
        r#"
        SELECT COALESCE(last_seen_ledger, 0)::BIGINT AS seq
        FROM soroban_sync_cursors
        WHERE job_name = 'soroban_pool_discovery'
        "#,
    )
    .fetch_optional(db)
    .await
    .map_err(|e| format!("query AMM ledger state: {e}"))?;
    let indexed_ledger = row.map(|r| r.get::<i64, _>("seq") as u64).unwrap_or(0);
    Ok(horizon_ledger.saturating_sub(indexed_ledger))
}

#[cfg(test)]
mod tests {
    use super::{compute_health_snapshot, HealthSidecar};
    use std::time::Duration;

    #[test]
    fn health_sidecar_writes_json_when_enabled() {
        let dir = std::env::temp_dir().join(format!("stellarroute-health-{}", std::process::id()));
        let file = dir.join("indexer-health.json");
        let _ = std::fs::remove_dir_all(&dir);

        let sidecar = HealthSidecar::new(file.to_string_lossy().into_owned(), Duration::from_secs(1));
        let snapshot = tokio::runtime::Runtime::new().unwrap().block_on(compute_health_snapshot(3, 4));

        sidecar.write_snapshot(&snapshot).unwrap();
        let json = std::fs::read_to_string(&file).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(value.get("ok").is_some());
        assert!(value.get("sdex_lag").is_some());
        assert!(value.get("amm_lag").is_some());
        assert!(value.get("ts").is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_default_has_no_sidecar_path() {
        let config = crate::config::IndexerConfig {
            stellar_horizon_url: "https://horizon.stellar.org".to_string(),
            stellar_horizon_fallback_urls: String::new(),
            horizon_mode: crate::config::HorizonMode::Poll,
            soroban_rpc_url: "https://rpc.stellar.org".to_string(),
            soroban_rpc_fallback_urls: String::new(),
            router_contract_address: String::new(),
            database_url: "postgres://user:pass@localhost/db".to_string(),
            poll_interval_secs: 2,
            amm_poll_interval_secs: 30,
            stale_threshold_secs: 300,
            horizon_limit: 200,
            max_connections: 10,
            min_connections: 2,
            connection_timeout_secs: 30,
            idle_timeout_secs: 600,
            max_lifetime_secs: 1800,
            maintenance_interval_mins: 60,
            snapshot_retention_days: 90,
            snapshot_compaction_hours: 24,
            partition_count: 1,
            hot_pair_allowlist: String::new(),
            hot_pair_volume_threshold: 1_000_000_000,
            hot_pair_window_secs: 300,
            partition_id: 0,
            indexer_health_file: None,
        };
        assert!(!config.health_file_enabled());
        assert!(crate::health_sidecar::HealthSidecar::from_config(&config).is_none());
    }

    #[test]
    fn config_can_enable_health_sidecar() {
        let config = crate::config::IndexerConfig {
            stellar_horizon_url: "https://horizon.stellar.org".to_string(),
            stellar_horizon_fallback_urls: String::new(),
            horizon_mode: crate::config::HorizonMode::Poll,
            soroban_rpc_url: "https://rpc.stellar.org".to_string(),
            soroban_rpc_fallback_urls: String::new(),
            router_contract_address: String::new(),
            database_url: "postgres://user:pass@localhost/db".to_string(),
            poll_interval_secs: 2,
            amm_poll_interval_secs: 30,
            stale_threshold_secs: 300,
            horizon_limit: 200,
            max_connections: 10,
            min_connections: 2,
            connection_timeout_secs: 30,
            idle_timeout_secs: 600,
            max_lifetime_secs: 1800,
            maintenance_interval_mins: 60,
            snapshot_retention_days: 90,
            snapshot_compaction_hours: 24,
            partition_count: 1,
            hot_pair_allowlist: String::new(),
            hot_pair_volume_threshold: 1_000_000_000,
            hot_pair_window_secs: 300,
            partition_id: 0,
            indexer_health_file: Some("/tmp/indexer-health.json".to_string()),
        };
        assert!(config.health_file_enabled());
        let sidecar = crate::health_sidecar::HealthSidecar::from_config(&config).unwrap();
        assert_eq!(sidecar.path(), "/tmp/indexer-health.json");
    }
}
