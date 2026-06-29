//! Deterministic simulation engine for routing analysis

use crate::error::Result;
use crate::optimizer::{HybridOptimizer, OptimizerDiagnostics};
use crate::pathfinder::LiquidityEdge;
use crate::policy::RoutingPolicy;
use serde::{Deserialize, Serialize};

/// Synthetic market perturbations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketShock {
    /// Percentage reduction in available liquidity (0.0 to 1.0)
    LiquidityDrain { venue_ref: String, percentage: f64 },
    /// Percentage increase in price (0.0 to 1.0)
    PriceJump { venue_ref: String, percentage: f64 },
    /// Complete removal of a venue
    VenueOutage { venue_ref: String },
}

/// Simulation scenario configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationScenario {
    pub name: String,
    pub from_asset: String,
    pub to_asset: String,
    pub amount_in: i128,
    pub shocks: Vec<MarketShock>,
    pub seed: u64,
}

/// Results of a simulation run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub scenario_name: String,
    pub baseline: OptimizerDiagnostics,
    pub shocked: OptimizerDiagnostics,
    /// Higher is better (0.0 to 1.0). Ratio of shocked output to baseline output.
    pub stability_score: f64,
    /// Difference in output amount
    pub output_delta: i128,
}

pub struct RouteSimulator {
    optimizer: HybridOptimizer,
}

impl RouteSimulator {
    pub fn new(optimizer: HybridOptimizer) -> Self {
        Self { optimizer }
    }

    /// Run a deterministic simulation scenario
    pub fn run_scenario(
        &self,
        scenario: &SimulationScenario,
        base_edges: &[LiquidityEdge],
        routing_policy: &RoutingPolicy,
    ) -> Result<SimulationResult> {
        // 1. Run baseline
        let baseline = self.optimizer.find_optimal_routes(
            &scenario.from_asset,
            &scenario.to_asset,
            base_edges,
            scenario.amount_in,
            routing_policy,
        )?;

        // 2. Apply shocks to edges
        let mut shocked_edges = base_edges.to_vec();
        for shock in &scenario.shocks {
            self.apply_shock(&mut shocked_edges, shock);
        }

        // 3. Run shocked simulation
        let shocked = self.optimizer.find_optimal_routes(
            &scenario.from_asset,
            &scenario.to_asset,
            &shocked_edges,
            scenario.amount_in,
            routing_policy,
        )?;

        // 4. Calculate metrics
        let output_delta = shocked.metrics.output_amount - baseline.metrics.output_amount;
        let stability_score = if baseline.metrics.output_amount > 0 {
            (shocked.metrics.output_amount as f64) / (baseline.metrics.output_amount as f64)
        } else {
            0.0
        };

        Ok(SimulationResult {
            scenario_name: scenario.name.clone(),
            baseline,
            shocked,
            stability_score,
            output_delta,
        })
    }

    fn apply_shock(&self, edges: &mut [LiquidityEdge], shock: &MarketShock) {
        match shock {
            MarketShock::LiquidityDrain {
                venue_ref,
                percentage,
            } => {
                for edge in edges.iter_mut() {
                    if edge.venue_ref == *venue_ref {
                        let drain = (edge.liquidity as f64 * percentage) as i128;
                        edge.liquidity = edge.liquidity.saturating_sub(drain);
                    }
                }
            }
            MarketShock::PriceJump {
                venue_ref,
                percentage,
            } => {
                for edge in edges.iter_mut() {
                    if edge.venue_ref == *venue_ref {
                        edge.price *= 1.0 + percentage;
                    }
                }
            }
            MarketShock::VenueOutage { venue_ref } => {
                for edge in edges.iter_mut() {
                    if edge.venue_ref == *venue_ref {
                        edge.liquidity = 0;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimizer::HybridOptimizer;
    use crate::pathfinder::{LiquidityEdge, PathfinderConfig};
    use crate::policy::RoutingPolicy;

    fn mock_edges() -> Vec<LiquidityEdge> {
        vec![
            LiquidityEdge {
                from: "XLM".to_string(),
                to: "USDC".to_string(),
                venue_type: "sdex".to_string(),
                venue_ref: "venue1".to_string(),
                liquidity: 1_000_000_000,
                price: 0.12,
                fee_bps: 0,
            },
            LiquidityEdge {
                from: "XLM".to_string(),
                to: "USDC".to_string(),
                venue_type: "amm".to_string(),
                venue_ref: "venue2".to_string(),
                liquidity: 1_000_000_000,
                price: 0.121,
                fee_bps: 30,
            },
        ]
    }

    #[test]
    fn test_simulator_liquidity_drain() {
        let optimizer = HybridOptimizer::new(PathfinderConfig::default());
        let simulator = RouteSimulator::new(optimizer);
        let edges = mock_edges();
        let policy = RoutingPolicy::default();

        let scenario = SimulationScenario {
            name: "test_drain".to_string(),
            from_asset: "XLM".to_string(),
            to_asset: "USDC".to_string(),
            amount_in: 100_000_000,
            shocks: vec![MarketShock::LiquidityDrain {
                venue_ref: "venue1".to_string(),
                percentage: 0.9,
            }],
            seed: 42,
        };

        let result = simulator.run_scenario(&scenario, &edges, &policy).unwrap();
        assert!(result.stability_score <= 1.0);
    }
}
