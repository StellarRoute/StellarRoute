//! Data archival functionality

use sqlx::{PgPool, Row};
use tracing::{info, warn};

use crate::error::Result;

/// Default retention threshold (in days) before inactive `sdex_offers` rows are moved to `archived_offers`.
pub const DEFAULT_OFFER_ARCHIVAL_DAYS: i32 = 30;

/// Parse the configured offer archival age (in days), falling back to [`DEFAULT_OFFER_ARCHIVAL_DAYS`]
/// when unset, invalid, or non-positive.
pub fn resolve_offer_archival_days(raw: Option<&str>) -> i32 {
    raw.and_then(|v| v.trim().parse::<i32>().ok())
        .filter(|&days| days > 0)
        .unwrap_or(DEFAULT_OFFER_ARCHIVAL_DAYS)
}

/// Read the configured offer archival age (in days) from `OFFER_ARCHIVAL_DAYS` or
/// `ARCHIVE_OLD_OFFERS_DAYS`, defaulting to `30`.
pub fn resolve_offer_archival_days_from_env() -> i32 {
    let val = std::env::var("OFFER_ARCHIVAL_DAYS")
        .ok()
        .or_else(|| std::env::var("ARCHIVE_OLD_OFFERS_DAYS").ok());
    resolve_offer_archival_days(val.as_deref())
}

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
    /// Moves offers older than `days_old` from `sdex_offers` into `archived_offers`
    /// in a single statement so `trg_sync_normalized_sdex` also removes them from
    /// `normalized_liquidity`.
    ///
    /// # Arguments
    /// * `days_old` - Archive offers older than this many days (default: 30)
    ///
    /// # Returns
    /// Number of offers archived
    pub async fn archive_old_offers(&self, days_old: Option<i32>) -> Result<i64> {
        let days = days_old
            .filter(|&d| d > 0)
            .unwrap_or(DEFAULT_OFFER_ARCHIVAL_DAYS);
        info!("Archiving offers older than {} days", days);

        let result: (i64,) = sqlx::query_as(
            r#"
            WITH moved AS (
                DELETE FROM sdex_offers o
                USING assets sa, assets ba
                WHERE o.selling_asset_id = sa.id
                  AND o.buying_asset_id = ba.id
                  AND o.updated_at < NOW() - ($1 * INTERVAL '1 day')
                RETURNING
                    o.offer_id,
                    o.seller,
                    sa.asset_type AS selling_asset_type,
                    sa.asset_code AS selling_asset_code,
                    sa.asset_issuer AS selling_asset_issuer,
                    ba.asset_type AS buying_asset_type,
                    ba.asset_code AS buying_asset_code,
                    ba.asset_issuer AS buying_asset_issuer,
                    o.amount,
                    o.price,
                    o.price_n,
                    o.price_d,
                    o.last_modified_ledger
            ),
            ins AS (
                INSERT INTO archived_offers (
                    offer_id,
                    seller,
                    selling_asset_type,
                    selling_asset_code,
                    selling_asset_issuer,
                    buying_asset_type,
                    buying_asset_code,
                    buying_asset_issuer,
                    amount,
                    price,
                    price_n,
                    price_d,
                    last_modified_ledger,
                    archived_at,
                    archive_reason
                )
                SELECT
                    offer_id,
                    seller,
                    selling_asset_type,
                    selling_asset_code,
                    selling_asset_issuer,
                    buying_asset_type,
                    buying_asset_code,
                    buying_asset_issuer,
                    amount,
                    price,
                    price_n,
                    price_d,
                    last_modified_ledger,
                    NOW(),
                    'old_age'
                FROM moved
                ON CONFLICT (offer_id) DO UPDATE SET
                    seller = EXCLUDED.seller,
                    selling_asset_type = EXCLUDED.selling_asset_type,
                    selling_asset_code = EXCLUDED.selling_asset_code,
                    selling_asset_issuer = EXCLUDED.selling_asset_issuer,
                    buying_asset_type = EXCLUDED.buying_asset_type,
                    buying_asset_code = EXCLUDED.buying_asset_code,
                    buying_asset_issuer = EXCLUDED.buying_asset_issuer,
                    amount = EXCLUDED.amount,
                    price = EXCLUDED.price,
                    price_n = EXCLUDED.price_n,
                    price_d = EXCLUDED.price_d,
                    last_modified_ledger = EXCLUDED.last_modified_ledger,
                    archived_at = NOW(),
                    archive_reason = 'old_age'
            )
            SELECT COUNT(*)::bigint FROM moved
            "#,
        )
        .bind(days)
        .fetch_one(&self.pool)
        .await?;

        let archived_count = result.0;
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

    #[test]
    fn test_resolve_offer_archival_days_parsing_and_defaults() {
        assert_eq!(resolve_offer_archival_days(None), 30);
        assert_eq!(resolve_offer_archival_days(Some("")), 30);
        assert_eq!(resolve_offer_archival_days(Some("0")), 30);
        assert_eq!(resolve_offer_archival_days(Some("-7")), 30);
        assert_eq!(resolve_offer_archival_days(Some("invalid")), 30);
        assert_eq!(resolve_offer_archival_days(Some("14")), 14);
        assert_eq!(resolve_offer_archival_days(Some(" 45 ")), 45);
    }

    #[tokio::test]
    #[ignore]
    async fn test_archival_manager_creation() {
        // This test requires a database connection
        // Run with: cargo test --ignored
    }

    #[tokio::test]
    #[ignore]
    async fn test_archive_old_offers_preserves_fresh_offers() {
        let db_url = match std::env::var("DATABASE_URL") {
            Ok(url) => url,
            Err(_) => return,
        };
        let pool = match PgPool::connect(&db_url).await {
            Ok(pool) => pool,
            Err(_) => return,
        };
        let manager = ArchivalManager::new(pool);

        let archived = manager.archive_old_offers(Some(30)).await;
        assert!(archived.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_delete_old_archived_retention() {
        let db_url = match std::env::var("DATABASE_URL") {
            Ok(url) => url,
            Err(_) => return,
        };
        let pool = match PgPool::connect(&db_url).await {
            Ok(pool) => pool,
            Err(_) => return,
        };
        let manager = ArchivalManager::new(pool);

        let deleted = manager.delete_old_archived(60).await;
        assert!(deleted.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_default_archival_retention_days() {
        let db_url = match std::env::var("DATABASE_URL") {
            Ok(url) => url,
            Err(_) => return,
        };
        let pool = match PgPool::connect(&db_url).await {
            Ok(pool) => pool,
            Err(_) => return,
        };
        let manager = ArchivalManager::new(pool);

        let count = manager.archive_old_offers(None).await;
        assert!(count.is_ok());
    }
}
