//! Hybrid route optimizer combining latency and execution quality

use crate::error::{Result, RoutingError};
use crate::impact::{AmmQuoteCalculator, OrderbookImpactCalculator};
use crate::pathfinder::{LiquidityEdge, Pathfinder, PathfinderConfig, SwapPath};
use crate::policy::RoutingPolicy;
use crate::risk::{RiskLimitConfig, RiskValidator, RouteExclusion};
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
        }
    }

    /// Create optimizer with risk limits
    pub fn with_risk_limits(config: PathfinderConfig, risk_config: RiskLimitConfig) -> Self {
        let mut optimizer = Self::new(config);
        optimizer.risk_validator = Some(RiskValidator::new(risk_config));
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
            flagged_venues: Vec::new(),
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
        let score = self.calculate_score(total_output, total_impact_bps, compute_time_us);

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

    /// Calculate normalized score using policy weights
    fn calculate_score(&self, output: i128, impact_bps: u32, compute_time_us: u64) -> f64 {
        let policy = self.active_policy();

        // Normalize metrics (simplified normalization)
        // Higher output is better (normalize by input amount assumption)
        let output_score = (output as f64 / 1_000_000_000.0).min(1.0); // Normalize to ~1B

        // Lower impact is better
        let impact_score = 1.0 - (impact_bps as f64 / 1000.0).min(1.0); // Normalize to 1000 bps

        // Lower compute time is better
        let latency_score = 1.0 - (compute_time_us as f64 / 1_000_000.0).min(1.0); // Normalize to 1ms

        // Weighted combination
        policy.output_weight * output_score
            + policy.impact_weight * impact_score
            + policy.latency_weight * latency_score
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
        };

        assert!(optimizer.add_policy(custom_policy).is_ok());
        assert!(optimizer.set_active_policy("custom").is_ok());
    }
}
