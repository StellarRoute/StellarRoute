//! StellarRoute Routing Engine
//!
//! Provides pathfinding algorithms for optimal swap routing across SDEX and Soroban AMM pools.
//! Supports N-hop paths with safety bounds, cycle prevention, and price impact calculation.

pub mod error;
pub mod health;
pub mod impact;
pub mod normalization;
pub mod optimizer;
pub mod simulator;
pub mod pathfinder;
pub mod policy;

pub use impact::{AmmQuoteCalculator, OrderbookImpactCalculator};
pub use optimizer::{
    HybridOptimizer, OptimizerDiagnostics, OptimizerPolicy, PolicyPresets, RouteMetrics,
};
pub use pathfinder::{LiquidityEdge, Pathfinder, PathfinderConfig, SwapPath};
pub use policy::RoutingPolicy;

/// Routing engine with integrated pathfinding and impact calculations
pub struct RoutingEngine {
    pathfinder: Pathfinder,
    amm_calculator: AmmQuoteCalculator,
    orderbook_calculator: OrderbookImpactCalculator,
    hybrid_optimizer: HybridOptimizer,
    routing_policy: RoutingPolicy,
}

impl RoutingEngine {
    /// Create a new routing engine instance with default config
    pub fn new() -> Self {
        Self::with_config(PathfinderConfig::default())
    }

    /// Create a new routing engine with custom config
    pub fn with_config(config: PathfinderConfig) -> Self {
        Self::with_config_and_policy(config, RoutingPolicy::default())
    }

    /// Create a new routing engine with custom config and routing policy
    pub fn with_config_and_policy(config: PathfinderConfig, policy: RoutingPolicy) -> Self {
        Self {
            pathfinder: Pathfinder::new(config.clone()),
            amm_calculator: AmmQuoteCalculator,
            orderbook_calculator: OrderbookImpactCalculator,
            hybrid_optimizer: HybridOptimizer::new(config),
            routing_policy: policy,
        }
    }

    /// Get reference to pathfinder
    pub fn pathfinder(&self) -> &Pathfinder {
        &self.pathfinder
    }

    /// Get reference to AMM calculator
    pub fn amm_calculator(&self) -> &AmmQuoteCalculator {
        &self.amm_calculator
    }

    /// Get reference to orderbook calculator
    pub fn orderbook_calculator(&self) -> &OrderbookImpactCalculator {
        &self.orderbook_calculator
    }

    /// Get reference to hybrid optimizer
    pub fn hybrid_optimizer(&self) -> &HybridOptimizer {
        &self.hybrid_optimizer
    }

    /// Get mutable reference to hybrid optimizer
    pub fn hybrid_optimizer_mut(&mut self) -> &mut HybridOptimizer {
        &mut self.hybrid_optimizer
    }

    /// Get reference to routing policy
    pub fn routing_policy(&self) -> &RoutingPolicy {
        &self.routing_policy
    }
}

impl Default for RoutingEngine {
    fn default() -> Self {
        Self::new()
    }
}
