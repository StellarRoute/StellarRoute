//! Integration tests for adaptive queue prioritization.
//!
//! These tests verify:
//! 1. Priority classification rules are applied correctly.
//! 2. Starvation prevention: low-priority jobs are not permanently blocked.
//! 3. Per-priority queue metrics are emitted.
//! 4. Under mixed load, high-priority jobs complete with lower tail latency.
//!
//! All tests that require a live PostgreSQL instance are marked `#[ignore]`
//! and must be run with `DATABASE_URL` set.

use std::sync::Arc;
use std::time::{Duration, Instant};

use stellarroute_api::worker::{
    PriorityClassifier, PriorityConfig, RequestPriority, RouteComputationTaskPayload,
    WorkerPoolConfig,
};

// ─── Unit tests (no DB required) ─────────────────────────────────────────────

#[test]
fn classify_normal_request() {
    let c = PriorityClassifier::default();
    assert_eq!(c.classify(1.0, false), RequestPriority::Normal);
    assert_eq!(c.classify(500.0, false), RequestPriority::Normal);
    assert_eq!(c.classify(999.99, false), RequestPriority::Normal);
}

#[test]
fn classify_high_request() {
    let c = PriorityClassifier::default();
    assert_eq!(c.classify(1_000.0, false), RequestPriority::High);
    assert_eq!(c.classify(50_000.0, false), RequestPriority::High);
    assert_eq!(c.classify(99_999.99, false), RequestPriority::High);
}

#[test]
fn classify_critical_request() {
    let c = PriorityClassifier::default();
    assert_eq!(c.classify(100_000.0, false), RequestPriority::Critical);
    assert_eq!(c.classify(1_000_000.0, false), RequestPriority::Critical);
}

#[test]
fn classify_batch_is_always_low() {
    let c = PriorityClassifier::default();
    // Even a huge amount is Low when is_batch = true
    assert_eq!(c.classify(999_999.0, true), RequestPriority::Low);
    assert_eq!(c.classify(1.0, true), RequestPriority::Low);
}

#[test]
fn custom_thresholds_are_respected() {
    let cfg = PriorityConfig {
        critical_amount_threshold: 10_000.0,
        high_amount_threshold: 100.0,
        ..Default::default()
    };
    let c = PriorityClassifier::new(cfg);
    assert_eq!(c.classify(50.0, false), RequestPriority::Normal);
    assert_eq!(c.classify(100.0, false), RequestPriority::High);
    assert_eq!(c.classify(10_000.0, false), RequestPriority::Critical);
}

#[test]
fn virtual_time_ordering_matches_priority() {
    let cfg = PriorityConfig::default();
    // Create four independent classifiers starting from zero to compare
    // the first virtual time each band would receive.
    let c_critical = PriorityClassifier::new(cfg.clone());
    let c_high = PriorityClassifier::new(cfg.clone());
    let c_normal = PriorityClassifier::new(cfg.clone());
    let c_low = PriorityClassifier::new(cfg.clone());

    let vt_critical = c_critical.next_virtual_time(RequestPriority::Critical);
    let vt_high = c_high.next_virtual_time(RequestPriority::High);
    let vt_normal = c_normal.next_virtual_time(RequestPriority::Normal);
    let vt_low = c_low.next_virtual_time(RequestPriority::Low);

    // Critical (weight 1) < High (weight 2) < Normal (weight 4) < Low (weight 8)
    assert!(
        vt_critical < vt_high,
        "critical vt ({}) must be < high vt ({})",
        vt_critical,
        vt_high
    );
    assert!(
        vt_high < vt_normal,
        "high vt ({}) must be < normal vt ({})",
        vt_high,
        vt_normal
    );
    assert!(
        vt_normal < vt_low,
        "normal vt ({}) must be < low vt ({})",
        vt_normal,
        vt_low
    );
}

#[test]
fn starvation_prevention_low_priority_eventually_advances() {
    let c = PriorityClassifier::default();

    // Simulate 100 critical jobs being enqueued.
    let mut last_critical_vt = 0i64;
    for _ in 0..100 {
        last_critical_vt = c.next_virtual_time(RequestPriority::Critical);
    }

    // Now enqueue one low-priority job.
    let low_vt = c.next_virtual_time(RequestPriority::Low);

    // The low-priority job's virtual time is finite and will eventually be
    // reached by the scheduler.  It must not be i64::MAX.
    assert!(
        low_vt < i64::MAX / 2,
        "low-priority virtual time ({}) should be finite",
        low_vt
    );

    // The low-priority job's virtual time is larger than the last critical
    // job's, but the gap is bounded by the weight ratio (8 vs 1 = 8x).
    let gap = low_vt - last_critical_vt;
    assert!(
        gap > 0,
        "low-priority job should have a higher virtual time than the last critical job"
    );
    // The gap should be at most weight_low (8) since both start from the same
    // global clock after the 100 critical jobs.
    assert!(
        gap <= 8,
        "gap ({}) should be bounded by the low-priority weight (8)",
        gap
    );
}

#[test]
fn priority_config_default_weights_are_ordered() {
    let cfg = PriorityConfig::default();
    assert!(cfg.weight_critical < cfg.weight_high);
    assert!(cfg.weight_high < cfg.weight_normal);
    assert!(cfg.weight_normal < cfg.weight_low);
}

#[test]
fn worker_pool_config_includes_priority() {
    let cfg = WorkerPoolConfig::default();
    // Verify the priority config is present and has sensible defaults.
    assert!(cfg.priority.critical_amount_threshold > cfg.priority.high_amount_threshold);
    assert!(cfg.priority.starvation_cap > 0);
}

// ─── Integration tests (require DATABASE_URL) ────────────────────────────────

/// Helper to build a test payload.
fn make_payload(amount: f64) -> RouteComputationTaskPayload {
    RouteComputationTaskPayload {
        base_asset: "native".to_string(),
        quote_asset: "USDC".to_string(),
        base_asset_id: uuid::Uuid::nil(),
        quote_asset_id: uuid::Uuid::nil(),
        amount,
        slippage_bps: 50,
        quote_type: "sell".to_string(),
    }
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn priority_jobs_are_dequeued_before_normal_jobs() {
    use stellarroute_api::worker::{JobQueue, RouteWorkerPool};

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
    });

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to connect to database");

    let queue = JobQueue::new(pool.clone());
    let config = WorkerPoolConfig::default();
    let worker_pool = Arc::new(RouteWorkerPool::new(config, queue));

    // Enqueue a normal-priority job first (lower amount).
    worker_pool
        .submit_job("native", "USDC:normal", make_payload(1.0))
        .await
        .expect("submit normal job");

    // Then enqueue a critical-priority job (large amount).
    worker_pool
        .submit_job("native", "USDC:critical", make_payload(200_000.0))
        .await
        .expect("submit critical job");

    // The critical job should be dequeued first despite being submitted second.
    let first = worker_pool
        .get_next_job()
        .await
        .expect("dequeue")
        .expect("job present");

    assert_eq!(
        first.priority,
        RequestPriority::Critical,
        "critical job should be dequeued before normal job"
    );

    // Clean up
    worker_pool
        .mark_success(&first)
        .await
        .expect("mark success");

    let second = worker_pool
        .get_next_job()
        .await
        .expect("dequeue")
        .expect("job present");

    assert_eq!(second.priority, RequestPriority::Normal);
    worker_pool
        .mark_success(&second)
        .await
        .expect("mark success");
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn per_priority_metrics_are_tracked() {
    use stellarroute_api::worker::{JobQueue, RouteWorkerPool};

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
    });

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to connect to database");

    let queue = JobQueue::new(pool.clone());
    let config = WorkerPoolConfig::default();
    let worker_pool = Arc::new(RouteWorkerPool::new(config, queue));

    // Submit one job per priority band.
    worker_pool
        .submit_job("native", "USDC:m1", make_payload(1.0))
        .await
        .expect("normal");
    worker_pool
        .submit_job("native", "USDC:m2", make_payload(5_000.0))
        .await
        .expect("high");
    worker_pool
        .submit_job("native", "USDC:m3", make_payload(200_000.0))
        .await
        .expect("critical");
    worker_pool
        .submit_job_with_options("native", "USDC:m4", make_payload(1.0), true)
        .await
        .expect("low/batch");

    let snapshot = worker_pool.metrics().await;

    // At least 4 jobs submitted in total.
    assert!(
        snapshot.total_submitted >= 4,
        "expected ≥4 submitted, got {}",
        snapshot.total_submitted
    );

    // Each band should have at least one submission.
    assert!(
        snapshot.submitted_by_priority[RequestPriority::Critical as usize] >= 1,
        "critical band should have ≥1 submission"
    );
    assert!(
        snapshot.submitted_by_priority[RequestPriority::High as usize] >= 1,
        "high band should have ≥1 submission"
    );
    assert!(
        snapshot.submitted_by_priority[RequestPriority::Normal as usize] >= 1,
        "normal band should have ≥1 submission"
    );
    assert!(
        snapshot.submitted_by_priority[RequestPriority::Low as usize] >= 1,
        "low band should have ≥1 submission"
    );
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn load_test_high_priority_tail_latency_under_mixed_load() {
    use stellarroute_api::worker::{JobQueue, RouteWorkerPool};

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
    });

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(20)
        .connect(&db_url)
        .await
        .expect("Failed to connect to database");

    let queue = JobQueue::new(pool.clone());
    let config = WorkerPoolConfig::default();
    let worker_pool = Arc::new(RouteWorkerPool::new(config, queue));

    // ── Phase 1: flood the queue with 50 normal-priority jobs ────────────
    for i in 0..50u32 {
        let quote = format!("USDC:load{}", i);
        worker_pool
            .submit_job("native", &quote, make_payload(1.0))
            .await
            .expect("submit normal");
    }

    // ── Phase 2: submit 5 critical-priority jobs ──────────────────────────
    let mut critical_submit_times = Vec::new();
    for i in 0..5u32 {
        let quote = format!("USDC:critical{}", i);
        let t = Instant::now();
        worker_pool
            .submit_job("native", &quote, make_payload(500_000.0))
            .await
            .expect("submit critical");
        critical_submit_times.push((quote, t));
    }

    // ── Phase 3: drain the queue and measure critical job wait times ──────
    let mut critical_latencies: Vec<Duration> = Vec::new();
    let mut normal_latencies: Vec<Duration> = Vec::new();
    let drain_start = Instant::now();

    while drain_start.elapsed() < Duration::from_secs(30) {
        match worker_pool.get_next_job().await.expect("dequeue") {
            None => break,
            Some(job) => {
                let latency = Instant::now().duration_since(drain_start);
                if job.priority == RequestPriority::Critical {
                    critical_latencies.push(latency);
                } else {
                    normal_latencies.push(latency);
                }
                worker_pool.mark_success(&job).await.expect("mark success");
            }
        }
    }

    // ── Assertions ────────────────────────────────────────────────────────

    // All critical jobs should have been dequeued.
    assert_eq!(
        critical_latencies.len(),
        5,
        "all 5 critical jobs should have been dequeued"
    );

    // Critical jobs should be dequeued before the bulk of normal jobs.
    // We check that the p95 latency of critical jobs is lower than the
    // median latency of normal jobs.
    let mut sorted_critical = critical_latencies.clone();
    sorted_critical.sort();
    let p95_critical = sorted_critical[sorted_critical.len() * 95 / 100];

    if normal_latencies.len() >= 2 {
        let mut sorted_normal = normal_latencies.clone();
        sorted_normal.sort();
        let median_normal = sorted_normal[sorted_normal.len() / 2];

        assert!(
            p95_critical <= median_normal,
            "p95 critical latency ({:?}) should be ≤ median normal latency ({:?})",
            p95_critical,
            median_normal
        );
    }

    // Verify per-priority metrics are populated.
    let snapshot = worker_pool.metrics().await;
    assert!(
        snapshot.completed_by_priority[RequestPriority::Critical as usize] >= 5,
        "at least 5 critical completions should be recorded"
    );
}
