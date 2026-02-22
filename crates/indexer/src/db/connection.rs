//! Database connection management

use sqlx::PgPool;
use tracing::{error, info};

use crate::config::IndexerConfig as Config;
use crate::error::{IndexerError, Result};

/// Database connection pool
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Create a new database connection pool
    pub async fn new(config: &Config) -> Result<Self> {
        info!("Connecting to database: {}", config.database_url);

        let pool = PgPool::connect(&config.database_url).await.map_err(|e| {
            error!("Failed to connect to database: {}", e);
            IndexerError::DatabaseConnection(format!(
                "Failed to connect to {}: {}",
                config.database_url, e
            ))
        })?;

        info!("Database connection established");
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
