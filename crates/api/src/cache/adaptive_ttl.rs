//! Adaptive quote TTL engine based on market volatility and depth
//!
//! Computes TTL values dynamically using volatility and liquidity depth metrics
//! to balance quote freshness with cache performance.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveTtlConfig {
    pub min_ttl_ms: u64,
    pub max_ttl_ms: u64,
    pub base_ttl_ms: u64,
    pub volatility_weight: f64,
    pub depth_weight: f64,
    pub volatility_threshold_low: f64,
    pub volatility_threshold_high: f64,
    pub depth_threshold_low: f64,
    pub depth_threshold_high: f64,
}

impl Default for AdaptiveTtlConfig {
    fn default() -> Self {
        Self {
            min_ttl_ms: 500,
            max_ttl_ms: 30_000,
            base_ttl_ms: 5_000,
            volatility_weight: 0.6,
            depth_weight: 0.4,
            volatility_threshold_low: 0.001,
            volatility_threshold_high: 0.05,
            depth_threshold_low: 1_000.0,
            depth_threshold_high: 1_000_000.0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MarketMetrics {
    pub volatility: f64,
    pub depth: f64,
    pub last_price: f64,
    pub price_change_1m: f64,
    pub trade_count_1m: u32,
}

#[derive(Debug, Clone)]
pub struct TtlDecision {
    pub ttl: Duration,
    pub ttl_ms: u64,
    pub volatility_factor: f64,
    pub depth_factor: f64,
    pub reason: TtlReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TtlReason {
    HighVolatility,
    LowVolatility,
    LowDepth,
    HighDepth,
    Balanced,
    NoMetrics,
}

impl std::fmt::Display for TtlReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TtlReason::HighVolatility => write!(f, "high_volatility"),
            TtlReason::LowVolatility => write!(f, "low_volatility"),
            TtlReason::LowDepth => write!(f, "low_depth"),
            TtlReason::HighDepth => write!(f, "high_depth"),
            TtlReason::Balanced => write!(f, "balanced"),
            TtlReason::NoMetrics => write!(f, "no_metrics"),
        }
    }
}

pub struct AdaptiveTtlEngine {
    config: AdaptiveTtlConfig,
    metrics: Arc<RwLock<HashMap<String, MarketMetrics>>>,
    decisions: Arc<RwLock<Vec<TtlDecisionRecord>>>,
}

#[derive(Debug, Clone)]
struct TtlDecisionRecord {
    _pair: String,
    decision: TtlDecision,
    _timestamp: std::time::Instant,
}

impl AdaptiveTtlEngine {
    pub fn new(config: AdaptiveTtlConfig) -> Self {
        info!(
            min_ttl_ms = config.min_ttl_ms,
            max_ttl_ms = config.max_ttl_ms,
            base_ttl_ms = config.base_ttl_ms,
            "Adaptive TTL engine initialized"
        );

        Self {
            config,
            metrics: Arc::new(RwLock::new(HashMap::new())),
            decisions: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn update_metrics(&self, pair: &str, metrics: MarketMetrics) {
        let mut store = self.metrics.write().await;
        store.insert(pair.to_string(), metrics);
        debug!(pair = pair, "Market metrics updated");
    }

    pub async fn compute_ttl(&self, pair: &str) -> TtlDecision {
        let metrics = self.metrics.read().await;

        let decision = if let Some(m) = metrics.get(pair) {
            self.calculate_ttl(m)
        } else {
            TtlDecision {
                ttl: Duration::from_millis(self.config.base_ttl_ms),
                ttl_ms: self.config.base_ttl_ms,
                volatility_factor: 1.0,
                depth_factor: 1.0,
                reason: TtlReason::NoMetrics,
            }
        };

        drop(metrics);

        let record = TtlDecisionRecord {
            _pair: pair.to_string(),
            decision: decision.clone(),
            _timestamp: std::time::Instant::now(),
        };

        let mut decisions = self.decisions.write().await;
        decisions.push(record);

        if decisions.len() > 10_000 {
            decisions.drain(0..5_000);
        }

        debug!(
            pair = pair,
            ttl_ms = decision.ttl_ms,
            volatility_factor = decision.volatility_factor,
            depth_factor = decision.depth_factor,
            reason = %decision.reason,
            "TTL computed"
        );

        decision
    }

    fn calculate_ttl(&self, metrics: &MarketMetrics) -> TtlDecision {
        let volatility_factor = self.compute_volatility_factor(metrics.volatility);
        let depth_factor = self.compute_depth_factor(metrics.depth);

        let combined_factor = (volatility_factor * self.config.volatility_weight)
            + (depth_factor * self.config.depth_weight);

        let ttl_ms = (self.config.base_ttl_ms as f64 * combined_factor) as u64;
        let ttl_ms = ttl_ms.clamp(self.config.min_ttl_ms, self.config.max_ttl_ms);

        let reason = self.determine_reason(metrics, volatility_factor, depth_factor);

        TtlDecision {
            ttl: Duration::from_millis(ttl_ms),
            ttl_ms,
            volatility_factor,
            depth_factor,
            reason,
        }
    }

    fn compute_volatility_factor(&self, volatility: f64) -> f64 {
        if volatility <= self.config.volatility_threshold_low {
            2.0
        } else if volatility >= self.config.volatility_threshold_high {
            0.2
        } else {
            let range =
                self.config.volatility_threshold_high - self.config.volatility_threshold_low;
            let normalized = (volatility - self.config.volatility_threshold_low) / range;
            2.0 - (1.8 * normalized)
        }
    }

    fn compute_depth_factor(&self, depth: f64) -> f64 {
        if depth <= self.config.depth_threshold_low {
            0.5
        } else if depth >= self.config.depth_threshold_high {
            1.5
        } else {
            let range = self.config.depth_threshold_high - self.config.depth_threshold_low;
            let normalized = (depth - self.config.depth_threshold_low) / range;
            0.5 + (1.0 * normalized)
        }
    }

    fn determine_reason(
        &self,
        metrics: &MarketMetrics,
        _volatility_factor: f64,
        _depth_factor: f64,
    ) -> TtlReason {
        if metrics.volatility >= self.config.volatility_threshold_high {
            TtlReason::HighVolatility
        } else if metrics.volatility <= self.config.volatility_threshold_low {
            TtlReason::LowVolatility
        } else if metrics.depth <= self.config.depth_threshold_low {
            TtlReason::LowDepth
        } else if metrics.depth >= self.config.depth_threshold_high {
            TtlReason::HighDepth
        } else {
            TtlReason::Balanced
        }
    }

    pub async fn get_stats(&self) -> AdaptiveTtlStats {
        let decisions = self.decisions.read().await;
        let metrics = self.metrics.read().await;

        let total_decisions = decisions.len();
        let mut reason_counts: HashMap<TtlReason, usize> = HashMap::new();
        let mut total_ttl_ms: u64 = 0;

        for record in decisions.iter() {
            *reason_counts.entry(record.decision.reason).or_insert(0) += 1;
            total_ttl_ms += record.decision.ttl_ms;
        }

        let avg_ttl_ms = if total_decisions > 0 {
            total_ttl_ms / total_decisions as u64
        } else {
            0
        };

        AdaptiveTtlStats {
            total_decisions,
            tracked_pairs: metrics.len(),
            avg_ttl_ms,
            reason_breakdown: reason_counts,
        }
    }

    pub async fn get_metrics(&self, pair: &str) -> Option<MarketMetrics> {
        let metrics = self.metrics.read().await;
        metrics.get(pair).cloned()
    }

    pub async fn clear_metrics(&self, pair: &str) {
        let mut metrics = self.metrics.write().await;
        metrics.remove(pair);
    }
}

#[derive(Debug, Clone)]
pub struct AdaptiveTtlStats {
    pub total_decisions: usize,
    pub tracked_pairs: usize,
    pub avg_ttl_ms: u64,
    pub reason_breakdown: HashMap<TtlReason, usize>,
}

pub struct VolatilityCalculator {
    window_size: usize,
    prices: HashMap<String, Vec<f64>>,
}

impl VolatilityCalculator {
    pub fn new(window_size: usize) -> Self {
        Self {
            window_size,
            prices: HashMap::new(),
        }
    }

    pub fn add_price(&mut self, pair: &str, price: f64) {
        let prices = self.prices.entry(pair.to_string()).or_default();
        prices.push(price);

        if prices.len() > self.window_size {
            prices.remove(0);
        }
    }

    pub fn calculate_volatility(&self, pair: &str) -> Option<f64> {
        let prices = self.prices.get(pair)?;

        if prices.len() < 2 {
            return None;
        }

        let returns: Vec<f64> = prices.windows(2).map(|w| (w[1] - w[0]) / w[0]).collect();

        if returns.is_empty() {
            return None;
        }

        let mean = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance =
            returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / returns.len() as f64;

        Some(variance.sqrt())
    }
}

pub struct DepthAggregator {
    depths: HashMap<String, f64>,
}

impl DepthAggregator {
    pub fn new() -> Self {
        Self {
            depths: HashMap::new(),
        }
    }

    pub fn update_depth(&mut self, pair: &str, bid_depth: f64, ask_depth: f64) {
        let total_depth = bid_depth + ask_depth;
        self.depths.insert(pair.to_string(), total_depth);
    }

    pub fn get_depth(&self, pair: &str) -> Option<f64> {
        self.depths.get(pair).copied()
    }
}

impl Default for DepthAggregator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> AdaptiveTtlConfig {
        AdaptiveTtlConfig::default()
    }

    #[tokio::test]
    async fn test_no_metrics_returns_base_ttl() {
        let engine = AdaptiveTtlEngine::new(test_config());
        let decision = engine.compute_ttl("XLM/USDC").await;

        assert_eq!(decision.ttl_ms, 5_000);
        assert_eq!(decision.reason, TtlReason::NoMetrics);
    }

    #[tokio::test]
    async fn test_high_volatility_reduces_ttl() {
        let engine = AdaptiveTtlEngine::new(test_config());

        let metrics = MarketMetrics {
            volatility: 0.10,
            depth: 500_000.0,
            ..Default::default()
        };
        engine.update_metrics("XLM/USDC", metrics).await;

        let decision = engine.compute_ttl("XLM/USDC").await;

        assert!(decision.ttl_ms < 5_000);
        assert_eq!(decision.reason, TtlReason::HighVolatility);
    }

    #[tokio::test]
    async fn test_low_volatility_increases_ttl() {
        let engine = AdaptiveTtlEngine::new(test_config());

        let metrics = MarketMetrics {
            volatility: 0.0005,
            depth: 500_000.0,
            ..Default::default()
        };
        engine.update_metrics("XLM/USDC", metrics).await;

        let decision = engine.compute_ttl("XLM/USDC").await;

        assert!(decision.ttl_ms > 5_000);
        assert_eq!(decision.reason, TtlReason::LowVolatility);
    }

    #[tokio::test]
    async fn test_low_depth_reduces_ttl() {
        let engine = AdaptiveTtlEngine::new(test_config());

        let metrics = MarketMetrics {
            volatility: 0.01,
            depth: 500.0,
            ..Default::default()
        };
        engine.update_metrics("XLM/USDC", metrics).await;

        let decision = engine.compute_ttl("XLM/USDC").await;

        assert_eq!(decision.reason, TtlReason::LowDepth);
    }

    #[tokio::test]
    async fn test_high_depth_increases_ttl() {
        let engine = AdaptiveTtlEngine::new(test_config());

        let metrics = MarketMetrics {
            volatility: 0.01,
            depth: 2_000_000.0,
            ..Default::default()
        };
        engine.update_metrics("XLM/USDC", metrics).await;

        let decision = engine.compute_ttl("XLM/USDC").await;

        assert_eq!(decision.reason, TtlReason::HighDepth);
    }

    #[tokio::test]
    async fn test_ttl_bounded_by_min_max() {
        let config = AdaptiveTtlConfig {
            min_ttl_ms: 1_000,
            max_ttl_ms: 10_000,
            ..Default::default()
        };
        let engine = AdaptiveTtlEngine::new(config);

        let extreme_volatile = MarketMetrics {
            volatility: 1.0,
            depth: 100.0,
            ..Default::default()
        };
        engine
            .update_metrics("VOLATILE/USD", extreme_volatile)
            .await;

        let decision = engine.compute_ttl("VOLATILE/USD").await;
        assert!(decision.ttl_ms >= 1_000);

        let calm = MarketMetrics {
            volatility: 0.0,
            depth: 10_000_000.0,
            ..Default::default()
        };
        engine.update_metrics("CALM/USD", calm).await;

        let decision = engine.compute_ttl("CALM/USD").await;
        assert!(decision.ttl_ms <= 10_000);
    }

    #[tokio::test]
    async fn test_balanced_market_conditions() {
        let engine = AdaptiveTtlEngine::new(test_config());

        let metrics = MarketMetrics {
            volatility: 0.02,
            depth: 100_000.0,
            ..Default::default()
        };
        engine.update_metrics("XLM/USDC", metrics).await;

        let decision = engine.compute_ttl("XLM/USDC").await;

        assert_eq!(decision.reason, TtlReason::Balanced);
    }

    #[tokio::test]
    async fn test_stats_tracking() {
        let engine = AdaptiveTtlEngine::new(test_config());

        let volatile = MarketMetrics {
            volatility: 0.10,
            depth: 500_000.0,
            ..Default::default()
        };
        engine.update_metrics("PAIR1", volatile).await;
        engine.compute_ttl("PAIR1").await;

        let calm = MarketMetrics {
            volatility: 0.0005,
            depth: 500_000.0,
            ..Default::default()
        };
        engine.update_metrics("PAIR2", calm).await;
        engine.compute_ttl("PAIR2").await;

        let stats = engine.get_stats().await;

        assert_eq!(stats.total_decisions, 2);
        assert_eq!(stats.tracked_pairs, 2);
        assert!(stats
            .reason_breakdown
            .contains_key(&TtlReason::HighVolatility));
        assert!(stats
            .reason_breakdown
            .contains_key(&TtlReason::LowVolatility));
    }

    #[test]
    fn test_volatility_calculator() {
        let mut calc = VolatilityCalculator::new(10);

        calc.add_price("XLM/USDC", 0.10);
        calc.add_price("XLM/USDC", 0.11);
        calc.add_price("XLM/USDC", 0.105);
        calc.add_price("XLM/USDC", 0.12);

        let vol = calc.calculate_volatility("XLM/USDC");
        assert!(vol.is_some());
        assert!(vol.unwrap() > 0.0);
    }

    #[test]
    fn test_depth_aggregator() {
        let mut agg = DepthAggregator::new();

        agg.update_depth("XLM/USDC", 50_000.0, 60_000.0);

        let depth = agg.get_depth("XLM/USDC");
        assert_eq!(depth, Some(110_000.0));
    }

    #[test]
    fn test_volatility_calculator_window() {
        let mut calc = VolatilityCalculator::new(3);

        calc.add_price("PAIR", 1.0);
        calc.add_price("PAIR", 1.1);
        calc.add_price("PAIR", 1.2);
        calc.add_price("PAIR", 1.3);

        let prices = &calc.prices["PAIR"];
        assert_eq!(prices.len(), 3);
        assert_eq!(prices[0], 1.1);
        assert_eq!(prices[2], 1.3);
    }
}
