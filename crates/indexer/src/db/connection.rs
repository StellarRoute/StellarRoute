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

        // Read migration files from migrations directory
        let migration_0001 = include_str!("../../migrations/0001_init.sql");
        let migration_0002 = include_str!("../../migrations/0002_performance_indexes.sql");
        let migration_0003 = include_str!("../../migrations/0003_trading_pairs_and_snapshots.sql");
        let migration_0004 = include_str!("../../migrations/0004_normalized_liquidity.sql");
        let migration_0005 = include_str!("../../migrations/0005_venue_health_scores.sql");
        let migration_0006 = include_str!("../../migrations/0006_maintenance_policies.sql");
        let migration_0007 =
            include_str!("../../migrations/0007_backfill_and_normalized_storage.sql");
        let migration_0008 = include_str!("../../migrations/0008_soroban_discovery_cursors.sql");

        // Execute migrations in order
        info!("Running migration 0001_init.sql");
        sqlx::query(migration_0001)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!("Migration 0001 failed: {}", e);
                IndexerError::DatabaseMigration(format!("Failed to run 0001_init.sql: {}", e))
            })?;

        info!("Running migration 0002_performance_indexes.sql");
        sqlx::query(migration_0002)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!("Migration 0002 failed: {}", e);
                IndexerError::DatabaseMigration(format!(
                    "Failed to run 0002_performance_indexes.sql: {}",
                    e
                ))
            })?;

        info!("Running migration 0003_trading_pairs_and_snapshots.sql");
        sqlx::query(migration_0003)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!("Migration 0003 failed: {}", e);
                IndexerError::DatabaseMigration(format!(
                    "Failed to run 0003_trading_pairs_and_snapshots.sql: {}",
                    e
                ))
            })?;

        info!("Running migration 0004_normalized_liquidity.sql");
        sqlx::query(migration_0004)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!("Migration 0004 failed: {}", e);
                IndexerError::DatabaseMigration(format!(
                    "Failed to run 0004_normalized_liquidity.sql: {}",
                    e
                ))
            })?;

        info!("Running migration 0005_venue_health_scores.sql");
        sqlx::query(migration_0005)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!("Migration 0005 failed: {}", e);
                IndexerError::DatabaseMigration(format!(
                    "Failed to run 0005_venue_health_scores.sql: {}",
                    e
                ))
            })?;

        info!("Running migration 0006_maintenance_policies.sql");
        sqlx::query(migration_0006)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!("Migration 0006 failed: {}", e);
                IndexerError::DatabaseMigration(format!(
                    "Failed to run 0006_maintenance_policies.sql: {}",
                    e
                ))
            })?;

        info!("Running migration 0007_backfill_and_normalized_storage.sql");
        sqlx::query(migration_0007)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!("Migration 0007 failed: {}", e);
                IndexerError::DatabaseMigration(format!(
                    "Failed to run 0007_backfill_and_normalized_storage.sql: {}",
                    e
                ))
            })?;

        info!("Running migration 0008_soroban_discovery_cursors.sql");
        sqlx::query(migration_0008)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!("Migration 0008 failed: {}", e);
                IndexerError::DatabaseMigration(format!(
                    "Failed to run 0008_soroban_discovery_cursors.sql: {}",
                    e
                ))
            })?;

        info!("Database migrations completed");
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
