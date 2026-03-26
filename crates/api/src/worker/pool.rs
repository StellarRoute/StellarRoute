//! Route computation worker pool

use crate::error::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use super::{
    backpressure::BackpressurePolicy,
    deduplication::DeduplicationCache,
    job::{RouteComputationJob, RouteComputationTaskPayload},
    queue::JobQueue,
    retry::RetryStrategy,
};

/// Configuration for the route worker pool
#[derive(Clone, Debug)]
pub struct WorkerPoolConfig {
    /// Number of worker threads
    pub num_workers: usize,
    /// Backpressure policy
    pub backpressure: BackpressurePolicy,
    /// Retry strategy
    pub retry_strategy: RetryStrategy,
    /// Deduplication cache TTL in seconds
    pub dedup_ttl_secs: u64,
}

impl Default for WorkerPoolConfig {
    fn default() -> Self {
        Self {
            num_workers: 10,
            backpressure: BackpressurePolicy::default(),
            retry_strategy: RetryStrategy::default(),
            dedup_ttl_secs: 300, // 5 minutes
        }
    }
}

/// State tracking for worker pool
#[derive(Default, Debug, Clone)]
struct PoolMetrics {
    total_submitted: Arc<RwLock<u64>>,
    total_completed: Arc<RwLock<u64>>,
    total_failed: Arc<RwLock<u64>>,
    total_rejected: Arc<RwLock<u64>>,
}

/// Distributed worker pool for route computation
pub struct RouteWorkerPool {
    config: WorkerPoolConfig,
    queue: JobQueue,
    dedup: DeduplicationCache,
    metrics: PoolMetrics,
}

impl RouteWorkerPool {
    pub fn new(config: WorkerPoolConfig, queue: JobQueue) -> Self {
        info!(
            "Initializing route worker pool with {} workers",
            config.num_workers
        );

        Self {
            config,
            queue,
            dedup: DeduplicationCache::new(),
            metrics: PoolMetrics::default(),
        }
    }

    /// Submit a route computation job to the queue
    pub async fn submit_job(
        &self,
        base: &str,
        quote: &str,
        payload: RouteComputationTaskPayload,
    ) -> Result<()> {
        // Check backpressure first
        let stats = self.queue.stats().await?;
        self.config
            .backpressure
            .should_accept(stats.pending, stats.processing)?;

        // Create job with retry policy
        let job = RouteComputationJob::new(
            base,
            quote,
            payload,
            self.config.retry_strategy.max_retries,
        );

        // Check if job already being processed (deduplication)
        if !self.dedup.try_add(&job.id).await {
            // Job is already being processed, increment rejected counter
            let mut rejected = self.metrics.total_rejected.write().await;
            *rejected += 1;
            return Ok(()); // Don't error out, just ignore duplicate
        }

        // Try to enqueue the job
        match self.queue.enqueue(&job).await {
            Ok(enqueued) => {
                if enqueued {
                    let mut submitted = self.metrics.total_submitted.write().await;
                    *submitted += 1;
                }
                Ok(())
            }
            Err(e) => {
                // Remove from dedup cache on failure
                self.dedup.remove(&job.id).await;
                Err(e)
            }
        }
    }

    /// Get next job for worker processing
    pub async fn get_next_job(&self) -> Result<Option<RouteComputationJob>> {
        self.queue.dequeue().await
    }

    /// Report successful job completion
    pub async fn mark_success(&self, job: &RouteComputationJob) -> Result<()> {
        let job_key = job.id.as_hash_key();
        self.queue.mark_completed(&job_key).await?;
        self.dedup.remove(&job.id).await;

        let mut completed = self.metrics.total_completed.write().await;
        *completed += 1;

        Ok(())
    }

    /// Report job failure with retry logic
    pub async fn mark_failure(&self, job: RouteComputationJob, error: &str) -> Result<()> {
        let job_key = job.id.as_hash_key();
        let job_id = job.id.clone();
        let is_exhausted = job.is_exhausted();
        let attempt = job.attempt;
        let max_retries = job.max_retries;

        // Check if we should retry
        if !is_exhausted && self.config.retry_strategy.is_retryable(error) {
            warn!(
                "Job {} failed (attempt {}/{}), retrying: {}",
                job_key, attempt, max_retries, error
            );
            self.queue.requeue(job).await?;
        } else {
            error!(
                "Job {} exhausted after {} attempts: {}",
                job_key, attempt, error
            );
            self.queue.mark_failed(&job_key, error).await?;

            let mut failed = self.metrics.total_failed.write().await;
            *failed += 1;
        }

        self.dedup.remove(&job_id).await;
        Ok(())
    }

    /// Get pool metrics snapshot
    pub async fn metrics(&self) -> PoolMetricsSnapshot {
        let submitted = *self.metrics.total_submitted.read().await;
        let completed = *self.metrics.total_completed.read().await;
        let failed = *self.metrics.total_failed.read().await;
        let rejected = *self.metrics.total_rejected.read().await;

        let queue_stats = self
            .queue
            .stats()
            .await
            .unwrap_or_else(|_| super::queue::QueueStats {
                pending: 0,
                processing: 0,
                completed: 0,
                failed: 0,
            });

        PoolMetricsSnapshot {
            total_submitted: submitted,
            total_completed: completed,
            total_failed: failed,
            total_rejected: rejected,
            pending_jobs: queue_stats.pending,
            processing_jobs: queue_stats.processing,
            queue_depth: queue_stats.total_backlog(),
            dedup_cache_size: self.dedup.size().await,
            load_score: self
                .config
                .backpressure
                .load_score(queue_stats.pending, queue_stats.processing),
        }
    }

    /// Perform periodic cleanup
    pub async fn cleanup(&self) -> Result<()> {
        self.dedup
            .cleanup_expired(self.config.dedup_ttl_secs)
            .await;
        Ok(())
    }
}

/// Snapshot of worker pool metrics
#[derive(Debug, Clone)]
pub struct PoolMetricsSnapshot {
    pub total_submitted: u64,
    pub total_completed: u64,
    pub total_failed: u64,
    pub total_rejected: u64,
    pub pending_jobs: usize,
    pub processing_jobs: usize,
    pub queue_depth: usize,
    pub dedup_cache_size: usize,
    pub load_score: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    // Note: These tests would require a test database
    // They're provided as examples for integration testing
}
