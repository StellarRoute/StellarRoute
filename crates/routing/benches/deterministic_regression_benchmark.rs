/// Criterion benchmark that exercises the deterministic regression runner.
///
/// This benchmark intentionally uses a fixed seed so results are comparable
/// across commits.  CI should run:
///
///   cargo bench --bench deterministic_regression_benchmark
///
/// and pipe the output through `cargo-criterion` or compare HTML reports.
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use stellarroute_routing::regression::{
    BenchmarkFixture, RegressionRunner, RegressionRunnerConfig,
};

fn bench_standard_suite(c: &mut Criterion) {
    let fixtures = BenchmarkFixture::standard_suite();
    let runner = RegressionRunner::new(RegressionRunnerConfig {
        iterations: 1,
        max_score_regression: 0.05,
        max_latency_regression_us: 10_000,
    });

    c.bench_function("regression_standard_suite", |b| {
        b.iter(|| {
            let report = runner.run(black_box(&fixtures), None);
            black_box(report)
        })
    });
}

fn bench_single_fixture_2hop(c: &mut Criterion) {
    let fixture = BenchmarkFixture {
        name: "bench_xlm_usdc_2hop".to_string(),
        seed: 0xCAFE_BABE,
        from_asset: "XLM".to_string(),
        to_asset: "USDC".to_string(),
        amount_in: 100_000_000,
        graph_size: 3,
    };
    let runner = RegressionRunner::new(RegressionRunnerConfig {
        iterations: 1,
        ..Default::default()
    });

    c.bench_function("regression_single_2hop", |b| {
        b.iter(|| {
            let report = runner.run(black_box(&[fixture.clone()]), None);
            black_box(report)
        })
    });
}

fn bench_single_fixture_large_graph(c: &mut Criterion) {
    let fixture = BenchmarkFixture {
        name: "bench_xlm_usdc_large".to_string(),
        seed: 0xCAFE_BABE,
        from_asset: "XLM".to_string(),
        to_asset: "USDC".to_string(),
        amount_in: 1_000_000_000,
        graph_size: 10,
    };
    let runner = RegressionRunner::new(RegressionRunnerConfig {
        iterations: 1,
        ..Default::default()
    });

    c.bench_function("regression_single_large_graph", |b| {
        b.iter(|| {
            let report = runner.run(black_box(&[fixture.clone()]), None);
            black_box(report)
        })
    });
}

criterion_group!(
    benches,
    bench_standard_suite,
    bench_single_fixture_2hop,
    bench_single_fixture_large_graph
);
criterion_main!(benches);
