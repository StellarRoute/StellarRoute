//! Adaptive backpressure and rate-limit handling for Horizon ingestion.
//!
//! When Horizon returns HTTP 429 the indexer must:
//! 1. Respect the `Retry-After` header when present.
//! 2. Apply full-jitter exponential backoff otherwise.
//! 3. Preserve cursor progress so no work is lost.
//! 4. Emit metrics so operators can observe throttle events and lag.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

/// Shared throttle state that survives across polling iterations.
#[derive(Debug, Clone)]
pub struct ThrottleState {
    inner: Arc<ThrottleInner>,
}

#[derive(Debug)]
struct ThrottleInner {
    /// Total number of 429 responses received.
    throttle_events: AtomicU64,
    /// Total milliseconds spent waiting due to throttling.
    throttle_wait_ms: AtomicU64,
    /// Current consecutive 429 count (resets on success).
    consecutive_429s: AtomicU64,
}

impl Default for ThrottleState {
    fn default() -> Self {
        Self::new()
    }
}

impl ThrottleState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(ThrottleInner {
                throttle_events: AtomicU64::new(0),
                throttle_wait_ms: AtomicU64::new(0),
                consecutive_429s: AtomicU64::new(0),
            }),
        }
    }

    /// Record a successful request — resets the consecutive counter.
    pub fn record_success(&self) {
        self.inner.consecutive_429s.store(0, Ordering::Relaxed);
    }

    /// Record a 429 response and return the delay to wait before retrying.
    ///
    /// Priority order for the delay:
    /// 1. `retry_after_secs` from the `Retry-After` header (if present and > 0).
    /// 2. Full-jitter exponential backoff based on consecutive 429 count.
    pub fn record_rate_limit(
        &self,
        retry_after_secs: Option<u64>,
        config: &BackoffConfig,
    ) -> Duration {
        let consecutive = self.inner.consecutive_429s.fetch_add(1, Ordering::Relaxed) + 1;
        self.inner.throttle_events.fetch_add(1, Ordering::Relaxed);

        let delay = if let Some(secs) = retry_after_secs.filter(|&s| s > 0) {
            // Honour the server's Retry-After directive.
            Duration::from_secs(secs)
        } else {
            // Full-jitter exponential backoff:
            //   cap = min(max_delay, base * 2^n)
            //   sleep = random(0, cap)
            let exp: u32 = (consecutive as u32).min(30);
            let cap_ms = config
                .base_delay_ms
                .saturating_mul(1u64 << exp)
                .min(config.max_delay_ms);
            let jitter_ms = (rand::random::<f64>() * cap_ms as f64) as u64;
            Duration::from_millis(jitter_ms.max(config.min_delay_ms))
        };

        self.inner
            .throttle_wait_ms
            .fetch_add(delay.as_millis() as u64, Ordering::Relaxed);

        warn!(
            consecutive_429s = consecutive,
            delay_ms = delay.as_millis(),
            retry_after_secs = retry_after_secs,
            "Horizon rate-limit hit; backing off"
        );

        delay
    }

    /// Total throttle events observed since process start.
    pub fn throttle_events(&self) -> u64 {
        self.inner.throttle_events.load(Ordering::Relaxed)
    }

    /// Total milliseconds spent waiting due to throttling.
    pub fn throttle_wait_ms(&self) -> u64 {
        self.inner.throttle_wait_ms.load(Ordering::Relaxed)
    }

    /// Current consecutive 429 count.
    pub fn consecutive_429s(&self) -> u64 {
        self.inner.consecutive_429s.load(Ordering::Relaxed)
    }
}

/// Configuration for the adaptive backoff algorithm.
#[derive(Debug, Clone)]
pub struct BackoffConfig {
    /// Minimum delay in milliseconds (floor for jitter).
    pub min_delay_ms: u64,
    /// Base delay in milliseconds used for the exponential calculation.
    pub base_delay_ms: u64,
    /// Maximum delay cap in milliseconds.
    pub max_delay_ms: u64,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            min_delay_ms: 500,
            base_delay_ms: 1_000,
            max_delay_ms: 60_000,
        }
    }
}

/// Parse the `Retry-After` header value.
///
/// Supports both integer seconds (`Retry-After: 30`) and HTTP-date formats.
/// Returns `None` when the header is absent or unparseable.
pub fn parse_retry_after(header_value: Option<&str>) -> Option<u64> {
    let value = header_value?.trim();

    // Try integer seconds first.
    if let Ok(secs) = value.parse::<u64>() {
        return Some(secs);
    }

    // Try HTTP-date (e.g. "Wed, 21 Oct 2015 07:28:00 GMT").
    // We compute the delta from now; if parsing fails we return None.
    if let Ok(dt) = httpdate::parse_http_date(value) {
        let now = std::time::SystemTime::now();
        if let Ok(delta) = dt.duration_since(now) {
            return Some(delta.as_secs());
        }
    }

    None
}

/// Perform a single throttled sleep, emitting a structured log.
pub async fn throttle_sleep(delay: Duration) {
    info!(
        delay_ms = delay.as_millis(),
        "Throttle backoff: sleeping before next Horizon request"
    );
    tokio::time::sleep(delay).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_retry_after_integer() {
        assert_eq!(parse_retry_after(Some("30")), Some(30));
        assert_eq!(parse_retry_after(Some("0")), Some(0));
        assert_eq!(parse_retry_after(None), None);
        assert_eq!(parse_retry_after(Some("not-a-number")), None);
    }

    #[test]
    fn test_throttle_state_success_resets_consecutive() {
        let state = ThrottleState::new();
        let cfg = BackoffConfig::default();
        state.record_rate_limit(Some(1), &cfg);
        state.record_rate_limit(Some(1), &cfg);
        assert_eq!(state.consecutive_429s(), 2);
        state.record_success();
        assert_eq!(state.consecutive_429s(), 0);
    }

    #[test]
    fn test_throttle_state_respects_retry_after() {
        let state = ThrottleState::new();
        let cfg = BackoffConfig::default();
        let delay = state.record_rate_limit(Some(10), &cfg);
        assert_eq!(delay, Duration::from_secs(10));
    }

    #[test]
    fn test_throttle_state_jitter_within_bounds() {
        let state = ThrottleState::new();
        let cfg = BackoffConfig {
            min_delay_ms: 100,
            base_delay_ms: 200,
            max_delay_ms: 5_000,
        };
        for _ in 0..50 {
            let delay = state.record_rate_limit(None, &cfg);
            assert!(delay.as_millis() >= 100);
            assert!(delay.as_millis() <= 5_000);
        }
    }

    #[test]
    fn test_throttle_events_counter() {
        let state = ThrottleState::new();
        let cfg = BackoffConfig::default();
        assert_eq!(state.throttle_events(), 0);
        state.record_rate_limit(Some(1), &cfg);
        state.record_rate_limit(Some(1), &cfg);
        assert_eq!(state.throttle_events(), 2);
    }

    #[test]
    fn test_throttle_wait_ms_accumulates() {
        let state = ThrottleState::new();
        let cfg = BackoffConfig::default();
        state.record_rate_limit(Some(2), &cfg); // 2000 ms
        state.record_rate_limit(Some(3), &cfg); // 3000 ms
        assert_eq!(state.throttle_wait_ms(), 5_000);
    }
}
