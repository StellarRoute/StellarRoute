//! Database connection management

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;
use tracing::{error, info};

use crate::config::IndexerConfig as Config;
use crate::error::{IndexerError, Result};

/// Ordered list of embedded SQL migrations applied by [`Database::migrate`].
pub(crate) const MIGRATIONS: &[(&str, &str)] = &[
    (
        "0001_init.sql",
        include_str!("../../migrations/0001_init.sql"),
    ),
    (
        "0002_performance_indexes.sql",
        include_str!("../../migrations/0002_performance_indexes.sql"),
    ),
    (
        "0003_trading_pairs_and_snapshots.sql",
        include_str!("../../migrations/0003_trading_pairs_and_snapshots.sql"),
    ),
    (
        "0004_normalized_liquidity.sql",
        include_str!("../../migrations/0004_normalized_liquidity.sql"),
    ),
    (
        "0005_venue_health_scores.sql",
        include_str!("../../migrations/0005_venue_health_scores.sql"),
    ),
    (
        "0006_maintenance_policies.sql",
        include_str!("../../migrations/0006_maintenance_policies.sql"),
    ),
    (
        "0007_backfill_and_normalized_storage.sql",
        include_str!("../../migrations/0007_backfill_and_normalized_storage.sql"),
    ),
    (
        "0008_soroban_discovery_cursors.sql",
        include_str!("../../migrations/0008_soroban_discovery_cursors.sql"),
    ),
    (
        "0009_finalize_unified_liquidity.sql",
        include_str!("../../migrations/0009_finalize_unified_liquidity.sql"),
    ),
    (
        "0010_asset_metadata.sql",
        include_str!("../../migrations/0010_asset_metadata.sql"),
    ),
    (
        "0011_trace_context_provenance.sql",
        include_str!("../../migrations/0011_trace_context_provenance.sql"),
    ),
    (
        "0012_contract_swap_activity.sql",
        include_str!("../../migrations/0012_contract_swap_activity.sql"),
    ),
    (
        "0013_amm_pools.sql",
        include_str!("../../migrations/0013_amm_pools.sql"),
    ),
    (
        "0014_assets_native_singleton.sql",
        include_str!("../../migrations/0014_assets_native_singleton.sql"),
    ),
    (
        "0015_swap_prepared_quotes.sql",
        include_str!("../../migrations/0015_swap_prepared_quotes.sql"),
    ),
    (
        "0016_notify_liquidity_update.sql",
        include_str!("../../migrations/0016_notify_liquidity_update.sql"),
    ),
];

/// Database connection pool
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Create a new database connection pool with configurable pool options.
    ///
    /// Pool settings are read from [`Config`] and can be tuned via environment
    /// variables (`DB_MAX_CONNECTIONS`, `DB_MIN_CONNECTIONS`,
    /// `DB_CONNECTION_TIMEOUT`, `DB_IDLE_TIMEOUT`, `DB_MAX_LIFETIME`).
    pub async fn new(config: &Config) -> Result<Self> {
        info!(
            "Connecting to database (pool: min={}, max={}, timeout={}s)",
            config.min_connections, config.max_connections, config.connection_timeout_secs,
        );

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(Duration::from_secs(config.connection_timeout_secs))
            .idle_timeout(Duration::from_secs(config.idle_timeout_secs))
            .max_lifetime(Duration::from_secs(config.max_lifetime_secs))
            .connect(&config.database_url)
            .await
            .map_err(|e| {
                error!("Failed to connect to database: {}", e);
                IndexerError::DatabaseConnection(format!("Failed to connect to database: {}", e))
            })?;

        info!(
            "Database connection pool established (max_connections={})",
            config.max_connections
        );
        Ok(Self { pool })
    }

    /// Get a reference to the connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Run database migrations
    pub async fn migrate(&self) -> Result<()> {
        info!("Running database migrations");

        sqlx::raw_sql(
            "create table if not exists _schema_migrations (
                name text primary key,
                applied_at timestamptz not null default now()
            )",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to ensure migration ledger table: {}", e);
            IndexerError::DatabaseMigration(format!("Failed to create migration ledger: {}", e))
        })?;

        for (name, sql) in MIGRATIONS {
            Self::execute_migration(&self.pool, name, sql).await?;
        }

        info!("Database migrations completed");
        Ok(())
    }

    async fn execute_migration(pool: &PgPool, name: &str, sql: &str) -> Result<()> {
        let already_applied: bool =
            sqlx::query_scalar("select exists(select 1 from _schema_migrations where name = $1)")
                .bind(name)
                .fetch_one(pool)
                .await
                .map_err(|e| {
                    error!("Failed to check migration status for {}: {}", name, e);
                    IndexerError::DatabaseMigration(format!(
                        "Failed to check migration {}: {}",
                        name, e
                    ))
                })?;

        if already_applied {
            info!("Skipping migration {} (already applied)", name);
            return Ok(());
        }

        info!("Running migration {}", name);
        sqlx::raw_sql(sql).execute(pool).await.map_err(|e| {
            error!("Migration {} failed: {}", name, e);
            IndexerError::DatabaseMigration(format!("Failed to run {}: {}", name, e))
        })?;

        sqlx::query("insert into _schema_migrations (name) values ($1)")
            .bind(name)
            .execute(pool)
            .await
            .map_err(|e| {
                error!("Failed to record migration {}: {}", name, e);
                IndexerError::DatabaseMigration(format!(
                    "Failed to record migration {}: {}",
                    name, e
                ))
            })?;

        Ok(())
    }

    /// Create a health monitor for this database
    pub fn health_monitor(&self) -> super::HealthMonitor {
        super::HealthMonitor::new(self.pool.clone())
    }

    /// Create an archival manager for this database
    pub fn archival_manager(&self) -> super::ArchivalManager {
        super::ArchivalManager::new(self.pool.clone())
    }

    /// Check database health
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(IndexerError::DatabaseQuery)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::MIGRATIONS;
    use std::collections::HashSet;
    use std::path::Path;

    fn extract_prefix(file_name: &str) -> std::result::Result<&str, String> {
        let prefix = file_name
            .split('_')
            .next()
            .filter(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
            .ok_or_else(|| format!("migration '{file_name}' lacks a numeric prefix"))?;
        Ok(prefix)
    }

    fn validate_migration_manifest(
        disk_files: &[String],
        runner_names: &[&str],
    ) -> std::result::Result<(), String> {
        if disk_files.is_empty() {
            return Err("no migration files found on disk".to_string());
        }

        let mut seen_disk_prefixes = HashSet::new();
        for file in disk_files {
            let prefix = extract_prefix(file)?;
            if !seen_disk_prefixes.insert(prefix.to_string()) {
                return Err(format!(
                    "duplicate migration prefix '{prefix}' on disk ({file})"
                ));
            }
        }

        let mut seen_runner_prefixes = HashSet::new();
        let mut last_num: Option<u32> = None;
        for &name in runner_names {
            let prefix = extract_prefix(name)?;
            if !seen_runner_prefixes.insert(prefix.to_string()) {
                return Err(format!(
                    "duplicate migration prefix '{prefix}' in runner ({name})"
                ));
            }
            let num: u32 = prefix
                .parse()
                .map_err(|e| format!("invalid migration number '{prefix}': {e}"))?;
            if let Some(prev) = last_num {
                if num <= prev {
                    return Err(format!(
                        "runner migrations out of order: {name} ({num}) after {prev}"
                    ));
                }
            }
            last_num = Some(num);
        }

        let runner_set: HashSet<&str> = runner_names.iter().copied().collect();
        for file in disk_files {
            if !runner_set.contains(file.as_str()) {
                return Err(format!(
                    "migration file '{file}' on disk is missing from Database::migrate runner list"
                ));
            }
        }

        let disk_set: HashSet<&str> = disk_files.iter().map(String::as_str).collect();
        for &name in runner_names {
            if !disk_set.contains(name) {
                return Err(format!(
                    "runner migration '{name}' does not exist in migrations directory"
                ));
            }
        }

        Ok(())
    }

    #[test]
    fn migration_runner_includes_all_sql_files_with_unique_prefixes() {
        let migrations_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
        let mut disk_files: Vec<String> = std::fs::read_dir(&migrations_dir)
            .expect("migrations directory must be readable")
            .filter_map(|entry| {
                let entry = entry.expect("valid dir entry");
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("sql") {
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .map(ToOwned::to_owned)
                } else {
                    None
                }
            })
            .collect();
        disk_files.sort();

        let runner_names: Vec<&str> = MIGRATIONS.iter().map(|(name, _)| *name).collect();
        validate_migration_manifest(&disk_files, &runner_names)
            .expect("all migrations on disk must have unique prefixes and be registered in runner");

        let notify_entry = MIGRATIONS
            .iter()
            .find(|(name, _)| *name == "0016_notify_liquidity_update.sql")
            .expect("0016_notify_liquidity_update.sql must be registered");
        assert!(notify_entry.1.contains("pg_notify('liquidity_update'"));
        assert!(notify_entry.1.contains("p_source_trace_id text default ''"));
        assert!(notify_entry.1.contains("p_source_span_id text default ''"));
    }

    #[test]
    fn migration_validator_rejects_missing_file_and_duplicate_prefix() {
        let missing_from_runner = vec!["0001_init.sql".to_string(), "0002_orphan.sql".to_string()];
        assert!(validate_migration_manifest(&missing_from_runner, &["0001_init.sql"]).is_err());

        let duplicate_prefix = vec![
            "0012_contract_swap_activity.sql".to_string(),
            "0012_notify_liquidity_update.sql".to_string(),
        ];
        assert!(validate_migration_manifest(
            &duplicate_prefix,
            &[
                "0012_contract_swap_activity.sql",
                "0012_notify_liquidity_update.sql",
            ]
        )
        .is_err());
    }
}
