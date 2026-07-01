//! Integration tests for risk limits in route selection

use stellarroute_routing::{
    pathfinder::{LiquidityEdge, PathfinderConfig},
    policy::RoutingPolicy,
    risk::{AssetRiskLimit, RiskLimitConfig},
    HybridOptimizer,
};

fn create_test_edges() -> Vec<LiquidityEdge> {
    vec![
        LiquidityEdge {
            from: "XLM".to_string(),
            to: "USDC".to_string(),
            venue_type: "sdex".to_string(),
            venue_ref: "sdex:XLM:USDC".to_string(),
            liquidity: 1_000_000_000,
            price: 0.10,
            fee_bps: 30,
        },
        LiquidityEdge {
            from: "USDC".to_string(),
            to: "EURC".to_string(),
            venue_type: "amm".to_string(),
            venue_ref: "amm:USDC:EURC".to_string(),
            liquidity: 500_000_000,
            price: 0.92,
            fee_bps: 25,
        },
        LiquidityEdge {
            from: "XLM".to_string(),
            to: "EURC".to_string(),
            venue_type: "sdex".to_string(),
            venue_ref: "sdex:XLM:EURC".to_string(),
            liquidity: 100_000_000,
            price: 0.092,
            fee_bps: 30,
        },
    ]
}

#[test]
fn test_permissive_policy_allows_routes() {
    let config = PathfinderConfig::default();
    let risk_config = RiskLimitConfig::permissive_policy();
    let optimizer = HybridOptimizer::with_risk_limits(config, risk_config);

    let edges = create_test_edges();
    let routing_policy = RoutingPolicy::default();

    let result = optimizer.find_optimal_routes("XLM", "USDC", &edges, 10_000_000, &routing_policy);

    assert!(result.is_ok());
    let diagnostics = result.unwrap();
    assert!(diagnostics.excluded_routes.is_empty());
}

#[test]
fn test_strict_policy_excludes_low_liquidity() {
    let config = PathfinderConfig::default();
    let risk_config = RiskLimitConfig::strict_policy();
    let mut optimizer = HybridOptimizer::with_risk_limits(config, risk_config);
    optimizer.set_active_policy("testing").unwrap();

    let edges = vec![LiquidityEdge {
        from: "XLM".to_string(),
        to: "USDC".to_string(),
        venue_type: "sdex".to_string(),
        venue_ref: "sdex:XLM:USDC".to_string(),
        liquidity: 10_000,
        price: 0.10,
        fee_bps: 30,
    }];
    let routing_policy = RoutingPolicy::default();

    let result = optimizer.find_optimal_routes("XLM", "USDC", &edges, 10_000_000, &routing_policy);

    assert!(
        result.is_err() || {
            let diagnostics = result.unwrap();
            !diagnostics.excluded_routes.is_empty()
        }
    );
}

#[test]
fn test_per_asset_overrides() {
    let config = PathfinderConfig::default();
    let risk_config = RiskLimitConfig::default().with_asset_limit(
        "USDC",
        AssetRiskLimit {
            max_exposure: 1_000,
            max_impact_bps: 10,
            liquidity_floor: 1_000_000_000,
            blacklisted: false,
        },
    );

    let optimizer = HybridOptimizer::with_risk_limits(config, risk_config);
    let edges = create_test_edges();
    let routing_policy = RoutingPolicy::default();

    let result = optimizer.find_optimal_routes("XLM", "USDC", &edges, 10_000_000, &routing_policy);

    assert!(
        result.is_err() || {
            let diagnostics = result.unwrap();
            !diagnostics.excluded_routes.is_empty()
        }
    );
}

#[test]
fn test_blacklisted_asset_excluded() {
    let config = PathfinderConfig::default();
    let risk_config = RiskLimitConfig::default().with_asset_limit(
        "USDC",
        AssetRiskLimit {
            blacklisted: true,
            ..Default::default()
        },
    );

    let optimizer = HybridOptimizer::with_risk_limits(config, risk_config);
    let edges = create_test_edges();
    let routing_policy = RoutingPolicy::default();

    let result = optimizer.find_optimal_routes("XLM", "USDC", &edges, 10_000_000, &routing_policy);

    if let Ok(diagnostics) = result {
        assert!(diagnostics
            .excluded_routes
            .iter()
            .any(|e| e.asset == "USDC"));
    }
}

#[test]
fn test_risk_limits_config_from_json() {
    let json = r#"{
        "global_defaults": {
            "max_exposure": 1000000000,
            "max_impact_bps": 200,
            "liquidity_floor": 5000000,
            "blacklisted": false
        },
        "per_asset": {
            "SCAM": {
                "max_exposure": 0,
                "max_impact_bps": 0,
                "liquidity_floor": 0,
                "blacklisted": true
            }
        }
    }"#;

    let config = RiskLimitConfig::from_json(json).unwrap();
    assert_eq!(config.global_defaults.max_impact_bps, 200);
    assert!(config.get_limit("SCAM").blacklisted);
    assert!(!config.get_limit("XLM").blacklisted);
}

#[test]
fn test_optimizer_without_risk_limits() {
    let config = PathfinderConfig::default();
    let optimizer = HybridOptimizer::new(config);

    let edges = create_test_edges();
    let routing_policy = RoutingPolicy::default();

    let result = optimizer.find_optimal_routes("XLM", "USDC", &edges, 10_000_000, &routing_policy);

    assert!(result.is_ok());
    let diagnostics = result.unwrap();
    assert!(diagnostics.excluded_routes.is_empty());
}

#[test]
fn test_set_and_clear_risk_limits() {
    let config = PathfinderConfig::default();
    let mut optimizer = HybridOptimizer::new(config);

    optimizer.set_risk_limits(RiskLimitConfig::strict_policy());

    optimizer.clear_risk_limits();

    let edges = create_test_edges();
    let routing_policy = RoutingPolicy::default();

    let result = optimizer.find_optimal_routes("XLM", "USDC", &edges, 10_000_000, &routing_policy);
    assert!(result.is_ok());
}
