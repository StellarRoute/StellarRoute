//! Hybrid route optimizer combining latency and execution quality

use crate::error::{Result, RoutingError};
use crate::impact::{AmmQuoteCalculator, OrderbookImpactCalculator};
use crate::pathfinder::{LiquidityEdge, Pathfinder, PathfinderConfig, SwapPath};
use crate::policy::RoutingPolicy;
use crate::risk::{RiskLimitConfig, RiskValidator, RouteExclusion};
use crate::scorer::{BenchmarkHarness, BenchmarkReport, ScorerInput, ScorerRegistry};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

/// Configuration for optimization policies
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptimizerPolicy {
    /// Weight for output amount (0.0 to 1.0)
    pub output_weight: f64,
    /// Weight for price impact (0.0 to 1.0)  
    pub impact_weight: f64,
    /// Weight for compute cost/latency (0.0 to 1.0)
    pub latency_weight: f64,
    /// Maximum acceptable price impact in basis points
    pub max_impact_bps: u32,
    /// Maximum computation time in milliseconds
    pub max_compute_time_ms: u64,
    /// Environment identifier for policy selection
    pub environment: String,
    /// Scorer name to activate. None means use the registry default.
    #[serde(default)]
    pub scorer: Option<String>,
}

impl Default for OptimizerPolicy {
    fn default() -> Self {
        Self {
            output_weight: 0.5,
            impact_weight: 0.3,
            latency_weight: 0.2,
            max_impact_bps: 500,       // 5%
            max_compute_time_ms: 1000, // 1 second
            environment: "production".to_string(),
            scorer: None,
        }
    }
}

impl OptimizerPolicy {
    /// Validate policy weights sum to approximately 1.0
    pub fn validate(&self) -> Result<()> {
        let total = self.output_weight + self.impact_weight + self.latency_weight;
        if (total - 1.0).abs() > 0.01 {
            return Err(RoutingError::InvalidAmount(
                "policy weights must sum to 1.0".to_string(),
            ));
        }

        if self.output_weight < 0.0 || self.impact_weight < 0.0 || self.latency_weight < 0.0 {
            return Err(RoutingError::InvalidAmount(
                "policy weights must be non-negative".to_string(),
            ));
        }

        Ok(())
    }

    /// Build a policy from environment variables.
    /// Reads `ROUTING_SCORER` into the `scorer` field.
    pub fn from_env() -> Self {
        let mut policy = Self::default();
        if let Ok(scorer) = std::env::var("ROUTING_SCORER") {
            policy.scorer = Some(scorer);
        }
        policy
    }
}

/// Predefined policies for different environments
pub struct PolicyPresets;

impl PolicyPresets {
    /// High-quality, low-latency for production
    pub fn production() -> OptimizerPolicy {
        OptimizerPolicy {
            output_weight: 0.5,
            impact_weight: 0.3,
            latency_weight: 0.2,
            max_impact_bps: 300,
            max_compute_time_ms: 500,
            environment: "production".to_string(),
            scorer: None,
        }
    }

    /// Maximum output quality for analysis
    pub fn analysis() -> OptimizerPolicy {
        OptimizerPolicy {
            output_weight: 0.7,
            impact_weight: 0.25,
            latency_weight: 0.05,
            max_impact_bps: 1000,
            max_compute_time_ms: 5000,
            environment: "analysis".to_string(),
            scorer: None,
        }
    }

    /// Fast response for real-time trading
    pub fn realtime() -> OptimizerPolicy {
        OptimizerPolicy {
            output_weight: 0.3,
            impact_weight: 0.2,
            latency_weight: 0.5,
            max_impact_bps: 500,
            max_compute_time_ms: 100,
            environment: "realtime".to_string(),
            scorer: None,
        }
    }

    /// Balanced for testing
    pub fn testing() -> OptimizerPolicy {
        OptimizerPolicy {
            output_weight: 0.4,
            impact_weight: 0.3,
            latency_weight: 0.3,
            max_impact_bps: 400,
            max_compute_time_ms: 2000,
            environment: "testing".to_string(),
            scorer: None,
        }
    }
}

/// Route scoring metrics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteMetrics {
    /// Estimated output amount
    pub output_amount: i128,
    /// Total price impact in basis points
    pub impact_bps: u32,
    /// Computation time in microseconds
    pub compute_time_us: u64,
    /// Number of hops in the route
    pub hop_count: usize,
    /// Normalized score (0.0 to 1.0)
    pub score: f64,
    /// Aggregate anomaly score (0.0 to 1.0)
    pub anomaly_score: f64,
    /// Reasons for detected anomalies
    pub anomaly_reasons: Vec<String>,
}

/// Optimizer diagnostics for selected route
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptimizerDiagnostics {
    /// Selected route path
    pub selected_path: SwapPath,
    /// Route metrics
    pub metrics: RouteMetrics,
    /// Alternative routes considered
    pub alternatives: Vec<(SwapPath, RouteMetrics)>,
    /// Policy used for optimization
    pub policy: OptimizerPolicy,
    /// Total computation time
    pub total_compute_time_ms: u64,
    /// Routes excluded due to risk limits
    #[serde(default)]
    pub excluded_routes: Vec<RouteExclusion>,
    /// Name of the scorer used to rank routes in this response.
    pub active_scorer_name: String,
    /// Venues flagged with anomalies but still included
    #[serde(default)]
    pub flagged_venues: Vec<crate::health::anomaly::AnomalyResult>,
}

/// Hybrid route optimizer with configurable policies
pub struct HybridOptimizer {
    pathfinder: Pathfinder,
    #[allow(dead_code)]
    amm_calculator: AmmQuoteCalculator,
    #[allow(dead_code)]
    orderbook_calculator: OrderbookImpactCalculator,
    policies: HashMap<String, OptimizerPolicy>,
    active_policy: String,
    risk_validator: Option<RiskValidator>,
    scorer_registry: ScorerRegistry,
}

impl HybridOptimizer {
    /// Create new optimizer with default policies
    pub fn new(config: PathfinderConfig) -> Self {
        let mut policies = HashMap::new();
        policies.insert("production".to_string(), PolicyPresets::production());
        policies.insert("analysis".to_string(), PolicyPresets::analysis());
        policies.insert("realtime".to_string(), PolicyPresets::realtime());
        policies.insert("testing".to_string(), PolicyPresets::testing());

        Self {
            pathfinder: Pathfinder::new(config),
            amm_calculator: AmmQuoteCalculator,
            orderbook_calculator: OrderbookImpactCalculator,
            policies,
            active_policy: "production".to_string(),
            risk_validator: None,
            scorer_registry: ScorerRegistry::new(),
        }
    }

    /// Create optimizer with risk limits
    pub fn with_risk_limits(config: PathfinderConfig, risk_config: RiskLimitConfig) -> Self {
        let mut optimizer = Self::new(config);
        optimizer.risk_validator = Some(RiskValidator::new(risk_config));
        optimizer
    }

    /// Create optimizer with a custom `ScorerRegistry`.
    pub fn with_scorer_registry(config: PathfinderConfig, registry: ScorerRegistry) -> Self {
        let mut optimizer = Self::new(config);
        optimizer.scorer_registry = registry;
        optimizer
    }

    /// Set risk limit configuration
    pub fn set_risk_limits(&mut self, config: RiskLimitConfig) {
        self.risk_validator = Some(RiskValidator::new(config));
    }

    /// Clear risk limits
    pub fn clear_risk_limits(&mut self) {
        self.risk_validator = None;
    }

    /// Add custom policy
    pub fn add_policy(&mut self, policy: OptimizerPolicy) -> Result<()> {
        policy.validate()?;
        self.policies.insert(policy.environment.clone(), policy);
        Ok(())
    }

    /// Set active policy by environment name
    pub fn set_active_policy(&mut self, environment: &str) -> Result<()> {
        if !self.policies.contains_key(environment) {
            return Err(RoutingError::InvalidAmount(format!(
                "policy '{}' not found",
                environment
            )));
        }
        self.active_policy = environment.to_string();
        Ok(())
    }

    /// Get current active policy
    pub fn active_policy(&self) -> &OptimizerPolicy {
        &self.policies[&self.active_policy]
    }

    /// Change the active scorer. Takes effect for all subsequent find_optimal_routes calls.
    pub fn set_scorer(&mut self, name: &str) -> crate::error::Result<()> {
        self.scorer_registry.set_active(name)
    }

    /// Run all registered scorers against the provided candidate set and return a comparison report.
    pub fn benchmark_scorers(
        &self,
        paths: &[crate::pathfinder::SwapPath],
        edges: &[crate::pathfinder::LiquidityEdge],
        amount_in: i128,
    ) -> BenchmarkReport {
        BenchmarkHarness::run(paths, edges, amount_in, &self.scorer_registry)
    }

    /// Find optimal routes using hybrid scoring with risk limit enforcement
    pub fn find_optimal_routes(
        &self,
        from: &str,
        to: &str,
        edges: &[LiquidityEdge],
        amount_in: i128,
        routing_policy: &RoutingPolicy,
    ) -> Result<OptimizerDiagnostics> {
        let graph = crate::compaction::CompactedGraph::from_edges(edges.to_vec());
        self.find_optimal_routes_compacted(from, to, &graph, amount_in, routing_policy)
    }

    /// Find optimal routes using a compacted graph
    pub fn find_optimal_routes_compacted(
        &self,
        from: &str,
        to: &str,
        graph: &crate::compaction::CompactedGraph,
        amount_in: i128,
        routing_policy: &RoutingPolicy,
    ) -> Result<OptimizerDiagnostics> {
        let start_time = Instant::now();
        let policy = self.active_policy();
        let mut excluded_routes = Vec::new();

        let paths =
            self.pathfinder
                .find_paths_compacted(from, to, graph, amount_in, routing_policy)?;

        if paths.is_empty() {
            return Err(RoutingError::NoRoute(from.to_string(), to.to_string()));
        }

        let mut scored_paths = Vec::new();
        for path in &paths {
            let metrics = self.calculate_route_metrics_compacted(path, graph, amount_in)?;

            if metrics.impact_bps > policy.max_impact_bps
                || metrics.compute_time_us > policy.max_compute_time_ms * 1000
            {
                continue;
            }

            // Risk validation logic remains same, but needs to lookup liquidity from compacted graph
            if let Some(ref validator) = self.risk_validator {
                let mut path_valid = true;
                for hop in &path.hops {
                    // This is inefficient but keep it for now
                    let mut edge_liquidity = 0;
                    if let Some(&from_idx) = graph.asset_map.get(&hop.source_asset) {
                        for edge in graph.get_neighbors(from_idx) {
                            if edge.venue_ref == hop.venue_ref {
                                edge_liquidity = edge.liquidity;
                                break;
                            }
                        }
                    }

                    if let Err(exclusion) =
                        validator.validate_impact(&hop.destination_asset, metrics.impact_bps)
                    {
                        excluded_routes.push(exclusion);
                        path_valid = false;
                        break;
                    }

                    if let Err(exclusion) =
                        validator.validate_liquidity(&hop.destination_asset, edge_liquidity)
                    {
                        excluded_routes.push(exclusion);
                        path_valid = false;
                        break;
                    }

                    if let Err(exclusion) =
                        validator.validate_exposure(&hop.destination_asset, amount_in)
                    {
                        excluded_routes.push(exclusion);
                        path_valid = false;
                        break;
                    }
                }

                if !path_valid {
                    continue;
                }
            }

            scored_paths.push((path.clone(), metrics));
        }

        if scored_paths.is_empty() {
            return Err(RoutingError::NoRoute(
                "".to_string(),
                "no routes meet policy or risk constraints".to_string(),
            ));
        }

        scored_paths.sort_by(|a, b| b.1.score.partial_cmp(&a.1.score).unwrap());

        let (selected_path, selected_metrics) = scored_paths[0].clone();
        let alternatives: Vec<(SwapPath, RouteMetrics)> =
            scored_paths.into_iter().skip(1).collect();

        let total_compute_time_ms = start_time.elapsed().as_millis() as u64;

        let span = tracing::Span::current();
        span.record("route.paths_evaluated", paths.len());
        span.record("route.compute_time_ms", total_compute_time_ms);

        Ok(OptimizerDiagnostics {
            selected_path,
            metrics: selected_metrics,
            alternatives,
            policy: policy.clone(),
            total_compute_time_ms: start_time.elapsed().as_millis() as u64,
            excluded_routes,
            active_scorer_name: self.scorer_registry.active_scorer_name().to_string(),
            flagged_venues: vec![],
        })
    }

    /// Calculate comprehensive route metrics using a compacted graph
    fn calculate_route_metrics_compacted(
        &self,
        path: &SwapPath,
        graph: &crate::compaction::CompactedGraph,
        amount_in: i128,
    ) -> Result<RouteMetrics> {
        let start_time = Instant::now();

        let mut total_output = amount_in;
        let mut total_impact_bps = 0u32;
        let mut max_anomaly_score = 0.0f64;
        let mut all_anomaly_reasons = Vec::new();

        // Simulate execution through each hop
        for hop in &path.hops {
            // Find corresponding edge in compacted graph
            let from_idx = *graph.asset_map.get(&hop.source_asset).ok_or_else(|| {
                RoutingError::NoRoute(hop.source_asset.clone(), hop.destination_asset.clone())
            })?;

            let edge = graph
                .get_neighbors(from_idx)
                .iter()
                .find(|e| {
                    graph.assets[e.to_idx as usize] == hop.destination_asset
                        && e.venue_ref == hop.venue_ref
                })
                .ok_or_else(|| {
                    RoutingError::NoRoute(hop.source_asset.clone(), hop.destination_asset.clone())
                })?;

            // Calculate impact based on venue type index
            let (output, impact_bps) = if edge.venue_type_idx == 1 {
                // Simulate AMM calculation (simplified)
                let estimated_output = (total_output * 9970) / 10000; // 0.3% fee
                (estimated_output, 30) // Simplified impact
            } else {
                // Simulate orderbook calculation
                let estimated_output = (total_output * 9980) / 10000; // 0.2% fee
                (estimated_output, 20) // Simplified impact
            };

            total_output = output;
            total_impact_bps = total_impact_bps.saturating_add(impact_bps);
            max_anomaly_score = max_anomaly_score.max(hop.anomaly_score);
            all_anomaly_reasons.extend(hop.anomaly_reasons.clone());
        }

        let compute_time_us = start_time.elapsed().as_micros() as u64;

        let scorer_input = ScorerInput {
            output_amount: total_output,
            impact_bps: total_impact_bps,
            compute_time_us,
            hop_count: path.hops.len(),
            policy: self.active_policy().clone(),
        };
        let scorer_output = self.scorer_registry.score(&scorer_input);
        let score = scorer_output.score;

        Ok(RouteMetrics {
            output_amount: total_output,
            impact_bps: total_impact_bps,
            compute_time_us,
            hop_count: path.hops.len(),
            score,
            anomaly_score: max_anomaly_score,
            anomaly_reasons: all_anomaly_reasons,
        })
    }

    /// Benchmark different policies for comparison
    pub fn benchmark_policies(
        &mut self,
        from: &str,
        to: &str,
        edges: &[LiquidityEdge],
        amount_in: i128,
        routing_policy: &RoutingPolicy,
    ) -> Result<Vec<(String, OptimizerDiagnostics)>> {
        let mut results = Vec::new();
        let original_policy = self.active_policy.clone();
        let policy_names: Vec<String> = self.policies.keys().cloned().collect();

        for env_name in policy_names {
            self.set_active_policy(&env_name)?;
            let diagnostics =
                self.find_optimal_routes(from, to, edges, amount_in, routing_policy)?;
            results.push((env_name.clone(), diagnostics));
        }

        // Restore original policy
        self.set_active_policy(&original_policy)?;
        Ok(results)
    }
}

impl Default for HybridOptimizer {
    fn default() -> Self {
        Self::new(PathfinderConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_validation() {
        let valid_policy = OptimizerPolicy::default();
        assert!(valid_policy.validate().is_ok());

        let invalid_policy = OptimizerPolicy {
            output_weight: 0.8,
            impact_weight: 0.8,
            latency_weight: 0.2, // Sum = 1.8
            scorer: None,
            ..Default::default()
        };
        assert!(invalid_policy.validate().is_err());
    }

    #[test]
    fn test_policy_presets() {
        let prod = PolicyPresets::production();
        assert!(prod.validate().is_ok());
        assert_eq!(prod.environment, "production");

        let analysis = PolicyPresets::analysis();
        assert!(analysis.output_weight > prod.output_weight);
        assert!(analysis.max_compute_time_ms > prod.max_compute_time_ms);
    }

    #[test]
    fn test_optimizer_creation() {
        let optimizer = HybridOptimizer::default();
        assert_eq!(optimizer.active_policy().environment, "production");
        assert!(optimizer.policies.contains_key("realtime"));
        assert!(optimizer.policies.contains_key("analysis"));
    }

    #[test]
    fn test_policy_switching() {
        let mut optimizer = HybridOptimizer::default();

        assert!(optimizer.set_active_policy("realtime").is_ok());
        assert_eq!(optimizer.active_policy().environment, "realtime");

        assert!(optimizer.set_active_policy("invalid").is_err());
    }

    #[test]
    fn test_custom_policy() {
        let mut optimizer = HybridOptimizer::default();

        let custom_policy = OptimizerPolicy {
            output_weight: 0.6,
            impact_weight: 0.3,
            latency_weight: 0.1,
            max_impact_bps: 200,
            max_compute_time_ms: 300,
            environment: "custom".to_string(),
            scorer: None,
        };

        assert!(optimizer.add_policy(custom_policy).is_ok());
        assert!(optimizer.set_active_policy("custom").is_ok());
    }

    #[test]
    fn test_set_scorer_delegates_to_registry() {
        let mut optimizer = HybridOptimizer::default();
        assert!(optimizer.set_scorer("fee_minimizing").is_ok());
        assert_eq!(optimizer.scorer_registry.active_scorer_name(), "fee_minimizing");
        assert!(optimizer.set_scorer("nonexistent").is_err());
    }

    #[test]
    fn test_with_scorer_registry_constructor() {
        let mut registry = ScorerRegistry::new();
        registry.set_active("output_maximizing").unwrap();
        let optimizer = HybridOptimizer::with_scorer_registry(PathfinderConfig::default(), registry);
        assert_eq!(optimizer.scorer_registry.active_scorer_name(), "output_maximizing");
    }

    #[test]
    fn test_policy_from_env_no_var() {
        // Ensure ROUTING_SCORER is not set for this test
        std::env::remove_var("ROUTING_SCORER");
        let policy = OptimizerPolicy::from_env();
        assert!(policy.scorer.is_none());
    }

    #[test]
    fn test_policy_from_env_with_var() {
        std::env::set_var("ROUTING_SCORER", "fee_minimizing");
        let policy = OptimizerPolicy::from_env();
        assert_eq!(policy.scorer, Some("fee_minimizing".to_string()));
        std::env::remove_var("ROUTING_SCORER");
    }

    #[test]
    fn test_find_optimal_routes_includes_active_scorer_name() {
        use crate::pathfinder::LiquidityEdge;
        use crate::policy::RoutingPolicy;

        let optimizer = HybridOptimizer::default();
        let edges = vec![LiquidityEdge {
            from: "XLM".to_string(),
            to: "USDC".to_string(),
            venue_type: "amm".to_string(),
            venue_ref: "pool1".to_string(),
            liquidity: 10_000_000,
            price: 0.1,
            fee_bps: 30,
        }];
        let policy = RoutingPolicy::default();
        let result = optimizer.find_optimal_routes("XLM", "USDC", &edges, 1_000_000, &policy);
        assert!(result.is_ok());
        let diag = result.unwrap();
        assert_eq!(diag.active_scorer_name, "default");
    }

    #[test]
    fn test_benchmark_scorers_returns_report() {
        use crate::pathfinder::{LiquidityEdge, PathHop, SwapPath};

        let optimizer = HybridOptimizer::default();
        let paths = vec![SwapPath {
            hops: vec![PathHop {
                source_asset: "XLM".to_string(),
                destination_asset: "USDC".to_string(),
                venue_type: "amm".to_string(),
                venue_ref: "pool1".to_string(),
                price: 0.1,
                fee_bps: 30,
            }],
            estimated_output: 900_000,
        }];
        let edges: Vec<LiquidityEdge> = vec![];
        let report = optimizer.benchmark_scorers(&paths, &edges, 1_000_000);
        // Should have 3 built-in scorers
        assert_eq!(report.scorer_results.len(), 3);
    }
}
