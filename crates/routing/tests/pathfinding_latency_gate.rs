//! CI latency gate for pathfinding on the representative graph fixture.
//!
//! Enforces the M2 readiness budget documented in `docs/readiness/M2_GUIDE.md`:
//! pathfinder initialization plus a single lookup must complete within 100ms.
//!
//! Emits a JSON summary on stdout for trend tracking in CI logs.

use serde::Serialize;
use std::time::Instant;
use stellarroute_routing::{
    pathfinder::{LiquidityEdge, Pathfinder, PathfinderConfig},
    policy::RoutingPolicy,
};

/// M2 pathfinding latency budget (milliseconds).
const LATENCY_BUDGET_MS: u128 = 100;

#[derive(Debug, Serialize)]
struct PathfindingLatencySummary {
    benchmark: &'static str,
    fixture: &'static str,
    init_us: u128,
    lookup_us: u128,
    total_us: u128,
    budget_ms: u128,
    paths_found: usize,
    passed: bool,
}

fn load_graph_fixture_edges() -> Vec<LiquidityEdge> {
    let fixture_data = include_str!("../fixtures/graph_fixture.json");
    serde_json::from_str(fixture_data).expect("graph_fixture.json must deserialize")
}

fn emit_summary(summary: &PathfindingLatencySummary) {
    let json = serde_json::to_string(summary).expect("summary must serialize to JSON");
    println!("PATHFINDING_LATENCY_SUMMARY={json}");
}

#[test]
fn pathfinding_latency_gate_under_100ms_budget() {
    let edges = load_graph_fixture_edges();
    assert!(!edges.is_empty(), "fixture graph must contain edges");

    let config = PathfinderConfig {
        min_liquidity_threshold: 100_000,
    };
    let policy = RoutingPolicy::default();

    let init_start = Instant::now();
    let pathfinder = Pathfinder::new(config);
    let init_us = init_start.elapsed().as_micros();

    let lookup_start = Instant::now();
    let paths = pathfinder
        .find_paths("XLM", "BTC", &edges, 100_000_000, &policy)
        .expect("pathfinding on fixture graph must succeed");
    let lookup_us = lookup_start.elapsed().as_micros();

    let total_us = init_us + lookup_us;
    let passed = total_us / 1_000 < LATENCY_BUDGET_MS;

    let summary = PathfindingLatencySummary {
        benchmark: "pathfinding_latency_gate",
        fixture: "graph_fixture.json",
        init_us,
        lookup_us,
        total_us,
        budget_ms: LATENCY_BUDGET_MS,
        paths_found: paths.len(),
        passed,
    };
    emit_summary(&summary);

    assert!(
        !paths.is_empty(),
        "fixture graph must yield at least one XLM → BTC route"
    );
    assert!(
        passed,
        "pathfinding latency gate failed: init={init_us}µs lookup={lookup_us}µs \
         total={total_us}µs (budget={LATENCY_BUDGET_MS}ms)"
    );
}
