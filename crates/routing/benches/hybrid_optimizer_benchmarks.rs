//! Benchmark suite for hybrid route optimizer quality/latency tradeoffs

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::time::Duration;
use stellarroute_routing::{
    BenchmarkHarness, HybridOptimizer, LiquidityEdge, OptimizerPolicy, PathfinderConfig,
    PolicyPresets, RoutingPolicy, ScorerInput, ScorerRegistry, SwapPath,
};

fn create_test_edges() -> Vec<LiquidityEdge> {
    vec![
        LiquidityEdge {
            from: "XLM".to_string(),
            to: "USDC".to_string(),
            venue_type: "amm".to_string(),
            venue_ref: "pool1".to_string(),
            liquidity: 1_000_000_000,
            price: 1.0,
            fee_bps: 30, // 100 XLM
            anomaly_score: 0.0,
            anomaly_reasons: vec![],
        },
        LiquidityEdge {
            from: "USDC".to_string(),
            to: "EURT".to_string(),
            venue_type: "orderbook".to_string(),
            venue_ref: "book1".to_string(),
            liquidity: 500_000_000,
            price: 1.0,
            fee_bps: 30, // 50 USDC
            anomaly_score: 0.0,
            anomaly_reasons: vec![],
        },
        LiquidityEdge {
            from: "XLM".to_string(),
            to: "EURT".to_string(),
            venue_type: "amm".to_string(),
            venue_ref: "pool2".to_string(),
            liquidity: 200_000_000,
            price: 1.0,
            fee_bps: 30, // 20 XLM
            anomaly_score: 0.0,
            anomaly_reasons: vec![],
        },
        LiquidityEdge {
            from: "EURT".to_string(),
            to: "BTC".to_string(),
            venue_type: "orderbook".to_string(),
            venue_ref: "book2".to_string(),
            liquidity: 100_000_000,
            price: 1.0,
            fee_bps: 30, // 10 EURT
            anomaly_score: 0.0,
            anomaly_reasons: vec![],
        },
        LiquidityEdge {
            from: "USDC".to_string(),
            to: "BTC".to_string(),
            venue_type: "amm".to_string(),
            venue_ref: "pool3".to_string(),
            liquidity: 300_000_000,
            price: 1.0,
            fee_bps: 30, // 30 USDC
            anomaly_score: 0.0,
            anomaly_reasons: vec![],
        },
    ]
}

fn bench_policy_comparison(c: &mut Criterion) {
    let edges = create_test_edges();
    let config = PathfinderConfig::default();

    let mut group = c.benchmark_group("policy_comparison");
    group.measurement_time(Duration::from_secs(10));

    let policies = vec![
        ("production", PolicyPresets::production()),
        ("analysis", PolicyPresets::analysis()),
        ("realtime", PolicyPresets::realtime()),
        ("testing", PolicyPresets::testing()),
    ];

    for (name, policy) in policies {
        let mut optimizer = HybridOptimizer::new(config.clone());
        optimizer.add_policy(policy).unwrap();
        optimizer.set_active_policy(name).unwrap();

        group.bench_with_input(
            BenchmarkId::new("find_optimal_routes", name),
            &("XLM", "BTC", 100_000_000), // 10 XLM
            |b, &(from, to, amount)| {
                b.iter(|| {
                    black_box(optimizer.find_optimal_routes(
                        black_box(from),
                        black_box(to),
                        black_box(&edges),
                        black_box(amount),
                        black_box(&RoutingPolicy::default()),
                    ))
                })
            },
        );
    }

    group.finish();
}

fn bench_latency_vs_quality(c: &mut Criterion) {
    let edges = create_test_edges();

    let mut group = c.benchmark_group("latency_vs_quality");
    group.measurement_time(Duration::from_secs(15));

    // Test different max compute times
    for max_time_ms in [50, 100, 200, 500, 1000, 2000].iter() {
        let mut policy = PolicyPresets::production();
        policy.max_compute_time_ms = *max_time_ms;

        let mut optimizer = HybridOptimizer::new(PathfinderConfig::default());
        optimizer.add_policy(policy).unwrap();
        optimizer.set_active_policy("custom").unwrap();

        group.bench_with_input(
            BenchmarkId::new("compute_time_limit", max_time_ms),
            &("XLM", "BTC", 100_000_000),
            |b, &(from, to, amount)| {
                b.iter(|| {
                    black_box(optimizer.find_optimal_routes(
                        black_box(from),
                        black_box(to),
                        black_box(&edges),
                        black_box(amount),
                        black_box(&RoutingPolicy::default()),
                    ))
                })
            },
        );
    }

    group.finish();
}

fn bench_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("scalability");
    group.measurement_time(Duration::from_secs(10));

    // Test different graph sizes
    for edge_count in [5, 10, 20, 50].iter() {
        let mut edges = create_test_edges();

        // Add more edges for scalability testing
        for i in 0..*edge_count {
            edges.push(LiquidityEdge {
                from: format!("ASSET{}", i % 3),
                to: format!("ASSET{}", (i + 1) % 3),
                venue_type: if i % 2 == 0 { "amm" } else { "orderbook" }.to_string(),
                venue_ref: format!("venue{}", i),
                liquidity: 100_000_000 * (i + 1) as i128,
                price: 1.0,
                fee_bps: 30,
                anomaly_score: 0.0,
                anomaly_reasons: vec![],
            });
        }

        let optimizer = HybridOptimizer::new(PathfinderConfig::default());
        let routing_policy = RoutingPolicy::default();

        group.bench_with_input(
            BenchmarkId::new("graph_size", edge_count),
            &("ASSET0", "ASSET2", 10_000_000),
            |b, &(from, to, amount)| {
                b.iter(|| {
                    black_box(optimizer.find_optimal_routes(
                        black_box(from),
                        black_box(to),
                        black_box(&edges),
                        black_box(amount),
                        black_box(&routing_policy),
                    ))
                })
            },
        );
    }

    group.finish();
}

fn bench_determinism(c: &mut Criterion) {
    let edges = create_test_edges();
    let optimizer = HybridOptimizer::new(PathfinderConfig::default());
    let routing_policy = RoutingPolicy::default();

    let mut group = c.benchmark_group("determinism");
    group.measurement_time(Duration::from_secs(5));

    // Run multiple times to verify deterministic behavior
    group.bench_function("deterministic_routing", |b| {
        b.iter(|| {
            let result1 = black_box(optimizer.find_optimal_routes(
                black_box("XLM"),
                black_box("BTC"),
                black_box(&edges),
                black_box(100_000_000),
                black_box(&routing_policy),
            ));

            let result2 = black_box(optimizer.find_optimal_routes(
                black_box("XLM"),
                black_box("BTC"),
                black_box(&edges),
                black_box(100_000_000),
                black_box(&routing_policy),
            ));

            // Results should be identical for deterministic behavior
            assert_eq!(
                result1.unwrap().metrics.output_amount,
                result2.unwrap().metrics.output_amount
            );
        })
    });

    group.finish();
}

fn bench_benchmark_policies(c: &mut Criterion) {
    let edges = create_test_edges();
    let mut optimizer = HybridOptimizer::new(PathfinderConfig::default());
    let routing_policy = RoutingPolicy::default();

    let mut group = c.benchmark_group("benchmark_policies");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("compare_all_policies", |b| {
        b.iter(|| {
            black_box(optimizer.benchmark_policies(
                black_box("XLM"),
                black_box("BTC"),
                black_box(&edges),
                black_box(100_000_000),
                black_box(&routing_policy),
            ))
        })
    });

    group.finish();
}

fn bench_scorer_throughput(c: &mut Criterion) {
    let policy = OptimizerPolicy::default();
    let input = ScorerInput {
        output_amount: 500_000_000,
        impact_bps: 100,
        compute_time_us: 50_000,
        hop_count: 2,
        policy: policy.clone(),
    };

    let mut group = c.benchmark_group("scorer_throughput");
    group.measurement_time(Duration::from_secs(5));

    let registry = ScorerRegistry::new();
    for (name, scorer) in registry.iter() {
        let scorer_name = name.to_string();
        group.bench_with_input(
            BenchmarkId::new("score", &scorer_name),
            &input,
            |b, inp| b.iter(|| black_box(scorer.score(black_box(inp)))),
        );
    }

    // Also benchmark via registry (includes clamping overhead)
    for scorer_name in ["default", "fee_minimizing", "output_maximizing"] {
        let mut reg = ScorerRegistry::new();
        reg.set_active(scorer_name).unwrap();
        group.bench_with_input(
            BenchmarkId::new("registry_score", scorer_name),
            &input,
            |b, inp| b.iter(|| black_box(reg.score(black_box(inp)))),
        );
    }

    group.finish();
}

fn bench_scorer_comparison(c: &mut Criterion) {
    let edges = create_test_edges();
    let optimizer = HybridOptimizer::new(PathfinderConfig::default());
    let routing_policy = RoutingPolicy::default();

    // Collect up to 50 candidate paths by running the optimizer on multiple pairs
    let mut all_paths: Vec<SwapPath> = Vec::new();
    let pairs = [
        ("XLM", "BTC"),
        ("XLM", "USDC"),
        ("XLM", "EURT"),
        ("USDC", "BTC"),
        ("USDC", "EURT"),
    ];
    for (from, to) in pairs {
        if let Ok(diag) =
            optimizer.find_optimal_routes(from, to, &edges, 100_000_000, &routing_policy)
        {
            all_paths.push(diag.selected_path);
            for (path, _) in diag.alternatives {
                all_paths.push(path);
            }
        }
    }
    // Pad to 50 paths by repeating if needed
    let target = 50.min(all_paths.len().max(1));
    let paths: Vec<_> = all_paths.iter().cycle().take(target).cloned().collect();

    let registry = ScorerRegistry::new();

    let mut group = c.benchmark_group("scorer_comparison");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("harness_all_scorers_50_paths", |b| {
        b.iter(|| {
            black_box(BenchmarkHarness::run(
                black_box(&paths),
                black_box(&edges),
                black_box(100_000_000i128),
                black_box(&registry),
            ))
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_policy_comparison,
    bench_latency_vs_quality,
    bench_scalability,
    bench_determinism,
    bench_benchmark_policies,
    bench_scorer_throughput,
    bench_scorer_comparison
);
criterion_main!(benches);
