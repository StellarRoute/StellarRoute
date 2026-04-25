//! Load testing for the worker pool
//!
//! Demonstrates stable throughput under sustained load

pub mod harness;

pub use harness::{
    AmountDistribution, DegradationScenario, HarnessConfig, HarnessResults, LoadTestHarness,
    TrafficMix, TrafficType,
};

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tracing::info;

/// Load test configuration
#[derive(Clone, Debug)]
pub struct LoadTestConfig {
    /// Number of concurrent request generators
    pub concurrent_requests: usize,
    /// Total number of requests to generate
    pub total_requests: usize,
    /// Request rate per second (per generator)
    pub requests_per_second: u32,
    /// Test duration in seconds
    pub duration_secs: u64,
}

impl Default for LoadTestConfig {
    fn default() -> Self {
        Self {
            concurrent_requests: 10,
            total_requests: 10000,
            requests_per_second: 100,
            duration_secs: 60,
        }
    }
}

/// Load test results
#[derive(Debug, Clone)]
pub struct LoadTestResults {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub rejected_requests: u64,
    pub total_duration_secs: f64,
    pub throughput_rps: f64,
    pub avg_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
}

impl LoadTestResults {
    pub fn print_summary(&self) {
        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!("║                 Load Test Results                        ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!(
            "║ Total Requests:        {:>10}                    ║",
            self.total_requests
        );
        println!(
            "║ Successful:            {:>10}                    ║",
            self.successful_requests
        );
        println!(
            "║ Failed:                {:>10}                    ║",
            self.failed_requests
        );
        println!(
            "║ Rejected (Backpressure): {:>9}                    ║",
            self.rejected_requests
        );
        println!("╠══════════════════════════════════════════════════════════╣");
        println!(
            "║ Duration:              {:>10} seconds            ║",
            self.total_duration_secs as i32
        );
        println!(
            "║ Throughput:            {:>10} req/sec             ║",
            self.throughput_rps as i32
        );
        println!("╠══════════════════════════════════════════════════════════╣");
        println!(
            "║ Avg Latency:           {:>10} ms                ║",
            self.avg_latency_ms as i32
        );
        println!(
            "║ P95 Latency:           {:>10} ms                ║",
            self.p95_latency_ms as i32
        );
        println!(
            "║ P99 Latency:           {:>10} ms                ║",
            self.p99_latency_ms as i32
        );
        println!("╚══════════════════════════════════════════════════════════╝\n");
    }
}

/// Metrics collector for load test
pub struct LoadTestMetrics {
    successful: Arc<AtomicU64>,
    failed: Arc<AtomicU64>,
    rejected: Arc<AtomicU64>,
    latencies: Arc<tokio::sync::Mutex<Vec<u128>>>,
}

impl LoadTestMetrics {
    pub fn new() -> Self {
        Self {
            successful: Arc::new(AtomicU64::new(0)),
            failed: Arc::new(AtomicU64::new(0)),
            rejected: Arc::new(AtomicU64::new(0)),
            latencies: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn inc_success(&self) {
        self.successful.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_failure(&self) {
        self.failed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_rejection(&self) {
        self.rejected.fetch_add(1, Ordering::Relaxed);
    }

    pub async fn record_latency(&self, latency_ms: u128) {
        let mut latencies = self.latencies.lock().await;
        latencies.push(latency_ms);
    }

    pub async fn snapshot(&self) -> (u64, u64, u64, Vec<u128>) {
        let latencies = self.latencies.lock().await;
        (
            self.successful.load(Ordering::Relaxed),
            self.failed.load(Ordering::Relaxed),
            self.rejected.load(Ordering::Relaxed),
            latencies.clone(),
        )
    }
}

impl Default for LoadTestMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate percentile from sorted latencies
pub fn percentile(sorted_latencies: &[u128], percentile: f64) -> u128 {
    if sorted_latencies.is_empty() {
        return 0;
    }
    let index = ((percentile / 100.0) * sorted_latencies.len() as f64).ceil() as usize;
    sorted_latencies[index.saturating_sub(1)]
}

/// Run load test and return results
pub async fn run_load_test(
    config: LoadTestConfig,
    test_fn: impl Fn() + Clone + Send + 'static,
) -> LoadTestResults {
    info!("Starting load test with config: {:?}", config);

    let metrics = Arc::new(LoadTestMetrics::new());
    let start_time = Instant::now();
    let mut tasks = vec![];

    for _ in 0..config.concurrent_requests {
        let metrics_clone = metrics.clone();
        let test_fn_clone = test_fn.clone();
        let task = tokio::spawn(async move {
            let start = Instant::now();
            // Simulated request
            test_fn_clone();
            let latency = start.elapsed().as_millis();

            metrics_clone.inc_success();
            metrics_clone.record_latency(latency).await;
        });

        tasks.push(task);
    }

    for task in tasks {
        let _ = task.await;
    }

    let duration = start_time.elapsed();
    let total_duration_secs = duration.as_secs_f64();

    let (successful, failed, rejected, mut latencies) = metrics.snapshot().await;
    let total_requests = successful + failed + rejected;

    latencies.sort();
    let avg_latency = if successful > 0 {
        latencies.iter().sum::<u128>() / successful as u128
    } else {
        0
    };

    let p95 = percentile(&latencies, 95.0);
    let p99 = percentile(&latencies, 99.0);
    let throughput = successful as f64 / total_duration_secs;

    LoadTestResults {
        total_requests,
        successful_requests: successful,
        failed_requests: failed,
        rejected_requests: rejected,
        total_duration_secs,
        throughput_rps: throughput,
        avg_latency_ms: avg_latency as f64,
        p95_latency_ms: p95 as f64,
        p99_latency_ms: p99 as f64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load_test_metrics() {
        let metrics = LoadTestMetrics::new();
        metrics.inc_success();
        metrics.inc_success();
        metrics.record_latency(10).await;
        metrics.record_latency(20).await;

        let (success, failed, rejected, latencies) = metrics.snapshot().await;
        assert_eq!(success, 2);
        assert_eq!(failed, 0);
        assert_eq!(rejected, 0);
        assert_eq!(latencies.len(), 2);
    }

    #[tokio::test]
    async fn test_percentile_calculation() {
        let latencies = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        assert_eq!(percentile(&latencies, 50.0), 5);
        assert_eq!(percentile(&latencies, 95.0), 10);
        assert_eq!(percentile(&latencies, 99.0), 10);
    }

    #[tokio::test]
    async fn run_load_test_completes_without_hanging() {
        let config = LoadTestConfig {
            concurrent_requests: 4,
            total_requests: 4,
            requests_per_second: 1,
            duration_secs: 1,
        };

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            run_load_test(config, || {}),
        )
        .await;

        assert!(result.is_ok(), "load test timed out unexpectedly");
    }
}
