//! Integration tests for deterministic route ordering (Issue #433)
//!
//! These tests verify that routes endpoint responses have deterministic ordering
//! to reduce client-side diff churn and flaky tests.

use stellarroute_api::models::{AssetInfo, RouteCandidate, RouteHop};
use stellarroute_api::ordering::{sort_routes, tie_break, OrderingConfig, SortDirection, SortKey};

fn make_route(score: f64, output: &str, hops: usize, impact: u32, venue: &str) -> RouteCandidate {
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
                source: venue.to_string(),
            })
            .collect(),
    }
}

#[test]
fn default_config_orders_by_score_descending() {
    let config = OrderingConfig::default();
    assert_eq!(config.primary_key, SortKey::Score);
    assert_eq!(config.primary_direction, SortDirection::Descending);
}

#[test]
fn routes_are_sorted_by_score_descending() {
    let mut routes = vec![
        make_route(0.5, "100", 1, 10, "sdex"),
        make_route(0.9, "100", 1, 10, "sdex"),
        make_route(0.7, "100", 1, 10, "sdex"),
    ];

    sort_routes(&mut routes, &OrderingConfig::default());

    assert_eq!(routes[0].score, 0.9);
    assert_eq!(routes[1].score, 0.7);
    assert_eq!(routes[2].score, 0.5);
}

#[test]
fn secondary_key_used_on_score_tie() {
    let mut routes = vec![
        make_route(0.9, "100", 1, 10, "sdex"),
        make_route(0.9, "200", 1, 10, "sdex"),
        make_route(0.9, "150", 1, 10, "sdex"),
    ];

    sort_routes(&mut routes, &OrderingConfig::default());

    // Secondary key is estimated_output, descending
    assert_eq!(routes[0].estimated_output, "200");
    assert_eq!(routes[1].estimated_output, "150");
    assert_eq!(routes[2].estimated_output, "100");
}

#[test]
fn tertiary_key_used_on_primary_and_secondary_tie() {
    let mut routes = vec![
        make_route(0.9, "100", 3, 10, "sdex"),
        make_route(0.9, "100", 1, 10, "sdex"),
        make_route(0.9, "100", 2, 10, "sdex"),
    ];

    sort_routes(&mut routes, &OrderingConfig::default());

    // Tertiary key is hop_count, ascending
    assert_eq!(routes[0].path.len(), 1);
    assert_eq!(routes[1].path.len(), 2);
    assert_eq!(routes[2].path.len(), 3);
}

#[test]
fn tie_break_prefers_fewer_hops() {
    let a = make_route(0.9, "100", 1, 10, "sdex");
    let b = make_route(0.9, "100", 2, 10, "sdex");

    assert_eq!(tie_break(&a, &b), std::cmp::Ordering::Less);
    assert_eq!(tie_break(&b, &a), std::cmp::Ordering::Greater);
}

#[test]
fn tie_break_prefers_lower_impact() {
    let a = make_route(0.9, "100", 1, 5, "sdex");
    let b = make_route(0.9, "100", 1, 10, "sdex");

    assert_eq!(tie_break(&a, &b), std::cmp::Ordering::Less);
    assert_eq!(tie_break(&b, &a), std::cmp::Ordering::Greater);
}

#[test]
fn tie_break_uses_venue_lexicographic_order() {
    let a = make_route(0.9, "100", 1, 10, "amm:pool-a");
    let b = make_route(0.9, "100", 1, 10, "sdex");

    // "amm:pool-a" < "sdex" lexicographically
    assert_eq!(tie_break(&a, &b), std::cmp::Ordering::Less);
}

#[test]
fn ordering_is_stable_across_multiple_sorts() {
    let routes1 = vec![
        make_route(0.5, "100", 1, 10, "sdex"),
        make_route(0.9, "200", 2, 5, "amm"),
        make_route(0.7, "150", 1, 15, "sdex"),
    ];

    let routes2 = routes1.clone();

    let mut sorted1 = routes1;
    let mut sorted2 = routes2;

    sort_routes(&mut sorted1, &OrderingConfig::default());
    sort_routes(&mut sorted2, &OrderingConfig::default());

    // Multiple sorts should produce identical results
    for (a, b) in sorted1.iter().zip(sorted2.iter()) {
        assert_eq!(a.score, b.score);
        assert_eq!(a.estimated_output, b.estimated_output);
        assert_eq!(a.path.len(), b.path.len());
    }
}

#[test]
fn empty_routes_handle_gracefully() {
    let mut routes: Vec<RouteCandidate> = vec![];
    sort_routes(&mut routes, &OrderingConfig::default());
    assert!(routes.is_empty());
}

#[test]
fn single_route_remains_unchanged() {
    let mut routes = vec![make_route(0.9, "100", 1, 10, "sdex")];
    sort_routes(&mut routes, &OrderingConfig::default());
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].score, 0.9);
}

#[test]
fn custom_ordering_config_works() {
    let config = OrderingConfig {
        primary_key: SortKey::ImpactBps,
        secondary_key: SortKey::Score,
        tertiary_key: SortKey::HopCount,
        primary_direction: SortDirection::Ascending,
        secondary_direction: SortDirection::Descending,
        tertiary_direction: SortDirection::Ascending,
    };

    let mut routes = vec![
        make_route(0.9, "100", 1, 20, "sdex"),
        make_route(0.7, "100", 1, 5, "sdex"),
        make_route(0.8, "100", 1, 10, "sdex"),
    ];

    sort_routes(&mut routes, &config);

    // Sorted by impact ascending
    assert_eq!(routes[0].impact_bps, 5);
    assert_eq!(routes[1].impact_bps, 10);
    assert_eq!(routes[2].impact_bps, 20);
}
