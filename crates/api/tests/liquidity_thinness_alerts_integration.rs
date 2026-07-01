use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use stellarroute_api::liquidity_alerts::{
    DepthSnapshot, LiquidityThinnessAlertPayload, LiquidityThinnessAlerts, PairThinnessThreshold,
    PairThinnessThresholdSnapshot,
};
use stellarroute_api::models::{AssetInfo, OrderbookLevel, OrderbookResponse};
use tokio::time::sleep;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn orderbook(bids: Vec<(&str, &str)>, asks: Vec<(&str, &str)>) -> OrderbookResponse {
    OrderbookResponse {
        base_asset: AssetInfo::native(),
        quote_asset: AssetInfo::credit("USDC".to_string(), None),
        bids: bids
            .into_iter()
            .map(|(price, amount)| OrderbookLevel {
                price: price.to_string(),
                amount: amount.to_string(),
                total: "0".to_string(),
            })
            .collect(),
        asks: asks
            .into_iter()
            .map(|(price, amount)| OrderbookLevel {
                price: price.to_string(),
                amount: amount.to_string(),
                total: "0".to_string(),
            })
            .collect(),
        timestamp: 1717171717,
    }
}

#[tokio::test]
async fn test_alert_fires_when_depth_below_threshold() {
    let mock_server = MockServer::start().await;

    let threshold = PairThinnessThreshold {
        min_bid_depth: Some(100.0),
        min_ask_depth: Some(50.0),
        cooldown_seconds: Some(60),
    };
    let mut thresholds = HashMap::new();
    thresholds.insert("native/USDC".to_string(), threshold);
    let alerts = Arc::new(LiquidityThinnessAlerts::with_thresholds_and_url(
        thresholds,
        Some(mock_server.uri()),
    ));

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock_server)
        .await;

    let thin_orderbook = orderbook(vec![("0.11", "25.0")], vec![("0.12", "70.0")]);
    alerts.maybe_alert(&thin_orderbook);

    sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_alert_does_not_fire_when_depth_above_threshold() {
    let mock_server = MockServer::start().await;

    let threshold = PairThinnessThreshold {
        min_bid_depth: Some(100.0),
        min_ask_depth: Some(50.0),
        cooldown_seconds: Some(60),
    };
    let mut thresholds = HashMap::new();
    thresholds.insert("native/USDC".to_string(), threshold);
    let alerts = Arc::new(LiquidityThinnessAlerts::with_thresholds_and_url(
        thresholds,
        Some(mock_server.uri()),
    ));

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&mock_server)
        .await;

    let healthy_orderbook = orderbook(vec![("0.11", "125.0")], vec![("0.12", "70.0")]);
    alerts.maybe_alert(&healthy_orderbook);

    sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_no_panic_when_webhook_url_unset() {
    let threshold = PairThinnessThreshold {
        min_bid_depth: Some(100.0),
        min_ask_depth: Some(50.0),
        cooldown_seconds: Some(60),
    };
    let mut thresholds = HashMap::new();
    thresholds.insert("native/USDC".to_string(), threshold);
    let alerts = Arc::new(LiquidityThinnessAlerts::with_thresholds_and_url(
        thresholds, None,
    ));

    let thin_orderbook = orderbook(vec![("0.11", "25.0")], vec![("0.12", "70.0")]);
    alerts.maybe_alert(&thin_orderbook);

    sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_webhook_failure_retry() {
    let mock_server = MockServer::start().await;

    let threshold = PairThinnessThreshold {
        min_bid_depth: Some(100.0),
        min_ask_depth: Some(50.0),
        cooldown_seconds: Some(60),
    };
    let mut thresholds = HashMap::new();
    thresholds.insert("native/USDC".to_string(), threshold);
    let alerts = Arc::new(LiquidityThinnessAlerts::with_thresholds_and_url(
        thresholds,
        Some(mock_server.uri()),
    ));

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(500))
        .expect(2)
        .mount(&mock_server)
        .await;

    let thin_orderbook = orderbook(vec![("0.11", "25.0")], vec![("0.12", "70.0")]);
    alerts.maybe_alert(&thin_orderbook);

    sleep(Duration::from_millis(3000)).await;
}
