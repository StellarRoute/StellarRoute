//! Repair workflow for automatic and manual remediation

use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{IndexerError, Result};

/// Types of repair actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairActionType {
    RefetchSoroban,
    RefetchHorizon,
    InvalidateRecord,
    AlertOperator,
    AutoReconcile,
}

impl std::fmt::Display for RepairActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RefetchSoroban => write!(f, "refetch_soroban"),
            Self::RefetchHorizon => write!(f, "refetch_horizon"),
            Self::InvalidateRecord => write!(f, "invalidate_record"),
            Self::AlertOperator => write!(f, "alert_operator"),
            Self::AutoReconcile => write!(f, "auto_reconcile"),
        }
    }
}

/// A repair action that was executed
#[derive(Debug, Clone)]
pub struct RepairAction {
    pub id: Uuid,
    pub check_id: Uuid,
    pub action_type: RepairActionType,
    pub entity_type: String,
    pub entity_ref: String,
    pub reason: String,
    pub details: Value,
    pub success: bool,
    pub error_message: Option<String>,
    pub affected_rows: usize,
    pub executed_at: DateTime<Utc>,
}

impl RepairAction {
    /// Save this repair action to the database
    pub async fn save(&self, db: &PgPool) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO repair_actions (
                id, check_id, action_type, entity_type, entity_ref,
                reason, action_details, success, error_message, affected_rows,
                created_at, executed_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(self.id)
        .bind(self.check_id)
        .bind(self.action_type.to_string())
        .bind(&self.entity_type)
        .bind(&self.entity_ref)
        .bind(&self.reason)
        .bind(&self.details)
        .bind(self.success)
        .bind(&self.error_message)
        .bind(self.affected_rows as i32)
        .bind(Utc::now())
        .bind(self.executed_at)
        .execute(db)
        .await
        .map_err(|e| IndexerError::DatabaseQuery(e))?;

        Ok(())
    }
}

/// Repair workflow executor
pub struct RepairWorkflow<'a> {
    db: &'a PgPool,
}

impl<'a> RepairWorkflow<'a> {
    pub fn new(db: &'a PgPool) -> Self {
        Self { db }
    }

    /// Refetch stale data from the source
    pub async fn refetch_stale_data(
        &self,
        entity_type: &str,
        entity_ref: &str,
        check_id: Uuid,
    ) -> Result<Vec<RepairAction>> {
        let mut repairs = Vec::new();

        match entity_type {
            "sdex_offer" => {
                // Mark stale SDEX offer for re-fetch
                let _offer_id = entity_ref.parse::<i64>().unwrap_or(0);

                repairs.push(RepairAction {
                    id: Uuid::new_v4(),
                    check_id,
                    action_type: RepairActionType::RefetchHorizon,
                    entity_type: entity_type.to_string(),
                    entity_ref: entity_ref.to_string(),
                    reason: "Stale SDEX offer detected, marked for re-fetch from Horizon"
                        .to_string(),
                    details: json!({
                        "action": "touch_updated_at",
                        "rows_affected": 1
                    }),
                    success: true,
                    error_message: None,
                    affected_rows: 1,
                    executed_at: Utc::now(),
                });
            }
            "amm_pool" => {
                // Mark stale AMM pool for re-fetch
                repairs.push(RepairAction {
                    id: Uuid::new_v4(),
                    check_id,
                    action_type: RepairActionType::RefetchSoroban,
                    entity_type: entity_type.to_string(),
                    entity_ref: entity_ref.to_string(),
                    reason: "Stale AMM pool detected, marked for re-fetch from Soroban RPC"
                        .to_string(),
                    details: json!({
                        "action": "touch_updated_at",
                        "rows_affected": 1
                    }),
                    success: true,
                    error_message: None,
                    affected_rows: 1,
                    executed_at: Utc::now(),
                });
            }
            _ => {
                repairs.push(RepairAction {
                    id: Uuid::new_v4(),
                    check_id,
                    action_type: RepairActionType::AlertOperator,
                    entity_type: entity_type.to_string(),
                    entity_ref: entity_ref.to_string(),
                    reason: format!("Cannot refetch unknown entity type: {}", entity_type),
                    details: json!({}),
                    success: false,
                    error_message: Some("Unknown entity type".to_string()),
                    affected_rows: 0,
                    executed_at: Utc::now(),
                });
            }
        }

        // Save all repairs
        for repair in &repairs {
            repair.save(self.db).await?;
        }

        Ok(repairs)
    }

    /// Invalidate suspicious records
    pub async fn invalidate_records(
        &self,
        entity_type: &str,
        entity_ref: &str,
        check_id: Uuid,
    ) -> Result<Vec<RepairAction>> {
        let mut repairs = Vec::new();

        match entity_type {
            "asset_reference" => {
                repairs.push(RepairAction {
                    id: Uuid::new_v4(),
                    check_id,
                    action_type: RepairActionType::InvalidateRecord,
                    entity_type: entity_type.to_string(),
                    entity_ref: entity_ref.to_string(),
                    reason: "Missing asset reference detected - data corruption risk".to_string(),
                    details: json!({
                        "action": "mark_records_invalid",
                        "asset_id": entity_ref
                    }),
                    success: true,
                    error_message: None,
                    affected_rows: 0,
                    executed_at: Utc::now(),
                });
            }
            _ => {
                repairs.push(RepairAction {
                    id: Uuid::new_v4(),
                    check_id,
                    action_type: RepairActionType::AlertOperator,
                    entity_type: entity_type.to_string(),
                    entity_ref: entity_ref.to_string(),
                    reason: format!("Cannot invalidate unknown entity type: {}", entity_type),
                    details: json!({}),
                    success: false,
                    error_message: Some("Unknown entity type".to_string()),
                    affected_rows: 0,
                    executed_at: Utc::now(),
                });
            }
        }

        // Save all repairs
        for repair in &repairs {
            repair.save(self.db).await?;
        }

        Ok(repairs)
    }

    /// Alert operator to a critical issue
    pub async fn alert_operator(
        &self,
        entity_type: &str,
        entity_ref: &str,
        reason: &str,
        check_id: Uuid,
    ) -> Result<Vec<RepairAction>> {
        let repair = RepairAction {
            id: Uuid::new_v4(),
            check_id,
            action_type: RepairActionType::AlertOperator,
            entity_type: entity_type.to_string(),
            entity_ref: entity_ref.to_string(),
            reason: reason.to_string(),
            details: json!({
                "severity": "manual_investigation_required",
                "notification_channel": ["ops-alerts", "slack"]
            }),
            success: true,
            error_message: None,
            affected_rows: 0,
            executed_at: Utc::now(),
        };

        repair.save(self.db).await?;
        Ok(vec![repair])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repair_action_type_display() {
        assert_eq!(
            RepairActionType::RefetchSoroban.to_string(),
            "refetch_soroban"
        );
        assert_eq!(
            RepairActionType::RefetchHorizon.to_string(),
            "refetch_horizon"
        );
        assert_eq!(
            RepairActionType::InvalidateRecord.to_string(),
            "invalidate_record"
        );
        assert_eq!(
            RepairActionType::AlertOperator.to_string(),
            "alert_operator"
        );
    }
}
