//! Integration tests for the reconciliation engine
//!
//! Simulates partial outages and recovery scenarios to verify the reconciliation
//! engine can detect and repair drift between Horizon and Soroban RPC data.
//!
//! Run with a live Postgres instance:
//! ```bash
//! docker-compose up -d
//! ./scripts/wait-for-dbs.sh
//! DATABASE_URL=postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute \
//!   cargo test -p stellarroute-indexer reconciliation_test -- --ignored
//! ```

mod test_fixture {
    use sqlx::PgPool;
    use stellarroute_indexer::config::IndexerConfig;
    use stellarroute_indexer::db::Database;
    use uuid::Uuid;

    pub fn database_url() -> String {
        std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
        })
    }

    fn test_indexer_config() -> IndexerConfig {
        IndexerConfig {
            stellar_horizon_url: "https://horizon-testnet.stellar.org".to_string(),
            horizon_mode: stellarroute_indexer::config::HorizonMode::Poll,
            soroban_rpc_url: "https://soroban-testnet.stellar.org".to_string(),
            router_contract_address: "CDUMMYROUTER".to_string(),
            database_url: database_url(),
            poll_interval_secs: 5,
            amm_poll_interval_secs: 30,
            stale_threshold_secs: 300,
            horizon_limit: 200,
            max_connections: 5,
            min_connections: 1,
            connection_timeout_secs: 30,
            idle_timeout_secs: 600,
            max_lifetime_secs: 1800,
            maintenance_interval_mins: 60,
            snapshot_retention_days: 90,
            snapshot_compaction_hours: 24,
            partition_count: 4,
            hot_pair_allowlist: String::new(),
            hot_pair_volume_threshold: 1_000_000_000,
            hot_pair_window_secs: 300,
            partition_id: 0,
        }
    }

    pub async fn setup_pool() -> PgPool {
        let config = test_indexer_config();
        let db = Database::new(&config)
            .await
            .expect("Failed to connect to test database");
        db.migrate()
            .await
            .expect("Failed to run database migrations");
        db.pool().clone()
    }

    pub async fn set_staleness_threshold(pool: &PgPool, seconds: i32) {
        sqlx::query(
            r#"
            INSERT INTO reconciliation_thresholds (check_type, staleness_threshold_secs, enabled)
            VALUES ('data_staleness', $1, true)
            ON CONFLICT (check_type) DO UPDATE
            SET staleness_threshold_secs = EXCLUDED.staleness_threshold_secs,
                enabled = true,
                updated_at = now()
            "#,
        )
        .bind(seconds)
        .execute(pool)
        .await
        .expect("Failed to configure staleness threshold");
    }

    pub async fn seed_native_and_usdc(pool: &PgPool) -> (Uuid, Uuid) {
        let native_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO assets (asset_type, asset_code, asset_issuer)
            VALUES ('native', NULL, NULL)
            ON CONFLICT (asset_type, asset_code, asset_issuer) DO UPDATE
            SET asset_type = EXCLUDED.asset_type
            RETURNING id
            "#,
        )
        .fetch_one(pool)
        .await
        .expect("Failed to seed native asset");

        let usdc_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO assets (asset_type, asset_code, asset_issuer)
            VALUES (
                'credit_alphanum4',
                'USDC',
                'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN'
            )
            ON CONFLICT (asset_type, asset_code, asset_issuer) DO UPDATE
            SET asset_type = EXCLUDED.asset_type
            RETURNING id
            "#,
        )
        .fetch_one(pool)
        .await
        .expect("Failed to seed USDC asset");

        (native_id, usdc_id)
    }

    pub async fn insert_stale_sdex_offer(
        pool: &PgPool,
        offer_id: i64,
        selling_asset_id: Uuid,
        buying_asset_id: Uuid,
        stale_for_secs: i32,
    ) {
        sqlx::query(
            r#"
            INSERT INTO sdex_offers (
                offer_id, seller, selling_asset_id, buying_asset_id,
                amount, price, last_modified_ledger, updated_at
            )
            VALUES ($1, $2, $3, $4, 1000, 0.25, 50000, now() - ($5::TEXT)::INTERVAL)
            ON CONFLICT (offer_id) DO UPDATE
            SET updated_at = EXCLUDED.updated_at,
                selling_asset_id = EXCLUDED.selling_asset_id,
                buying_asset_id = EXCLUDED.buying_asset_id
            "#,
        )
        .bind(offer_id)
        .bind(format!("GTEST{offer_id}"))
        .bind(selling_asset_id)
        .bind(buying_asset_id)
        .bind(format!("{stale_for_secs} seconds"))
        .execute(pool)
        .await
        .expect("Failed to insert stale SDEX offer");
    }

    pub async fn refresh_sdex_offer(pool: &PgPool, offer_id: i64) {
        sqlx::query("UPDATE sdex_offers SET updated_at = now() WHERE offer_id = $1")
            .bind(offer_id)
            .execute(pool)
            .await
            .expect("Failed to refresh SDEX offer");
    }

    pub async fn cleanup_sdex_offer(pool: &PgPool, offer_id: i64) {
        sqlx::query("DELETE FROM sdex_offers WHERE offer_id = $1")
            .bind(offer_id)
            .execute(pool)
            .await
            .ok();
    }

    pub async fn seed_liquidity_anomaly(
        pool: &PgPool,
        pool_address: &str,
        selling_asset_id: Uuid,
        buying_asset_id: Uuid,
    ) {
        sqlx::query(
            r#"
            INSERT INTO amm_pool_reserves (
                pool_address, selling_asset_id, buying_asset_id,
                reserve_selling, reserve_buying, fee_bps, last_updated_ledger, updated_at
            )
            VALUES ($1, $2, $3, 200, 5000, 30, 50001, now())
            ON CONFLICT (pool_address) DO UPDATE
            SET reserve_selling = EXCLUDED.reserve_selling,
                reserve_buying = EXCLUDED.reserve_buying,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(pool_address)
        .bind(selling_asset_id)
        .bind(buying_asset_id)
        .execute(pool)
        .await
        .expect("Failed to seed AMM pool reserves");

        sqlx::query(
            r#"
            INSERT INTO amm_pool_reserve_history (
                pool_address, reserve_selling, reserve_buying, recorded_at
            )
            VALUES
                ($1, 1000, 5000, now() - interval '10 minutes'),
                ($1, 200, 5000, now())
            "#,
        )
        .bind(pool_address)
        .execute(pool)
        .await
        .expect("Failed to seed AMM reserve history");
    }

    pub async fn cleanup_liquidity_fixture(pool: &PgPool, pool_address: &str) {
        sqlx::query("DELETE FROM amm_pool_reserve_history WHERE pool_address = $1")
            .bind(pool_address)
            .execute(pool)
            .await
            .ok();
        sqlx::query("DELETE FROM amm_pool_reserves WHERE pool_address = $1")
            .bind(pool_address)
            .execute(pool)
            .await
            .ok();
    }

    pub async fn count_liquidity_breaches(pool: &PgPool) -> i64 {
        sqlx::query_scalar(
            r#"
            SELECT COUNT(*)::bigint
            FROM drift_events
            WHERE drift_category = 'liquidity'
              AND breach = true
            "#,
        )
        .fetch_one(pool)
        .await
        .unwrap_or(0)
    }
}

#[cfg(test)]
mod integration_tests {
    use sqlx::Row;
    use stellarroute_indexer::reconciliation::ReconciliationEngine;

    use super::test_fixture::*;

    /// SDEX offers stop updating (Horizon outage) and recover after refresh.
    #[tokio::test]
    #[ignore = "requires PostgreSQL (set DATABASE_URL)"]
    async fn test_horizon_outage_detection_and_recovery() {
        let pool = setup_pool().await;
        set_staleness_threshold(&pool, 60).await;

        let offer_id = 9_879_001_i64;
        let (native_id, usdc_id) = seed_native_and_usdc(&pool).await;
        insert_stale_sdex_offer(&pool, offer_id, native_id, usdc_id, 600).await;

        let engine = ReconciliationEngine::new(pool.clone())
            .await
            .expect("Failed to create reconciliation engine");
        let outage_run = engine
            .run_reconciliation_cycle()
            .await
            .expect("Reconciliation cycle failed during outage simulation");

        assert!(
            outage_run.checks_failed > 0,
            "expected stale SDEX offers to fail reconciliation checks"
        );
        assert!(outage_run.total_drift_events > 0);

        let stale_checks = sqlx::query(
            r#"
            SELECT COUNT(*) AS count
            FROM reconciliation_checks
            WHERE check_type = 'data_staleness'
              AND entity_type = 'sdex_offer'
              AND entity_ref = $1
            "#,
        )
        .bind(offer_id.to_string())
        .fetch_one(&pool)
        .await
        .expect("Failed to query staleness checks");
        assert!(stale_checks.get::<i64, _>("count") > 0);

        refresh_sdex_offer(&pool, offer_id).await;

        let recovery_started = chrono::Utc::now();
        let _recovered_run = engine
            .run_reconciliation_cycle()
            .await
            .expect("Reconciliation cycle failed during recovery");

        let remaining_stale = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)::bigint
            FROM sdex_offers
            WHERE offer_id = $1
              AND NOW() - updated_at > interval '60 seconds'
            "#,
        )
        .bind(offer_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to verify offer freshness");

        assert_eq!(remaining_stale, 0);

        let test_failures_after_recovery = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)::bigint
            FROM reconciliation_checks
            WHERE check_type = 'data_staleness'
              AND entity_type = 'sdex_offer'
              AND entity_ref = $1
              AND drift_severity IN ('warning', 'critical')
              AND created_at >= $2
            "#,
        )
        .bind(offer_id.to_string())
        .bind(recovery_started)
        .fetch_one(&pool)
        .await
        .expect("Failed to query post-recovery staleness checks");

        assert_eq!(test_failures_after_recovery, 0);

        cleanup_sdex_offer(&pool, offer_id).await;
    }

    /// AMM pool reserves drain suddenly and reconciliation flags a liquidity breach.
    #[tokio::test]
    #[ignore = "requires PostgreSQL (set DATABASE_URL)"]
    async fn test_liquidity_anomaly_detection() {
        let pool = setup_pool().await;
        let pool_address = "CRECONTEST879002";
        let (native_id, usdc_id) = seed_native_and_usdc(&pool).await;

        seed_liquidity_anomaly(&pool, pool_address, native_id, usdc_id).await;

        let engine = ReconciliationEngine::new(pool.clone())
            .await
            .expect("Failed to create reconciliation engine");
        let run = engine
            .run_reconciliation_cycle()
            .await
            .expect("Reconciliation cycle failed during liquidity anomaly simulation");

        assert!(run.checks_failed > 0);
        assert!(run.critical_drift_events > 0);

        let liquidity_checks = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)::bigint
            FROM reconciliation_checks
            WHERE check_type = 'liquidity_anomaly'
              AND entity_ref = $1
            "#,
        )
        .bind(pool_address)
        .fetch_one(&pool)
        .await
        .expect("Failed to query liquidity checks");
        assert!(liquidity_checks > 0);

        let breaches = count_liquidity_breaches(&pool).await;
        assert!(breaches > 0);

        cleanup_liquidity_fixture(&pool, pool_address).await;
    }

    /// Post-recovery reconciliation pass reports a clean cycle after fixtures are removed.
    #[tokio::test]
    #[ignore = "requires PostgreSQL (set DATABASE_URL)"]
    async fn test_post_recovery_reconciliation_pass() {
        let pool = setup_pool().await;
        set_staleness_threshold(&pool, 60).await;

        let offer_id = 9_879_003_i64;
        let pool_address = "CRECONTEST879003";
        let (native_id, usdc_id) = seed_native_and_usdc(&pool).await;

        insert_stale_sdex_offer(&pool, offer_id, native_id, usdc_id, 600).await;
        seed_liquidity_anomaly(&pool, pool_address, native_id, usdc_id).await;

        let engine = ReconciliationEngine::new(pool.clone())
            .await
            .expect("Failed to create reconciliation engine");
        let failing_run = engine
            .run_reconciliation_cycle()
            .await
            .expect("Initial reconciliation cycle failed");
        assert!(failing_run.checks_failed > 0);

        refresh_sdex_offer(&pool, offer_id).await;
        cleanup_liquidity_fixture(&pool, pool_address).await;

        let recovery_started = chrono::Utc::now();
        let _recovered_run = engine
            .run_reconciliation_cycle()
            .await
            .expect("Post-recovery reconciliation cycle failed");

        let test_failures_after_recovery = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)::bigint
            FROM reconciliation_checks
            WHERE drift_severity IN ('warning', 'critical')
              AND entity_ref IN ($1, $2)
              AND created_at >= $3
            "#,
        )
        .bind(offer_id.to_string())
        .bind(pool_address)
        .bind(recovery_started)
        .fetch_one(&pool)
        .await
        .expect("Failed to query post-recovery checks");

        assert_eq!(
            test_failures_after_recovery, 0,
            "post-recovery cycle should not flag cleaned test fixtures"
        );

        let run_count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*)::bigint FROM reconciliation_runs")
                .fetch_one(&pool)
                .await
                .expect("Failed to count reconciliation runs");
        assert!(run_count >= 2);

        cleanup_sdex_offer(&pool, offer_id).await;
    }
}

#[cfg(test)]
mod unit_tests {
    use stellarroute_indexer::reconciliation::{CheckType, DriftSeverity};

    #[test]
    fn test_check_type_display() {
        assert_eq!(CheckType::DataStaleness.to_string(), "data_staleness");
        assert_eq!(CheckType::PriceDivergence.to_string(), "price_divergence");
        assert_eq!(CheckType::LedgerAlignment.to_string(), "ledger_alignment");
        assert_eq!(CheckType::LiquidityAnomaly.to_string(), "liquidity_anomaly");
        assert_eq!(CheckType::AssetMapping.to_string(), "asset_mapping");
    }

    #[test]
    fn test_drift_severity_ordering() {
        assert!(DriftSeverity::Info < DriftSeverity::Warning);
        assert!(DriftSeverity::Warning < DriftSeverity::Critical);
        assert!(DriftSeverity::Critical > DriftSeverity::Info);
    }

    #[test]
    fn test_drift_severity_display() {
        assert_eq!(DriftSeverity::Info.to_string(), "info");
        assert_eq!(DriftSeverity::Warning.to_string(), "warning");
        assert_eq!(DriftSeverity::Critical.to_string(), "critical");
    }
}
