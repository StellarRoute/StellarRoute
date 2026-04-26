//! Pathfinding algorithms for swap routing with N-hop support and safety bounds

use crate::error::{Result, RoutingError};
use crate::policy::RoutingPolicy;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tracing::instrument;

/// Configuration for path discovery
#[derive(Clone, Debug)]
pub struct PathfinderConfig {
    /// Minimum liquidity threshold for intermediate assets
    pub min_liquidity_threshold: i128,
}

impl Default for PathfinderConfig {
    fn default() -> Self {
        Self {
            min_liquidity_threshold: 1_000_000, // 1 unit in e7
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
    pub price: f64,
    pub fee_bps: u32,
    #[serde(default)]
    pub anomaly_score: f64,
    #[serde(default)]
    pub anomaly_reasons: Vec<String>,
}

/// Represents a path through liquidity sources
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SwapPath {
    pub hops: Vec<PathHop>,
    pub estimated_output: i128,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PathHop {
    pub source_asset: String,
    pub destination_asset: String,
    pub venue_type: String,
    pub venue_ref: String,
    pub price: f64,
    pub fee_bps: u32,
    #[serde(default)]
    pub anomaly_score: f64,
    #[serde(default)]
    pub anomaly_reasons: Vec<String>,
}

/// N-hop pathfinder with safety bounds
pub struct Pathfinder {
    config: PathfinderConfig,
}

impl Pathfinder {
    pub fn new(config: PathfinderConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &PathfinderConfig {
        &self.config
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
        let compacted = crate::compaction::CompactedGraph::from_edges(edges.to_vec());
        self.find_paths_compacted(from, to, &compacted, amount_in, policy)
    }

    /// Find optimal N-hop paths using a compacted graph representation
    pub fn find_paths_compacted(
        &self,
        from: &str,
        to: &str,
        graph: &crate::compaction::CompactedGraph,
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

        let from_idx = graph
            .asset_map
            .get(from)
            .cloned()
            .ok_or_else(|| RoutingError::NoRoute(from.to_string(), to.to_string()))?;
        let to_idx = graph
            .asset_map
            .get(to)
            .cloned()
            .ok_or_else(|| RoutingError::NoRoute(from.to_string(), to.to_string()))?;

        let paths = self.bfs_paths_compacted(graph, from_idx, to_idx, amount_in, policy)?;

        if paths.is_empty() {
            return Err(RoutingError::NoRoute(from.to_string(), to.to_string()));
        }

        tracing::Span::current().record("route.paths_found", paths.len());

        Ok(paths)
    }

    fn bfs_paths_compacted(
        &self,
        graph: &crate::compaction::CompactedGraph,
        from_idx: u32,
        to_idx: u32,
        amount_in: i128,
        policy: &RoutingPolicy,
    ) -> Result<Vec<SwapPath>> {
        let mut paths = Vec::new();
        let mut queue = VecDeque::new();

        let mut initial_visited = std::collections::HashSet::new();
        initial_visited.insert(from_idx);
        queue.push_back((from_idx, Vec::new(), initial_visited, amount_in));

        while let Some((current_idx, path_hops, visited, estimated_output)) = queue.pop_front() {
            if path_hops.len() >= policy.max_hops {
                continue;
            }

            if current_idx == to_idx {
                paths.push(SwapPath {
                    hops: path_hops.clone(),
                    estimated_output,
                });
                continue;
            }

            // Explore neighbors
            if let Some(neighbors) = graph.get(&current) {
                for edge in neighbors {
                    // Cycle prevention
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
                        anomaly_score: edge.anomaly_score,
                        anomaly_reasons: edge.anomaly_reasons.clone(),
                    };

                    // Simple output estimation (50bps slippage per hop)
                    let estimated_after_hop = (estimated_output * 9950) / 10000;

                    let mut new_hops = path_hops.clone();
                    new_hops.push(hop);

                    queue.push_back((edge.to.clone(), new_hops, new_visited, estimated_after_hop));
            for edge in graph.get_neighbors(current_idx) {
                let venue_type = if edge.venue_type_idx == 1 {
                    "amm"
                } else {
                    "sdex"
                };
                if !policy.is_venue_allowed(venue_type) {
                    continue;
                }

                if edge.liquidity < self.config.min_liquidity_threshold {
                    continue;
                }

                if visited.contains(&edge.to_idx) {
                    continue;
                }

                let mut new_visited = visited.clone();
                new_visited.insert(edge.to_idx);

                let hop = crate::pathfinder::PathHop {
                    source_asset: graph.assets[current_idx as usize].clone(),
                    destination_asset: graph.assets[edge.to_idx as usize].clone(),
                    venue_type: venue_type.to_string(),
                    venue_ref: edge.venue_ref.clone(),
                    price: edge.price,
                    fee_bps: edge.fee_bps,
                    anomaly_score: edge.anomaly_score as f64,
                    anomaly_reasons: vec![],
                };

                let estimated_after_hop = (estimated_output * 9950) / 10000;

                let mut new_hops = path_hops.clone();
                new_hops.push(hop);

                queue.push_back((edge.to_idx, new_hops, new_visited, estimated_after_hop));
            }
        }

        Ok(paths)
    }
}
