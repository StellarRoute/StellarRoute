//! Deterministic benchmark runner for routing regression detection.
//!
//! # Purpose
//! Provides repeatable, seed-controlled benchmark fixtures so that CI can
//! detect regressions in route quality (score) and routing latency across
//! commits.
//!
//! # How it works
//! 1. A [`BenchmarkFixture`] describes a named test case: a seed, the
//!    from/to asset pair, the swap amount, and the edge set to use.
//! 2. [`RegressionRunner`] runs all fixtures, records latency and the top
//!    route score for each, and compares them against a stored baseline.
//! 3. A [`RegressionReport`] is produced with per-fixture deltas and a
//!    `passed` flag suitable for CI exit-code gating.
//! 4. Baselines are persisted as JSON via [`BaselineStore`] so they can be
//!    updated and committed alongside source code.
//!
//! # CI usage
//! ```text
//! # First run: write baseline
//! cargo test --package stellarroute-routing regression -- --nocapture
//!
//! # Subsequent runs: compare against baseline; fail if regression detected
//! cargo test --package stellarroute-routing regression
//! ```
//!
//! # Seeding
//! All randomness in synthetic edge generation uses `rand_chacha::ChaCha8Rng`
//! seeded from the fixture's `seed` field, guaranteeing identical outputs
//! across platforms and Rust versions.

use crate::pathfinder::{LiquidityEdge, Pathfinder, PathfinderConfig};
use crate::policy::RoutingPolicy;
use crate::scorer::{BenchmarkHarness, ScorerRegistry};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::Instant;

// ── Fixture ───────────────────────────────────────────────────────────────────

/// A single named regression test case.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BenchmarkFixture {
    /// Unique name for this fixture (used as the baseline key).
    pub name: String,
    /// RNG seed – changing this changes the generated graph.
    pub seed: u64,
    /// Source asset.
    pub from_asset: String,
    /// Destination asset.
    pub to_asset: String,
    /// Amount in (e7 scale).
    pub amount_in: i128,
    /// Number of synthetic intermediate assets to generate.
    pub graph_size: usize,
}

impl BenchmarkFixture {
    /// Deterministically generate a [`Vec<LiquidityEdge>`] from this fixture's
    /// seed and graph parameters.
    pub fn generate_edges(&self) -> Vec<LiquidityEdge> {
        let mut rng = ChaCha8Rng::seed_from_u64(self.seed);
        let assets: Vec<String> = (0..self.graph_size)
            .map(|i| format!("ASSET_{i}"))
            .collect();

        // Always include direct and indirect paths between from/to
        let mut edges = vec![
            LiquidityEdge {
                from: self.from_asset.clone(),
                to: self.to_asset.clone(),
                venue_type: "amm".to_string(),
                venue_ref: "direct_pool".to_string(),
                liquidity: rng.gen_range(500_000_000..5_000_000_000i128),
                price: rng.gen_range(0.8..1.2),
                fee_bps: rng.gen_range(10..100),
                anomaly_score: 0.0,
                anomaly_reasons: vec![],
            },
            LiquidityEdge {
                from: self.from_asset.clone(),
                to: self.to_asset.clone(),
                venue_type: "sdex".to_string(),
                venue_ref: "direct_offer".to_string(),
                liquidity: rng.gen_range(200_000_000..2_000_000_000i128),
                price: rng.gen_range(0.85..1.15),
                fee_bps: rng.gen_range(5..50),
                anomaly_score: 0.0,
                anomaly_reasons: vec![],
            },
        ];

        // Add intermediate hop edges
        for asset in &assets {
            let liquidity: i128 = rng.gen_range(100_000_000..10_000_000_000i128);
            let venue_type = if rng.gen_bool(0.5) { "amm" } else { "sdex" };
            edges.push(LiquidityEdge {
                from: self.from_asset.clone(),
                to: asset.clone(),
                venue_type: venue_type.to_string(),
                venue_ref: format!("{}_in_pool", asset),
                liquidity,
                price: rng.gen_range(0.5..2.0),
                fee_bps: rng.gen_range(5..200),
                anomaly_score: 0.0,
                anomaly_reasons: vec![],
            });
            edges.push(LiquidityEdge {
                from: asset.clone(),
                to: self.to_asset.clone(),
                venue_type: venue_type.to_string(),
                venue_ref: format!("{}_out_pool", asset),
                liquidity: rng.gen_range(100_000_000..10_000_000_000i128),
                price: rng.gen_range(0.5..2.0),
                fee_bps: rng.gen_range(5..200),
                anomaly_score: 0.0,
                anomaly_reasons: vec![],
            });
        }

        edges
    }

    /// Built-in set of regression fixtures covering common trading pairs.
    pub fn standard_suite() -> Vec<BenchmarkFixture> {
        vec![
            BenchmarkFixture {
                name: "xlm_to_usdc_2hop".to_string(),
                seed: 0xDEAD_BEEF_0001,
                from_asset: "XLM".to_string(),
                to_asset: "USDC".to_string(),
                amount_in: 100_000_000,
                graph_size: 3,
            },
            BenchmarkFixture {
                name: "xlm_to_btc_4hop".to_string(),
                seed: 0xDEAD_BEEF_0002,
                from_asset: "XLM".to_string(),
                to_asset: "BTC".to_string(),
                amount_in: 500_000_000,
                graph_size: 6,
            },
            BenchmarkFixture {
                name: "usdc_to_eurt_direct".to_string(),
                seed: 0xDEAD_BEEF_0003,
                from_asset: "USDC".to_string(),
                to_asset: "EURT".to_string(),
                amount_in: 10_000_000_000,
                graph_size: 2,
            },
            BenchmarkFixture {
                name: "xlm_to_usdc_large_graph".to_string(),
                seed: 0xDEAD_BEEF_0004,
                from_asset: "XLM".to_string(),
                to_asset: "USDC".to_string(),
                amount_in: 1_000_000_000,
                graph_size: 10,
            },
        ]
    }
}

// ── Baseline ──────────────────────────────────────────────────────────────────

/// Baseline measurements for a single fixture.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteRegressionEntry {
    /// Fixture name.
    pub fixture_name: String,
    /// Best route score recorded at baseline time.
    pub baseline_score: f64,
    /// Median routing latency at baseline time, in microseconds.
    pub baseline_latency_us: u64,
}

/// Persisted collection of baseline measurements.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BaselineStore {
    pub entries: Vec<RouteRegressionEntry>,
}

impl BaselineStore {
    /// Look up the baseline for a fixture by name.
    pub fn get(&self, fixture_name: &str) -> Option<&RouteRegressionEntry> {
        self.entries.iter().find(|e| e.fixture_name == fixture_name)
    }

    /// Upsert a baseline entry.
    pub fn set(&mut self, entry: RouteRegressionEntry) {
        if let Some(existing) = self.entries.iter_mut().find(|e| e.fixture_name == entry.fixture_name) {
            *existing = entry;
        } else {
            self.entries.push(entry);
        }
    }

    /// Serialize to JSON for storage alongside source code.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON.
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

// ── Per-fixture result ────────────────────────────────────────────────────────

/// Outcome of running a single fixture.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FixtureResult {
    pub fixture_name: String,
    /// Best route score in this run.
    pub score: f64,
    /// Routing latency for this run, in microseconds.
    pub latency_us: u64,
    /// Delta from baseline score (positive = improvement, negative = regression).
    pub score_delta: Option<f64>,
    /// Delta from baseline latency in microseconds (positive = slower, negative = faster).
    pub latency_delta_us: Option<i64>,
    /// Whether this fixture passes the regression thresholds.
    pub passed: bool,
    /// Human-readable reason if the fixture failed.
    pub failure_reason: Option<String>,
}

// ── Regression report ─────────────────────────────────────────────────────────

/// CI-friendly summary of all fixture runs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegressionReport {
    /// Per-fixture results.
    pub results: Vec<FixtureResult>,
    /// `true` iff all fixtures passed their regression thresholds.
    pub passed: bool,
    /// Total number of fixtures run.
    pub total: usize,
    /// Number of fixtures that failed.
    pub failed: usize,
}

impl RegressionReport {
    /// Format as a CI-friendly human-readable summary (plain text).
    pub fn ci_summary(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "Regression runner: {}/{} passed\n",
            self.total - self.failed,
            self.total
        ));
        for r in &self.results {
            let status = if r.passed { "PASS" } else { "FAIL" };
            out.push_str(&format!(
                "  [{status}] {name}: score={score:.4} latency={lat}µs",
                name = r.fixture_name,
                score = r.score,
                lat = r.latency_us,
            ));
            if let Some(ds) = r.score_delta {
                out.push_str(&format!(" score_delta={ds:+.4}"));
            }
            if let Some(dl) = r.latency_delta_us {
                out.push_str(&format!(" latency_delta={dl:+}µs"));
            }
            if let Some(ref reason) = r.failure_reason {
                out.push_str(&format!(" ({reason})"));
            }
            out.push('\n');
        }
        out
    }
}

// ── Config ────────────────────────────────────────────────────────────────────

/// Configuration for the regression runner.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegressionRunnerConfig {
    /// Maximum acceptable score regression (e.g. 0.05 = 5% drop allowed).
    pub max_score_regression: f64,
    /// Maximum acceptable latency increase in microseconds.
    pub max_latency_regression_us: u64,
    /// Number of times each fixture is run to compute median latency.
    pub iterations: usize,
}

impl Default for RegressionRunnerConfig {
    fn default() -> Self {
        Self {
            max_score_regression: 0.05,
            max_latency_regression_us: 5_000,
            iterations: 5,
        }
    }
}

// ── Runner ────────────────────────────────────────────────────────────────────

/// Runs benchmark fixtures and compares results against an optional baseline.
pub struct RegressionRunner {
    config: RegressionRunnerConfig,
    pathfinder: Pathfinder,
    routing_policy: RoutingPolicy,
    scorer_registry: ScorerRegistry,
}

impl RegressionRunner {
    /// Create a runner with default pathfinder and scorer settings.
    pub fn new(config: RegressionRunnerConfig) -> Self {
        Self {
            config,
            pathfinder: Pathfinder::new(PathfinderConfig::default()),
            routing_policy: RoutingPolicy::default(),
            scorer_registry: ScorerRegistry::new(),
        }
    }

    /// Run all fixtures, comparing against `baseline` if provided.
    pub fn run(
        &self,
        fixtures: &[BenchmarkFixture],
        baseline: Option<&BaselineStore>,
    ) -> RegressionReport {
        let mut results = Vec::new();

        for fixture in fixtures {
            let result = self.run_fixture(fixture, baseline);
            results.push(result);
        }

        let failed = results.iter().filter(|r| !r.passed).count();
        RegressionReport {
            total: results.len(),
            failed,
            passed: failed == 0,
            results,
        }
    }

    fn run_fixture(
        &self,
        fixture: &BenchmarkFixture,
        baseline: Option<&BaselineStore>,
    ) -> FixtureResult {
        let edges = fixture.generate_edges();
        let mut latencies = Vec::with_capacity(self.config.iterations);
        let mut best_score = 0.0_f64;

        for _ in 0..self.config.iterations {
            let start = Instant::now();
            let paths = self
                .pathfinder
                .find_paths(
                    &fixture.from_asset,
                    &fixture.to_asset,
                    &edges,
                    fixture.amount_in,
                    &self.routing_policy,
                )
                .unwrap_or_default();
            let latency = start.elapsed().as_micros() as u64;
            latencies.push(latency);

            if !paths.is_empty() {
                let report = BenchmarkHarness::run(&paths, &edges, fixture.amount_in, &self.scorer_registry);
                if let Some(top) = report.scorer_results.iter().find(|r| r.scorer_name == "default") {
                    if let Some((_, out)) = top.ranked_paths.first() {
                        best_score = best_score.max(out.score);
                    }
                }
            }
        }

        latencies.sort_unstable();
        let median_latency = latencies[latencies.len() / 2];

        let baseline_entry = baseline.and_then(|b| b.get(&fixture.name));
        let score_delta = baseline_entry.map(|b| best_score - b.baseline_score);
        let latency_delta = baseline_entry
            .map(|b| median_latency as i64 - b.baseline_latency_us as i64);

        let (passed, failure_reason) = self.evaluate(best_score, median_latency, baseline_entry);

        FixtureResult {
            fixture_name: fixture.name.clone(),
            score: best_score,
            latency_us: median_latency,
            score_delta,
            latency_delta_us: latency_delta,
            passed,
            failure_reason,
        }
    }

    fn evaluate(
        &self,
        score: f64,
        latency_us: u64,
        baseline: Option<&RouteRegressionEntry>,
    ) -> (bool, Option<String>) {
        let Some(b) = baseline else {
            // No baseline yet – first run always passes; caller should persist results.
            return (true, None);
        };

        let score_drop = b.baseline_score - score;
        if score_drop > self.config.max_score_regression {
            return (
                false,
                Some(format!(
                    "score regressed by {score_drop:.4} (threshold: {:.4})",
                    self.config.max_score_regression
                )),
            );
        }

        let latency_increase = latency_us.saturating_sub(b.baseline_latency_us);
        if latency_increase > self.config.max_latency_regression_us {
            return (
                false,
                Some(format!(
                    "latency regressed by {latency_increase}µs (threshold: {}µs)",
                    self.config.max_latency_regression_us
                )),
            );
        }

        (true, None)
    }

    /// Convenience: build a new baseline by running all fixtures once.
    pub fn build_baseline(&self, fixtures: &[BenchmarkFixture]) -> BaselineStore {
        let report = self.run(fixtures, None);
        let mut store = BaselineStore::default();
        for result in &report.results {
            store.set(RouteRegressionEntry {
                fixture_name: result.fixture_name.clone(),
                baseline_score: result.score,
                baseline_latency_us: result.latency_us,
            });
        }
        store
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_fixture() -> BenchmarkFixture {
        BenchmarkFixture {
            name: "test_xlm_usdc".to_string(),
            seed: 42,
            from_asset: "XLM".to_string(),
            to_asset: "USDC".to_string(),
            amount_in: 100_000_000,
            graph_size: 3,
        }
    }

    #[test]
    fn test_seeded_fixture_is_deterministic() {
        let f = simple_fixture();
        let edges_a = f.generate_edges();
        let edges_b = f.generate_edges();
        assert_eq!(
            serde_json::to_string(&edges_a).unwrap(),
            serde_json::to_string(&edges_b).unwrap(),
            "seeded edge generation must be deterministic"
        );
    }

    #[test]
    fn test_different_seeds_produce_different_edges() {
        let f1 = BenchmarkFixture { seed: 1, ..simple_fixture() };
        let f2 = BenchmarkFixture { seed: 2, ..simple_fixture() };
        let e1 = f1.generate_edges();
        let e2 = f2.generate_edges();
        // At minimum the liquidity values should differ
        let liq1: Vec<i128> = e1.iter().map(|e| e.liquidity).collect();
        let liq2: Vec<i128> = e2.iter().map(|e| e.liquidity).collect();
        assert_ne!(liq1, liq2);
    }

    #[test]
    fn test_runner_produces_report() {
        let runner = RegressionRunner::new(RegressionRunnerConfig {
            iterations: 2,
            ..Default::default()
        });
        let fixtures = vec![simple_fixture()];
        let report = runner.run(&fixtures, None);
        assert_eq!(report.total, 1);
        assert!(report.passed);
        assert_eq!(report.results.len(), 1);
    }

    #[test]
    fn test_baseline_roundtrip() {
        let runner = RegressionRunner::new(RegressionRunnerConfig {
            iterations: 2,
            ..Default::default()
        });
        let fixtures = vec![simple_fixture()];
        let baseline = runner.build_baseline(&fixtures);
        assert_eq!(baseline.entries.len(), 1);

        let json = baseline.to_json().unwrap();
        let restored = BaselineStore::from_json(&json).unwrap();
        assert_eq!(restored.entries.len(), 1);
        assert_eq!(restored.entries[0].fixture_name, "test_xlm_usdc");
    }

    #[test]
    fn test_no_regression_against_identical_baseline() {
        let runner = RegressionRunner::new(RegressionRunnerConfig {
            iterations: 2,
            ..Default::default()
        });
        let fixtures = vec![simple_fixture()];
        let baseline = runner.build_baseline(&fixtures);
        let report = runner.run(&fixtures, Some(&baseline));
        assert!(report.passed, "identical baseline should not trigger regression");
    }

    #[test]
    fn test_score_regression_detected() {
        let runner = RegressionRunner::new(RegressionRunnerConfig {
            iterations: 1,
            max_score_regression: 0.001, // very tight threshold
            max_latency_regression_us: 1_000_000,
        });
        let fixtures = vec![simple_fixture()];
        let mut baseline = BaselineStore::default();
        // Inject an artificially high baseline score
        baseline.set(RouteRegressionEntry {
            fixture_name: "test_xlm_usdc".to_string(),
            baseline_score: 1.0,
            baseline_latency_us: 1,
        });
        let report = runner.run(&fixtures, Some(&baseline));
        assert!(!report.passed, "should detect score regression vs inflated baseline");
        assert_eq!(report.failed, 1);
    }

    #[test]
    fn test_ci_summary_format() {
        let runner = RegressionRunner::new(RegressionRunnerConfig {
            iterations: 1,
            ..Default::default()
        });
        let fixtures = vec![simple_fixture()];
        let report = runner.run(&fixtures, None);
        let summary = report.ci_summary();
        assert!(summary.contains("PASS") || summary.contains("FAIL"));
        assert!(summary.contains("test_xlm_usdc"));
    }

    #[test]
    fn test_standard_suite_is_deterministic() {
        let suite_a = BenchmarkFixture::standard_suite();
        let suite_b = BenchmarkFixture::standard_suite();
        for (a, b) in suite_a.iter().zip(suite_b.iter()) {
            let ea = a.generate_edges();
            let eb = b.generate_edges();
            assert_eq!(
                serde_json::to_string(&ea).unwrap(),
                serde_json::to_string(&eb).unwrap(),
                "standard suite fixture {} must be deterministic",
                a.name
            );
        }
    }

    #[test]
    fn test_report_serializes_to_json() {
        let runner = RegressionRunner::new(RegressionRunnerConfig {
            iterations: 1,
            ..Default::default()
        });
        let report = runner.run(&[simple_fixture()], None);
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("test_xlm_usdc"));
    }
}
