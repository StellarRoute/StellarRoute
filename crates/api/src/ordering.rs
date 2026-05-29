//! API-level deterministic ordering for routes endpoint responses
//!
//! Guarantees deterministic ordering of routes endpoint results to reduce
//! client-side diff churn and flaky tests.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

use crate::models::{RouteCandidate, RouteHop};

/// Configuration for deterministic route ordering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderingConfig {
    /// Primary sort key
    pub primary_key: SortKey,
    /// Secondary sort key (used when primary is equal)
    pub secondary_key: SortKey,
    /// Tertiary sort key (used when primary and secondary are equal)
    pub tertiary_key: SortKey,
    /// Sort direction for primary key
    pub primary_direction: SortDirection,
    /// Sort direction for secondary key
    pub secondary_direction: SortDirection,
    /// Sort direction for tertiary key
    pub tertiary_direction: SortDirection,
}

impl Default for OrderingConfig {
    fn default() -> Self {
        Self {
            primary_key: SortKey::Score,
            secondary_key: SortKey::EstimatedOutput,
            tertiary_key: SortKey::HopCount,
            primary_direction: SortDirection::Descending,
            secondary_direction: SortDirection::Descending,
            tertiary_direction: SortDirection::Ascending,
        }
    }
}

/// Available sort keys for route ordering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortKey {
    /// Route score (higher is better)
    Score,
    /// Estimated output amount
    EstimatedOutput,
    /// Price impact in basis points
    ImpactBps,
    /// Number of hops in the route
    HopCount,
    /// First venue in the path
    FirstVenue,
    /// Policy used for optimization
    PolicyUsed,
}

/// Sort direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl SortDirection {
    fn apply(&self, ordering: Ordering) -> Ordering {
        match self {
            SortDirection::Ascending => ordering,
            SortDirection::Descending => ordering.reverse(),
        }
    }
}

/// Sort routes deterministically according to configuration
pub fn sort_routes(routes: &mut [RouteCandidate], config: &OrderingConfig) {
    routes.sort_by(|a, b| compare_routes(a, b, config));
}

/// Compare two routes for deterministic ordering
fn compare_routes(a: &RouteCandidate, b: &RouteCandidate, config: &OrderingConfig) -> Ordering {
    let primary = compare_by_key(a, b, config.primary_key);
    if primary != Ordering::Equal {
        return config.primary_direction.apply(primary);
    }

    let secondary = compare_by_key(a, b, config.secondary_key);
    if secondary != Ordering::Equal {
        return config.secondary_direction.apply(secondary);
    }

    let tertiary = compare_by_key(a, b, config.tertiary_key);
    config.tertiary_direction.apply(tertiary)
}

/// Compare two routes by a specific key
fn compare_by_key(a: &RouteCandidate, b: &RouteCandidate, key: SortKey) -> Ordering {
    match key {
        SortKey::Score => {
            // Use total ordering for floats
            a.score.partial_cmp(&b.score).unwrap_or(Ordering::Equal)
        }
        SortKey::EstimatedOutput => {
            // Parse as f64 for comparison, handle parse errors gracefully
            let a_val = parse_output(&a.estimated_output);
            let b_val = parse_output(&b.estimated_output);
            a_val.partial_cmp(&b_val).unwrap_or(Ordering::Equal)
        }
        SortKey::ImpactBps => a.impact_bps.cmp(&b.impact_bps),
        SortKey::HopCount => a.path.len().cmp(&b.path.len()),
        SortKey::FirstVenue => {
            let a_venue = a.path.first().map(|h| &h.source).unwrap_or(&String::new());
            let b_venue = b.path.first().map(|h| &h.source).unwrap_or(&String::new());
            a_venue.cmp(b_venue)
        }
        SortKey::PolicyUsed => a.policy_used.cmp(&b.policy_used),
    }
}

/// Parse estimated output string to f64
fn parse_output(s: &str) -> f64 {
    s.parse().unwrap_or(0.0)
}

/// Tie-breaker logic for routes with identical scores
///
/// When routes have the same score, we use a deterministic tie-breaker:
/// 1. Prefer fewer hops (lower complexity)
/// 2. Prefer lower impact (better execution)
/// 3. Prefer lexicographically smaller first venue (deterministic)
pub fn tie_break(a: &RouteCandidate, b: &RouteCandidate) -> Ordering {
    // Fewer hops is better
    match a.path.len().cmp(&b.path.len()) {
        Ordering::Equal => {}
        other => return other,
    }

    // Lower impact is better
    match a.impact_bps.cmp(&b.impact_bps) {
        Ordering::Equal => {}
        other => return other,
    }

    // Lexicographic order of first venue for determinism
    let a_venue = a.path.first().map(|h| &h.source).unwrap_or(&String::new());
    let b_venue = b.path.first().map(|h| &h.source).unwrap_or(&String::new());
    a_venue.cmp(b_venue)
}

/// Normalize route for consistent comparison
pub fn normalize_route(route: &mut RouteCandidate) {
    // Ensure path hops are in consistent order (they should be already)
    // Normalize numeric strings to fixed precision
    route.estimated_output = normalize_numeric_string(&route.estimated_output);
    for hop in &mut route.path {
        hop.price = normalize_numeric_string(&hop.price);
        hop.amount_out_of_hop = normalize_numeric_string(&hop.amount_out_of_hop);
    }
}

/// Normalize a numeric string to 7 decimal places
fn normalize_numeric_string(s: &str) -> String {
    let val: f64 = s.parse().unwrap_or(0.0);
    format!("{:.7}", val)
}

/// Documentation for stable sort keys
pub const SORT_KEY_DOCUMENTATION: &str = r#"
# Route Ordering Documentation

Routes are sorted deterministically using the following keys:

1. **Score** (primary, descending): Higher scores indicate better routes
2. **Estimated Output** (secondary, descending): More output is better
3. **Hop Count** (tertiary, ascending): Fewer hops is simpler

## Tie-Breaker Logic

When routes have identical scores:
1. Prefer fewer hops (lower complexity)
2. Prefer lower price impact (better execution)
3. Prefer lexicographically smaller first venue (deterministic)

## Backward Compatibility

The ordering change is backward compatible:
- Clients that don't depend on order are unaffected
- Clients that do depend on order get consistent results
- The semantic meaning of routes is unchanged
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AssetInfo, RouteHop};

    fn make_route(score: f64, output: &str, hops: usize, impact: u32) -> RouteCandidate {
        RouteCandidate {
            estimated_output: output.to_string(),
            impact_bps: impact,
            score,
            policy_used: "production".to_string(),
            path: (0..hops)
                .map(|_| RouteHop {
                    from_asset: AssetInfo::native(),
                    to_asset: AssetInfo::native(),
                    price: "1.0000000".to_string(),
                    amount_out_of_hop: "1.0000000".to_string(),
                    fee_bps: 0,
                    source: "sdex".to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn sort_routes_orders_by_score_descending() {
        let mut routes = vec![
            make_route(0.5, "100", 1, 10),
            make_route(0.9, "100", 1, 10),
            make_route(0.7, "100", 1, 10),
        ];
        sort_routes(&mut routes, &OrderingConfig::default());
        assert_eq!(routes[0].score, 0.9);
        assert_eq!(routes[1].score, 0.7);
        assert_eq!(routes[2].score, 0.5);
    }

    #[test]
    fn sort_routes_uses_secondary_key_on_tie() {
        let mut routes = vec![
            make_route(0.9, "100", 1, 10),
            make_route(0.9, "200", 1, 10),
            make_route(0.9, "150", 1, 10),
        ];
        sort_routes(&mut routes, &OrderingConfig::default());
        assert_eq!(routes[0].estimated_output, "200");
        assert_eq!(routes[1].estimated_output, "150");
        assert_eq!(routes[2].estimated_output, "100");
    }

    #[test]
    fn tie_break_prefers_fewer_hops() {
        let a = make_route(0.9, "100", 1, 10);
        let b = make_route(0.9, "100", 2, 10);
        assert_eq!(tie_break(&a, &b), Ordering::Less);
    }

    #[test]
    fn tie_break_prefers_lower_impact() {
        let a = make_route(0.9, "100", 1, 5);
        let b = make_route(0.9, "100", 1, 10);
        assert_eq!(tie_break(&a, &b), Ordering::Less);
    }

    #[test]
    fn normalize_numeric_string_handles_precision() {
        assert_eq!(normalize_numeric_string("1.23456789"), "1.2345679");
        assert_eq!(normalize_numeric_string("1.5"), "1.5000000");
    }
}
