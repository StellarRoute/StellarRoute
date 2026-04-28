use criterion::{black_box, criterion_group, criterion_main, Criterion};
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
                    anomaly_score: 0.0,
                    anomaly_reasons: vec![],
                },
                LiquidityEdge {
                    from: "USDC".to_string(),
                    to: "BTC".to_string(),
                    venue_type: "amm".to_string(),
                    venue_ref: "pool1".to_string(),
                    liquidity: 500_000_000,
                    price: 1.0,
                    fee_bps: 30,
                    anomaly_score: 0.0,
                    anomaly_reasons: vec![],
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
                    anomaly_score: 0.0,
                    anomaly_reasons: vec![],
                },
                LiquidityEdge {
                    from: "USDC".to_string(),
                    to: "EUR".to_string(),
                    venue_type: "amm".to_string(),
                    venue_ref: "pool1".to_string(),
                    liquidity: 1_500_000_000,
                    price: 1.0,
                    fee_bps: 30,
                    anomaly_score: 0.0,
                    anomaly_reasons: vec![],
                },
                LiquidityEdge {
                    from: "EUR".to_string(),
                    to: "GBP".to_string(),
                    venue_type: "sdex".to_string(),
                    venue_ref: "offer2".to_string(),
                    liquidity: 1_000_000_000,
                    price: 1.0,
                    fee_bps: 30,
                    anomaly_score: 0.0,
                    anomaly_reasons: vec![],
                },
                LiquidityEdge {
                    from: "GBP".to_string(),
                    to: "BTC".to_string(),
                    venue_type: "amm".to_string(),
                    venue_ref: "pool2".to_string(),
                    liquidity: 800_000_000,
                    price: 1.0,
                    fee_bps: 30,
                    anomaly_score: 0.0,
                    anomaly_reasons: vec![],
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
                    anomaly_score: 0.0,
                    anomaly_reasons: vec![],
                },
                LiquidityEdge {
                    from: "XLM".to_string(),
                    to: "BTC".to_string(),
                    venue_type: "amm".to_string(),
                    venue_ref: "pool3".to_string(),
                    liquidity: 1_200_000_000,
                    price: 1.0,
                    fee_bps: 30,
                    anomaly_score: 0.0,
                    anomaly_reasons: vec![],
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

criterion_group!(
    benches,
    bench_pathfinding_2hop,
    bench_pathfinding_4hop,
    bench_pathfinding_fixture,
    bench_amm_quote,
    bench_amm_quote_large_trade
);
criterion_main!(benches);
