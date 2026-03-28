//! Data archival functionality

use sqlx::{PgPool, Row};
use tracing::{info, warn};

use crate::error::Result;

/// Data archival manager
pub struct ArchivalManager {
    pool: PgPool,
}

impl ArchivalManager {
    /// Create a new archival manager
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Archive old offers (older than specified days)
    ///
    /// This moves old offers from the main table to the archive table
    /// to keep the main table performant.
    ///
    /// # Arguments
    /// * `days_old` - Archive offers older than this many days (default: 30)
    ///
    /// # Returns
    /// Number of offers archived
    pub async fn archive_old_offers(&self, days_old: Option<i32>) -> Result<i64> {
        let days = days_old.unwrap_or(30);
        info!("Archiving offers older than {} days", days);

        let result: (Option<i32>,) = sqlx::query_as(
            r#"
            select archive_old_offers($1)
            "#,
        )
        .bind(days)
        .fetch_one(&self.pool)
        .await?;

        let archived_count = result.0.unwrap_or(0) as i64;
        info!("Archived {} offers", archived_count);

        Ok(archived_count)
    }

    /// Get count of archived offers
    pub async fn get_archived_count(&self) -> Result<i64> {
        let result: (i64,) = sqlx::query_as(
            r#"
            select count(*) from archived_offers
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result.0)
    }

    /// Delete archived offers older than specified days
    ///
    /// This permanently deletes archived offers to free up space.
    /// Use with caution!
    ///
    /// # Arguments
    /// * `days_old` - Delete archived offers older than this many days
    ///
    /// # Returns
    /// Number of archived offers deleted
    pub async fn delete_old_archived(&self, days_old: i32) -> Result<i64> {
        warn!(
            "Permanently deleting archived offers older than {} days",
            days_old
        );

        let result = sqlx::query(
            r#"
            delete from archived_offers
            where archived_at < now() - interval '1 day' * $1
            "#,
        )
        .bind(days_old)
        .execute(&self.pool)
        .await?;

        let deleted_count = result.rows_affected() as i64;
        warn!("Permanently deleted {} archived offers", deleted_count);

        Ok(deleted_count)
    }

    /// Refresh the orderbook summary materialized view
    ///
    /// This updates pre-aggregated statistics for fast queries
    pub async fn refresh_orderbook_summary(&self) -> Result<()> {
        info!("Refreshing orderbook summary materialized view");

        sqlx::query("select refresh_orderbook_summary()")
            .execute(&self.pool)
            .await?;

        info!("Orderbook summary refreshed");
        Ok(())
    }

    /// Compact old orderbook snapshots
    ///
    /// # Arguments
    /// * `threshold_hours` - Hours after which snapshots are compacted
    /// * `retention_days` - Days after which snapshots are deleted
    pub async fn compact_snapshots(
        &self,
        threshold_hours: i32,
        retention_days: i32,
    ) -> Result<i32> {
        let start = std::time::Instant::now();
        info!(
            "Running snapshot compaction (threshold: {}h, retention: {}d)",
            threshold_hours, retention_days
        );

        let result: (i32,) = sqlx::query_as(
            r#"
            select compact_orderbook_snapshots($1, $2)
            "#,
        )
        .bind(threshold_hours)
        .bind(retention_days)
        .fetch_one(&self.pool)
        .await?;

        let affected = result.0;
        let duration_ms = start.elapsed().as_millis();

        info!(
            target: "stellarroute.indexer.maintenance",
            metric = "stellarroute.indexer.snapshot_compaction",
            deleted_count = affected,
            duration_ms = duration_ms,
            "Snapshot compaction completed"
        );

        Ok(affected)
    }

    /// Run general retention policies for various tables
    pub async fn run_retention_cleanup(&self) -> Result<()> {
        let start = std::time::Instant::now();
        info!("Running general retention policies cleanup");

        let rows = sqlx::query(
            r#"
            select table_name, deleted_count from apply_retention_policies()
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        for row in rows {
            let table_name: String = row.get(0);
            let deleted_count: i64 = row.get(1);

            info!(
                target: "stellarroute.indexer.maintenance",
                metric = "stellarroute.indexer.retention_cleanup",
                table = table_name,
                deleted_count = deleted_count,
                "Retention cleanup for table {} completed",
                table_name
            );
        }

        info!(
            "General retention cleanup completed in {}ms",
            start.elapsed().as_millis()
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_archival_manager_creation() {
        // This test requires a database connection
        // Run with: cargo test --ignored
    }
}
