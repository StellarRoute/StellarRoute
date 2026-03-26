//! Main reconciliation engine orchestration

use chrono::Utc;
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use super::consistency::{
    check_asset_mapping, check_data_staleness, check_ledger_alignment, check_liquidity_anomalies,
    check_price_divergence, CheckThresholds, ConsistencyCheckResult, DriftSeverity,
};
use super::metrics::{DriftMetrics, ReconciliationMetrics};
use crate::error::Result;

/// Main reconciliation engine
pub struct ReconciliationEngine {
    db: PgPool,
    thresholds: CheckThresholds,
    metrics: ReconciliationMetrics,
}

impl ReconciliationEngine {
    /// Create a new reconciliation engine
    pub async fn new(db: PgPool) -> Result<Self> {
        let thresholds = CheckThresholds::load_from_db(&db).await?;
        let metrics = ReconciliationMetrics::new();

        Ok(Self {
            db,
            thresholds,
            metrics,
        })
    }

    /// Run a complete reconciliation cycle
    pub async fn run_reconciliation_cycle(&self) -> Result<ReconciliationRun> {
        let run_id = Uuid::new_v4();
        let started_at = Utc::now();

        info!("Starting reconciliation cycle: {}", run_id);

        let mut checks_passed = 0;
        let mut checks_failed = 0;
        let mut drift_events = Vec::new();
        let mut _repairs_attempted = 0;
        let mut _successful_repairs = 0;

        // Run all consistency checks
        let check_results = self.run_all_checks().await?;
        let checks_executed = check_results.len();

        for result in check_results {
            // Save the check result
            let _check_id = result.save(&self.db).await?;

            // Emit drift metrics
            let drift_metric = DriftMetrics::from_check_result(&result);
            drift_metric.save(&self.db).await?;
            drift_events.push(drift_metric);

            // Track pass/fail
            if result.severity == DriftSeverity::Info {
                checks_passed += 1;
            } else {
                checks_failed += 1;
            }
        }

        let completed_at = Utc::now();
        let duration_ms = (completed_at - started_at).num_milliseconds() as i64;

        // Create and return reconciliation run summary
        let run = ReconciliationRun {
            id: run_id,
            started_at,
            completed_at,
            checks_executed,
            checks_passed,
            checks_failed,
            total_drift_events: drift_events.len(),
            critical_drift_events: drift_events
                .iter()
                .filter(|d| d.severity == DriftSeverity::Critical)
                .count(),
            total_repairs_attempted: _repairs_attempted,
            successful_repairs: _successful_repairs,
            failed_repairs: 0,
            duration_ms,
        };

        run.save(&self.db).await?;
        self.metrics.record_cycle(&run);

        info!(
            "Reconciliation cycle complete: id={}, duration={}ms, checks_executed={}, drift_events={}",
            run_id, duration_ms, checks_executed, drift_events.len()
        );

        Ok(run)
    }

    /// Run all consistency checks
    async fn run_all_checks(&self) -> Result<Vec<ConsistencyCheckResult>> {
        let mut all_results = Vec::new();

        // Asset mapping check (always run, has no thresholds)
        let asset_results = check_asset_mapping(&self.db).await?;
        all_results.extend(asset_results);

        // Data staleness check
        let stale_results = check_data_staleness(&self.db, &self.thresholds).await?;
        all_results.extend(stale_results);

        // Price divergence check
        let price_results = check_price_divergence(&self.db, &self.thresholds).await?;
        all_results.extend(price_results);

        // Liquidity anomaly check
        let liquidity_results = check_liquidity_anomalies(&self.db, &self.thresholds).await?;
        all_results.extend(liquidity_results);

        // Ledger alignment check
        let ledger_results = check_ledger_alignment(&self.db, &self.thresholds).await?;
        all_results.extend(ledger_results);

        Ok(all_results)
    }

    /// Get current reconciliation metrics
    pub fn get_metrics(&self) -> &ReconciliationMetrics {
        &self.metrics
    }
}

/// Summary of a reconciliation run
#[derive(Debug, Clone)]
pub struct ReconciliationRun {
    pub id: Uuid,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: chrono::DateTime<chrono::Utc>,
    pub checks_executed: usize,
    pub checks_passed: usize,
    pub checks_failed: usize,
    pub total_drift_events: usize,
    pub critical_drift_events: usize,
    pub total_repairs_attempted: usize,
    pub successful_repairs: usize,
    pub failed_repairs: usize,
    pub duration_ms: i64,
}

impl ReconciliationRun {
    /// Save this run to the database
    pub async fn save(&self, db: &PgPool) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO reconciliation_runs (
                id, run_started_at, run_completed_at,
                checks_requested, checks_executed, checks_passed, checks_failed,
                total_drift_events, critical_drift_events,
                total_repairs_attempted, successful_repairs, failed_repairs,
                duration_ms
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
        )
        .bind(self.id)
        .bind(self.started_at)
        .bind(self.completed_at)
        .bind(self.checks_executed as i32)
        .bind(self.checks_executed as i32)
        .bind(self.checks_passed as i32)
        .bind(self.checks_failed as i32)
        .bind(self.total_drift_events as i32)
        .bind(self.critical_drift_events as i32)
        .bind(self.total_repairs_attempted as i32)
        .bind(self.successful_repairs as i32)
        .bind(self.failed_repairs as i32)
        .bind(self.duration_ms)
        .execute(db)
        .await
        .map_err(|e| crate::error::IndexerError::DatabaseQuery(e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reconciliation_run_construction() {
        let run = ReconciliationRun {
            id: Uuid::new_v4(),
            started_at: Utc::now(),
            completed_at: Utc::now(),
            checks_executed: 5,
            checks_passed: 3,
            checks_failed: 2,
            total_drift_events: 2,
            critical_drift_events: 1,
            total_repairs_attempted: 1,
            successful_repairs: 1,
            failed_repairs: 0,
            duration_ms: 100,
        };

        assert_eq!(run.checks_executed, 5);
        assert_eq!(run.checks_passed + run.checks_failed, run.checks_executed);
    }
}
