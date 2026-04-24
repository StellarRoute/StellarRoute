use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};

use crate::load_test::{percentile, LoadTestMetrics};

/// Traffic mix configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrafficMix {
    /// Percentage of requests targeting SDEX-heavy pairs (0.0 to 1.0)
    pub sdex_weight: f64,
    /// Percentage of requests targeting AMM-heavy pairs (0.0 to 1.0)
    pub amm_weight: f64,
    /// Percentage of requests targeting mixed pairs (0.0 to 1.0)
    pub mixed_weight: f64,
}

impl Default for TrafficMix {
    fn default() -> Self {
        Self {
            sdex_weight: 0.4,
            amm_weight: 0.4,
            mixed_weight: 0.2,
        }
    }
}

/// Amount distribution configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AmountDistribution {
    pub min_amount: f64,
    pub max_amount: f64,
    /// If true, uses log-normal distribution, otherwise uniform
    pub log_normal: bool,
}

impl Default for AmountDistribution {
    fn default() -> Self {
        Self {
            min_amount: 0.1,
            max_amount: 1000.0,
            log_normal: true,
        }
    }
}

/// Dependency degradation configuration
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DegradationScenario {
    /// Simulated database latency in milliseconds
    pub db_latency_ms: u64,
    /// Percentage of database requests that fail (0.0 to 1.0)
    pub db_error_rate: f64,
    /// Simulated RPC latency in milliseconds
    pub rpc_latency_ms: u64,
    /// Percentage of RPC requests that fail (0.0 to 1.0)
    pub rpc_error_rate: f64,
}

/// Comprehensive Load Test Configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HarnessConfig {
    pub name: String,
    pub concurrent_users: usize,
    pub total_requests: usize,
    pub requests_per_second: u32,
    pub duration_secs: u64,
    pub traffic_mix: TrafficMix,
    pub amount_distribution: AmountDistribution,
    pub degradation: DegradationScenario,
}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            name: "default_load_test".to_string(),
            concurrent_users: 10,
            total_requests: 1000,
            requests_per_second: 50,
            duration_secs: 30,
            traffic_mix: TrafficMix::default(),
            amount_distribution: AmountDistribution::default(),
            degradation: DegradationScenario::default(),
        }
    }
}

/// Detailed results for the harness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessResults {
    pub config: HarnessConfig,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub error_rate: f64,
    pub total_duration_secs: f64,
    pub throughput_rps: f64,
    pub p50_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionReport {
    pub name: String,
    pub latency_p95_delta_pct: f64,
    pub throughput_delta_pct: f64,
    pub error_rate_delta: f64,
    pub is_regression: bool,
}

impl HarnessResults {
    pub fn compare_with_baseline(&self, baseline: &HarnessResults) -> RegressionReport {
        let latency_delta =
            (self.p95_latency_ms - baseline.p95_latency_ms) / baseline.p95_latency_ms * 100.0;
        let throughput_delta =
            (self.throughput_rps - baseline.throughput_rps) / baseline.throughput_rps * 100.0;
        let error_delta = self.error_rate - baseline.error_rate;

        // Consider it a regression if p95 latency increased by > 15% or error rate increased by > 1%
        let is_regression = latency_delta > 15.0 || error_delta > 0.01;

        RegressionReport {
            name: self.config.name.clone(),
            latency_p95_delta_pct: latency_delta,
            throughput_delta_pct: throughput_delta,
            error_rate_delta: error_delta,
            is_regression,
        }
    }

    pub fn print_summary(&self) {
        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!("║             Harness Load Test Results                    ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║ Name: {:<42} ║", self.config.name);
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
            "║ Error Rate:            {:>10.2}%                   ║",
            self.error_rate * 100.0
        );
        println!("╠══════════════════════════════════════════════════════════╣");
        println!(
            "║ P50 Latency:           {:>10.2} ms                ║",
            self.p50_latency_ms
        );
        println!(
            "║ P95 Latency:           {:>10.2} ms                ║",
            self.p95_latency_ms
        );
        println!(
            "║ P99 Latency:           {:>10.2} ms                ║",
            self.p99_latency_ms
        );
        println!(
            "║ Throughput:            {:>10.2} req/sec             ║",
            self.throughput_rps
        );
        println!("╚══════════════════════════════════════════════════════════╝\n");
    }
}

pub struct LoadTestHarness {
    config: HarnessConfig,
    metrics: Arc<LoadTestMetrics>,
}

impl LoadTestHarness {
    pub fn new(config: HarnessConfig) -> Self {
        Self {
            config,
            metrics: Arc::new(LoadTestMetrics::new()),
        }
    }

    pub async fn run<F, Fut>(&self, request_gen: F) -> HarnessResults
    where
        F: Fn(TrafficType, f64) -> Fut + Clone + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), String>> + Send,
    {
        info!("Starting load test harness: {}", self.config.name);
        let start_time = Instant::now();
        let (tx, mut _rx) = mpsc::channel::<()>(self.config.concurrent_users * 2);
        drop(tx); // Unused for now, but keeping for future worker coordination

        // Spawn workers
        let mut workers = vec![];
        for _ in 0..self.config.concurrent_users {
            let config = self.config.clone();
            let metrics = self.metrics.clone();
            let request_gen = request_gen.clone();

            let worker = tokio::spawn(async move {
                let interval = Duration::from_secs_f64(
                    1.0 / (config.requests_per_second as f64 / config.concurrent_users as f64),
                );
                let mut ticker = tokio::time::interval(interval);

                for _ in 0..(config.total_requests / config.concurrent_users) {
                    ticker.tick().await;

                    let (traffic_type, amount) = {
                        let mut rng = rand::thread_rng();
                        (
                            select_traffic_type(&config.traffic_mix, &mut rng),
                            generate_amount(&config.amount_distribution, &mut rng),
                        )
                    };

                    let req_start = Instant::now();

                    // Simulate degradation
                    if config.degradation.db_latency_ms > 0 {
                        tokio::time::sleep(Duration::from_millis(config.degradation.db_latency_ms))
                            .await;
                    }

                    let should_fail = {
                        let mut rng = rand::thread_rng();
                        let mut fail = false;
                        if config.degradation.db_error_rate > 0.0
                            && rng.gen::<f64>() < config.degradation.db_error_rate
                        {
                            fail = true;
                        }
                        if config.degradation.rpc_error_rate > 0.0
                            && rng.gen::<f64>() < config.degradation.rpc_error_rate
                        {
                            fail = true;
                        }
                        fail
                    };

                    let (traffic_type, amount, should_fail) = {
                        let mut rng = rand::thread_rng();
                        let traffic_type = select_traffic_type(&config.traffic_mix, &mut rng);
                        let amount = generate_amount(&config.amount_distribution, &mut rng);

                        let mut should_fail = false;
                        if config.degradation.db_error_rate > 0.0
                            && rng.gen::<f64>() < config.degradation.db_error_rate
                        {
                            should_fail = true;
                        }
                        if config.degradation.rpc_error_rate > 0.0
                            && rng.gen::<f64>() < config.degradation.rpc_error_rate
                        {
                            should_fail = true;
                        }
                        (traffic_type, amount, should_fail)
                    };

                    let result = if should_fail {
                        Err("Simulated dependency failure".to_string())
                    } else {
                        request_gen(traffic_type, amount).await
                    };
                    let latency = req_start.elapsed().as_millis();

                    match result {
                        Ok(_) => {
                            metrics.inc_success();
                            metrics.record_latency(latency).await;
                        }
                        Err(e) => {
                            warn!("Request failed: {}", e);
                            metrics.inc_failure();
                        }
                    }
                }
            });
            workers.push(worker);
        }

        // Wait for completion or timeout
        let timeout_duration = Duration::from_secs(self.config.duration_secs + 10);
        let _ = tokio::time::timeout(timeout_duration, async {
            for w in workers {
                let _ = w.await;
            }
        })
        .await;

        let duration = start_time.elapsed();
        let (successful, failed, _rejected, mut latencies) = self.metrics.snapshot().await;

        latencies.sort();

        let p50 = percentile(&latencies, 50.0) as f64;
        let p95 = percentile(&latencies, 95.0) as f64;
        let p99 = percentile(&latencies, 99.0) as f64;

        let total_reqs = successful + failed;
        let error_rate = if total_reqs > 0 {
            failed as f64 / total_reqs as f64
        } else {
            0.0
        };

        HarnessResults {
            config: self.config.clone(),
            total_requests: total_reqs,
            successful_requests: successful,
            failed_requests: failed,
            error_rate,
            total_duration_secs: duration.as_secs_f64(),
            throughput_rps: successful as f64 / duration.as_secs_f64(),
            p50_latency_ms: p50,
            p95_latency_ms: p95,
            p99_latency_ms: p99,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn save_results(&self, results: &HarnessResults) -> std::io::Result<()> {
        let path = format!("load_test_results_{}.json", self.config.name);
        let file = std::fs::File::create(path)?;
        serde_json::to_writer_pretty(file, results)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_harness_mock_run() {
        let config = HarnessConfig {
            name: "mock_test".to_string(),
            concurrent_users: 2,
            total_requests: 10,
            requests_per_second: 100,
            duration_secs: 1,
            ..Default::default()
        };

        let harness = LoadTestHarness::new(config);
        let results = harness
            .run(|_, _| async {
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok(())
            })
            .await;

        assert_eq!(results.total_requests, 10);
        assert_eq!(results.successful_requests, 10);
        assert!(results.p50_latency_ms >= 10.0);
    }

    #[tokio::test]
    async fn test_harness_regression_detection() {
        let config = HarnessConfig::default();
        let results_a = HarnessResults {
            config: config.clone(),
            total_requests: 100,
            successful_requests: 100,
            failed_requests: 0,
            error_rate: 0.0,
            total_duration_secs: 1.0,
            throughput_rps: 100.0,
            p50_latency_ms: 10.0,
            p95_latency_ms: 15.0,
            p99_latency_ms: 20.0,
            timestamp: "".to_string(),
        };

        let results_b = HarnessResults {
            p95_latency_ms: 20.0, // > 15% increase
            ..results_a.clone()
        };

        let report = results_b.compare_with_baseline(&results_a);
        assert!(report.is_regression);
        assert!(report.latency_p95_delta_pct > 30.0);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TrafficType {
    Sdex,
    Amm,
    Mixed,
}

fn select_traffic_type(mix: &TrafficMix, rng: &mut impl Rng) -> TrafficType {
    let r: f64 = rng.gen();
    if r < mix.sdex_weight {
        TrafficType::Sdex
    } else if r < mix.sdex_weight + mix.amm_weight {
        TrafficType::Amm
    } else {
        TrafficType::Mixed
    }
}

fn generate_amount(dist: &AmountDistribution, rng: &mut impl Rng) -> f64 {
    if dist.log_normal {
        // Simple log-normal approximation for demo
        let r: f64 = rng.gen();
        let log_min = dist.min_amount.ln();
        let log_max = dist.max_amount.ln();
        (log_min + r * (log_max - log_min)).exp()
    } else {
        rng.gen_range(dist.min_amount..dist.max_amount)
    }
}
