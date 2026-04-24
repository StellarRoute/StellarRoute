use stellarroute_routing::health::anomaly::{AnomalyConfig, LiquidityAnomalyDetector};
use stellarroute_routing::pathfinder::{LiquidityEdge, Pathfinder, PathfinderConfig};
use stellarroute_routing::optimizer::{HybridOptimizer, OptimizerPolicy};
use stellarroute_routing::policy::RoutingPolicy;

#[test]
fn test_anomaly_detection_integration() {
    let config = AnomalyConfig {
        reserve_shift_threshold: 0.5,
        depth_collapse_threshold: 0.8,
        alert_threshold: 0.7,
    };
    let mut detector = LiquidityAnomalyDetector::new(config);

    // 1. Normal scenario
    let venue_ref = "amm:XLM_USDC";
    let res1 = detector.update_and_detect(venue_ref, Some((1000, 1000)), None);
    assert!(res1.score == 0.0);

    // 2. Small shift (20%) - should not be anomalous
    let res2 = detector.update_and_detect(venue_ref, Some((1200, 1200)), None);
    assert!(res2.score < 0.7);
    assert!(!detector.is_anomalous(&res2));

    // 3. Large shift (70%) - should be anomalous
    let res3 = detector.update_and_detect(venue_ref, Some((300, 300)), None);
    assert!(res3.score >= 0.7);
    assert!(detector.is_anomalous(&res3));
    assert!(!res3.reasons.is_empty());
}

#[test]
fn test_optimizer_flags_anomalies() {
    let mut optimizer = HybridOptimizer::new(PathfinderConfig::default());
    
    // Create edges with anomalies
    let edges = vec![
        LiquidityEdge {
            from: "XLM".to_string(),
            to: "USDC".to_string(),
            venue_type: "amm".to_string(),
            venue_ref: "anomalous_pool".to_string(),
            liquidity: 1000,
            price: 1.0,
            fee_bps: 30,
            anomaly_score: 0.8,
            anomaly_reasons: vec!["Sudden reserve shift: 70%".to_string()],
        },
        LiquidityEdge {
            from: "XLM".to_string(),
            to: "USDC".to_string(),
            venue_type: "sdex".to_string(),
            venue_ref: "healthy_offer".to_string(),
            liquidity: 1000,
            price: 1.01,
            fee_bps: 20,
            anomaly_score: 0.0,
            anomaly_reasons: vec![],
        },
    ];

    let routing_policy = RoutingPolicy::default();
    let result = optimizer.find_optimal_routes("XLM", "USDC", &edges, 100, &routing_policy).unwrap();

    // The anomalous route might still be selected if it's better, but it should be flagged
    assert!(result.metrics.anomaly_score > 0.0 || !result.flagged_venues.is_empty());
}
