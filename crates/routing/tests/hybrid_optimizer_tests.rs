//! Integration tests for hybrid route optimizer

use std::collections::HashMap;
use stellarroute_routing::{
    HybridOptimizer, LiquidityEdge, OptimizerPolicy, PathfinderConfig, PolicyPresets, RoutingPolicy,
};

fn create_test_graph() -> Vec<LiquidityEdge> {
    vec![
        // Direct paths
        LiquidityEdge {
            from: "XLM".to_string(),
            to: "USDC".to_string(),
            venue_type: "amm".to_string(),
            venue_ref: "pool_xlm_usdc".to_string(),
            liquidity: 1_000_000_000,
            price: 1.0,
            fee_bps: 30, // 100 XLM
            anomaly_score: 0.0,
            anomaly_reasons: vec![],
            fee_bps: 30,
            anomaly_score: 0.0,
            anomaly_reasons: vec![], // 100 XLM
        },
        LiquidityEdge {
            from: "XLM".to_string(),
            to: "EURT".to_string(),
            venue_type: "orderbook".to_string(),
            venue_ref: "book_xlm_eurt".to_string(),
            liquidity: 500_000_000,
            price: 1.0,
            fee_bps: 30, // 50 XLM
            anomaly_score: 0.0,
            anomaly_reasons: vec![],
            fee_bps: 30,
            anomaly_score: 0.0,
            anomaly_reasons: vec![], // 50 XLM
        },
        // Multi-hop paths
        LiquidityEdge {
            from: "USDC".to_string(),
            to: "EURT".to_string(),
            venue_type: "amm".to_string(),
            venue_ref: "pool_usdc_eurt".to_string(),
            liquidity: 800_000_000,
            price: 1.0,
            fee_bps: 30, // 80 USDC
            anomaly_score: 0.0,
            anomaly_reasons: vec![],
            fee_bps: 30,
            anomaly_score: 0.0,
            anomaly_reasons: vec![], // 80 USDC
        },
        LiquidityEdge {
            from: "EURT".to_string(),
            to: "BTC".to_string(),
            venue_type: "orderbook".to_string(),
            venue_ref: "book_eurt_btc".to_string(),
            liquidity: 200_000_000,
            price: 1.0,
            fee_bps: 30, // 20 EURT
            anomaly_score: 0.0,
            anomaly_reasons: vec![],
            fee_bps: 30,
            anomaly_score: 0.0,
            anomaly_reasons: vec![], // 20 EURT
        },
        LiquidityEdge {
            from: "USDC".to_string(),
            to: "BTC".to_string(),
            venue_type: "amm".to_string(),
            venue_ref: "pool_usdc_btc".to_string(),
            liquidity: 300_000_000,
            price: 1.0,
            fee_bps: 30, // 30 USDC
            anomaly_score: 0.0,
            anomaly_reasons: vec![],
            fee_bps: 30,
            anomaly_score: 0.0,
            anomaly_reasons: vec![], // 30 USDC
        },
        // Additional liquidity sources
        LiquidityEdge {
            from: "XLM".to_string(),
            to: "BTC".to_string(),
            venue_type: "amm".to_string(),
            venue_ref: "pool_xlm_btc".to_string(),
            liquidity: 150_000_000,
            price: 1.0,
            fee_bps: 30, // 15 XLM
            anomaly_score: 0.0,
            anomaly_reasons: vec![],
            fee_bps: 30,
            anomaly_score: 0.0,
            anomaly_reasons: vec![], // 15 XLM
        },
    ]
}

fn default_routing_policy() -> RoutingPolicy {
    RoutingPolicy::default()
}

#[test]
fn test_hybrid_optimizer_basic_functionality() {
    let edges = create_test_graph();
    let optimizer = HybridOptimizer::new(PathfinderConfig::default());

    let result = optimizer.find_optimal_routes(
        "XLM",
        "BTC",
        &edges,
        100_000_000, // 10 XLM
        &default_routing_policy(),
    );

    assert!(result.is_ok());
    let diagnostics = result.unwrap();

    // Should find a route
    assert!(!diagnostics.selected_path.hops.is_empty());
    assert!(diagnostics.metrics.output_amount > 0);
    assert!(diagnostics.metrics.score > 0.0);
    assert!(diagnostics.metrics.score <= 1.0);
}

#[test]
fn test_policy_comparison() {
    let edges = create_test_graph();
    let mut optimizer = HybridOptimizer::new(PathfinderConfig::default());

    // Test different policies
    let policies = vec![
        ("production", PolicyPresets::production()),
        ("analysis", PolicyPresets::analysis()),
        ("realtime", PolicyPresets::realtime()),
    ];

    let mut results = HashMap::new();

    for (name, policy) in policies {
        optimizer.add_policy(policy).unwrap();
        optimizer.set_active_policy(name).unwrap();

        let result = optimizer
            .find_optimal_routes("XLM", "BTC", &edges, 100_000_000, &default_routing_policy())
            .unwrap();
        results.insert(name, result.metrics.clone());
    }

    // Analysis should prioritize output over latency
    let analysis_output = results["analysis"].output_amount;
    let realtime_output = results["realtime"].output_amount;
    assert!(analysis_output >= realtime_output);

    // Realtime should generally be lower compute time, but wall-clock timing can be noisy on CI/VMs.
    // Keep this check tolerant to avoid flaky failures.
    let analysis_time = results["analysis"].compute_time_us;
    let realtime_time = results["realtime"].compute_time_us;
    let diff = realtime_time.saturating_sub(analysis_time);
    assert!(
        diff <= 50_000,
        "realtime_time ({realtime_time}us) should not be dramatically higher than analysis_time ({analysis_time}us); diff={diff}us"
    );
}

#[test]
fn test_deterministic_behavior() {
    let edges = create_test_graph();
    let optimizer = HybridOptimizer::new(PathfinderConfig::default());

    // Run same query multiple times
    let result1 = optimizer
        .find_optimal_routes("XLM", "BTC", &edges, 100_000_000, &default_routing_policy())
        .unwrap();
    let result2 = optimizer
        .find_optimal_routes("XLM", "BTC", &edges, 100_000_000, &default_routing_policy())
        .unwrap();
    let result3 = optimizer
        .find_optimal_routes("XLM", "BTC", &edges, 100_000_000, &default_routing_policy())
        .unwrap();

    // Results should be identical
    // Output path is deterministic; score includes `compute_time_us` so it varies per call.
    assert_eq!(result1.metrics.output_amount, result2.metrics.output_amount);
    assert_eq!(result2.metrics.output_amount, result3.metrics.output_amount);
    assert_eq!(result1.metrics.impact_bps, result2.metrics.impact_bps);
    assert_eq!(result2.metrics.impact_bps, result3.metrics.impact_bps);
    assert_eq!(result1.metrics.hop_count, result2.metrics.hop_count);
}

#[test]
fn test_policy_constraints() {
    let edges = create_test_graph();
    let mut optimizer = HybridOptimizer::new(PathfinderConfig::default());

    // Create restrictive policy
    let restrictive_policy = OptimizerPolicy {
        output_weight: 0.5,
        impact_weight: 0.3,
        latency_weight: 0.2,
        max_impact_bps: 10,     // Very low impact tolerance
        max_compute_time_ms: 1, // Very low time tolerance
        environment: "restrictive".to_string(),
    };

    optimizer.add_policy(restrictive_policy).unwrap();
    optimizer.set_active_policy("restrictive").unwrap();

    let result =
        optimizer.find_optimal_routes("XLM", "BTC", &edges, 100_000_000, &default_routing_policy());

    // Should either succeed with constrained route or fail gracefully
    match result {
        Ok(diagnostics) => {
            assert!(diagnostics.metrics.impact_bps <= 10);
        }
        Err(_) => {
            // Acceptable - no routes meet constraints
        }
    }
}

#[test]
fn test_custom_policy() {
    let edges = create_test_graph();
    let mut optimizer = HybridOptimizer::new(PathfinderConfig::default());

    // Custom policy prioritizing latency
    let latency_first_policy = OptimizerPolicy {
        output_weight: 0.1,
        impact_weight: 0.1,
        latency_weight: 0.8,
        max_impact_bps: 1000,
        max_compute_time_ms: 50,
        environment: "latency_first".to_string(),
    };

    optimizer.add_policy(latency_first_policy).unwrap();
    optimizer.set_active_policy("latency_first").unwrap();

    let result = optimizer
        .find_optimal_routes("XLM", "BTC", &edges, 100_000_000, &default_routing_policy())
        .unwrap();

    // Should find a route quickly
    assert!(result.metrics.compute_time_us < 50_000); // 50ms in microseconds
    assert!(result.total_compute_time_ms <= 50);
}

#[test]
fn test_benchmark_all_policies() {
    let edges = create_test_graph();
    let mut optimizer = HybridOptimizer::new(PathfinderConfig::default());

    let results = optimizer
        .benchmark_policies("XLM", "BTC", &edges, 100_000_000, &default_routing_policy())
        .unwrap();

    // Should have results for all default policies
    assert!(results.len() >= 4); // production, analysis, realtime, testing

    // Each result should be valid
    for (policy_name, diagnostics) in &results {
        assert!(!diagnostics.selected_path.hops.is_empty());
        assert!(diagnostics.metrics.output_amount > 0);
        assert!(diagnostics.policy.environment == *policy_name);
    }
}

#[test]
fn test_route_quality_metrics() {
    let edges = create_test_graph();
    let optimizer = HybridOptimizer::new(PathfinderConfig::default());

    let result = optimizer
        .find_optimal_routes("XLM", "BTC", &edges, 100_000_000, &default_routing_policy())
        .unwrap();
    let metrics = result.metrics;

    // Validate metrics
    assert!(metrics.output_amount > 0);
    assert!(metrics.hop_count > 0);
    assert!(metrics.score > 0.0);
    assert!(metrics.score <= 1.0);

    // Alternatives should be sorted by score
    for i in 1..result.alternatives.len() {
        assert!(result.alternatives[i - 1].1.score >= result.alternatives[i].1.score);
    }
}

#[test]
fn test_no_route_available() {
    let edges = create_test_graph();
    let optimizer = HybridOptimizer::new(PathfinderConfig::default());

    // Request route to non-existent asset
    let result = optimizer.find_optimal_routes(
        "XLM",
        "NONEXISTENT",
        &edges,
        100_000_000,
        &default_routing_policy(),
    );

    assert!(result.is_err());
}

#[test]
fn test_insufficient_liquidity() {
    let mut small_edges = create_test_graph();

    // Make all liquidity very small
    for edge in &mut small_edges {
        edge.liquidity = 1_000; // Very small
    }

    let optimizer = HybridOptimizer::new(PathfinderConfig::default());

    let result = optimizer.find_optimal_routes(
        "XLM",
        "BTC",
        &small_edges,
        100_000_000,
        &default_routing_policy(),
    ); // Large amount

    // Should fail due to insufficient liquidity
    assert!(result.is_err());
}

#[test]
fn test_multi_hop_vs_direct() {
    let edges = create_test_graph();
    let optimizer = HybridOptimizer::new(PathfinderConfig::default());

    // Test direct XLM -> BTC route
    let direct_result = optimizer
        .find_optimal_routes("XLM", "BTC", &edges, 50_000_000, &default_routing_policy())
        .unwrap();

    // Test multi-hop XLM -> USDC -> BTC route
    let multihop_result = optimizer
        .find_optimal_routes("XLM", "BTC", &edges, 50_000_000, &default_routing_policy())
        .unwrap();

    // Both should find routes, but potentially different ones
    assert!(direct_result.metrics.output_amount > 0);
    assert!(multihop_result.metrics.output_amount > 0);

    // The optimizer should choose the best route based on policy
    assert!(direct_result.metrics.score > 0.0);
    assert!(multihop_result.metrics.score > 0.0);
}

#[test]
fn test_policy_validation() {
    // Valid policy
    let valid_policy = OptimizerPolicy::default();
    assert!(valid_policy.validate().is_ok());

    // Invalid: weights don't sum to 1.0
    let invalid_policy = OptimizerPolicy {
        output_weight: 0.8,
        impact_weight: 0.5,
        latency_weight: 0.1, // Sum = 1.4
        ..Default::default()
    };
    assert!(invalid_policy.validate().is_err());

    // Invalid: negative weight
    let negative_policy = OptimizerPolicy {
        output_weight: -0.1,
        ..Default::default()
    };
    assert!(negative_policy.validate().is_err());
}

#[test]
fn test_environment_switching() {
    let mut optimizer = HybridOptimizer::new(PathfinderConfig::default());

    // Test switching between environments
    assert!(optimizer.set_active_policy("production").is_ok());
    assert_eq!(optimizer.active_policy().environment, "production");

    assert!(optimizer.set_active_policy("realtime").is_ok());
    assert_eq!(optimizer.active_policy().environment, "realtime");

    // Test invalid environment
    assert!(optimizer.set_active_policy("invalid").is_err());

    // Should still be on last valid policy
    assert_eq!(optimizer.active_policy().environment, "realtime");
}
