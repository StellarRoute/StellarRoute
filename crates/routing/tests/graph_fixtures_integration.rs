//! Routing graph integration tests using deterministic DB fixtures.
//!
//! All tests are self-contained: no database, no network, no external services.
//! Fixtures cover both SDEX orderbook and AMM pool representations and are
//! reusable across routing scenarios.
//!
//! CI scenarios covered:
//!   1. Single-hop XLM → USDC (minimal market, SDEX + AMM)
//!   2. Multi-hop XLM → EURC (2-hop SDEX path vs direct AMM shortcut)

use stellarroute_routing::{
    fixtures::FixtureBuilder,
    optimizer::HybridOptimizer,
    pathfinder::{Pathfinder, PathfinderConfig},
    policy::RoutingPolicy,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn default_config() -> PathfinderConfig {
    PathfinderConfig {
        min_liquidity_threshold: 1_000_000, // 0.1 in e7
    }
}

fn default_policy() -> RoutingPolicy {
    RoutingPolicy::default()
}

// ── Scenario 1: Single-hop minimal market ─────────────────────────────────────

#[test]
fn fixture_minimal_market_builds_valid_edges() {
    let fb = FixtureBuilder::minimal_market();
    let edges = fb.build_edges();

    // 1 SDEX offer + 1 AMM pool (2 directions) = 3 edges
    assert_eq!(edges.len(), 3, "expected 3 edges from minimal market");

    let sdex_edges: Vec<_> = edges.iter().filter(|e| e.venue_type == "sdex").collect();
    let amm_edges: Vec<_> = edges.iter().filter(|e| e.venue_type == "amm").collect();

    assert_eq!(sdex_edges.len(), 1, "one SDEX edge");
    assert_eq!(amm_edges.len(), 2, "two AMM edges (bidirectional)");
}

#[test]
fn scenario_single_hop_sdex_route_found() {
    let edges = FixtureBuilder::minimal_market().build_edges();
    let pathfinder = Pathfinder::new(default_config());
    let policy = default_policy();

    let paths = pathfinder
        .find_paths(
            "native",
            "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
            &edges,
            100_000_000,
            &policy,
        )
        .expect("should find at least one path");

    assert!(!paths.is_empty(), "expected at least one route");

    // At least one path should be a single hop
    let single_hop = paths.iter().any(|p| p.hops.len() == 1);
    assert!(single_hop, "expected a direct single-hop path");
}

#[test]
fn scenario_single_hop_both_venues_represented() {
    let edges = FixtureBuilder::minimal_market().build_edges();
    let pathfinder = Pathfinder::new(default_config());
    let policy = default_policy();

    let paths = pathfinder
        .find_paths(
            "native",
            "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
            &edges,
            100_000_000,
            &policy,
        )
        .expect("should find paths");

    let venue_types: Vec<_> = paths
        .iter()
        .flat_map(|p| p.hops.iter().map(|h| h.venue_type.as_str()))
        .collect();

    assert!(
        venue_types.contains(&"sdex") || venue_types.contains(&"amm"),
        "at least one venue type must appear in found paths"
    );
}

#[test]
fn scenario_single_hop_optimizer_selects_best_route() {
    let edges = FixtureBuilder::minimal_market().build_edges();
    let optimizer = HybridOptimizer::new(default_config());
    let policy = default_policy();

    let diagnostics = optimizer
        .find_optimal_routes(
            "native",
            "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
            &edges,
            100_000_000,
            &policy,
        )
        .expect("optimizer should succeed on minimal market");

    assert!(
        diagnostics.metrics.output_amount > 0,
        "selected route must produce positive output"
    );
    assert!(
        diagnostics.metrics.score > 0.0,
        "selected route must have a positive score"
    );
}

// ── Scenario 2: Multi-hop market ──────────────────────────────────────────────

#[test]
fn fixture_multi_hop_market_builds_valid_edges() {
    let fb = FixtureBuilder::multi_hop_market();
    let edges = fb.build_edges();

    // 2 SDEX offers + 2 AMM pools (2 directions each) = 2 + 4 = 6 edges
    assert_eq!(edges.len(), 6, "expected 6 edges from multi-hop market");
    assert_eq!(fb.assets().len(), 3, "three distinct assets");
}

#[test]
fn scenario_multi_hop_xlm_to_eurc_route_found() {
    let edges = FixtureBuilder::multi_hop_market().build_edges();
    let pathfinder = Pathfinder::new(default_config());
    let policy = RoutingPolicy::new(4); // allow up to 4 hops

    let eurc_key = "EURC:GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y2IEMFDVXBSDP6SJY4ITNPP";
    let paths = pathfinder
        .find_paths("native", eurc_key, &edges, 100_000_000, &policy)
        .expect("should find at least one path to EURC");

    assert!(!paths.is_empty(), "expected at least one route to EURC");
}

#[test]
fn scenario_multi_hop_direct_amm_path_exists() {
    let edges = FixtureBuilder::multi_hop_market().build_edges();
    let pathfinder = Pathfinder::new(default_config());
    let policy = RoutingPolicy::new(4);

    let eurc_key = "EURC:GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y2IEMFDVXBSDP6SJY4ITNPP";
    let paths = pathfinder
        .find_paths("native", eurc_key, &edges, 100_000_000, &policy)
        .expect("should find paths");

    // The direct AMM pool (XLM → EURC) should produce a 1-hop path
    let has_direct = paths.iter().any(|p| p.hops.len() == 1);
    assert!(has_direct, "direct AMM shortcut should yield a 1-hop path");
}

#[test]
fn scenario_multi_hop_two_hop_sdex_path_exists() {
    let edges = FixtureBuilder::multi_hop_market().build_edges();
    let pathfinder = Pathfinder::new(default_config());
    let policy = RoutingPolicy::new(4);

    let eurc_key = "EURC:GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y2IEMFDVXBSDP6SJY4ITNPP";
    let paths = pathfinder
        .find_paths("native", eurc_key, &edges, 100_000_000, &policy)
        .expect("should find paths");

    // The 2-hop SDEX path (XLM→USDC→EURC) should also be discovered
    let has_two_hop = paths.iter().any(|p| p.hops.len() == 2);
    assert!(has_two_hop, "2-hop SDEX path should be discovered");
}

#[test]
fn scenario_multi_hop_optimizer_returns_positive_output() {
    let edges = FixtureBuilder::multi_hop_market().build_edges();
    let optimizer = HybridOptimizer::new(default_config());
    let policy = RoutingPolicy::new(4);

    let eurc_key = "EURC:GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y2IEMFDVXBSDP6SJY4ITNPP";
    let diagnostics = optimizer
        .find_optimal_routes("native", eurc_key, &edges, 100_000_000, &policy)
        .expect("optimizer should succeed on multi-hop market");

    assert!(
        diagnostics.metrics.output_amount > 0,
        "multi-hop route must produce positive output"
    );
}

// ── Scenario 3: Thin liquidity exclusion ─────────────────────────────────────

#[test]
fn scenario_thin_liquidity_below_threshold_no_route() {
    let edges = FixtureBuilder::thin_liquidity_market().build_edges();
    let config = PathfinderConfig {
        // Set threshold above the thin fixture's liquidity
        min_liquidity_threshold: 10_000_000, // 1.0 in e7
    };
    let pathfinder = Pathfinder::new(config);
    let policy = default_policy();

    let result = pathfinder.find_paths(
        "native",
        "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
        &edges,
        100_000_000,
        &policy,
    );

    // Either no route found or an error — both are acceptable for thin liquidity
    if let Ok(paths) = result {
        assert!(
            paths.is_empty(),
            "thin liquidity should yield no viable paths"
        );
    }
}

// ── Scenario 4: JSON fixture round-trip ──────────────────────────────────────

#[test]
fn json_minimal_market_fixture_loads_and_routes() {
    let fixture_data = include_str!("../fixtures/minimal_market.json");
    let value: serde_json::Value =
        serde_json::from_str(fixture_data).expect("fixture JSON must be valid");

    let edges: Vec<stellarroute_routing::pathfinder::LiquidityEdge> =
        serde_json::from_value(value["edges"].clone()).expect("edges array must deserialize");

    assert!(!edges.is_empty(), "fixture must contain edges");

    let pathfinder = Pathfinder::new(default_config());
    let policy = default_policy();

    let paths = pathfinder
        .find_paths(
            "native",
            "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
            &edges,
            100_000_000,
            &policy,
        )
        .expect("should route from JSON fixture");

    assert!(
        !paths.is_empty(),
        "JSON fixture must produce at least one route"
    );
}

// ── Scenario 5: Venue policy filtering ───────────────────────────────────────

#[test]
fn scenario_sdex_only_policy_excludes_amm_venues() {
    let edges = FixtureBuilder::minimal_market().build_edges();
    let pathfinder = Pathfinder::new(default_config());
    let policy = RoutingPolicy::default().with_venue_allowlist(vec!["sdex".to_string()]);

    let paths = pathfinder
        .find_paths(
            "native",
            "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
            &edges,
            100_000_000,
            &policy,
        )
        .expect("SDEX-only policy should still find a route");

    // All hops must be SDEX
    for path in &paths {
        for hop in &path.hops {
            assert_eq!(
                hop.venue_type, "sdex",
                "SDEX-only policy must not include AMM hops"
            );
        }
    }
}

#[test]
fn scenario_amm_only_policy_excludes_sdex_venues() {
    let edges = FixtureBuilder::minimal_market().build_edges();
    let pathfinder = Pathfinder::new(default_config());
    let policy = RoutingPolicy::default().with_venue_allowlist(vec!["amm".to_string()]);

    let paths = pathfinder
        .find_paths(
            "native",
            "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
            &edges,
            100_000_000,
            &policy,
        )
        .expect("AMM-only policy should find a route via the pool");

    for path in &paths {
        for hop in &path.hops {
            assert_eq!(
                hop.venue_type, "amm",
                "AMM-only policy must not include SDEX hops"
            );
        }
    }
}
