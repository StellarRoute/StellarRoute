//! Latency-aware adaptive routing with online policy tuning

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptivePolicy {
    /// Min latency budget (ms)
    pub min_latency_ms: u64,
    /// Max latency budget (ms)
    pub max_latency_ms: u64,
    /// Quality threshold (0.0-1.0)
    pub min_quality: f64,
    /// Tuning step size
    pub tuning_step: f64,
}

impl Default for AdaptivePolicy {
    fn default() -> Self {
        Self {
            min_latency_ms: 10,
            max_latency_ms: 500,
            min_quality: 0.7,
            tuning_step: 0.05,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub latency_ms: u64,
    pub quality_score: f64,
    pub routes_explored: usize,
    pub best_price_found: f64,
}

pub struct AdaptiveRouter {
    policy: AdaptivePolicy,
    frozen: Arc<AtomicBool>,
    current_latency_budget: Arc<AtomicU64>,
    total_requests: Arc<AtomicU64>,
    quality_sum: Arc<AtomicU64>,
}

#[derive(Error, Debug)]
pub enum AdaptiveError {
    #[error("Adaptation frozen due to emergency")]
    Frozen,
    #[error("Quality below minimum threshold")]
    QualityBelowThreshold,
}

impl AdaptiveRouter {
    pub fn new(policy: AdaptivePolicy) -> Self {
        let initial_budget = (policy.min_latency_ms + policy.max_latency_ms) / 2;
        Self {
            policy,
            frozen: Arc::new(AtomicBool::new(false)),
            current_latency_budget: Arc::new(AtomicU64::new(initial_budget)),
            total_requests: Arc::new(AtomicU64::new(0)),
            quality_sum: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn freeze(&self) {
        self.frozen.store(true, Ordering::SeqCst);
        warn!("Adaptive routing frozen due to emergency");
    }

    pub fn unfreeze(&self) {
        self.frozen.store(false, Ordering::SeqCst);
        info!("Adaptive routing unfrozen");
    }

    pub fn get_latency_budget(&self) -> u64 {
        self.current_latency_budget.load(Ordering::Acquire)
    }

    pub fn adapt(&self, metrics: &QualityMetrics) -> Result<(), AdaptiveError> {
        if self.frozen.load(Ordering::Acquire) {
            return Err(AdaptiveError::Frozen);
        }

        if metrics.quality_score < self.policy.min_quality {
            return Err(AdaptiveError::QualityBelowThreshold);
        }

        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.quality_sum
            .fetch_add((metrics.quality_score * 1000.0) as u64, Ordering::Relaxed);

        let avg_quality = self.quality_sum.load(Ordering::Acquire) as f64
            / (self.total_requests.load(Ordering::Acquire) as f64 * 1000.0);

        let current_budget = self.get_latency_budget();
        let mut new_budget = current_budget;

        if metrics.latency_ms < current_budget && avg_quality > 0.85 {
            // Can afford to increase latency budget for better quality
            new_budget = ((current_budget as f64) * (1.0 + self.policy.tuning_step)) as u64;
        } else if metrics.latency_ms > current_budget && avg_quality < 0.75 {
            // Need to reduce latency budget
            new_budget = ((current_budget as f64) * (1.0 - self.policy.tuning_step)) as u64;
        }

        new_budget = new_budget
            .max(self.policy.min_latency_ms)
            .min(self.policy.max_latency_ms);

        self.current_latency_budget
            .store(new_budget, Ordering::Release);
        Ok(())
    }

    pub fn avg_quality(&self) -> f64 {
        let total = self.total_requests.load(Ordering::Acquire);
        if total == 0 {
            return 0.0;
        }
        self.quality_sum.load(Ordering::Acquire) as f64 / (total as f64 * 1000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_budget_adjustment() {
        let policy = AdaptivePolicy {
            min_latency_ms: 50,
            max_latency_ms: 500,
            min_quality: 0.7,
            tuning_step: 0.1,
        };

        let router = AdaptiveRouter::new(policy);
        let initial = router.get_latency_budget();

        router
            .adapt(&QualityMetrics {
                latency_ms: initial / 2,
                quality_score: 0.9,
                routes_explored: 10,
                best_price_found: 0.95,
            })
            .unwrap();

        let next = router.get_latency_budget();
        assert!(next > initial);
    }

    #[test]
    fn test_emergency_freeze() {
        let router = AdaptiveRouter::new(AdaptivePolicy::default());
        router.freeze();

        let result = router.adapt(&QualityMetrics {
            latency_ms: 100,
            quality_score: 0.8,
            routes_explored: 5,
            best_price_found: 1.0,
        });

        assert!(matches!(result, Err(AdaptiveError::Frozen)));
    }
}
