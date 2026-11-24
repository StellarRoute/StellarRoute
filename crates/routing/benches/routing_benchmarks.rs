use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use stellarroute_routing::{
    pathfinder::{LiquidityEdge, Pathfinder, PathfinderConfig},
    AmmQuoteCalculator, RoutingPolicy,
};

fn bench_pathfinding_2hop(c: &mut Criterion) {
    c.bench_function("pathfind_2hop", |b| {
        b.iter(|| {
            let config = PathfinderConfig {
                min_liquidity_threshold: 100_000,
            };
            let pathfinder = Pathfinder::new(config);
            let routing_policy = RoutingPolicy::default();

            let edges = vec![
                LiquidityEdge {
                    from: "XLM".to_string(),
                    to: "USDC".to_string(),
                    venue_type: "sdex".to_string(),
                    venue_ref: "offer1".to_string(),
                    liquidity: 1_000_000_000,
                    price: 1.0,
                    fee_bps: 30,
                    ..Default::default()
                },
                LiquidityEdge {
                    from: "USDC".to_string(),
                    to: "BTC".to_string(),
                    venue_type: "amm".to_string(),
                    venue_ref: "pool1".to_string(),
                    liquidity: 500_000_000,
                    price: 1.0,
                    fee_bps: 30,
                    ..Default::default()
                },
            ];

            let _ = pathfinder.find_paths(
                "XLM",
                "BTC",
                &edges,
                black_box(100_000_000),
                &routing_policy,
            );
        })
    });
}

fn bench_pathfinding_4hop(c: &mut Criterion) {
    c.bench_function("pathfind_4hop_realistic", |b| {
        b.iter(|| {
            let config = PathfinderConfig {
                min_liquidity_threshold: 100_000,
            };
            let pathfinder = Pathfinder::new(config);
            let routing_policy = RoutingPolicy::default();

            let edges = vec![
                LiquidityEdge {
                    from: "XLM".to_string(),
                    to: "USDC".to_string(),
                    venue_type: "sdex".to_string(),
                    venue_ref: "offer1".to_string(),
                    liquidity: 2_000_000_000,
                    price: 1.0,
                    fee_bps: 30,
                    ..Default::default()
                },
                LiquidityEdge {
                    from: "USDC".to_string(),
                    to: "EUR".to_string(),
                    venue_type: "amm".to_string(),
                    venue_ref: "pool1".to_string(),
                    liquidity: 1_500_000_000,
                    price: 1.0,
                    fee_bps: 30,
                    ..Default::default()
                },
                LiquidityEdge {
                    from: "EUR".to_string(),
                    to: "GBP".to_string(),
                    venue_type: "sdex".to_string(),
                    venue_ref: "offer2".to_string(),
                    liquidity: 1_000_000_000,
                    price: 1.0,
                    fee_bps: 30,
                    ..Default::default()
                },
                LiquidityEdge {
                    from: "GBP".to_string(),
                    to: "BTC".to_string(),
                    venue_type: "amm".to_string(),
                    venue_ref: "pool2".to_string(),
                    liquidity: 800_000_000,
                    price: 1.0,
                    fee_bps: 30,
                    ..Default::default()
                },
                // Additional cross-links
                LiquidityEdge {
                    from: "USDC".to_string(),
                    to: "BTC".to_string(),
                    venue_type: "sdex".to_string(),
                    venue_ref: "offer3".to_string(),
                    liquidity: 3_000_000_000,
                    price: 1.0,
                    fee_bps: 30,
                    ..Default::default()
                },
                LiquidityEdge {
                    from: "XLM".to_string(),
                    to: "BTC".to_string(),
                    venue_type: "amm".to_string(),
                    venue_ref: "pool3".to_string(),
                    liquidity: 1_200_000_000,
                    price: 1.0,
                    fee_bps: 30,
                    ..Default::default()
                },
            ];

            let _ = pathfinder.find_paths(
                "XLM",
                "BTC",
                &edges,
                black_box(500_000_000),
                &routing_policy,
            );
        })
    });
}

fn bench_amm_quote(c: &mut Criterion) {
    c.bench_function("amm_quote_constant_product", |b| {
        b.iter(|| {
            let calc = AmmQuoteCalculator;
            let _ = calc.quote_constant_product(
                black_box(100_000_000),
                black_box(10_000_000_000),
                black_box(10_000_000_000),
                black_box(30),
            );
        })
    });
}

fn bench_amm_quote_large_trade(c: &mut Criterion) {
    c.bench_function("amm_quote_large_trade_4M_reserve", |b| {
        b.iter(|| {
            let calc = AmmQuoteCalculator;
            let _ = calc.quote_constant_product(
                black_box(2_000_000_000),
                black_box(40_000_000_000),
                black_box(40_000_000_000),
                black_box(30),
            );
        })
    });
}

fn bench_pathfinding_fixture(c: &mut Criterion) {
    c.bench_function("pathfind_fixture", |b| {
        // Load the representative graph fixture
        let fixture_data = include_str!("../fixtures/graph_fixture.json");
        let edges: Vec<LiquidityEdge> =
            serde_json::from_str(fixture_data).expect("Valid fixture data");

        b.iter(|| {
            let config = PathfinderConfig {
                min_liquidity_threshold: 100_000,
            };
            let pathfinder = Pathfinder::new(config);
            let routing_policy = RoutingPolicy::default();

            let _ = pathfinder.find_paths(
                "XLM",
                "BTC",
                &edges,
                black_box(100_000_000),
                &routing_policy,
            );
        })
    });
}

fn build_dense_synthetic_graph(asset_count: usize, edge_count: usize) -> Vec<LiquidityEdge> {
    let mut rng = ChaCha8Rng::seed_from_u64(0x57E1_1A20_1648);
    let mut assets = Vec::with_capacity(asset_count);
    assets.push("XLM".to_string());
    assets.push("USDC".to_string());
    assets.push("EURC".to_string());
    assets.push("BTC".to_string());
    for idx in 4..asset_count {
        assets.push(format!("ASSET_{idx:03}"));
    }

    let fee_tiers = [0_u32, 10, 25, 30, 100];
    let mut edges = Vec::with_capacity(edge_count);

    // Connect XLM and USDC hubs broadly so 2..4-hop simple paths fan out densely,
    // mirroring real Stellar liquidity concentrated around XLM/USDC bridges.
    let hub_span = (asset_count - 4).min(120);
    for (offset, mid) in assets[4..4 + hub_span].iter().enumerate() {
        for hub in ["XLM", "USDC", "EURC", "BTC"] {
            if edges.len() + 2 > edge_count {
                break;
            }
            let price = rng.gen_range(0.05..15.0);
            let fee_bps = fee_tiers[offset % fee_tiers.len()];
            edges.push(LiquidityEdge {
                from: hub.to_string(),
                to: mid.clone(),
                venue_type: if offset % 2 == 0 { "amm" } else { "sdex" }.to_string(),
                venue_ref: format!("hub_out_{}", edges.len()),
                liquidity: rng.gen_range(100_000_000..10_000_000_000),
                price,
                fee_bps,
                ..Default::default()
            });
            edges.push(LiquidityEdge {
                from: mid.clone(),
                to: hub.to_string(),
                venue_type: if offset % 2 == 0 { "amm" } else { "sdex" }.to_string(),
                venue_ref: format!("hub_in_{}", edges.len()),
                liquidity: rng.gen_range(100_000_000..10_000_000_000),
                price: 1.0 / price,
                fee_bps,
                ..Default::default()
            });
        }
    }

    while edges.len() < edge_count {
        let from_idx = rng.gen_range(0..asset_count);
        let mut to_idx = rng.gen_range(0..asset_count - 1);
        if to_idx >= from_idx {
            to_idx += 1;
        }
        let venue_type = if rng.gen_bool(0.5) { "amm" } else { "sdex" };
        let fee_bps = fee_tiers[rng.gen_range(0..fee_tiers.len())];
        edges.push(LiquidityEdge {
            from: assets[from_idx].clone(),
            to: assets[to_idx].clone(),
            venue_type: venue_type.to_string(),
            venue_ref: format!("dense_venue_{}", edges.len()),
            liquidity: rng.gen_range(100_000_000..10_000_000_000),
            price: rng.gen_range(0.05..20.0),
            fee_bps,
            ..Default::default()
        });
    }

    edges
}

fn bench_pathfinding_dense_graph(c: &mut Criterion) {
    let edges = build_dense_synthetic_graph(200, 2_000);
    let config = PathfinderConfig {
        min_liquidity_threshold: 100_000,
    };
    let pathfinder = Pathfinder::new(config);
    let routing_policy = RoutingPolicy::default();

    c.bench_function("pathfind_dense_200_assets_2000_edges", |b| {
        b.iter(|| {
            let paths = pathfinder
                .find_paths(
                    "XLM",
                    "BTC",
                    black_box(&edges),
                    black_box(100_000_000),
                    &routing_policy,
                )
                .expect("dense graph should have routes");
            black_box(paths);
        })
    });
}

criterion_group!(
    benches,
    bench_pathfinding_2hop,
    bench_pathfinding_4hop,
    bench_pathfinding_fixture,
    bench_pathfinding_dense_graph,
    bench_amm_quote,
    bench_amm_quote_large_trade
);
criterion_main!(benches);
