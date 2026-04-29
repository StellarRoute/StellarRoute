/**
 * Hierarchical cache invalidation graph
 *
 * Maps liquidity updates to impacted cache keys at pair and route levels.
 * Enables efficient selective invalidation instead of full cache clears.
 *
 * Architecture:
 *   - PairInvalidationGraph: tracks pair → affected pairs + routes
 *   - When pair (A, B) updates, invalidate: quotes for (A,B), routes containing (A,B),
 *     and parent pairs that use (A,B) as intermediate
 *   - Fallback: if graph insertion fails, fall back to full cache clear
 */

use dashmap::DashMap;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Represents a trading pair (asset_a, asset_b)
#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct Pair {
    pub base: String,
    pub quote: String,
}

impl Pair {
    pub fn new(base: impl Into<String>, quote: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            quote: quote.into(),
        }
    }

    pub fn reverse(&self) -> Self {
        Pair::new(self.quote.clone(), self.base.clone())
    }

    pub fn canonical(&self) -> Self {
        if self.base < self.quote {
            self.clone()
        } else {
            self.reverse()
        }
    }
}

/// Cache key for a quote or route
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum CacheKey {
    /// Quote cache key: pair + amount
    Quote {
        base: String,
        quote: String,
        amount: u64,
    },
    /// Route cache key: pair + amount + route_hash
    Route {
        base: String,
        quote: String,
        amount: u64,
        route_hash: String,
    },
    /// Orderbook cache key: pair
    Orderbook {
        base: String,
        quote: String,
    },
}

impl CacheKey {
    pub fn pair(&self) -> Pair {
        match self {
            CacheKey::Quote { base, quote, .. } => Pair::new(base.clone(), quote.clone()),
            CacheKey::Route { base, quote, .. } => Pair::new(base.clone(), quote.clone()),
            CacheKey::Orderbook { base, quote } => Pair::new(base.clone(), quote.clone()),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            CacheKey::Quote { base, quote, amount } => {
                format!("quote:{}:{}:{}", base, quote, amount)
            }
            CacheKey::Route {
                base,
                quote,
                amount,
                route_hash,
            } => {
                format!("route:{}:{}:{}:{}", base, quote, amount, route_hash)
            }
            CacheKey::Orderbook { base, quote } => {
                format!("orderbook:{}:{}", base, quote)
            }
        }
    }
}

/// Dependency graph tracking:
///   - pair → affected quote cache keys
///   - pair → affected route cache keys
///   - pair → child pairs (pairs that depend on this pair)
///   - pair → parent pairs (pairs that contain this pair as intermediate)
pub struct PairInvalidationGraph {
    /// Maps pair → set of quote cache keys
    pair_to_quotes: Arc<DashMap<Pair, HashSet<String>>>,

    /// Maps pair → set of route cache keys
    pair_to_routes: Arc<DashMap<Pair, HashSet<String>>>,

    /// Maps pair → child pairs (pairs that depend on this pair)
    /// Example: If route contains (XLM, USDC) → (USDC, EURC), then (XLM, EURC) depends on both
    pair_to_children: Arc<DashMap<Pair, HashSet<Pair>>>,

    /// Maps pair → parent pairs (pairs that have this pair as intermediate hop)
    /// Example: (XLM, EURC) is parent of (XLM, USDC) and (USDC, EURC) if route is XLM→USDC→EURC
    pair_to_parents: Arc<DashMap<Pair, HashSet<Pair>>>,
}

impl PairInvalidationGraph {
    pub fn new() -> Self {
        Self {
            pair_to_quotes: Arc::new(DashMap::new()),
            pair_to_routes: Arc::new(DashMap::new()),
            pair_to_children: Arc::new(DashMap::new()),
            pair_to_parents: Arc::new(DashMap::new()),
        }
    }

    /// Register a quote cache key for a pair
    pub fn register_quote(&self, pair: &Pair, cache_key: impl Into<String>) {
        let key = cache_key.into();
        self.pair_to_quotes
            .entry(pair.clone())
            .or_insert_with(HashSet::new)
            .insert(key);
    }

    /// Register a route cache key for a pair
    pub fn register_route(&self, pair: &Pair, cache_key: impl Into<String>) {
        let key = cache_key.into();
        self.pair_to_routes
            .entry(pair.clone())
            .or_insert_with(HashSet::new)
            .insert(key);
    }

    /// Register a dependency: route contains hops [pair1, pair2, pair3]
    /// Creates graph edges:
    ///   - pair2 and pair3 are children of pair1
    ///   - pair1 and pair3 are parents of pair2 (since it's in the middle)
    ///   - pair1 and pair2 are parents of pair3
    pub fn register_route_dependency(&self, hops: &[Pair]) {
        if hops.len() < 2 {
            return; // Single asset, no dependencies
        }

        for (i, hop) in hops.iter().enumerate() {
            for (j, other) in hops.iter().enumerate() {
                if i != j {
                    if j > i {
                        // downstream: other depends on hop
                        self.pair_to_children
                            .entry(hop.clone())
                            .or_insert_with(HashSet::new)
                            .insert(other.clone());
                    } else {
                        // upstream: other is a parent of hop
                        self.pair_to_parents
                            .entry(hop.clone())
                            .or_insert_with(HashSet::new)
                            .insert(other.clone());
                    }
                }
            }
        }
    }

    /// Get all affected cache keys when pair is invalidated
    ///
    /// Returns:
    ///   - Direct: quotes + routes for this pair + orderbook
    ///   - Cascading: quotes + routes for dependent child pairs
    ///   - Cascading: quotes + routes for parent pairs
    pub fn get_affected_keys(&self, pair: &Pair) -> HashSet<String> {
        let mut affected = HashSet::new();

        // 1. Direct impact: this pair's cache keys
        if let Some(quotes) = self.pair_to_quotes.get(pair) {
            affected.extend(quotes.iter().cloned());
        }
        if let Some(routes) = self.pair_to_routes.get(pair) {
            affected.extend(routes.iter().cloned());
        }
        affected.insert(format!("orderbook:{}:{}", pair.base, pair.quote));

        // 2. Cascading: child pairs (pairs that depend on this pair)
        if let Some(children) = self.pair_to_children.get(pair) {
            for child in children.iter() {
                if let Some(quotes) = self.pair_to_quotes.get(child) {
                    affected.extend(quotes.iter().cloned());
                }
                if let Some(routes) = self.pair_to_routes.get(child) {
                    affected.extend(routes.iter().cloned());
                }
            }
        }

        // 3. Cascading: parent pairs (pairs that contain this pair as intermediate)
        if let Some(parents) = self.pair_to_parents.get(pair) {
            for parent in parents.iter() {
                if let Some(quotes) = self.pair_to_quotes.get(parent) {
                    affected.extend(quotes.iter().cloned());
                }
                if let Some(routes) = self.pair_to_routes.get(parent) {
                    affected.extend(routes.iter().cloned());
                }
            }
        }

        affected
    }

    /// Clear all entries for a pair (cleanup)
    pub fn clear_pair(&self, pair: &Pair) {
        self.pair_to_quotes.remove(pair);
        self.pair_to_routes.remove(pair);
        self.pair_to_children.remove(pair);
        self.pair_to_parents.remove(pair);
    }

    /// Get current graph size (for monitoring)
    pub fn size(&self) -> GraphSize {
        GraphSize {
            pair_quote_entries: self.pair_to_quotes.len(),
            pair_route_entries: self.pair_to_routes.len(),
            total_quote_keys: self
                .pair_to_quotes
                .iter()
                .map(|r| r.value().len())
                .sum(),
            total_route_keys: self
                .pair_to_routes
                .iter()
                .map(|r| r.value().len())
                .sum(),
            dependency_edges: self.pair_to_children.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GraphSize {
    pub pair_quote_entries: usize,
    pub pair_route_entries: usize,
    pub total_quote_keys: usize,
    pub total_route_keys: usize,
    pub dependency_edges: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_query() {
        let graph = PairInvalidationGraph::new();
        let pair = Pair::new("XLM", "USDC");

        graph.register_quote(&pair, "quote:XLM:USDC:1000");
        graph.register_route(&pair, "route:XLM:USDC:1000:abc123");

        let affected = graph.get_affected_keys(&pair);
        assert!(affected.contains("quote:XLM:USDC:1000"));
        assert!(affected.contains("route:XLM:USDC:1000:abc123"));
        assert!(affected.contains("orderbook:XLM:USDC"));
    }

    #[test]
    fn test_route_dependency_cascade() {
        let graph = PairInvalidationGraph::new();

        // Route: XLM → USDC → EURC
        let pair1 = Pair::new("XLM", "USDC");
        let pair2 = Pair::new("USDC", "EURC");
        let route = Pair::new("XLM", "EURC"); // parent route

        graph.register_quote(&pair1, "quote:XLM:USDC:1000");
        graph.register_quote(&pair2, "quote:USDC:EURC:500");
        graph.register_quote(&route, "quote:XLM:EURC:1000");

        graph.register_route_dependency(&[pair1.clone(), pair2.clone()]);

        // Invalidate XLM:USDC
        let affected = graph.get_affected_keys(&pair1);

        // Should include pair2's quotes (cascading)
        assert!(affected.contains("quote:USDC:EURC:500"));
    }

    #[test]
    fn test_clear_pair() {
        let graph = PairInvalidationGraph::new();
        let pair = Pair::new("XLM", "USDC");

        graph.register_quote(&pair, "quote:XLM:USDC:1000");
        assert_eq!(graph.get_affected_keys(&pair).len(), 2); // quote + orderbook

        graph.clear_pair(&pair);
        assert_eq!(graph.get_affected_keys(&pair).len(), 1); // only orderbook
    }

    #[test]
    fn test_pair_canonical() {
        let pair1 = Pair::new("XLM", "USDC");
        let pair2 = Pair::new("USDC", "XLM");

        assert_eq!(pair1.canonical(), pair2.canonical());
    }
}
