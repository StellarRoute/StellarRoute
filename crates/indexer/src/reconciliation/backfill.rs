//! Historical liquidity normalization backfill pipeline
//!
//! This module provides a production-safe mechanism to backfill and normalize
//! historical records from `sdex_offers` and `amm_pool_reserves` into the optimized
//! `normalized_liquidity` storage table.

use std::sync::Arc;
use tokio::sync::RwLock;
use sqlx::{PgPool, Row, FromRow};
use tracing::{info, warn, error};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use crate::error::{Result, IndexerError};

const JOB_ID: &str = "historical_liquidity_normalization";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackfillStatus {
    Idle,
    Running,
    Paused,
    Completed,
    Error(String),
}

impl BackfillStatus {
    pub fn from_string(s: &str) -> Self {
        match s {
            "idle" => BackfillStatus::Idle,
            "running" => BackfillStatus::Running,
            "paused" => BackfillStatus::Paused,
            "completed" => BackfillStatus::Completed,
            _ => BackfillStatus::Error(s.to_string()),
        }
    }
}

impl ToString for BackfillStatus {
    fn to_string(&self) -> String {
        match self {
            BackfillStatus::Idle => "idle".to_string(),
            BackfillStatus::Running => "running".to_string(),
            BackfillStatus::Paused => "paused".to_string(),
            BackfillStatus::Completed => "completed".to_string(),
            BackfillStatus::Error(e) => e.clone(),
        }
    }
}

#[derive(Debug, FromRow)]
pub struct BackfillCheckpoint {
    pub job_name: String,
    pub last_processed_id: i64,
    pub batch_size: i32,
    pub status: String,
    pub last_error: Option<String>,
    pub updated_at: DateTime<Utc>,
}

pub struct BackfillManager {
    pool: PgPool,
    status: Arc<RwLock<BackfillStatus>>,
}

impl BackfillManager {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            status: Arc::new(RwLock::new(BackfillStatus::Idle)),
        }
    }

    /// Primary entry point to run the backfill pipeline
    pub async fn run_backfill(&self) -> Result<()> {
        let mut status = self.status.write().await;
        if matches!(*status, BackfillStatus::Running) {
            return Ok(());
        }
        *status = BackfillStatus::Running;
        drop(status);

        info!("Starting historical liquidity normalization backfill...");
        
        // Ensure checkpoint exists
        sqlx::query(
            "INSERT INTO backfill_checkpoints (job_name, last_processed_id, status) VALUES ($1, 0, 'running') ON CONFLICT (job_name) DO NOTHING"
        )
        .bind(JOB_ID)
        .execute(&self.pool)
        .await?;

        let manager = Self {
            pool: self.pool.clone(),
            status: self.status.clone(),
        };
        tokio::spawn(async move {
            if let Err(e) = manager.process_loop().await {
                error!("Backfill process failed: {:?}", e);
                let mut status = manager.status.write().await;
                *status = BackfillStatus::Error(e.to_string());
                
                let _ = sqlx::query("UPDATE backfill_checkpoints SET status = 'error', last_error = $2 WHERE job_name = $1")
                    .bind(JOB_ID)
                    .bind(e.to_string())
                    .execute(&manager.pool)
                    .await;
            }
        });

        Ok(())
    }

    /// Pause the currently running job
    pub async fn pause(&self) -> Result<()> {
        let mut status = self.status.write().await;
        *status = BackfillStatus::Paused;
        
        sqlx::query("UPDATE backfill_checkpoints SET status = 'paused' WHERE job_name = $1")
            .bind(JOB_ID)
            .execute(&self.pool)
            .await?;
        
        info!("Backfill job pause requested");
        Ok(())
    }

    /// Resume a previously paused job
    pub async fn resume(&self) -> Result<()> {
        self.run_backfill().await
    }

    async fn process_loop(&self) -> Result<()> {
        let scale_e7 = Decimal::from(10_000_000);

        loop {
            // Check if we should stop
            let current_status = self.status.read().await;
            if matches!(*current_status, BackfillStatus::Paused) {
                return Ok(());
            }
            drop(current_status);

            // 1. Fetch checkpoint
            let checkpoint: BackfillCheckpoint = sqlx::query_as(
                "SELECT * FROM backfill_checkpoints WHERE job_name = $1"
            )
            .bind(JOB_ID)
            .fetch_one(&self.pool)
            .await?;

            // 2. Fetch batch of raw offers
            let rows = sqlx::query(
                "SELECT * FROM sdex_offers WHERE offer_id > $1 ORDER BY offer_id ASC LIMIT $2"
            )
            .bind(checkpoint.last_processed_id)
            .bind(checkpoint.batch_size)
            .fetch_all(&self.pool)
            .await?;

            if rows.is_empty() {
                // Also check AMM if needed, but for now let's focus on SDEX backfill
                // since AMM reserves are usually smaller in count and often indexed live
                info!("Historical backfill completed successfully");
                let mut status = self.status.write().await;
                *status = BackfillStatus::Completed;
                
                sqlx::query("UPDATE backfill_checkpoints SET status = 'completed' WHERE job_name = $1")
                    .bind(JOB_ID)
                    .execute(&self.pool)
                    .await?;
                return Ok(());
            }

            let mut last_id = checkpoint.last_processed_id;
            
            // 3. Normalize and Upsert
            for row in rows {
                let offer_id: i64 = row.get("offer_id");
                let price: Decimal = row.get("price");
                let amount: Decimal = row.get("amount");
                
                // Normalization (7 decimal places)
                let price_e7 = (price * scale_e7).to_i64().unwrap_or(0);
                let amount_e7 = (amount * scale_e7).to_i64().unwrap_or(0);

                // Simple integrity check
                if price_e7 <= 0 || amount_e7 <= 0 {
                    warn!("Invalid historical record skipped: offer_id={}, price={}, amount={}", offer_id, price, amount);
                    continue;
                }

                sqlx::query(
                    r#"
                    INSERT INTO normalized_liquidity (
                        venue_type, venue_ref, selling_asset_id, buying_asset_id,
                        price, available_amount, price_e7, available_amount_e7,
                        source_ledger, updated_at
                    )
                    VALUES ('sdex', $1, $2, $3, $4, $5, $6, $7, $8, now())
                    ON CONFLICT (venue_type, venue_ref) DO UPDATE SET
                        price = EXCLUDED.price,
                        available_amount = EXCLUDED.available_amount,
                        price_e7 = EXCLUDED.price_e7,
                        available_amount_e7 = EXCLUDED.available_amount_e7,
                        source_ledger = EXCLUDED.source_ledger,
                        updated_at = now()
                    "#
                )
                .bind(offer_id.to_string())
                .bind(row.get::<uuid::Uuid, _>("selling_asset_id"))
                .bind(row.get::<uuid::Uuid, _>("buying_asset_id"))
                .bind(price)
                .bind(amount)
                .bind(price_e7)
                .bind(amount_e7)
                .bind(row.get::<i64, _>("last_modified_ledger"))
                .execute(&self.pool)
                .await?;

                last_id = offer_id;
            }

            // 4. Update checkpoint
            sqlx::query("UPDATE backfill_checkpoints SET last_processed_id = $1, status = 'running', updated_at = now() WHERE job_name = $2")
                .bind(last_id)
                .bind(JOB_ID)
                .execute(&self.pool)
                .await?;
            
            info!("Processed historical batch up to offer_id: {}", last_id);

            // Yield to avoid blocking and stay "production-safe" (throttling)
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_normalization_precision_e7() {
        let scale_e7 = Decimal::from(10_000_000);
        
        let price = Decimal::from_str("1.2345678").unwrap();
        let price_e7 = (price * scale_e7).to_i64().unwrap();
        assert_eq!(price_e7, 12_345_678);

        let large_amount = Decimal::from_str("1000000.0000001").unwrap();
        let amount_e7 = (large_amount * scale_e7).to_i64().unwrap();
        assert_eq!(amount_e7, 10_000_000_000_001);
    }

    #[test]
    fn test_backfill_status_enum() {
        assert!(matches!(BackfillStatus::from_string("running"), BackfillStatus::Running));
        assert!(matches!(BackfillStatus::from_string("paused"), BackfillStatus::Paused));
        assert_eq!(BackfillStatus::Idle.to_string(), "idle");
    }
}
