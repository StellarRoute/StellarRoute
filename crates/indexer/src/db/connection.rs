//! Database connection management

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;
use tracing::{error, info};

use crate::config::IndexerConfig as Config;
use crate::error::{IndexerError, Result};

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

        // Read migration files from migrations directory
        let migrations: &[(&str, &str)] = &[
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
        ];

        for (name, sql) in migrations {
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
