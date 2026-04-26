//! Route computation worker pool with adaptive priority scheduling.
//!
//! The pool classifies every incoming job via [`PriorityClassifier`] and
//! stores the resulting priority band + WFQ virtual time alongside the job
//! in the database.  Workers dequeue jobs in virtual-time order, which
//! ensures high-priority requests are served first while preventing
//! starvation of lower-priority ones.

use crate::error::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use super::{
    backpressure::BackpressurePolicy,
    deduplication::DeduplicationCache,
    job::{RouteComputationJob, RouteComputationTaskPayload},
    priority::{PriorityClassifier, PriorityConfig},
    queue::JobQueue,
    retry::RetryStrategy,
};

/// Configuration for the route worker pool.
#[derive(Clone, Debug)]
pub struct WorkerPoolConfig {
    /// Number of worker threads.
    pub num_workers: usize,
    /// Backpressure policy.
    pub backpressure: BackpressurePolicy,
    /// Retry strategy.
    pub retry_strategy: RetryStrategy,
    /// Deduplication cache TTL in seconds.
    pub dedup_ttl_secs: u64,
    /// Priority classification and WFQ scheduler configuration.
    pub priority: PriorityConfig,
}

impl Default for WorkerPoolConfig {
    fn default() -> Self {
        Self {
            num_workers: 10,
            backpressure: BackpressurePolicy::default(),
            retry_strategy: RetryStrategy::default(),
            dedup_ttl_secs: 300, // 5 minutes
            priority: PriorityConfig::default(),
        }
    }
}

/// Internal metrics counters.
#[derive(Default, Debug, Clone)]
struct PoolMetrics {
    total_submitted: Arc<RwLock<u64>>,
    total_completed: Arc<RwLock<u64>>,
    total_failed: Arc<RwLock<u64>>,
    total_rejected: Arc<RwLock<u64>>,
    /// Per-priority submission counters: `[critical, high, normal, low]`.
    submitted_by_priority: [Arc<RwLock<u64>>; 4],
    /// Per-priority completion counters: `[critical, high, normal, low]`.
    completed_by_priority: [Arc<RwLock<u64>>; 4],
}

impl PoolMetrics {
    fn new() -> Self {
        Self {
            total_submitted: Arc::new(RwLock::new(0)),
            total_completed: Arc::new(RwLock::new(0)),
            total_failed: Arc::new(RwLock::new(0)),
            total_rejected: Arc::new(RwLock::new(0)),
            submitted_by_priority: [
                Arc::new(RwLock::new(0)),
                Arc::new(RwLock::new(0)),
                Arc::new(RwLock::new(0)),
                Arc::new(RwLock::new(0)),
            ],
            completed_by_priority: [
                Arc::new(RwLock::new(0)),
                Arc::new(RwLock::new(0)),
                Arc::new(RwLock::new(0)),
                Arc::new(RwLock::new(0)),
            ],
        }
    }
}

/// Distributed worker pool for route computation with adaptive prioritization.
pub struct RouteWorkerPool {
    config: WorkerPoolConfig,
    queue: JobQueue,
    dedup: DeduplicationCache,
    metrics: PoolMetrics,
    /// Priority classifier shared with the rest of the application.
    pub classifier: PriorityClassifier,
}

impl RouteWorkerPool {
    pub fn new(config: WorkerPoolConfig, queue: JobQueue) -> Self {
        info!(
            "Initializing route worker pool with {} workers (priority scheduling enabled)",
            config.num_workers
        );

        let classifier = PriorityClassifier::new(config.priority.clone());

        Self {
            config,
            queue,
            dedup: DeduplicationCache::new(),
            metrics: PoolMetrics::new(),
            classifier,
        }
    }

    /// Submit a route computation job to the priority queue.
    ///
    /// The job is classified into a priority band based on `amount` and
    /// `is_batch`.  A WFQ virtual time is computed and stored with the job
    /// so that the dequeue query can order jobs correctly.
    pub async fn submit_job(
        &self,
        base: &str,
        quote: &str,
        payload: RouteComputationTaskPayload,
    ) -> Result<()> {
        self.submit_job_with_options(base, quote, payload, false)
            .await
    }

    /// Like [`submit_job`] but allows the caller to mark the request as a
    /// batch job (which forces [`RequestPriority::Low`]).
    pub async fn submit_job_with_options(
        &self,
        base: &str,
        quote: &str,
        payload: RouteComputationTaskPayload,
        is_batch: bool,
    ) -> Result<()> {
        // ── 1. Backpressure check ─────────────────────────────────────────
        let stats = self.queue.stats().await?;
        self.config
            .backpressure
            .should_accept(stats.pending, stats.processing)?;

        // ── 2. Classify priority ──────────────────────────────────────────
        let priority = self.classifier.classify(payload.amount, is_batch);
        let virtual_time = self.classifier.next_virtual_time(priority);

        // ── 3. Build job ──────────────────────────────────────────────────
        let job =
            RouteComputationJob::new(base, quote, payload, self.config.retry_strategy.max_retries)
                .with_priority(priority, virtual_time);

        // ── 4. Deduplication ──────────────────────────────────────────────
        if !self.dedup.try_add(&job.id).await {
            let mut rejected = self.metrics.total_rejected.write().await;
            *rejected += 1;
            return Ok(()); // Duplicate — silently ignore
        }

        // ── 5. Enqueue ────────────────────────────────────────────────────
        match self.queue.enqueue(&job).await {
            Ok(enqueued) => {
                if enqueued {
                    let mut submitted = self.metrics.total_submitted.write().await;
                    *submitted += 1;

                    let idx = priority as usize;
                    let mut band = self.metrics.submitted_by_priority[idx].write().await;
                    *band += 1;

                    // Emit Prometheus counter
                    crate::metrics::record_queue_submission(priority.as_str());

                    info!(
                        priority = %priority,
                        virtual_time,
                        "Job enqueued for {}/{}",
                        base,
                        quote
                    );
                }
                Ok(())
            }
            Err(e) => {
                self.dedup.remove(&job.id).await;
                Err(e)
            }
        }
    }

    /// Get next job for worker processing (WFQ order).
    pub async fn get_next_job(&self) -> Result<Option<RouteComputationJob>> {
        self.queue.dequeue().await
    }

    /// Report successful job completion.
    pub async fn mark_success(&self, job: &RouteComputationJob) -> Result<()> {
        let job_key = job.id.as_hash_key();
        self.queue.mark_completed(&job_key).await?;
        self.dedup.remove(&job.id).await;

        let mut completed = self.metrics.total_completed.write().await;
        *completed += 1;

        let idx = job.priority as usize;
        let mut band = self.metrics.completed_by_priority[idx].write().await;
        *band += 1;

        crate::metrics::record_queue_completion(job.priority.as_str());

        Ok(())
    }

    /// Report job failure with retry logic.
    pub async fn mark_failure(&self, job: RouteComputationJob, error: &str) -> Result<()> {
        let job_key = job.id.as_hash_key();
        let job_id = job.id.clone();
        let is_exhausted = job.is_exhausted();
        let attempt = job.attempt;
        let max_retries = job.max_retries;

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

    /// Get a full metrics snapshot including per-priority breakdowns.
    pub async fn metrics(&self) -> PoolMetricsSnapshot {
        let submitted = *self.metrics.total_submitted.read().await;
        let completed = *self.metrics.total_completed.read().await;
        let failed = *self.metrics.total_failed.read().await;
        let rejected = *self.metrics.total_rejected.read().await;

        let mut submitted_by_priority = [0u64; 4];
        let mut completed_by_priority = [0u64; 4];
        for i in 0..4 {
            submitted_by_priority[i] = *self.metrics.submitted_by_priority[i].read().await;
            completed_by_priority[i] = *self.metrics.completed_by_priority[i].read().await;
        }

        let queue_stats = self
            .queue
            .stats()
            .await
            .unwrap_or(super::queue::QueueStats {
                pending: 0,
                processing: 0,
                completed: 0,
                failed: 0,
                pending_by_priority: [0; 4],
                processing_by_priority: [0; 4],
            });

        PoolMetricsSnapshot {
            total_submitted: submitted,
            total_completed: completed,
            total_failed: failed,
            total_rejected: rejected,
            submitted_by_priority,
            completed_by_priority,
            pending_jobs: queue_stats.pending,
            processing_jobs: queue_stats.processing,
            pending_by_priority: queue_stats.pending_by_priority,
            processing_by_priority: queue_stats.processing_by_priority,
            queue_depth: queue_stats.total_backlog(),
            dedup_cache_size: self.dedup.size().await,
            load_score: self
                .config
                .backpressure
                .load_score(queue_stats.pending, queue_stats.processing),
            virtual_clock: self.classifier.current_virtual_clock(),
        }
    }

    /// Perform periodic cleanup (expired dedup entries).
    pub async fn cleanup(&self) -> Result<()> {
        self.dedup.cleanup_expired(self.config.dedup_ttl_secs).await;
        Ok(())
    }
}

/// Full snapshot of worker pool metrics.
#[derive(Debug, Clone)]
pub struct PoolMetricsSnapshot {
    pub total_submitted: u64,
    pub total_completed: u64,
    pub total_failed: u64,
    pub total_rejected: u64,
    /// `[critical, high, normal, low]` lifetime submission counts.
    pub submitted_by_priority: [u64; 4],
    /// `[critical, high, normal, low]` lifetime completion counts.
    pub completed_by_priority: [u64; 4],
    pub pending_jobs: usize,
    pub processing_jobs: usize,
    /// `[critical, high, normal, low]` current pending counts.
    pub pending_by_priority: [usize; 4],
    /// `[critical, high, normal, low]` current processing counts.
    pub processing_by_priority: [usize; 4],
    pub queue_depth: usize,
    pub dedup_cache_size: usize,
    pub load_score: u32,
    /// Current WFQ virtual clock value.
    pub virtual_clock: i64,
}
