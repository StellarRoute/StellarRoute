//! Durable, priority-aware job queue backed by PostgreSQL.
//!
//! Jobs are dequeued in weighted-fair-queuing order:
//!   `ORDER BY virtual_time ASC, created_at ASC`
//!
//! This guarantees that higher-priority jobs are served first while
//! lower-priority jobs are never starved indefinitely.

use crate::error::{ApiError, Result};
use chrono::Utc;
use serde_json::Value;
use sqlx::{PgPool, Row};
use std::sync::Arc;

use super::job::{RouteComputationJob, RouteComputationTaskPayload};
use super::priority::RequestPriority;

/// Database-backed job queue for durable task persistence.
pub struct JobQueue {
    db: PgPool,
}

impl JobQueue {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Enqueue a new route computation job.
    ///
    /// Returns `true` if the job was inserted, `false` if it already exists
    /// (deduplication via `ON CONFLICT DO NOTHING`).
    pub async fn enqueue(&self, job: &RouteComputationJob) -> Result<bool> {
        let job_key = job.id.as_hash_key();
        let payload = serde_json::to_value(&job.payload).map_err(|e| {
            ApiError::Internal(Arc::new(anyhow::anyhow!(
                "Failed to serialize payload: {}",
                e
            )))
        })?;

        let priority_val = job.priority as i16;

        let result = sqlx::query(
            r#"
            INSERT INTO route_computation_jobs (
                job_key, status, payload, attempt, max_retries,
                priority, virtual_time, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (job_key) DO NOTHING
            "#,
        )
        .bind(&job_key)
        .bind("pending")
        .bind(payload)
        .bind(job.attempt as i32)
        .bind(job.max_retries as i32)
        .bind(priority_val)
        .bind(job.virtual_time)
        .bind(job.created_at)
        .bind(Utc::now())
        .execute(&self.db)
        .await
        .map_err(|e| {
            ApiError::Internal(Arc::new(anyhow::anyhow!("Failed to enqueue job: {}", e)))
        })?;

        Ok(result.rows_affected() > 0)
    }

    /// Dequeue the next pending job using WFQ ordering.
    ///
    /// Jobs are selected by `(virtual_time ASC, created_at ASC)` so that
    /// higher-priority (lower virtual_time) jobs are always served first,
    /// while lower-priority jobs are never permanently starved.
    pub async fn dequeue(&self) -> Result<Option<RouteComputationJob>> {
        let row = sqlx::query(
            r#"
            UPDATE route_computation_jobs
            SET status = 'processing', updated_at = NOW()
            WHERE id = (
                SELECT id FROM route_computation_jobs
                WHERE status = 'pending'
                ORDER BY virtual_time ASC, created_at ASC
                LIMIT 1
                FOR UPDATE SKIP LOCKED
            )
            RETURNING id, job_key, payload, attempt, max_retries,
                      priority, virtual_time, created_at
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

            let priority = RequestPriority::from_i16(r.get::<i16, _>("priority"));
            let virtual_time: i64 = r.get("virtual_time");

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
                priority,
                virtual_time,
            }))
        } else {
            Ok(None)
        }
    }

    /// Mark job as completed.
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

    /// Mark job as failed.
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

    /// Requeue a job for retry, preserving its original priority and virtual time.
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

    /// Get queue stats, broken down by priority band.
    pub async fn stats(&self) -> Result<QueueStats> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE status = 'pending')::BIGINT                          AS pending,
                COUNT(*) FILTER (WHERE status = 'processing')::BIGINT                       AS processing,
                COUNT(*) FILTER (WHERE status = 'completed')::BIGINT                        AS completed,
                COUNT(*) FILTER (WHERE status = 'failed')::BIGINT                           AS failed,
                COUNT(*) FILTER (WHERE status = 'pending' AND priority = 0)::BIGINT         AS pending_critical,
                COUNT(*) FILTER (WHERE status = 'pending' AND priority = 1)::BIGINT         AS pending_high,
                COUNT(*) FILTER (WHERE status = 'pending' AND priority = 2)::BIGINT         AS pending_normal,
                COUNT(*) FILTER (WHERE status = 'pending' AND priority = 3)::BIGINT         AS pending_low,
                COUNT(*) FILTER (WHERE status = 'processing' AND priority = 0)::BIGINT      AS processing_critical,
                COUNT(*) FILTER (WHERE status = 'processing' AND priority = 1)::BIGINT      AS processing_high,
                COUNT(*) FILTER (WHERE status = 'processing' AND priority = 2)::BIGINT      AS processing_normal,
                COUNT(*) FILTER (WHERE status = 'processing' AND priority = 3)::BIGINT      AS processing_low
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
            pending_by_priority: [
                row.get::<i64, _>("pending_critical") as usize,
                row.get::<i64, _>("pending_high") as usize,
                row.get::<i64, _>("pending_normal") as usize,
                row.get::<i64, _>("pending_low") as usize,
            ],
            processing_by_priority: [
                row.get::<i64, _>("processing_critical") as usize,
                row.get::<i64, _>("processing_high") as usize,
                row.get::<i64, _>("processing_normal") as usize,
                row.get::<i64, _>("processing_low") as usize,
            ],
        })
    }
}

/// Queue statistics snapshot, including per-priority breakdowns.
#[derive(Debug, Clone)]
pub struct QueueStats {
    pub pending: usize,
    pub processing: usize,
    pub completed: usize,
    pub failed: usize,
    /// `[critical, high, normal, low]` pending counts.
    pub pending_by_priority: [usize; 4],
    /// `[critical, high, normal, low]` processing counts.
    pub processing_by_priority: [usize; 4],
}

impl QueueStats {
    pub fn total_backlog(&self) -> usize {
        self.pending + self.processing
    }

    /// Pending count for a specific priority band.
    pub fn pending_for(&self, priority: RequestPriority) -> usize {
        self.pending_by_priority[priority as usize]
    }

    /// Processing count for a specific priority band.
    pub fn processing_for(&self, priority: RequestPriority) -> usize {
        self.processing_by_priority[priority as usize]
    }
}
