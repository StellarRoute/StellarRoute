use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    pub base_timeout_ms: u64,
    pub min_timeout_ms: u64,
    pub max_timeout_ms: u64,
    pub ema_alpha: f64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            base_timeout_ms: 500,
            min_timeout_ms: 100,
            max_timeout_ms: 2000,
            ema_alpha: 0.1,
        }
    }
}

pub struct TimeoutController {
    config: TimeoutConfig,
    ema_latency_ms: AtomicU64,
}

impl TimeoutController {
    pub fn new(config: TimeoutConfig) -> Self {
        let base_ms = config.base_timeout_ms;
        Self {
            config,
            ema_latency_ms: AtomicU64::new(base_ms),
        }
    }

    pub fn record_latency(&self, latency: Duration) {
        let latency_ms = latency.as_millis() as u64;
        let mut current_ema = self.ema_latency_ms.load(Ordering::Relaxed);

        loop {
            let new_ema = (current_ema as f64 * (1.0 - self.config.ema_alpha)
                + latency_ms as f64 * self.config.ema_alpha) as u64;

            match self.ema_latency_ms.compare_exchange_weak(
                current_ema,
                new_ema,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_ema = actual,
            }
        }
    }

    pub fn calculate_timeout(&self, health_score: f64) -> Duration {
        let ema = self.ema_latency_ms.load(Ordering::Relaxed);

        // Timeout increases as health score decreases (protections against slow deps)
        // or decreases as health score increases?
        // Actually, if health is poor (score -> 0), we want SHORTER timeouts to fail fast.
        // If health is good (score -> 1), we can afford the base timeout.

        let adjusted_ema = (ema as f64 * 1.5).max(self.config.base_timeout_ms as f64);

        // Scale by health score: if health is 0.5, we might want to cap the timeout more strictly.
        let target_ms = adjusted_ema * health_score.max(0.2); // minimum 20% of adjusted ema

        let final_ms = target_ms as u64;
        let clamped_ms = final_ms.clamp(self.config.min_timeout_ms, self.config.max_timeout_ms);

        Duration::from_millis(clamped_ms)
    }

    pub fn current_ema_ms(&self) -> u64 {
        self.ema_latency_ms.load(Ordering::Relaxed)
    }
}
