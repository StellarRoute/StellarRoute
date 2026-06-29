//! Pathfinding algorithms for swap routing with N-hop support and safety bounds

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
    /// Score indicating routing irregularities or flash-loan anomalies
    pub anomaly_score: Option<f64>,
    /// Categorized structural anomalies or reasons for transaction flags
    pub anomaly_reasons: Option<Vec<String>>,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PathHop {
    pub source_asset: String,
    pub destination_asset: String,
    pub venue_type: String,
    pub venue_ref: String,
    pub price: f64,
    pub fee_bps: u32,
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
            if !policy.is_venue_allowed(&edge.venue_type) {
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