//! Pathfinding algorithms for swap routing with N-hop support and safety bounds

use crate::cross_chain::{is_bridge_edge, BridgeEdgeMeta};
use crate::error::{Result, RoutingError};
use crate::policy::{RouteDiagnostic, RoutingPolicy};
use serde::{Deserialize, Serialize};
use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, HashMap};
use tracing::instrument;

const BPS_DENOM: i128 = 10_000;
const PRICE_SCALE: i128 = 1_000_000_000;
const INLINE_MAX_HOPS: usize = 8;

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

#[derive(Clone, Copy, Debug)]
struct IndexedEdge {
    edge_idx: u32,
    to_id: u32,
}

struct IndexedGraph {
    adj: Vec<Vec<IndexedEdge>>,
    num_nodes: usize,
}

#[derive(Clone, Debug)]
enum VisitedSet {
    Inline([u64; 4]),
    Words(Vec<u64>),
}

impl VisitedSet {
    fn new(num_nodes: usize, start: u32) -> Self {
        let mut set = if num_nodes <= 256 {
            Self::Inline([0; 4])
        } else {
            Self::Words(vec![0; num_nodes.div_ceil(64)])
        };
        set.insert(start);
        set
    }

    #[inline]
    fn contains(&self, node: u32) -> bool {
        let idx = (node >> 6) as usize;
        let mask = 1_u64 << (node & 63);
        match self {
            Self::Inline(words) => (words[idx] & mask) != 0,
            Self::Words(words) => (words[idx] & mask) != 0,
        }
    }

    #[inline]
    fn insert(&mut self, node: u32) {
        let idx = (node >> 6) as usize;
        let mask = 1_u64 << (node & 63);
        match self {
            Self::Inline(words) => words[idx] |= mask,
            Self::Words(words) => words[idx] |= mask,
        }
    }

    #[inline]
    fn with(&self, node: u32) -> Self {
        let mut copy = self.clone();
        copy.insert(node);
        copy
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum HopIndices {
    Inline {
        len: u8,
        edges: [u32; INLINE_MAX_HOPS],
    },
    Heap(Vec<u32>),
}

impl HopIndices {
    fn empty(max_hops: usize) -> Self {
        if max_hops <= INLINE_MAX_HOPS {
            Self::Inline {
                len: 0,
                edges: [0; INLINE_MAX_HOPS],
            }
        } else {
            Self::Heap(Vec::with_capacity(max_hops))
        }
    }

    #[inline]
    fn pushed(&self, edge_idx: u32) -> Self {
        match self {
            Self::Inline { len, edges } => {
                let usize_len = *len as usize;
                if usize_len < INLINE_MAX_HOPS {
                    let mut next = *edges;
                    next[usize_len] = edge_idx;
                    Self::Inline {
                        len: len + 1,
                        edges: next,
                    }
                } else {
                    let mut vec = Vec::with_capacity(usize_len + 1);
                    vec.extend_from_slice(&edges[..usize_len]);
                    vec.push(edge_idx);
                    Self::Heap(vec)
                }
            }
            Self::Heap(vec) => {
                let mut next = Vec::with_capacity(vec.len() + 1);
                next.extend_from_slice(vec);
                next.push(edge_idx);
                Self::Heap(next)
            }
        }
    }

    fn as_slice(&self) -> &[u32] {
        match self {
            Self::Inline { len, edges } => &edges[..(*len as usize)],
            Self::Heap(vec) => vec.as_slice(),
        }
    }
}

#[derive(Clone, Debug)]
struct SearchState {
    node: u32,
    estimated_output: i128,
    seq: u64,
    visited: VisitedSet,
    hops: HopIndices,
}

impl SearchState {
    #[inline]
    fn better_cmp(a: &Self, b: &Self) -> Ordering {
        b.estimated_output
            .cmp(&a.estimated_output)
            .then_with(|| a.seq.cmp(&b.seq))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RankedCandidate {
    estimated_output: i128,
    hop_len: usize,
    seq: u64,
    hops: HopIndices,
}

impl Ord for RankedCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.estimated_output
            .cmp(&other.estimated_output)
            .then_with(|| other.hop_len.cmp(&self.hop_len))
            .then_with(|| other.seq.cmp(&self.seq))
    }
}

impl PartialOrd for RankedCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[inline]
fn checked_mul_div(value: i128, numer: i128, denom: i128) -> Option<i128> {
    if value < 0 || numer < 0 || denom <= 0 {
        return None;
    }
    if value == 0 || numer == 0 {
        return Some(0);
    }
    let q = value.checked_div(denom)?;
    let r = value.checked_rem(denom)?;
    let head = q.checked_mul(numer)?;
    let tail = r.checked_mul(numer)?.checked_div(denom)?;
    head.checked_add(tail)
}

/// N-hop pathfinder with safety bounds
pub struct Pathfinder {
    config: PathfinderConfig,
}

impl Pathfinder {
    pub fn new(config: PathfinderConfig) -> Self {
        Self { config }
    }

    /// Compute estimated output after traversing a single edge using checked `i128`
    /// arithmetic over `edge.price` (destination units per source unit) and `edge.fee_bps`.
    ///
    /// Edges with omitted/non-positive `price` (e.g. `0.0` in topology-only fixtures)
    /// fall back to unit parity (`1.0`) while still applying `fee_bps`.
    pub(crate) fn estimate_hop_output(amount_in: i128, price: f64, fee_bps: u32) -> Option<i128> {
        if amount_in < 0 {
            return None;
        }
        if amount_in == 0 || fee_bps >= BPS_DENOM as u32 {
            return Some(0);
        }

        let fee_numer = BPS_DENOM.checked_sub(i128::from(fee_bps))?;
        let after_fee = checked_mul_div(amount_in, fee_numer, BPS_DENOM)?;

        if !price.is_finite() || price <= 0.0 {
            return Some(after_fee);
        }

        let max_scaled = i128::MAX / PRICE_SCALE;
        let scaled_f64 = (price * (PRICE_SCALE as f64)).round();
        if !scaled_f64.is_finite() || scaled_f64 > (max_scaled as f64) {
            return None;
        }
        if scaled_f64 <= 0.0 {
            return Some(0);
        }

        let price_scaled = scaled_f64 as i128;
        checked_mul_div(after_fee, price_scaled, PRICE_SCALE)
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

        let Some(graph) = self.build_indexed_graph(edges, from, to, policy) else {
            return Err(RoutingError::NoRoute(from.to_string(), to.to_string()));
        };

        let raw_paths = self.bfs_paths(
            edges,
            &graph,
            amount_in,
            policy.max_hops,
            policy.max_candidate_paths,
        )?;

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

    fn build_indexed_graph<'a>(
        &self,
        edges: &'a [LiquidityEdge],
        from: &'a str,
        to: &'a str,
        policy: &RoutingPolicy,
    ) -> Option<IndexedGraph> {
        const FROM_ID: u32 = 0;
        const TO_ID: u32 = 1;

        let mut asset_ids: HashMap<&'a str, u32> = HashMap::new();
        asset_ids.insert(from, FROM_ID);
        asset_ids.insert(to, TO_ID);
        let mut adj: Vec<Vec<IndexedEdge>> = vec![Vec::new(), Vec::new()];
        let mut has_incoming_to_dest = false;

        for (idx, edge) in edges.iter().enumerate() {
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

            if !policy.is_asset_allowed(&edge.to) {
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

            let Ok(edge_idx) = u32::try_from(idx) else {
                continue;
            };

            let from_id = *asset_ids.entry(edge.from.as_str()).or_insert_with(|| {
                let id = adj.len() as u32;
                adj.push(Vec::new());
                id
            });
            let to_id = *asset_ids.entry(edge.to.as_str()).or_insert_with(|| {
                let id = adj.len() as u32;
                adj.push(Vec::new());
                id
            });

            if to_id == TO_ID {
                has_incoming_to_dest = true;
            }

            adj[from_id as usize].push(IndexedEdge { edge_idx, to_id });
        }

        if adj[FROM_ID as usize].is_empty() || !has_incoming_to_dest {
            return None;
        }

        Some(IndexedGraph {
            num_nodes: adj.len(),
            adj,
        })
    }

    fn bfs_paths(
        &self,
        edges: &[LiquidityEdge],
        graph: &IndexedGraph,
        amount_in: i128,
        max_hops: usize,
        max_candidate_paths: usize,
    ) -> Result<Vec<SwapPath>> {
        const FROM_ID: u32 = 0;
        const TO_ID: u32 = 1;

        if max_hops == 0 || max_candidate_paths == 0 {
            return Ok(Vec::new());
        }

        let per_node_cap = (max_candidate_paths / 16).clamp(8, 32);
        let global_beam_cap = max_candidate_paths.saturating_mul(8).max(256);

        let mut top_paths: BinaryHeap<Reverse<RankedCandidate>> =
            BinaryHeap::with_capacity(max_candidate_paths.min(1024));
        let mut seq_counter: u64 = 0;

        let mut frontier = vec![SearchState {
            node: FROM_ID,
            estimated_output: amount_in,
            seq: 0,
            visited: VisitedSet::new(graph.num_nodes, FROM_ID),
            hops: HopIndices::empty(max_hops),
        }];

        let mut per_node_buckets: Vec<Vec<SearchState>> =
            (0..graph.num_nodes).map(|_| Vec::new()).collect();
        let mut touched_nodes: Vec<usize> = Vec::with_capacity(graph.num_nodes);

        for depth in 0..max_hops {
            let next_depth = depth + 1;
            let is_last_hop = next_depth == max_hops;

            for state in &frontier {
                for ie in &graph.adj[state.node as usize] {
                    if is_last_hop && ie.to_id != TO_ID {
                        continue;
                    }

                    if state.visited.contains(ie.to_id) {
                        continue;
                    }

                    let edge = &edges[ie.edge_idx as usize];
                    let Some(estimated_after_hop) =
                        Self::estimate_hop_output(state.estimated_output, edge.price, edge.fee_bps)
                    else {
                        continue;
                    };

                    seq_counter += 1;
                    let next_hops = state.hops.pushed(ie.edge_idx);

                    if ie.to_id == TO_ID {
                        let candidate = RankedCandidate {
                            estimated_output: estimated_after_hop,
                            hop_len: next_depth,
                            seq: seq_counter,
                            hops: next_hops,
                        };
                        if top_paths.len() < max_candidate_paths {
                            top_paths.push(Reverse(candidate));
                        } else if let Some(worst) = top_paths.peek() {
                            if candidate > worst.0 {
                                top_paths.pop();
                                top_paths.push(Reverse(candidate));
                            }
                        }
                    } else if !is_last_hop {
                        let bucket_idx = ie.to_id as usize;
                        if per_node_buckets[bucket_idx].is_empty() {
                            touched_nodes.push(bucket_idx);
                        }
                        per_node_buckets[bucket_idx].push(SearchState {
                            node: ie.to_id,
                            estimated_output: estimated_after_hop,
                            seq: seq_counter,
                            visited: state.visited.with(ie.to_id),
                            hops: next_hops,
                        });
                    }
                }
            }

            if is_last_hop {
                break;
            }

            frontier.clear();
            for &node_idx in &touched_nodes {
                let bucket = &mut per_node_buckets[node_idx];
                if bucket.len() > per_node_cap {
                    bucket.select_nth_unstable_by(per_node_cap - 1, SearchState::better_cmp);
                    bucket.truncate(per_node_cap);
                }
                frontier.append(bucket);
            }
            touched_nodes.clear();

            if frontier.is_empty() {
                break;
            }

            if frontier.len() > global_beam_cap {
                frontier.select_nth_unstable_by(global_beam_cap - 1, SearchState::better_cmp);
                frontier.truncate(global_beam_cap);
            }
            frontier.sort_unstable_by_key(|s| s.seq);
        }

        let mut winners: Vec<RankedCandidate> = top_paths.into_iter().map(|r| r.0).collect();
        winners.sort_by(|a, b| b.cmp(a));

        let paths = winners
            .into_iter()
            .map(|cand| {
                let hops = cand
                    .hops
                    .as_slice()
                    .iter()
                    .map(|&edge_idx| {
                        let edge = &edges[edge_idx as usize];
                        PathHop {
                            source_asset: edge.from.clone(),
                            destination_asset: edge.to.clone(),
                            venue_type: edge.venue_type.clone(),
                            venue_ref: edge.venue_ref.clone(),
                            price: edge.price,
                            fee_bps: edge.fee_bps,
                            provider: edge.provider.clone(),
                            bridge: edge.bridge.clone(),
                        }
                    })
                    .collect();
                SwapPath {
                    hops,
                    estimated_output: cand.estimated_output,
                }
            })
            .collect();

        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::FixtureBuilder;
    use crate::optimizer::HybridOptimizer;
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
            provider: None,
            bridge: None,
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
            provider: None,
            bridge: None,
        }]
    }

    #[test]
    fn test_thin_book_no_panic() {
        let pathfinder = Pathfinder::new(PathfinderConfig {
            min_liquidity_threshold: 1,
        });
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
                provider: None,
                bridge: None,
            },
            LiquidityEdge {
                from: "B".to_string(),
                to: "C".to_string(),
                venue_type: "sdex".to_string(),
                venue_ref: "sdex:2".to_string(),
                liquidity: 10_000_000,
                price: 1.0,
                fee_bps: 0,
                provider: None,
                bridge: None,
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

    #[test]
    fn test_estimated_output_reflects_edge_price_and_fee() {
        let pathfinder = Pathfinder::new(PathfinderConfig::default());
        let policy = test_policy();

        let edges = vec![
            // Direct AMM pool: price 0.10, fee 30 bps -> 100_000_000 * 0.997 * 0.10 = 9_970_000
            LiquidityEdge {
                from: "XLM".to_string(),
                to: "USDC".to_string(),
                venue_type: "amm".to_string(),
                venue_ref: "pool_low".to_string(),
                liquidity: 50_000_000,
                price: 0.10,
                fee_bps: 30,
                ..Default::default()
            },
            // Better direct SDEX offer: price 0.12, fee 10 bps -> 100_000_000 * 0.999 * 0.12 = 11_988_000
            LiquidityEdge {
                from: "XLM".to_string(),
                to: "USDC".to_string(),
                venue_type: "sdex".to_string(),
                venue_ref: "offer_best".to_string(),
                liquidity: 50_000_000,
                price: 0.12,
                fee_bps: 10,
                ..Default::default()
            },
            // Two-hop path: XLM -> EURC (0.09, 0 bps) -> USDC (1.20, 25 bps)
            // Hop 1: 100_000_000 * 0.09 = 9_000_000
            // Hop 2: 9_000_000 * 0.9975 = 8_977_500; * 1.20 = 10_773_000
            LiquidityEdge {
                from: "XLM".to_string(),
                to: "EURC".to_string(),
                venue_type: "sdex".to_string(),
                venue_ref: "offer_xlm_eurc".to_string(),
                liquidity: 50_000_000,
                price: 0.09,
                fee_bps: 0,
                ..Default::default()
            },
            LiquidityEdge {
                from: "EURC".to_string(),
                to: "USDC".to_string(),
                venue_type: "amm".to_string(),
                venue_ref: "pool_eurc_usdc".to_string(),
                liquidity: 50_000_000,
                price: 1.20,
                fee_bps: 25,
                ..Default::default()
            },
        ];

        let paths = pathfinder
            .find_paths("XLM", "USDC", &edges, 100_000_000, &policy)
            .expect("routes should be discovered");

        assert_eq!(paths.len(), 3);
        assert_eq!(paths[0].hops[0].venue_ref, "offer_best");
        assert_eq!(paths[0].estimated_output, 11_988_000);
        assert_eq!(paths[1].hops.len(), 2);
        assert_eq!(paths[1].estimated_output, 10_773_000);
        assert_eq!(paths[2].hops[0].venue_ref, "pool_low");
        assert_eq!(paths[2].estimated_output, 9_970_000);
    }

    #[test]
    fn test_path_enumeration_bounded_by_max_candidate_paths() {
        let pathfinder = Pathfinder::new(PathfinderConfig {
            min_liquidity_threshold: 1_000,
        });

        let mut edges = Vec::new();
        for idx in 1..=40 {
            let mid = format!("MID_{idx}");
            edges.push(LiquidityEdge {
                from: "XLM".to_string(),
                to: mid.clone(),
                venue_type: "amm".to_string(),
                venue_ref: format!("in_{idx}"),
                liquidity: 10_000_000,
                price: 1.0,
                fee_bps: 10,
                ..Default::default()
            });
            edges.push(LiquidityEdge {
                from: mid,
                to: "USDC".to_string(),
                venue_type: "sdex".to_string(),
                venue_ref: format!("out_{idx}"),
                liquidity: 10_000_000,
                price: 1.0 + (idx as f64) * 0.01,
                fee_bps: 0,
                ..Default::default()
            });
        }

        let capped_policy = RoutingPolicy::default().with_max_candidate_paths(5);
        let paths = pathfinder
            .find_paths("XLM", "USDC", &edges, 100_000_000, &capped_policy)
            .expect("bounded pathfinding must succeed");

        assert_eq!(
            paths.len(),
            5,
            "enumeration must respect max_candidate_paths"
        );
        assert_eq!(paths[0].hops[1].venue_ref, "out_40");
        assert_eq!(paths[1].hops[1].venue_ref, "out_39");
        assert_eq!(paths[4].hops[1].venue_ref, "out_36");
        for win in paths.windows(2) {
            assert!(win[0].estimated_output >= win[1].estimated_output);
        }
    }

    #[test]
    fn test_existing_fixtures_preserve_best_route_under_default_policy() {
        let optimizer = HybridOptimizer::default();
        let policy = RoutingPolicy::default();

        // 1. Minimal market fixture: SDEX offer (20 bps simulated impact) beats AMM (30 bps)
        let minimal_edges = FixtureBuilder::minimal_market().build_edges();
        let usdc_key = "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";
        let min_diag = optimizer
            .find_optimal_routes("native", usdc_key, &minimal_edges, 100_000_000, &policy)
            .expect("minimal market optimal route");
        assert_eq!(min_diag.selected_path.hops.len(), 1);
        assert_eq!(min_diag.selected_path.hops[0].venue_ref, "sdex:1001");

        // 2. Multi-hop market fixture: direct 1-hop AMM shortcut beats 2-hop alternatives
        let multi_edges = FixtureBuilder::multi_hop_market().build_edges();
        let eurc_key = "EURC:GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y2IEMFDVXBSDP6SJY4ITNPP";
        let multi_diag = optimizer
            .find_optimal_routes("native", eurc_key, &multi_edges, 100_000_000, &policy)
            .expect("multi-hop market optimal route");
        assert_eq!(multi_diag.selected_path.hops.len(), 1);
        assert_eq!(
            multi_diag.selected_path.hops[0].venue_ref,
            "CAMMPOOL2XLMEURC000000000000000000000000000000000000000002"
        );

        // 3. Representative JSON graph fixture: XLM -> USDC (pool1) -> BTC (offer2)
        let graph_fixture: Vec<LiquidityEdge> =
            serde_json::from_str(include_str!("../fixtures/graph_fixture.json"))
                .expect("valid graph_fixture.json");
        let fix_diag = optimizer
            .find_optimal_routes("XLM", "BTC", &graph_fixture, 100_000_000, &policy)
            .expect("graph_fixture optimal route");
        assert_eq!(fix_diag.selected_path.hops.len(), 2);
        assert_eq!(fix_diag.selected_path.hops[0].venue_ref, "pool1");
        assert_eq!(fix_diag.selected_path.hops[1].venue_ref, "offer2");
    }

    #[test]
    fn test_checked_hop_output_overflow_prunes_safely() {
        assert!(Pathfinder::estimate_hop_output(i128::MAX / 2, 4.0, 0).is_none());
        assert_eq!(
            Pathfinder::estimate_hop_output(100_000_000, 1.0, 10_000),
            Some(0)
        );
    }
}
