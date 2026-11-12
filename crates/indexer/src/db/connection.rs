//! Database connection management

use sqlx::postgres::PgPoolOptions;
use sqlx::{Connection, PgConnection, PgPool};
use std::time::Duration;
use tracing::{error, info, warn};

use crate::config::IndexerConfig as Config;
use crate::error::{IndexerError, Result};

/// Session-level PostgreSQL advisory lock key used to serialize indexer schema
/// migrations across concurrent partitioned replicas (`"SRINDXMI"`).
pub const MIGRATION_ADVISORY_LOCK_KEY: i64 = 0x5352_494E_4458_4D49;

/// Opt-out header directive for migration files that contain statements
/// PostgreSQL forbids inside a transaction block (for example,
/// `CREATE INDEX CONCURRENTLY` or `ALTER TYPE ... ADD VALUE` on older versions).
///
/// Place `-- stellarroute:no-transaction` (or `-- no-transaction`) in the
/// leading SQL comment header before the first SQL statement to run that file
/// outside a `BEGIN ... COMMIT` block while still holding the global migration
/// advisory lock.
pub const NO_TRANSACTION_DIRECTIVE: &str = "-- stellarroute:no-transaction";

const EMBEDDED_MIGRATIONS: &[(&str, &str)] = &[
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
];

/// Returns `true` when the leading comment header of `sql` opts out of wrapping
/// the migration in a transaction via [`NO_TRANSACTION_DIRECTIVE`] (or
/// `-- no-transaction`).
pub fn is_non_transactional_migration(sql: &str) -> bool {
    let trimmed = sql.trim_start_matches('\u{feff}');
    for line in trimmed.lines() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        if let Some(comment) = l.strip_prefix("--") {
            let directive = comment.trim().to_ascii_lowercase();
            if directive == "stellarroute:no-transaction" || directive == "no-transaction" {
                return true;
            }
            continue;
        }
        break;
    }
    false
}

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

    /// Wrap an existing [`PgPool`] in a [`Database`] handle.
    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get a reference to the connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Run embedded database migrations atomically while holding a PostgreSQL
    /// session advisory lock (`MIGRATION_ADVISORY_LOCK_KEY`).
    ///
    /// By default each migration's SQL and its `_schema_migrations` ledger row
    /// are committed in a single transaction so a crash or DDL error leaves no
    /// partial schema and no ledger entry. Migrations whose leading comments
    /// include [`NO_TRANSACTION_DIRECTIVE`] execute outside a transaction block
    /// while still serialized under the advisory lock.
    pub async fn migrate(&self) -> Result<()> {
        self.migrate_with_list(EMBEDDED_MIGRATIONS).await
    }

    /// Run the provided `(name, sql)` migration list under the indexer
    /// PostgreSQL advisory lock.
    pub async fn migrate_with_list(&self, migrations: &[(&str, &str)]) -> Result<()> {
        info!("Running database migrations");

        let mut lock_conn = self.pool.acquire().await.map_err(|e| {
            error!(
                "Failed to acquire database connection for migrations: {}",
                e
            );
            IndexerError::DatabaseMigration(format!(
                "Failed to acquire connection for migrations: {}",
                e
            ))
        })?;

        let acquired_immediately: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
            .bind(MIGRATION_ADVISORY_LOCK_KEY)
            .fetch_one(&mut *lock_conn)
            .await
            .map_err(|e| {
                error!("Failed to probe migration advisory lock: {}", e);
                IndexerError::DatabaseMigration(format!(
                    "Failed to probe migration advisory lock: {}",
                    e
                ))
            })?;

        if !acquired_immediately {
            info!(
                lock_key = MIGRATION_ADVISORY_LOCK_KEY,
                "Waiting for indexer migration advisory lock held by another replica"
            );
            sqlx::query("SELECT pg_advisory_lock($1)")
                .bind(MIGRATION_ADVISORY_LOCK_KEY)
                .execute(&mut *lock_conn)
                .await
                .map_err(|e| {
                    error!("Failed while waiting for migration advisory lock: {}", e);
                    IndexerError::DatabaseMigration(format!(
                        "Failed to acquire migration advisory lock: {}",
                        e
                    ))
                })?;
            info!(
                lock_key = MIGRATION_ADVISORY_LOCK_KEY,
                "Acquired indexer migration advisory lock after waiting"
            );
        }

        let migration_result = Self::run_locked_migrations(&mut lock_conn, migrations).await;

        if let Err(unlock_err) = sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(MIGRATION_ADVISORY_LOCK_KEY)
            .execute(&mut *lock_conn)
            .await
        {
            warn!(
                lock_key = MIGRATION_ADVISORY_LOCK_KEY,
                "Failed to explicitly release migration advisory lock (connection will be closed): {}",
                unlock_err
            );
            let _ = lock_conn.close().await;
        }

        migration_result?;

        info!("Database migrations completed");
        Ok(())
    }

    async fn run_locked_migrations(
        conn: &mut PgConnection,
        migrations: &[(&str, &str)],
    ) -> Result<()> {
        sqlx::raw_sql(
            "create table if not exists _schema_migrations (
                name text primary key,
                applied_at timestamptz not null default now()
            )",
        )
        .execute(&mut *conn)
        .await
        .map_err(|e| {
            error!("Failed to ensure migration ledger table: {}", e);
            IndexerError::DatabaseMigration(format!("Failed to create migration ledger: {}", e))
        })?;

        for (name, sql) in migrations {
            Self::execute_migration(&mut *conn, name, sql).await?;
        }

        Ok(())
    }

    async fn execute_migration(conn: &mut PgConnection, name: &str, sql: &str) -> Result<()> {
        let already_applied: bool =
            sqlx::query_scalar("select exists(select 1 from _schema_migrations where name = $1)")
                .bind(name)
                .fetch_one(&mut *conn)
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

        if is_non_transactional_migration(sql) {
            info!(
                migration = name,
                "Running non-transactional migration (opted out via header directive)"
            );
            sqlx::raw_sql(sql).execute(&mut *conn).await.map_err(|e| {
                error!("Non-transactional migration {} failed: {}", name, e);
                IndexerError::DatabaseMigration(format!("Failed to run {}: {}", name, e))
            })?;

            sqlx::query("insert into _schema_migrations (name) values ($1)")
                .bind(name)
                .execute(&mut *conn)
                .await
                .map_err(|e| {
                    error!("Failed to record migration {}: {}", name, e);
                    IndexerError::DatabaseMigration(format!(
                        "Failed to record migration {}: {}",
                        name, e
                    ))
                })?;

            return Ok(());
        }

        info!("Running migration {}", name);
        let mut tx = conn.begin().await.map_err(|e| {
            error!("Failed to start transaction for migration {}: {}", name, e);
            IndexerError::DatabaseMigration(format!(
                "Failed to begin transaction for {}: {}",
                name, e
            ))
        })?;

        sqlx::raw_sql(sql).execute(&mut *tx).await.map_err(|e| {
            error!(
                "Migration {} failed (rolling back transaction): {}",
                name, e
            );
            IndexerError::DatabaseMigration(format!("Failed to run {}: {}", name, e))
        })?;

        sqlx::query("insert into _schema_migrations (name) values ($1)")
            .bind(name)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                error!("Failed to record migration {}: {}", name, e);
                IndexerError::DatabaseMigration(format!(
                    "Failed to record migration {}: {}",
                    name, e
                ))
            })?;

        tx.commit().await.map_err(|e| {
            error!("Failed to commit migration {}: {}", name, e);
            IndexerError::DatabaseMigration(format!("Failed to commit {}: {}", name, e))
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
    use super::*;

    #[test]
    fn detects_no_transaction_directive_in_leading_comments() {
        let sql_canonical = "-- Migration 0099\n-- stellarroute:no-transaction\nCREATE INDEX CONCURRENTLY idx_x ON t(c);";
        assert!(is_non_transactional_migration(sql_canonical));

        let sql_short = "\n  -- NO-TRANSACTION \nCREATE INDEX CONCURRENTLY idx_y ON t(c);";
        assert!(is_non_transactional_migration(sql_short));
    }

    #[test]
    fn regular_migrations_default_to_transactional() {
        let sql = "-- Regular migration\nCREATE TABLE demo (id int primary key);";
        assert!(!is_non_transactional_migration(sql));

        let trailing_comment =
            "CREATE TABLE demo (id int primary key);\n-- stellarroute:no-transaction";
        assert!(!is_non_transactional_migration(trailing_comment));
    }

    #[test]
    fn embedded_migrations_default_to_transactional() {
        assert_eq!(EMBEDDED_MIGRATIONS.len(), 15);
        for (name, sql) in EMBEDDED_MIGRATIONS {
            assert!(
                !is_non_transactional_migration(sql),
                "Expected embedded migration {} to be transactional",
                name
            );
        }
    }
}
