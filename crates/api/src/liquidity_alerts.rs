//! Liquidity thinness webhook alerts for orderbook snapshots.

use std::{collections::HashMap, sync::Arc, time::Duration};

use chrono::{DateTime, TimeZone, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::models::{AssetInfo, OrderbookLevel, OrderbookResponse, OrderbookSummary};

const DEFAULT_COOLDOWN_SECONDS: u64 = 300;
const WEBHOOK_ENV: &str = "LIQUIDITY_THINNESS_ALERT_WEBHOOK_URL";
const THRESHOLDS_ENV: &str = "LIQUIDITY_THINNESS_ALERT_THRESHOLDS";

#[derive(Debug, Clone, Deserialize)]
pub struct PairThinnessThreshold {
    #[serde(default)]
    pub min_bid_depth: Option<f64>,
    #[serde(default)]
    pub min_ask_depth: Option<f64>,
    #[serde(default)]
    pub cooldown_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiquidityThinnessAlertPayload {
    pub event: &'static str,
    pub pair: String,
    pub base_asset: AssetInfo,
    pub quote_asset: AssetInfo,
    pub threshold: PairThinnessThresholdSnapshot,
    pub depth_snapshot: DepthSnapshot,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PairThinnessThresholdSnapshot {
    pub min_bid_depth: Option<f64>,
    pub min_ask_depth: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DepthSnapshot {
    pub bid_depth: f64,
    pub ask_depth: f64,
    pub bid_quote_depth: f64,
    pub ask_quote_depth: f64,
    pub bid_levels: usize,
    pub ask_levels: usize,
    pub best_bid: Option<String>,
    pub best_ask: Option<String>,
}

pub struct LiquidityThinnessAlerts {
    webhook_url: Option<String>,
    thresholds: HashMap<String, PairThinnessThreshold>,
    last_sent_at: DashMap<String, DateTime<Utc>>,
    client: reqwest::Client,
}

impl LiquidityThinnessAlerts {
    pub fn from_env() -> Self {
        let webhook_url = std::env::var(WEBHOOK_ENV)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let thresholds = std::env::var(THRESHOLDS_ENV)
            .ok()
            .map(|raw| Self::parse_thresholds(&raw))
            .unwrap_or_default();

        if webhook_url.is_some() && thresholds.is_empty() {
            warn!(
                "{} is set but {} is empty or invalid; liquidity thinness alerts are disabled",
                WEBHOOK_ENV, THRESHOLDS_ENV
            );
        }

        Self {
            webhook_url,
            thresholds,
            last_sent_at: DashMap::new(),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap_or_else(|err| {
                    warn!("Failed to build alert webhook HTTP client: {}", err);
                    reqwest::Client::new()
                }),
        }
    }

    pub fn disabled() -> Self {
        Self {
            webhook_url: None,
            thresholds: HashMap::new(),
            last_sent_at: DashMap::new(),
            client: reqwest::Client::new(),
        }
    }

    fn parse_thresholds(raw: &str) -> HashMap<String, PairThinnessThreshold> {
        match serde_json::from_str::<HashMap<String, PairThinnessThreshold>>(raw) {
            Ok(thresholds) => thresholds
                .into_iter()
                .map(|(pair, threshold)| (normalize_pair_key(&pair), threshold))
                .collect(),
            Err(err) => {
                warn!(
                    "Failed to parse {} as pair threshold JSON: {}",
                    THRESHOLDS_ENV, err
                );
                HashMap::new()
            }
        }
    }

    pub fn with_thresholds(thresholds: HashMap<String, PairThinnessThreshold>) -> Self {
        Self {
            thresholds: thresholds
                .into_iter()
                .map(|(pair, threshold)| (normalize_pair_key(&pair), threshold))
                .collect(),
            ..Self::disabled()
        }
    }

    pub fn maybe_alert(self: &Arc<Self>, orderbook: &OrderbookResponse) {
        let Some(payload) = self.evaluate(orderbook, Utc::now()) else {
            return;
        };

        let Some(webhook_url) = self.webhook_url.clone() else {
            debug!(
                pair = payload.pair,
                "Liquidity thinness threshold breached without configured webhook URL"
            );
            return;
        };

        let alerts = Arc::clone(self);
        tokio::spawn(async move {
            alerts.send(webhook_url, payload).await;
        });
    }

    pub fn evaluate(
        &self,
        orderbook: &OrderbookResponse,
        now: DateTime<Utc>,
    ) -> Option<LiquidityThinnessAlertPayload> {
        let pair = pair_key(&orderbook.base_asset, &orderbook.quote_asset);
        let threshold = self.thresholds.get(&pair)?;
        let depth_snapshot = DepthSnapshot::from_orderbook(orderbook);

        let bid_is_thin = threshold
            .min_bid_depth
            .is_some_and(|minimum| depth_snapshot.bid_depth < minimum);
        let ask_is_thin = threshold
            .min_ask_depth
            .is_some_and(|minimum| depth_snapshot.ask_depth < minimum);

        if !bid_is_thin && !ask_is_thin {
            return None;
        }

        let cooldown = Duration::from_secs(
            threshold
                .cooldown_seconds
                .unwrap_or(DEFAULT_COOLDOWN_SECONDS),
        );
        if self.is_in_cooldown(&pair, now, cooldown) {
            return None;
        }

        self.last_sent_at.insert(pair.clone(), now);

        Some(LiquidityThinnessAlertPayload {
            event: "liquidity_thinness",
            pair,
            base_asset: orderbook.base_asset.clone(),
            quote_asset: orderbook.quote_asset.clone(),
            threshold: PairThinnessThresholdSnapshot {
                min_bid_depth: threshold.min_bid_depth,
                min_ask_depth: threshold.min_ask_depth,
            },
            depth_snapshot,
            timestamp: timestamp_from_orderbook(orderbook).unwrap_or(now),
        })
    }

    fn is_in_cooldown(&self, pair: &str, now: DateTime<Utc>, cooldown: Duration) -> bool {
        let Some(last_sent_at) = self.last_sent_at.get(pair) else {
            return false;
        };

        let elapsed = now
            .signed_duration_since(*last_sent_at)
            .to_std()
            .unwrap_or_default();
        elapsed < cooldown
    }

    async fn send(&self, webhook_url: String, payload: LiquidityThinnessAlertPayload) {
        match self.client.post(&webhook_url).json(&payload).send().await {
            Ok(response) if response.status().is_success() => {
                debug!(
                    pair = payload.pair,
                    status = %response.status(),
                    "Sent liquidity thinness webhook"
                );
            }
            Ok(response) => {
                warn!(
                    pair = payload.pair,
                    status = %response.status(),
                    "Liquidity thinness webhook returned non-success status"
                );
            }
            Err(err) => {
                warn!(
                    pair = payload.pair,
                    error = %err,
                    "Failed to send liquidity thinness webhook"
                );
            }
        }
    }
}

impl DepthSnapshot {
    fn from_orderbook(orderbook: &OrderbookResponse) -> Self {
        let (bid_depth, bid_quote_depth) = side_depth(&orderbook.bids);
        let (ask_depth, ask_quote_depth) = side_depth(&orderbook.asks);

        Self {
            bid_depth,
            ask_depth,
            bid_quote_depth,
            ask_quote_depth,
            bid_levels: orderbook.bids.len(),
            ask_levels: orderbook.asks.len(),
            best_bid: orderbook.bids.first().map(|level| level.price.clone()),
            best_ask: orderbook.asks.first().map(|level| level.price.clone()),
        }
    }
}

fn side_depth(levels: &[OrderbookLevel]) -> (f64, f64) {
    levels.iter().fold((0.0, 0.0), |(base, quote), level| {
        let amount = level.amount.parse::<f64>().unwrap_or_default();
        let price = level.price.parse::<f64>().unwrap_or_default();
        (base + amount, quote + amount * price)
    })
}

fn pair_key(base: &AssetInfo, quote: &AssetInfo) -> String {
    let (norm_base, norm_quote) =
        stellarroute_routing::normalize_pair_owned(&base.to_canonical(), &quote.to_canonical());
    format!("{}/{}", norm_base, norm_quote)
}

fn normalize_pair_key(pair: &str) -> String {
    pair.split_once('/')
        .map(|(base, quote)| {
            let (norm_base, norm_quote) = stellarroute_routing::normalize_pair_owned(base, quote);
            format!("{}/{}", norm_base, norm_quote)
        })
        .unwrap_or_else(|| stellarroute_routing::normalize_asset(pair))
}

fn timestamp_from_orderbook(orderbook: &OrderbookResponse) -> Option<DateTime<Utc>> {
    Utc.timestamp_opt(orderbook.timestamp, 0).single()
}

#[cfg(test)]
mod tests {
    use super::*;

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
            summary: OrderbookSummary {
                bid: None,
                ask: None,
                spread_bps: None,
                midpoint: None,
            },
            timestamp: 1_717_171_717,
        }
    }

    #[test]
    fn synthetic_thin_orderbook_builds_alert_payload() {
        let alerts = LiquidityThinnessAlerts::with_thresholds(HashMap::from([(
            "native/USDC".to_string(),
            PairThinnessThreshold {
                min_bid_depth: Some(100.0),
                min_ask_depth: Some(50.0),
                cooldown_seconds: Some(60),
            },
        )]));

        let payload = alerts
            .evaluate(
                &orderbook(vec![("0.11", "25.0")], vec![("0.12", "70.0")]),
                Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap(),
            )
            .expect("thin bids should alert");

        assert_eq!(payload.event, "liquidity_thinness");
        assert_eq!(payload.pair, "native/USDC");
        assert_eq!(payload.depth_snapshot.bid_depth, 25.0);
        assert_eq!(payload.depth_snapshot.ask_depth, 70.0);
        assert_eq!(payload.threshold.min_bid_depth, Some(100.0));
    }

    #[test]
    fn healthy_synthetic_orderbook_does_not_alert() {
        let alerts = LiquidityThinnessAlerts::with_thresholds(HashMap::from([(
            "XLM/USDC".to_string(),
            PairThinnessThreshold {
                min_bid_depth: Some(100.0),
                min_ask_depth: Some(50.0),
                cooldown_seconds: Some(60),
            },
        )]));

        let payload = alerts.evaluate(
            &orderbook(vec![("0.11", "125.0")], vec![("0.12", "70.0")]),
            Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap(),
        );

        assert!(payload.is_none());
    }

    #[test]
    fn cooldown_suppresses_repeated_thin_orderbook_alerts() {
        let alerts = LiquidityThinnessAlerts::with_thresholds(HashMap::from([(
            "native/USDC".to_string(),
            PairThinnessThreshold {
                min_bid_depth: Some(100.0),
                min_ask_depth: None,
                cooldown_seconds: Some(120),
            },
        )]));
        let thin = orderbook(vec![("0.11", "25.0")], vec![("0.12", "70.0")]);
        let start = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();

        assert!(alerts.evaluate(&thin, start).is_some());
        assert!(alerts
            .evaluate(&thin, start + chrono::Duration::seconds(60))
            .is_none());
        assert!(alerts
            .evaluate(&thin, start + chrono::Duration::seconds(121))
            .is_some());
    }
}
