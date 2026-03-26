//! Drift metrics collection and emission

use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::PgPool;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use uuid::Uuid;

use super::consistency::{ConsistencyCheckResult, DriftSeverity};
use super::engine::ReconciliationRun;
use crate::error::{IndexerError, Result};

/// Thread-safe drift metrics collector
pub struct ReconciliationMetrics {
    total_checks: Arc<AtomicU64>,
    passed_checks: Arc<AtomicU64>,
    failed_checks: Arc<AtomicU64>,
    critical_drifts: Arc<AtomicU64>,
    repairs_attempted: Arc<AtomicU64>,
    repairs_successful: Arc<AtomicU64>,
}

impl ReconciliationMetrics {
    pub fn new() -> Self {
        Self {
            total_checks: Arc::new(AtomicU64::new(0)),
            passed_checks: Arc::new(AtomicU64::new(0)),
            failed_checks: Arc::new(AtomicU64::new(0)),
            critical_drifts: Arc::new(AtomicU64::new(0)),
            repairs_attempted: Arc::new(AtomicU64::new(0)),
            repairs_successful: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Record a reconciliation cycle
    pub fn record_cycle(&self, run: &ReconciliationRun) {
        self.total_checks
            .fetch_add(run.checks_executed as u64, Ordering::Relaxed);
        self.passed_checks
            .fetch_add(run.checks_passed as u64, Ordering::Relaxed);
        self.failed_checks
            .fetch_add(run.checks_failed as u64, Ordering::Relaxed);
        self.critical_drifts
            .fetch_add(run.critical_drift_events as u64, Ordering::Relaxed);
        self.repairs_attempted
            .fetch_add(run.total_repairs_attempted as u64, Ordering::Relaxed);
        self.repairs_successful
            .fetch_add(run.successful_repairs as u64, Ordering::Relaxed);
    }

    /// Get current metrics snapshot
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            total_checks: self.total_checks.load(Ordering::Relaxed),
            passed_checks: self.passed_checks.load(Ordering::Relaxed),
            failed_checks: self.failed_checks.load(Ordering::Relaxed),
            critical_drifts: self.critical_drifts.load(Ordering::Relaxed),
            repairs_attempted: self.repairs_attempted.load(Ordering::Relaxed),
            repairs_successful: self.repairs_successful.load(Ordering::Relaxed),
        }
    }
}

impl Default for ReconciliationMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of reconciliation metrics
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub total_checks: u64,
    pub passed_checks: u64,
    pub failed_checks: u64,
    pub critical_drifts: u64,
    pub repairs_attempted: u64,
    pub repairs_successful: u64,
}

impl MetricsSnapshot {
    /// Calculate success rate percentage
    pub fn success_rate_pct(&self) -> f64 {
        if self.total_checks == 0 {
            100.0
        } else {
            (self.passed_checks as f64 / self.total_checks as f64) * 100.0
        }
    }

    /// Calculate repair success rate
    pub fn repair_success_rate_pct(&self) -> f64 {
        if self.repairs_attempted == 0 {
            100.0
        } else {
            (self.repairs_successful as f64 / self.repairs_attempted as f64) * 100.0
        }
    }
}

/// Recorded drift metric event
#[derive(Debug, Clone)]
pub struct DriftMetrics {
    pub check_id: Option<Uuid>,
    pub entity_type: String,
    pub entity_ref: String,
    pub drift_category: String,
    pub metric_name: String,
    pub metric_value: Option<f64>,
    pub metric_unit: Option<String>,
    pub threshold_value: Option<f64>,
    pub breach: bool,
    pub severity: DriftSeverity,
    pub recorded_at: DateTime<Utc>,
}

impl DriftMetrics {
    /// Create a drift metric from a consistency check result
    pub fn from_check_result(result: &ConsistencyCheckResult) -> Self {
        let (category, metric_name, threshold) = match result.check_type {
            super::consistency::CheckType::DataStaleness => (
                "staleness".to_string(),
                "staleness_secs".to_string(),
                Some(300.0),
            ),
            super::consistency::CheckType::PriceDivergence => (
                "price".to_string(),
                "price_divergence_pct".to_string(),
                Some(2.5),
            ),
            super::consistency::CheckType::LiquidityAnomaly => (
                "liquidity".to_string(),
                "reserve_change_pct".to_string(),
                Some(15.0),
            ),
            super::consistency::CheckType::LedgerAlignment => (
                "ledger".to_string(),
                "ledger_lag_blocks".to_string(),
                Some(100.0),
            ),
            super::consistency::CheckType::AssetMapping => (
                "asset_mapping".to_string(),
                "missing_asset_references".to_string(),
                None,
            ),
        };

        let metric_value = result.drift_percentage;
        let breach = metric_value
            .zip(threshold)
            .map(|(mv, tv)| mv > tv)
            .unwrap_or(result.severity != DriftSeverity::Info);

        Self {
            check_id: None,
            entity_type: result.entity_type.clone(),
            entity_ref: result.entity_ref.clone(),
            drift_category: category,
            metric_name,
            metric_value,
            metric_unit: Some("percent".to_string()),
            threshold_value: threshold,
            breach,
            severity: result.severity,
            recorded_at: result.timestamp,
        }
    }

    /// Save this drift event to the database
    pub async fn save(&self, db: &PgPool) -> Result<Uuid> {
        let id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO drift_events (
                id, check_id, entity_type, entity_ref,
                drift_category, metric_name, metric_value, metric_unit,
                threshold_value, breach, metadata, recorded_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(id)
        .bind(self.check_id)
        .bind(&self.entity_type)
        .bind(&self.entity_ref)
        .bind(&self.drift_category)
        .bind(&self.metric_name)
        .bind(self.metric_value)
        .bind(&self.metric_unit)
        .bind(self.threshold_value)
        .bind(self.breach)
        .bind(json!({ "severity": self.severity.to_string() }))
        .bind(self.recorded_at)
        .execute(db)
        .await
        .map_err(|e| IndexerError::DatabaseQuery(e))?;

        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_success_rate() {
        let metrics = MetricsSnapshot {
            total_checks: 100,
            passed_checks: 95,
            failed_checks: 5,
            critical_drifts: 1,
            repairs_attempted: 1,
            repairs_successful: 1,
        };

        assert_eq!(metrics.success_rate_pct(), 95.0);
        assert_eq!(metrics.repair_success_rate_pct(), 100.0);
    }

    #[test]
    fn test_metrics_zero_checks() {
        let metrics = MetricsSnapshot {
            total_checks: 0,
            passed_checks: 0,
            failed_checks: 0,
            critical_drifts: 0,
            repairs_attempted: 0,
            repairs_successful: 0,
        };

        assert_eq!(metrics.success_rate_pct(), 100.0);
        assert_eq!(metrics.repair_success_rate_pct(), 100.0);
    }
}
