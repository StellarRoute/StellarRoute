use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use stellarroute_api::{
    load_test::{
        AmountDistribution, DegradationScenario, HarnessConfig, LoadTestHarness, TrafficMix,
        TrafficType,
    },
    state::DatabasePools,
    Server, ServerConfig,
};
use tower::ServiceExt;

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance"]
async fn test_harness_mixed_traffic_profile() {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
    });

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(50)
        .connect(&db_url)
        .await
        .expect("Failed to connect to database");

    let server = Server::new(ServerConfig::default(), DatabasePools::new(pool, None)).await;
    let router = server.into_router();

    let config = HarnessConfig {
        name: "mixed_traffic_test".to_string(),
        concurrent_users: 5,
        total_requests: 50,
        requests_per_second: 20,
        duration_secs: 10,
        traffic_mix: TrafficMix {
            sdex_weight: 0.5,
            amm_weight: 0.3,
            mixed_weight: 0.2,
        },
        amount_distribution: AmountDistribution {
            min_amount: 1.0,
            max_amount: 100.0,
            log_normal: true,
        },
        degradation: DegradationScenario::default(),
    };

    let harness = LoadTestHarness::new(config);

    let router_clone = router.clone();
    let results = harness
        .run(move |traffic_type, amount| {
            let router = router_clone.clone();
            async move {
                // Select pairs based on traffic type
                let (base, quote) = match traffic_type {
                    TrafficType::Sdex => ("native", "USDC"), // Typical SDEX pair
                    TrafficType::Amm => ("native", "XLM"),   // Typical AMM pair (demo)
                    TrafficType::Mixed => ("USDC", "XLM"),
                };

                let uri = format!("/api/v1/quote/{}/{}?amount={}", base, quote, amount);
                let request = Request::builder().uri(uri).body(Body::empty()).unwrap();

                let response = (*router)
                    .clone()
                    .oneshot(request)
                    .await
                    .map_err(|e| e.to_string())?;

                if response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND
                {
                    Ok(())
                } else {
                    Err(format!("Unexpected status: {}", response.status()))
                }
            }
        })
        .await;

    results.print_summary();
    harness
        .save_results(&results)
        .expect("Failed to save results");

    assert!(results.total_requests >= 40); // Allow some slack for timing
    assert!(results.successful_requests + results.failed_requests == results.total_requests);

    // Verify file was created
    assert!(std::path::Path::new("load_test_results_mixed_traffic_test.json").exists());
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance"]
async fn test_harness_degradation_scenario() {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
    });

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(50)
        .connect(&db_url)
        .await
        .expect("Failed to connect to database");

    let server = Server::new(ServerConfig::default(), DatabasePools::new(pool, None)).await;
    let router = server.into_router();

    let config = HarnessConfig {
        name: "degradation_test".to_string(),
        concurrent_users: 2,
        total_requests: 20,
        requests_per_second: 10,
        duration_secs: 5,
        degradation: DegradationScenario {
            db_latency_ms: 100,
            db_error_rate: 0.2,
            ..Default::default()
        },
        ..Default::default()
    };

    let harness = LoadTestHarness::new(config);

    let router_clone = router.clone();
    let results = harness
        .run(move |_, _| {
            let router = router_clone.clone();
            async move {
                let request = Request::builder()
                    .uri("/api/v1/quote/native/USDC?amount=1")
                    .body(Body::empty())
                    .unwrap();

                let response = (*router)
                    .clone()
                    .oneshot(request)
                    .await
                    .map_err(|e| e.to_string())?;
                if response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND
                {
                    Ok(())
                } else {
                    Err(format!("Unexpected status: {}", response.status()))
                }
            }
        })
        .await;

    results.print_summary();

    // Check that latency is affected by simulated db_latency
    assert!(results.p50_latency_ms >= 100.0);
    // Check that some requests failed due to simulated error rate
    assert!(results.failed_requests > 0);
}
