use stellarroute_api::cache::{AdaptiveTtlConfig, AdaptiveTtlEngine, MarketMetrics, TtlReason};

fn calm_market_config() -> AdaptiveTtlConfig {
    AdaptiveTtlConfig {
        min_ttl_ms: 1_000,
        max_ttl_ms: 60_000,
        base_ttl_ms: 10_000,
        volatility_weight: 0.6,
        depth_weight: 0.4,
        volatility_threshold_low: 0.001,
        volatility_threshold_high: 0.05,
        depth_threshold_low: 10_000.0,
        depth_threshold_high: 1_000_000.0,
    }
}

fn volatile_market_config() -> AdaptiveTtlConfig {
    AdaptiveTtlConfig {
        min_ttl_ms: 100,
        max_ttl_ms: 5_000,
        base_ttl_ms: 1_000,
        volatility_weight: 0.8,
        depth_weight: 0.2,
        volatility_threshold_low: 0.0005,
        volatility_threshold_high: 0.02,
        depth_threshold_low: 5_000.0,
        depth_threshold_high: 500_000.0,
    }
}

#[tokio::test]
async fn test_calm_market_fixture() {
    let engine = AdaptiveTtlEngine::new(calm_market_config());

    let metrics = MarketMetrics {
        volatility: 0.0005,
        depth: 500_000.0,
        last_price: 0.10,
        price_change_1m: 0.0001,
        trade_count_1m: 10,
    };
    engine.update_metrics("XLM/USDC", metrics).await;

    let decision = engine.compute_ttl("XLM/USDC").await;

    assert!(
        decision.ttl_ms > 10_000,
        "Calm market should have longer TTL"
    );
    assert_eq!(decision.reason, TtlReason::LowVolatility);
}

#[tokio::test]
async fn test_volatile_market_fixture() {
    let engine = AdaptiveTtlEngine::new(volatile_market_config());

    let metrics = MarketMetrics {
        volatility: 0.05,
        depth: 100_000.0,
        last_price: 0.08,
        price_change_1m: 0.02,
        trade_count_1m: 500,
    };
    engine.update_metrics("XLM/USDC", metrics).await;

    let decision = engine.compute_ttl("XLM/USDC").await;

    assert!(
        decision.ttl_ms < 1_000,
        "Volatile market should have shorter TTL"
    );
    assert_eq!(decision.reason, TtlReason::HighVolatility);
}

#[tokio::test]
async fn test_multiple_pairs_independent_ttl() {
    let engine = AdaptiveTtlEngine::new(calm_market_config());

    let calm_metrics = MarketMetrics {
        volatility: 0.0005,
        depth: 800_000.0,
        ..Default::default()
    };
    engine.update_metrics("USDC/EUR", calm_metrics).await;

    let volatile_metrics = MarketMetrics {
        volatility: 0.08,
        depth: 50_000.0,
        ..Default::default()
    };
    engine.update_metrics("DOGE/XLM", volatile_metrics).await;

    let calm_decision = engine.compute_ttl("USDC/EUR").await;
    let volatile_decision = engine.compute_ttl("DOGE/XLM").await;

    assert!(calm_decision.ttl_ms > volatile_decision.ttl_ms);
}

#[tokio::test]
async fn test_metrics_update_changes_ttl() {
    let engine = AdaptiveTtlEngine::new(calm_market_config());

    let initial_metrics = MarketMetrics {
        volatility: 0.001,
        depth: 200_000.0,
        ..Default::default()
    };
    engine.update_metrics("XLM/USDC", initial_metrics).await;
    let initial_decision = engine.compute_ttl("XLM/USDC").await;

    let volatile_metrics = MarketMetrics {
        volatility: 0.10,
        depth: 50_000.0,
        ..Default::default()
    };
    engine.update_metrics("XLM/USDC", volatile_metrics).await;
    let volatile_decision = engine.compute_ttl("XLM/USDC").await;

    assert!(initial_decision.ttl_ms > volatile_decision.ttl_ms);
}

#[tokio::test]
async fn test_stats_accumulate_correctly() {
    let engine = AdaptiveTtlEngine::new(calm_market_config());

    for i in 0..10 {
        let metrics = MarketMetrics {
            volatility: 0.001 * (i as f64 + 1.0),
            depth: 100_000.0,
            ..Default::default()
        };
        engine.update_metrics(&format!("PAIR{}", i), metrics).await;
        engine.compute_ttl(&format!("PAIR{}", i)).await;
    }

    let stats = engine.get_stats().await;
    assert_eq!(stats.total_decisions, 10);
    assert_eq!(stats.tracked_pairs, 10);
    assert!(stats.avg_ttl_ms > 0);
}

#[tokio::test]
async fn test_depth_only_market_conditions() {
    let config = AdaptiveTtlConfig {
        volatility_weight: 0.0,
        depth_weight: 1.0,
        ..calm_market_config()
    };
    let engine = AdaptiveTtlEngine::new(config);

    let low_depth = MarketMetrics {
        volatility: 0.02,
        depth: 5_000.0,
        ..Default::default()
    };
    engine.update_metrics("LOW/USD", low_depth).await;

    let high_depth = MarketMetrics {
        volatility: 0.02,
        depth: 2_000_000.0,
        ..Default::default()
    };
    engine.update_metrics("HIGH/USD", high_depth).await;

    let low_decision = engine.compute_ttl("LOW/USD").await;
    let high_decision = engine.compute_ttl("HIGH/USD").await;

    assert!(low_decision.ttl_ms < high_decision.ttl_ms);
}

#[tokio::test]
async fn test_ttl_observability() {
    let engine = AdaptiveTtlEngine::new(calm_market_config());

    let metrics = MarketMetrics {
        volatility: 0.02,
        depth: 300_000.0,
        ..Default::default()
    };
    engine.update_metrics("XLM/USDC", metrics).await;

    let decision = engine.compute_ttl("XLM/USDC").await;

    assert!(decision.volatility_factor > 0.0);
    assert!(decision.depth_factor > 0.0);
    assert!(decision.ttl_ms > 0);

    let stored_metrics = engine.get_metrics("XLM/USDC").await;
    assert!(stored_metrics.is_some());
}
