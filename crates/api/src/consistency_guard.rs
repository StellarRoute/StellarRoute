//! Read-after-write consistency guards for freshly indexed offers
//!
//! Prevents quotes from reading pre-commit offer rows during indexer writes
//! using PostgreSQL transaction isolation levels and visibility rules.

use sqlx::{Postgres, Transaction};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{debug, warn};

/// Consistency strategy for handling concurrent reads during writes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsistencyStrategy {
    /// Use REPEATABLE READ isolation level (snapshot-based)
    SnapshotIsolation,
    /// Use READ COMMITTED with explicit version checking
    VersionChecking,
    /// Use SERIALIZABLE isolation (strictest)
    Serializable,
}

impl ConsistencyStrategy {
    /// Get PostgreSQL isolation level string
    pub fn isolation_level(&self) -> &'static str {
        match self {
            Self::SnapshotIsolation => "REPEATABLE READ",
            Self::VersionChecking => "READ COMMITTED",
            Self::Serializable => "SERIALIZABLE",
        }
    }
}

/// Metrics for consistency guard activations
#[derive(Debug, Default)]
pub struct ConsistencyMetrics {
    /// Total number of guarded reads
    pub guarded_reads: AtomicU64,
    /// Number of stale reads prevented
    pub stale_reads_prevented: AtomicU64,
    /// Number of retries due to conflicts
    pub conflict_retries: AtomicU64,
}

impl ConsistencyMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_guarded_read(&self) {
        self.guarded_reads.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_stale_read_prevented(&self) {
        self.stale_reads_prevented.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_conflict_retry(&self) {
        self.conflict_retries.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> (u64, u64, u64) {
        (
            self.guarded_reads.load(Ordering::Relaxed),
            self.stale_reads_prevented.load(Ordering::Relaxed),
            self.conflict_retries.load(Ordering::Relaxed),
        )
    }
}

/// Consistency guard for read operations
pub struct ConsistencyGuard {
    strategy: ConsistencyStrategy,
    metrics: Arc<ConsistencyMetrics>,
}

impl ConsistencyGuard {
    pub fn new(strategy: ConsistencyStrategy, metrics: Arc<ConsistencyMetrics>) -> Self {
        Self { strategy, metrics }
    }

    /// Begin a guarded read transaction
    pub async fn begin_read_transaction<'a>(
        &self,
        pool: &sqlx::PgPool,
    ) -> Result<Transaction<'a, Postgres>, sqlx::Error> {
        self.metrics.record_guarded_read();

        let mut tx = pool.begin().await?;

        // Set isolation level based on strategy
        let isolation_level = self.strategy.isolation_level();
        sqlx::query(&format!(
            "SET TRANSACTION ISOLATION LEVEL {}",
            isolation_level
        ))
        .execute(&mut *tx)
        .await?;

        debug!(
            isolation_level = isolation_level,
            "Started guarded read transaction"
        );

        Ok(tx)
    }

    /// Check if a quote read should be blocked due to ongoing writes
    pub async fn check_visibility(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        asset_pair: (&str, &str),
    ) -> Result<bool, sqlx::Error> {
        match self.strategy {
            ConsistencyStrategy::SnapshotIsolation => {
                // Snapshot isolation guarantees we see a consistent view
                Ok(true)
            }
            ConsistencyStrategy::VersionChecking => {
                // Check if there are uncommitted writes for this asset pair
                let (base, quote) = asset_pair;
                let has_pending_writes = sqlx::query_scalar::<_, bool>(
                    r#"
                    SELECT EXISTS(
                        SELECT 1 FROM pg_locks l
                        JOIN pg_class c ON l.relation = c.oid
                        WHERE c.relname IN ('sdex_offers', 'amm_pool_reserves')
                        AND l.locktype = 'relation'
                        AND l.mode IN ('RowExclusiveLock', 'ShareRowExclusiveLock')
                        AND l.granted = true
                        AND l.pid != pg_backend_pid()
                    )
                    "#,
                )
                .fetch_one(&mut **tx)
                .await?;

                if has_pending_writes {
                    self.metrics.record_stale_read_prevented();
                    warn!(
                        base = base,
                        quote = quote,
                        "Prevented stale read due to ongoing write"
                    );
                    Ok(false)
                } else {
                    Ok(true)
                }
            }
            ConsistencyStrategy::Serializable => {
                // Serializable will automatically detect conflicts
                Ok(true)
            }
        }
    }

    /// Get current metrics
    pub fn metrics(&self) -> &Arc<ConsistencyMetrics> {
        &self.metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isolation_levels() {
        assert_eq!(
            ConsistencyStrategy::SnapshotIsolation.isolation_level(),
            "REPEATABLE READ"
        );
        assert_eq!(
            ConsistencyStrategy::VersionChecking.isolation_level(),
            "READ COMMITTED"
        );
        assert_eq!(
            ConsistencyStrategy::Serializable.isolation_level(),
            "SERIALIZABLE"
        );
    }

    #[test]
    fn test_metrics() {
        let metrics = ConsistencyMetrics::new();

        metrics.record_guarded_read();
        metrics.record_guarded_read();
        metrics.record_stale_read_prevented();

        let (guarded, stale, _conflicts) = metrics.snapshot();
        assert_eq!(guarded, 2);
        assert_eq!(stale, 1);
    }
}
