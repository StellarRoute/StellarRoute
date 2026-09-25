//! Pathfinding algorithms for swap routing with N-hop support and safety bounds

use crate::cross_chain::{is_bridge_edge, BridgeEdgeMeta};
use crate::error::{Result, RoutingError};
use crate::policy::{RouteDiagnostic, RoutingPolicy};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use tracing::instrument;

/// Configuration for path discovery
#[derive(Clone, Debug)]
pub struct PathfinderConfig {
    pub min_liquidity_threshold: i128,
}

impl Default for PathfinderConfig {
    fn default() -> Self {
        Self {
            min_liquidity_threshold: 1_000_000,
        }
    }
}

/// Represents a liquidity edge in the routing graph
///
/// `from` / `to` may be legacy Stellar identifiers or CAIP-19 chain-scoped ids.
/// Bridge/cross-chain edges optionally carry [`BridgeEdgeMeta`] (abstraction only).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiquidityEdge {
    pub from: String,
    pub to: String,
    pub venue_type: String,
    pub venue_ref: String,
    pub liquidity: i128,
    #[serde(default)]
    pub price: f64,
    #[serde(default = "default_fee_bps")]
    pub fee_bps: u32,
    /// Optional liquidity provider id (DEX venue operator or bridge adapter).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Optional bridge / cross-chain metadata. Absent for same-chain DEX edges.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridge: Option<BridgeEdgeMeta>,
}

impl Default for LiquidityEdge {
    fn default() -> Self {
        Self {
            from: String::new(),
            to: String::new(),
            venue_type: String::new(),
            venue_ref: String::new(),
            liquidity: 0,
            price: 0.0,
            fee_bps: default_fee_bps(),
            provider: None,
            bridge: None,
        }
    }
}

fn default_fee_bps() -> u32 {
    30
}

/// Represents a path through liquidity sources
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SwapPath {
    pub hops: Vec<PathHop>,
    pub estimated_output: i128,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct PathHop {
    pub source_asset: String,
    pub destination_asset: String,
    pub venue_type: String,
    pub venue_ref: String,
    pub price: f64,
    pub fee_bps: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridge: Option<BridgeEdgeMeta>,
}

/// N-hop pathfinder with safety bounds
pub struct Pathfinder {
    config: PathfinderConfig,
}

impl Pathfinder {
    pub fn new(config: PathfinderConfig) -> Self {
        Self { config }
    }

    /// Find optimal N-hop paths with cycle prevention and depth limits
    #[instrument(skip(self, edges, policy), fields(
        route.from = %from,
        route.to = %to,
        route.edges_count = edges.len(),
        route.paths_found = tracing::field::Empty
    ))]
    pub fn find_paths(
        &self,
        from: &str,
        to: &str,
        edges: &[LiquidityEdge],
        amount_in: i128,
        policy: &RoutingPolicy,
    ) -> Result<Vec<SwapPath>> {
        if from.is_empty() || to.is_empty() {
            return Err(RoutingError::InvalidPair(
                "source or destination is empty".to_string(),
            ));
        }

        if from == to {
            return Err(RoutingError::InvalidPair(
                "source and destination must differ".to_string(),
            ));
        }

        if amount_in <= 0 {
            return Err(RoutingError::InvalidAmount(
                "amount_in must be positive".to_string(),
            ));
        }

        let graph = self.build_graph(edges, policy)?;

        let raw_paths = self.bfs_paths(&graph, from, to, amount_in, policy.max_hops)?;

        if raw_paths.is_empty() {
            return Err(RoutingError::NoRoute(from.to_string(), to.to_string()));
        }

        // 🔥 APPLY POLICY FILTER (CRITICAL REQUIREMENT)
        let mut diagnostics: Vec<RouteDiagnostic> = Vec::new();

        let filtered_paths: Vec<SwapPath> = raw_paths
            .into_iter()
            .enumerate()
            .filter_map(|(idx, path)| {
                // Convert PathHop -> RouteHop (policy-compatible)
                let hops_for_policy = path
                    .hops
                    .iter()
                    .map(|h| crate::policy::RouteHop {
                        venue_type: h.venue_type.clone(),
                        asset: h.destination_asset.clone(),
                        provider: h
                            .provider
                            .clone()
                            .or_else(|| h.bridge.as_ref().and_then(|b| b.provider.clone())),
                        bridge: h.bridge.clone(),
                    })
                    .collect::<Vec<_>>();

                let route_id = format!("route_{}", idx);

                if let Some(diag) = policy.should_exclude_route(&route_id, &hops_for_policy) {
                    diagnostics.push(diag);
                    None
                } else {
                    Some(path)
                }
            })
            .collect();

        // You could log diagnostics if needed (safe exposure)
        if !diagnostics.is_empty() {
            tracing::debug!(
                excluded_routes = diagnostics.len(),
                "routes excluded by policy"
            );
        }

        if filtered_paths.is_empty() {
            return Err(RoutingError::NoRoute(from.to_string(), to.to_string()));
        }

        tracing::Span::current().record("route.paths_found", filtered_paths.len());

        Ok(filtered_paths)
    }

    fn build_graph(
        &self,
        edges: &[LiquidityEdge],
        policy: &RoutingPolicy,
    ) -> Result<HashMap<String, Vec<LiquidityEdge>>> {
        let mut graph: HashMap<String, Vec<LiquidityEdge>> = HashMap::new();

        for edge in edges {
            // Hard-disable bridges unless explicitly opted in (default: false).
            if is_bridge_edge(&edge.venue_type, edge.bridge.as_ref()) && !policy.allow_bridge_edges
            {
                continue;
            }

            // Even when opted in, reject contradictory bridge metadata.
            if let Some(bridge) = edge.bridge.as_ref() {
                if bridge
                    .validate_against_endpoints(&edge.from, &edge.to)
                    .is_err()
                {
                    continue;
                }
            }

            if !policy.is_venue_allowed(&edge.venue_type) {
                continue;
            }

            let provider = edge
                .provider
                .as_deref()
                .or_else(|| edge.bridge.as_ref().and_then(|b| b.provider.as_deref()));
            if !policy.is_provider_allowed(provider) {
                continue;
            }

            if edge.liquidity < self.config.min_liquidity_threshold {
                continue;
            }

            graph
                .entry(edge.from.clone())
                .or_default()
                .push(edge.clone());
        }

        Ok(graph)
    }

    fn bfs_paths(
        &self,
        graph: &HashMap<String, Vec<LiquidityEdge>>,
        from: &str,
        to: &str,
        amount_in: i128,
        max_hops: usize,
    ) -> Result<Vec<SwapPath>> {
        let mut paths = Vec::new();
        let mut queue = VecDeque::new();

        let mut initial_visited = std::collections::HashSet::new();
        initial_visited.insert(from.to_string());

        queue.push_back((from.to_string(), Vec::new(), initial_visited, amount_in));

        while let Some((current, path_hops, visited, estimated_output)) = queue.pop_front() {
            if path_hops.len() >= max_hops {
                continue;
            }

            if current == to {
                paths.push(SwapPath {
                    hops: path_hops.clone(),
                    estimated_output,
                });
                continue;
            }

            if let Some(neighbors) = graph.get(&current) {
                for edge in neighbors {
                    if visited.contains(&edge.to) {
                        continue;
                    }

                    let mut new_visited = visited.clone();
                    new_visited.insert(edge.to.clone());

                    let hop = PathHop {
                        source_asset: edge.from.clone(),
                        destination_asset: edge.to.clone(),
                        venue_type: edge.venue_type.clone(),
                        venue_ref: edge.venue_ref.clone(),
                        price: edge.price,
                        fee_bps: edge.fee_bps,
                        provider: edge.provider.clone(),
                        bridge: edge.bridge.clone(),
                    };

                    let estimated_after_hop = (estimated_output * 9950) / 10000;

                    let mut new_hops = path_hops.clone();
                    new_hops.push(hop);

                    queue.push_back((edge.to.clone(), new_hops, new_visited, estimated_after_hop));
                }
            }
        }

        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::RoutingPolicy;

    fn test_policy() -> RoutingPolicy {
        RoutingPolicy::default()
    }

    fn thin_book_edges() -> Vec<LiquidityEdge> {
        vec![LiquidityEdge {
            from: "native".to_string(),
            to: "USDC:issuer".to_string(),
            venue_type: "sdex".to_string(),
            venue_ref: "sdex:1001".to_string(),
            liquidity: 100, // Very thin liquidity
            price: 0.1,
            fee_bps: 0,
        }]
    }

    fn normal_edges() -> Vec<LiquidityEdge> {
        vec![LiquidityEdge {
            from: "native".to_string(),
            to: "USDC:issuer".to_string(),
            venue_type: "sdex".to_string(),
            venue_ref: "sdex:1001".to_string(),
            liquidity: 10_000_000,
            price: 0.1,
            fee_bps: 0,
        }]
    }

    #[test]
    fn test_thin_book_no_panic() {
        let pathfinder = Pathfinder::new(PathfinderConfig::default());
        let policy = test_policy();
        let edges = thin_book_edges();

        // Should not panic even with thin liquidity
        let result = pathfinder.find_paths("native", "USDC:issuer", &edges, 1_000_000, &policy);
        assert!(result.is_ok());
    }

    #[test]
    fn test_thin_book_respects_min_liquidity() {
        let config = PathfinderConfig {
            min_liquidity_threshold: 1_000, // Higher than thin book liquidity
        };
        let pathfinder = Pathfinder::new(config);
        let policy = test_policy();
        let edges = thin_book_edges();

        // Thin liquidity edge should be filtered out
        let result = pathfinder.find_paths("native", "USDC:issuer", &edges, 1_000_000, &policy);
        assert!(result.is_err());
    }

    #[test]
    fn test_oversized_amount_no_panic() {
        let pathfinder = Pathfinder::new(PathfinderConfig::default());
        let policy = test_policy();
        let edges = normal_edges();

        // Should not panic with very large amounts
        let oversized = i128::MAX / 2;
        let result = pathfinder.find_paths("native", "USDC:issuer", &edges, oversized, &policy);
        assert!(result.is_ok());
    }

    #[test]
    fn test_oversized_amount_respects_max_hops() {
        let mut policy = test_policy();
        policy.max_hops = 1;

        let pathfinder = Pathfinder::new(PathfinderConfig::default());

        // Multi-hop edges
        let edges = vec![
            LiquidityEdge {
                from: "A".to_string(),
                to: "B".to_string(),
                venue_type: "sdex".to_string(),
                venue_ref: "sdex:1".to_string(),
                liquidity: 10_000_000,
                price: 1.0,
                fee_bps: 0,
            },
            LiquidityEdge {
                from: "B".to_string(),
                to: "C".to_string(),
                venue_type: "sdex".to_string(),
                venue_ref: "sdex:2".to_string(),
                liquidity: 10_000_000,
                price: 1.0,
                fee_bps: 0,
            },
        ];

        // Should not find 2-hop path when max_hops=1
        let result = pathfinder.find_paths("A", "C", &edges, 1_000_000, &policy);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_edges_returns_no_route() {
        let pathfinder = Pathfinder::new(PathfinderConfig::default());
        let policy = test_policy();
        let edges: Vec<LiquidityEdge> = vec![];

        let result = pathfinder.find_paths("native", "USDC:issuer", &edges, 1_000_000, &policy);
        assert!(result.is_err());
    }

    #[test]
    fn test_same_source_dest_returns_error() {
        let pathfinder = Pathfinder::new(PathfinderConfig::default());
        let policy = test_policy();
        let edges = normal_edges();

        let result = pathfinder.find_paths("native", "native", &edges, 1_000_000, &policy);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_amount_returns_error() {
        let pathfinder = Pathfinder::new(PathfinderConfig::default());
        let policy = test_policy();
        let edges = normal_edges();

        let result = pathfinder.find_paths("native", "USDC:issuer", &edges, 0, &policy);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_amount_returns_error() {
        let pathfinder = Pathfinder::new(PathfinderConfig::default());
        let policy = test_policy();
        let edges = normal_edges();

        let result = pathfinder.find_paths("native", "USDC:issuer", &edges, -1_000_000, &policy);
        assert!(result.is_err());
    }
}
