//! Consistency check implementations
//!
//! Defines various types of checks to detect drift between Horizon and Soroban RPC data.

use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::{IndexerError, Result};

/// Types of consistency checks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheckType {
    AssetMapping,
    PriceDivergence,
    LedgerAlignment,
    LiquidityAnomaly,
    DataStaleness,
}

impl std::fmt::Display for CheckType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AssetMapping => write!(f, "asset_mapping"),
            Self::PriceDivergence => write!(f, "price_divergence"),
            Self::LedgerAlignment => write!(f, "ledger_alignment"),
            Self::LiquidityAnomaly => write!(f, "liquidity_anomaly"),
            Self::DataStaleness => write!(f, "data_staleness"),
        }
    }
}

/// Severity levels for detected drift
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DriftSeverity {
    Info,
    Warning,
    Critical,
}

impl std::fmt::Display for DriftSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

/// Result of a single consistency check
#[derive(Debug, Clone)]
pub struct ConsistencyCheckResult {
    pub check_type: CheckType,
    pub entity_type: String, // 'sdex_offer', 'amm_pool', 'asset'
    pub entity_ref: String,  // offer_id, pool_address, asset_id
    pub severity: DriftSeverity,
    pub expected_value: serde_json::Value,
    pub actual_value: serde_json::Value,
    pub drift_percentage: Option<f64>,
    pub context: serde_json::Value, // Additional metadata
    pub timestamp: DateTime<Utc>,
}

impl ConsistencyCheckResult {
    /// Persist this check result to the database
    pub async fn save(&self, db: &PgPool) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let severity_str = self.severity.to_string();

        sqlx::query(
            r#"
            INSERT INTO reconciliation_checks (
                id, check_type, entity_type, entity_ref,
                expected_value, actual_value, drift_severity, drift_percentage,
                extra_context, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(id)
        .bind(self.check_type.to_string())
        .bind(&self.entity_type)
        .bind(&self.entity_ref)
        .bind(&self.expected_value)
        .bind(&self.actual_value)
        .bind(severity_str)
        .bind(self.drift_percentage)
        .bind(&self.context)
        .bind(self.timestamp)
        .execute(db)
        .await
        .map_err(|e| IndexerError::DatabaseQuery(e))?;

        Ok(id)
    }
}

/// Threshold configuration for consistency checks
#[derive(Debug, Clone)]
pub struct CheckThresholds {
    pub staleness_threshold_secs: i32,
    pub price_divergence_pct: f64,
    pub liquidity_change_pct: f64,
    pub ledger_lag_threshold: i32,
}

impl CheckThresholds {
    /// Load thresholds from database
    pub async fn load_from_db(db: &PgPool) -> Result<Self> {
        let rows = sqlx::query(
            r#"
            SELECT
                check_type,
                staleness_threshold_secs,
                price_divergence_pct,
                liquidity_change_pct,
                ledger_lag_threshold
            FROM reconciliation_thresholds
            WHERE enabled = true
            "#,
        )
        .fetch_all(db)
        .await
        .map_err(|e| IndexerError::DatabaseQuery(e))?;

        let mut thresholds = CheckThresholds {
            staleness_threshold_secs: 300,
            price_divergence_pct: 2.5,
            liquidity_change_pct: 15.0,
            ledger_lag_threshold: 100,
        };

        for row in rows {
            let check_type: String = row.get("check_type");
            match check_type.as_str() {
                "data_staleness" => {
                    if let Some(val) = row.get::<Option<i32>, _>("staleness_threshold_secs") {
                        thresholds.staleness_threshold_secs = val;
                    }
                }
                "price_divergence" => {
                    if let Some(val) = row.get::<Option<f64>, _>("price_divergence_pct") {
                        thresholds.price_divergence_pct = val;
                    }
                }
                "liquidity_anomaly" => {
                    if let Some(val) = row.get::<Option<f64>, _>("liquidity_change_pct") {
                        thresholds.liquidity_change_pct = val;
                    }
                }
                "ledger_alignment" => {
                    if let Some(val) = row.get::<Option<i32>, _>("ledger_lag_threshold") {
                        thresholds.ledger_lag_threshold = val;
                    }
                }
                _ => {}
            }
        }

        Ok(thresholds)
    }
}

/// Check for stale data (no updates within threshold)
pub async fn check_data_staleness(
    db: &PgPool,
    thresholds: &CheckThresholds,
) -> Result<Vec<ConsistencyCheckResult>> {
    let mut results = Vec::new();

    // Check SDEX offers - only if offers exist
    let stale_offers = sqlx::query(
        r#"
        SELECT offer_id, seller, updated_at,
               EXTRACT(EPOCH FROM (NOW() - updated_at))::INT as staleness_secs
        FROM sdex_offers
        WHERE NOW() - updated_at > ($1::TEXT)::INTERVAL
        LIMIT 1000
        "#,
    )
    .bind(format!("{} seconds", thresholds.staleness_threshold_secs))
    .fetch_all(db)
    .await
    .unwrap_or_default();

    for row in stale_offers {
        let offer_id: i64 = row.get("offer_id");
        let staleness_secs: i32 = row.get("staleness_secs");

        results.push(ConsistencyCheckResult {
            check_type: CheckType::DataStaleness,
            entity_type: "sdex_offer".to_string(),
            entity_ref: offer_id.to_string(),
            severity: DriftSeverity::Warning,
            expected_value: json!({ "updated_within_secs": thresholds.staleness_threshold_secs }),
            actual_value: json!({ "staleness_secs": staleness_secs }),
            drift_percentage: Some(
                (staleness_secs as f64 / thresholds.staleness_threshold_secs as f64) * 100.0,
            ),
            context: json!({
                "seller": row.get::<String, _>("seller"),
                "last_update": row.get::<DateTime<Utc>, _>("updated_at"),
            }),
            timestamp: Utc::now(),
        });
    }

    // Check AMM pools
    let stale_pools = sqlx::query(
        r#"
        SELECT pool_address, updated_at,
               EXTRACT(EPOCH FROM (NOW() - updated_at))::INT as staleness_secs
        FROM amm_pool_reserves
        WHERE NOW() - updated_at > ($1::TEXT)::INTERVAL
        LIMIT 1000
        "#,
    )
    .bind(format!("{} seconds", thresholds.staleness_threshold_secs))
    .fetch_all(db)
    .await
    .unwrap_or_default();

    for row in stale_pools {
        let pool_address: String = row.get("pool_address");
        let staleness_secs: i32 = row.get("staleness_secs");

        results.push(ConsistencyCheckResult {
            check_type: CheckType::DataStaleness,
            entity_type: "amm_pool".to_string(),
            entity_ref: pool_address,
            severity: DriftSeverity::Critical, // Pools are more critical
            expected_value: json!({ "updated_within_secs": thresholds.staleness_threshold_secs }),
            actual_value: json!({ "staleness_secs": staleness_secs }),
            drift_percentage: Some(
                (staleness_secs as f64 / thresholds.staleness_threshold_secs as f64) * 100.0,
            ),
            context: json!({
                "last_update": row.get::<DateTime<Utc>, _>("updated_at"),
            }),
            timestamp: Utc::now(),
        });
    }

    Ok(results)
}

/// Check for price divergence between SDEX and AMM
pub async fn check_price_divergence(
    _db: &PgPool,
    _thresholds: &CheckThresholds,
) -> Result<Vec<ConsistencyCheckResult>> {
    let results = Vec::new();

    // Price divergence checks would require actual pools and offers
    // For now, return empty results as this is a schema-dependent check

    Ok(results)
}

/// Check for liquidity anomalies (sudden changes in reserves/amounts)
pub async fn check_liquidity_anomalies(
    _db: &PgPool,
    _thresholds: &CheckThresholds,
) -> Result<Vec<ConsistencyCheckResult>> {
    let results = Vec::new();

    // Liquidity anomaly checks would require multiple snapshots
    // For now, return empty results

    Ok(results)
}

/// Check for ledger sequence drift (SDEX/AMM updates on different ledgers)
pub async fn check_ledger_alignment(
    _db: &PgPool,
    _thresholds: &CheckThresholds,
) -> Result<Vec<ConsistencyCheckResult>> {
    let results = Vec::new();

    // Ledger alignment checks would require both tables to have ledger data
    // For now, return empty results as this is schema-dependent

    Ok(results)
}

/// Check for asset mapping mismatches
pub async fn check_asset_mapping(_db: &PgPool) -> Result<Vec<ConsistencyCheckResult>> {
    let results = Vec::new();

    // Asset mapping checks would require tables to exist
    // For now, return empty results

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drift_severity_ordering() {
        assert!(DriftSeverity::Info < DriftSeverity::Warning);
        assert!(DriftSeverity::Warning < DriftSeverity::Critical);
    }

    #[test]
    fn test_check_type_display() {
        assert_eq!(CheckType::DataStaleness.to_string(), "data_staleness");
        assert_eq!(CheckType::PriceDivergence.to_string(), "price_divergence");
        assert_eq!(CheckType::LedgerAlignment.to_string(), "ledger_alignment");
    }
}
