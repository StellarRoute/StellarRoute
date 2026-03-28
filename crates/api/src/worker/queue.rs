//! Durable job queue using database

use crate::error::{ApiError, Result};
use chrono::Utc;
use serde_json::Value;
use sqlx::{PgPool, Row};
use std::sync::Arc;

use super::job::{RouteComputationJob, RouteComputationTaskPayload};

/// Database-backed job queue for durable task persistence
pub struct JobQueue {
    db: PgPool,
}

impl JobQueue {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Enqueue a new route computation job
    /// Returns true if job was enqueued, false if it already exists
    pub async fn enqueue(&self, job: &RouteComputationJob) -> Result<bool> {
        let job_key = job.id.as_hash_key();
        let payload = serde_json::to_value(&job.payload).map_err(|e| {
            ApiError::Internal(Arc::new(anyhow::anyhow!(
                "Failed to serialize payload: {}",
                e
            )))
        })?;

        // Try to insert; if it already exists, return false (deduplication)
        let result = sqlx::query(
            r#"
            INSERT INTO route_computation_jobs (
                job_key, status, payload, attempt, max_retries, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (job_key) DO NOTHING
            "#,
        )
        .bind(&job_key)
        .bind("pending")
        .bind(payload)
        .bind(job.attempt as i32)
        .bind(job.max_retries as i32)
        .bind(job.created_at)
        .bind(Utc::now())
        .execute(&self.db)
        .await
        .map_err(|e| {
            ApiError::Internal(Arc::new(anyhow::anyhow!("Failed to enqueue job: {}", e)))
        })?;

        Ok(result.rows_affected() > 0)
    }

    /// Get next pending job for processing
    pub async fn dequeue(&self) -> Result<Option<RouteComputationJob>> {
        let row = sqlx::query(
            r#"
            UPDATE route_computation_jobs
            SET status = 'processing', updated_at = NOW()
            WHERE id = (
                SELECT id FROM route_computation_jobs
                WHERE status = 'pending'
                ORDER BY created_at ASC
                LIMIT 1
                FOR UPDATE SKIP LOCKED
            )
            RETURNING id, job_key, payload, attempt, max_retries, created_at
            "#,
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|e| {
            ApiError::Internal(Arc::new(anyhow::anyhow!("Failed to dequeue job: {}", e)))
        })?;

        if let Some(r) = row {
            let payload_json: Value = r.get("payload");
            let payload: RouteComputationTaskPayload = serde_json::from_value(payload_json)
                .map_err(|e| {
                    ApiError::Internal(Arc::new(anyhow::anyhow!("Failed to parse payload: {}", e)))
                })?;

            Ok(Some(RouteComputationJob {
                id: super::job::JobId::new(
                    &payload.base_asset,
                    &payload.quote_asset,
                    &format!("{:.7}", payload.amount),
                    &payload.quote_type,
                ),
                payload,
                created_at: r.get("created_at"),
                attempt: r.get::<i32, _>("attempt") as u32,
                max_retries: r.get::<i32, _>("max_retries") as u32,
            }))
        } else {
            Ok(None)
        }
    }

    /// Mark job as completed
    pub async fn mark_completed(&self, job_key: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE route_computation_jobs
            SET status = 'completed', updated_at = NOW()
            WHERE job_key = $1
            "#,
        )
        .bind(job_key)
        .execute(&self.db)
        .await
        .map_err(|e| {
            ApiError::Internal(Arc::new(anyhow::anyhow!(
                "Failed to mark job as completed: {}",
                e
            )))
        })?;

        Ok(())
    }

    /// Mark job as failed
    pub async fn mark_failed(&self, job_key: &str, error: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE route_computation_jobs
            SET status = 'failed', error_message = $1, updated_at = NOW()
            WHERE job_key = $2
            "#,
        )
        .bind(error)
        .bind(job_key)
        .execute(&self.db)
        .await
        .map_err(|e| {
            ApiError::Internal(Arc::new(anyhow::anyhow!(
                "Failed to mark job as failed: {}",
                e
            )))
        })?;

        Ok(())
    }

    /// Requeue job for retry
    pub async fn requeue(&self, job: RouteComputationJob) -> Result<()> {
        let job_key = job.id.as_hash_key();
        let next_attempt = job.attempt + 1;

        sqlx::query(
            r#"
            UPDATE route_computation_jobs
            SET status = 'pending', attempt = $1, updated_at = NOW()
            WHERE job_key = $2
            "#,
        )
        .bind(next_attempt as i32)
        .bind(&job_key)
        .execute(&self.db)
        .await
        .map_err(|e| {
            ApiError::Internal(Arc::new(anyhow::anyhow!("Failed to requeue job: {}", e)))
        })?;

        Ok(())
    }

    /// Get queue stats
    pub async fn stats(&self) -> Result<QueueStats> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE status = 'pending')::BIGINT as pending,
                COUNT(*) FILTER (WHERE status = 'processing')::BIGINT as processing,
                COUNT(*) FILTER (WHERE status = 'completed')::BIGINT as completed,
                COUNT(*) FILTER (WHERE status = 'failed')::BIGINT as failed
            FROM route_computation_jobs
            "#,
        )
        .fetch_one(&self.db)
        .await
        .map_err(|e| {
            ApiError::Internal(Arc::new(anyhow::anyhow!(
                "Failed to get queue stats: {}",
                e
            )))
        })?;

        Ok(QueueStats {
            pending: row.get::<i64, _>("pending") as usize,
            processing: row.get::<i64, _>("processing") as usize,
            completed: row.get::<i64, _>("completed") as usize,
            failed: row.get::<i64, _>("failed") as usize,
        })
    }
}

#[derive(Debug, Clone)]
pub struct QueueStats {
    pub pending: usize,
    pub processing: usize,
    pub completed: usize,
    pub failed: usize,
}

impl QueueStats {
    pub fn total_backlog(&self) -> usize {
        self.pending + self.processing
    }
}
