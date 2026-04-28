//! Route scoring interface and built-in implementations

use crate::optimizer::OptimizerPolicy;
use crate::pathfinder::{LiquidityEdge, SwapPath};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Normalization constants for DefaultScorer
pub(crate) const OUTPUT_NORM: f64 = 1_000_000_000.0;
pub(crate) const IMPACT_NORM: f64 = 1_000.0;
pub(crate) const LATENCY_NORM: f64 = 1_000_000.0;

/// All data available to a scorer when evaluating a candidate route.
#[derive(Clone, Debug)]
pub struct ScorerInput {
    /// Estimated output amount in the smallest token unit (e7 scale).
    pub output_amount: i128,
    /// Cumulative price impact across all hops, in basis points.
    pub impact_bps: u32,
    /// Wall-clock time to compute this route, in microseconds.
    pub compute_time_us: u64,
    /// Number of hops in the route.
    pub hop_count: usize,
    /// The active optimizer policy, carrying weights and limits.
    pub policy: OptimizerPolicy,
}

/// The result of scoring a candidate route.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScorerOutput {
    /// Normalized score in [0.0, 1.0]. Values outside this range are clamped
    /// by HybridOptimizer before use.
    pub score: f64,
    /// Optional per-component sub-scores for diagnostics and introspection.
    /// Keys are human-readable component names (e.g. "output", "impact", "latency").
    pub diagnostics: Option<HashMap<String, f64>>,
}

/// A pluggable route scoring algorithm.
///
/// Implementations must be `Send + Sync` to support concurrent evaluation
/// under Tokio. The trait is object-safe so implementations can be stored
/// as `Box<dyn RouteScorer>`.
pub trait RouteScorer: Send + Sync {
    /// Score a candidate route given its metrics and the active policy.
    ///
    /// Implementations SHOULD return a value in `[0.0, 1.0]`. Values outside
    /// this range will be clamped by `HybridOptimizer` with a `tracing::warn!`.
    fn score(&self, input: &ScorerInput) -> ScorerOutput;
}

/// Replicates the existing HybridOptimizer::calculate_score() formula exactly.
pub struct DefaultScorer;

impl RouteScorer for DefaultScorer {
    fn score(&self, input: &ScorerInput) -> ScorerOutput {
        let policy = &input.policy;
        let output_score = (input.output_amount as f64 / OUTPUT_NORM).min(1.0);
        let impact_score = 1.0 - (input.impact_bps as f64 / IMPACT_NORM).min(1.0);
        let latency_score = 1.0 - (input.compute_time_us as f64 / LATENCY_NORM).min(1.0);

        let score = policy.output_weight * output_score
            + policy.impact_weight * impact_score
            + policy.latency_weight * latency_score;

        let mut diagnostics = HashMap::new();
        diagnostics.insert("output".to_string(), output_score);
        diagnostics.insert("impact".to_string(), impact_score);
        diagnostics.insert("latency".to_string(), latency_score);

        ScorerOutput {
            score,
            diagnostics: Some(diagnostics),
        }
    }
}

/// Scores routes purely by minimizing fee/impact cost.
pub struct FeeMinimizingScorer;

impl RouteScorer for FeeMinimizingScorer {
    fn score(&self, input: &ScorerInput) -> ScorerOutput {
        let max_impact = input.policy.max_impact_bps as f64;
        let score = if max_impact > 0.0 {
            1.0 - (input.impact_bps as f64 / max_impact).min(1.0)
        } else {
            0.0
        };
        ScorerOutput {
            score,
            diagnostics: None,
        }
    }
}

/// Scores routes purely by maximizing output amount.
pub struct OutputMaximizingScorer;

impl RouteScorer for OutputMaximizingScorer {
    fn score(&self, input: &ScorerInput) -> ScorerOutput {
        let score = (input.output_amount as f64 / OUTPUT_NORM).min(1.0);
        ScorerOutput {
            score,
            diagnostics: None,
        }
    }
}

/// Runtime registry mapping scorer names to implementations.
pub struct ScorerRegistry {
    scorers: HashMap<String, Box<dyn RouteScorer>>,
    active: String,
}

impl ScorerRegistry {
    /// Create a new registry with all built-in scorers registered.
    /// Reads `ROUTING_SCORER` env var to set the initial active scorer.
    pub fn new() -> Self {
        let mut registry = Self {
            scorers: HashMap::new(),
            active: "default".to_string(),
        };
        // Register built-ins (these are infallible — names are unique)
        registry.scorers.insert("default".to_string(), Box::new(DefaultScorer));
        registry.scorers.insert("fee_minimizing".to_string(), Box::new(FeeMinimizingScorer));
        registry.scorers.insert("output_maximizing".to_string(), Box::new(OutputMaximizingScorer));

        // Read ROUTING_SCORER env var
        if let Ok(name) = std::env::var("ROUTING_SCORER") {
            if registry.scorers.contains_key(&name) {
                registry.active = name;
            } else {
                tracing::warn!(
                    scorer = %name,
                    "ROUTING_SCORER names an unregistered scorer; falling back to \"default\""
                );
            }
        }

        registry
    }

    /// Register a scorer under a unique name.
    pub fn register(&mut self, name: &str, scorer: Box<dyn RouteScorer>) -> crate::error::Result<()> {
        if self.scorers.contains_key(name) {
            return Err(crate::error::RoutingError::DuplicateScorer(name.to_string()));
        }
        self.scorers.insert(name.to_string(), scorer);
        Ok(())
    }

    /// Set the active scorer by name.
    pub fn set_active(&mut self, name: &str) -> crate::error::Result<()> {
        if !self.scorers.contains_key(name) {
            return Err(crate::error::RoutingError::UnknownScorer(name.to_string()));
        }
        self.active = name.to_string();
        Ok(())
    }

    /// Return the name of the currently active scorer.
    pub fn active_scorer_name(&self) -> &str {
        &self.active
    }

    /// Iterate over all registered (name, scorer) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &dyn RouteScorer)> {
        self.scorers.iter().map(|(k, v)| (k.as_str(), v.as_ref()))
    }

    /// Score a route using the currently active scorer.
    /// Clamps the result to [0.0, 1.0] and emits tracing::warn! if clamping occurs.
    pub fn score(&self, input: &ScorerInput) -> ScorerOutput {
        let scorer = self.scorers.get(&self.active).expect("active scorer must be registered");
        let mut output = scorer.score(input);
        let raw = output.score;
        // f64::clamp handles NaN by returning the lower bound (0.0)
        let clamped = raw.clamp(0.0, 1.0);
        if clamped != raw || raw.is_nan() {
            tracing::warn!(
                scorer = %self.active,
                raw_score = %raw,
                "scorer returned out-of-range score; clamped to [0.0, 1.0]"
            );
        }
        output.score = clamped;
        output
    }
}

impl Default for ScorerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Results for a single scorer within a BenchmarkReport.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScorerResult {
    /// Registered name of the scorer.
    pub scorer_name: String,
    /// Paths sorted by descending score.
    pub ranked_paths: Vec<(SwapPath, ScorerOutput)>,
    /// Wall-clock duration of the scoring pass.
    #[serde(with = "duration_serde")]
    pub duration: std::time::Duration,
}

/// Side-by-side comparison of all registered scorers over a shared route set.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BenchmarkReport {
    /// Per-scorer results, one entry per registered scorer.
    pub scorer_results: Vec<ScorerResult>,
    /// True if any two scorers disagree on the top-ranked path.
    pub top_path_disagreement: bool,
}

// Serde helper for std::time::Duration (serialize as nanoseconds u64)
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        d.as_nanos().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        let nanos = u128::deserialize(d)?;
        Ok(Duration::from_nanos(nanos as u64))
    }
}

/// Runs all registered scorers against the same candidate route set.
pub struct BenchmarkHarness;

impl BenchmarkHarness {
    /// Score all registered scorers against the provided paths and return a report.
    /// Does NOT mutate the registry's active scorer selection.
    pub fn run(
        paths: &[SwapPath],
        _edges: &[LiquidityEdge],
        amount_in: i128,
        registry: &ScorerRegistry,
    ) -> BenchmarkReport {
        let mut scorer_results = Vec::new();

        for (scorer_name, scorer) in registry.iter() {
            let start = std::time::Instant::now();
            let mut ranked: Vec<(SwapPath, ScorerOutput)> = paths
                .iter()
                .map(|path| {
                    let input = ScorerInput {
                        output_amount: path.estimated_output,
                        impact_bps: 0, // impact not available at harness level; scorers use what's provided
                        compute_time_us: 0,
                        hop_count: path.hops.len(),
                        policy: OptimizerPolicy::default(),
                    };
                    // Build a richer input using amount_in as a proxy for output when estimated_output is 0
                    let output_amount = if path.estimated_output > 0 {
                        path.estimated_output
                    } else {
                        amount_in
                    };
                    let real_input = ScorerInput {
                        output_amount,
                        ..input
                    };
                    let output = scorer.score(&real_input);
                    (path.clone(), output)
                })
                .collect();

            // Sort descending by score
            ranked.sort_by(|a, b| b.1.score.partial_cmp(&a.1.score).unwrap_or(std::cmp::Ordering::Equal));

            scorer_results.push(ScorerResult {
                scorer_name: scorer_name.to_string(),
                ranked_paths: ranked,
                duration: start.elapsed(),
            });
        }

        // Detect top-path disagreement: true iff any two scorers have different top-ranked paths
        let top_path_disagreement = if scorer_results.len() < 2 {
            false
        } else {
            let first_top = scorer_results[0].ranked_paths.first().map(|(p, _)| &p.hops);
            scorer_results[1..].iter().any(|r| {
                r.ranked_paths.first().map(|(p, _)| &p.hops) != first_top
            })
        };

        BenchmarkReport {
            scorer_results,
            top_path_disagreement,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scorer_output_serde_roundtrip() {
        let mut diagnostics = HashMap::new();
        diagnostics.insert("output".to_string(), 0.8);
        diagnostics.insert("impact".to_string(), 0.9);

        let output = ScorerOutput {
            score: 0.85,
            diagnostics: Some(diagnostics),
        };

        let json = serde_json::to_string(&output).unwrap();
        let deserialized: ScorerOutput = serde_json::from_str(&json).unwrap();

        assert_eq!(output.score, deserialized.score);
        assert_eq!(output.diagnostics, deserialized.diagnostics);
    }

    #[test]
    fn test_default_scorer_produces_valid_score() {
        let policy = OptimizerPolicy {
            output_weight: 0.5,
            impact_weight: 0.3,
            latency_weight: 0.2,
            max_impact_bps: 500,
            max_compute_time_ms: 1000,
            environment: "testing".to_string(),
            scorer: None,
        };

        let input = ScorerInput {
            output_amount: 500_000_000,
            impact_bps: 100,
            compute_time_us: 50_000,
            hop_count: 2,
            policy,
        };

        let scorer = DefaultScorer;
        let output = scorer.score(&input);

        assert!(output.score >= 0.0 && output.score <= 1.0);
        assert!(output.diagnostics.is_some());
    }

    // Property tests using proptest
    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        // Helper to generate valid OptimizerPolicy with weights summing to 1.0
        fn valid_policy_strategy() -> impl Strategy<Value = OptimizerPolicy> {
            (0.0..=1.0f64, 0.0..=1.0f64, 1u32..=1000u32, 100u64..=5000u64).prop_map(
                |(w1, w2, max_impact, max_time)| {
                    let remaining = 1.0 - w1;
                    let w2 = w2.min(remaining);
                    let w3 = 1.0 - w1 - w2;
                    OptimizerPolicy {
                        output_weight: w1,
                        impact_weight: w2,
                        latency_weight: w3,
                        max_impact_bps: max_impact,
                        max_compute_time_ms: max_time,
                        environment: "testing".to_string(),
                        scorer: None,
                    }
                },
            )
        }

        // Feature: route-scorer-plugin, Property 1: DefaultScorer Parity
        proptest! {
            #[test]
            fn prop_default_scorer_parity(
                output_amount in 0i128..=i64::MAX as i128,
                impact_bps in 0u32..=10000u32,
                compute_time_us in 0u64..=10_000_000u64,
                policy in valid_policy_strategy()
            ) {
                let input = ScorerInput {
                    output_amount,
                    impact_bps,
                    compute_time_us,
                    hop_count: 2,
                    policy: policy.clone(),
                };

                let scorer = DefaultScorer;
                let output = scorer.score(&input);

                // Compute inline formula
                let output_score = (output_amount as f64 / OUTPUT_NORM).min(1.0);
                let impact_score = 1.0 - (impact_bps as f64 / IMPACT_NORM).min(1.0);
                let latency_score = 1.0 - (compute_time_us as f64 / LATENCY_NORM).min(1.0);
                let expected = policy.output_weight * output_score
                    + policy.impact_weight * impact_score
                    + policy.latency_weight * latency_score;

                prop_assert!((output.score - expected).abs() < 1e-12);
            }
        }

        // Feature: route-scorer-plugin, Property 5: FeeMinimizingScorer Formula
        proptest! {
            #[test]
            fn prop_fee_minimizing_scorer_formula(
                impact_bps in 0u32..=10000u32,
                max_impact_bps in 1u32..=10000u32
            ) {
                let policy = OptimizerPolicy {
                    output_weight: 0.5,
                    impact_weight: 0.3,
                    latency_weight: 0.2,
                    max_impact_bps,
                    max_compute_time_ms: 1000,
                    environment: "testing".to_string(),
                    scorer: None,
                };

                let input = ScorerInput {
                    output_amount: 1_000_000_000,
                    impact_bps,
                    compute_time_us: 100_000,
                    hop_count: 2,
                    policy,
                };

                let scorer = FeeMinimizingScorer;
                let output = scorer.score(&input);

                let expected = 1.0 - (impact_bps as f64 / max_impact_bps as f64).min(1.0);
                prop_assert!((output.score - expected).abs() < 1e-12);
            }
        }

        // Feature: route-scorer-plugin, Property 6: OutputMaximizingScorer Formula
        proptest! {
            #[test]
            fn prop_output_maximizing_scorer_formula(
                output_amount in 0i128..=i64::MAX as i128
            ) {
                let policy = OptimizerPolicy {
                    output_weight: 0.5,
                    impact_weight: 0.3,
                    latency_weight: 0.2,
                    max_impact_bps: 500,
                    max_compute_time_ms: 1000,
                    environment: "testing".to_string(),
                    scorer: None,
                };

                let input = ScorerInput {
                    output_amount,
                    impact_bps: 100,
                    compute_time_us: 100_000,
                    hop_count: 2,
                    policy,
                };

                let scorer = OutputMaximizingScorer;
                let output = scorer.score(&input);

                let expected = (output_amount as f64 / OUTPUT_NORM).min(1.0);
                prop_assert!((output.score - expected).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn test_registry_default_construction() {
        let registry = ScorerRegistry::new();
        assert_eq!(registry.active_scorer_name(), "default");
        // All three built-ins present
        let names: Vec<&str> = registry.iter().map(|(n, _)| n).collect();
        assert!(names.contains(&"default"));
        assert!(names.contains(&"fee_minimizing"));
        assert!(names.contains(&"output_maximizing"));
    }

    #[test]
    fn test_registry_set_active_valid() {
        let mut registry = ScorerRegistry::new();
        assert!(registry.set_active("fee_minimizing").is_ok());
        assert_eq!(registry.active_scorer_name(), "fee_minimizing");
    }

    #[test]
    fn test_registry_set_active_unknown() {
        let mut registry = ScorerRegistry::new();
        let result = registry.set_active("nonexistent");
        assert!(result.is_err());
        assert_eq!(registry.active_scorer_name(), "default");
    }

    #[test]
    fn test_registry_duplicate_registration() {
        let mut registry = ScorerRegistry::new();
        let result = registry.register("default", Box::new(DefaultScorer));
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_score_clamping() {
        // A scorer that returns out-of-range values
        struct OutOfRangeScorer;
        impl RouteScorer for OutOfRangeScorer {
            fn score(&self, _input: &ScorerInput) -> ScorerOutput {
                ScorerOutput { score: 1.5, diagnostics: None }
            }
        }
        let mut registry = ScorerRegistry::new();
        registry.register("out_of_range", Box::new(OutOfRangeScorer)).unwrap();
        registry.set_active("out_of_range").unwrap();

        let policy = OptimizerPolicy::default();
        let input = ScorerInput {
            output_amount: 1_000_000,
            impact_bps: 50,
            compute_time_us: 1_000,
            hop_count: 1,
            policy,
        };
        let output = registry.score(&input);
        assert!(output.score >= 0.0 && output.score <= 1.0);
    }
}
