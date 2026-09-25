//! Configurable failure retry strategy

use std::time::Duration;

/// Retry strategy for failed route computations
#[derive(Clone, Debug)]
pub struct RetryStrategy {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial backoff delay in milliseconds
    pub initial_backoff_ms: u64,
    /// Maximum backoff delay in milliseconds
    pub max_backoff_ms: u64,
    /// Exponential backoff multiplier (typically 2.0)
    pub backoff_multiplier: f64,
    /// Types of errors to retry (by default, retry all transient errors)
    pub retryable_errors: RetryableErrorTypes,
}

#[derive(Clone, Debug)]
pub enum RetryableErrorTypes {
    /// Retry all errors
    All,
    /// Retry only transient errors (network, timeouts, etc.)
    TransientOnly,
    /// Custom list of error codes
    Custom(Vec<String>),
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 10000,
            backoff_multiplier: 2.0,
            retryable_errors: RetryableErrorTypes::TransientOnly,
        }
    }
}

impl RetryStrategy {
    /// Calculate backoff delay for given attempt number
    pub fn backoff_delay(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::ZERO;
        }

        let delay_ms = (self.initial_backoff_ms as f64
            * self.backoff_multiplier.powi(attempt as i32 - 1)) as u64;
        let capped = delay_ms.min(self.max_backoff_ms);
        Duration::from_millis(capped)
    }

    /// Check if error is retryable
    pub fn is_retryable(&self, error_code: &str) -> bool {
        match &self.retryable_errors {
            RetryableErrorTypes::All => true,
            RetryableErrorTypes::TransientOnly => {
                // Transient error codes: timeouts, connection errors, server errors
                matches!(
                    error_code,
                    "timeout" | "connection_error" | "service_unavailable" | "internal_error"
                )
            }
            RetryableErrorTypes::Custom(codes) => codes.contains(&error_code.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_calculation() {
        let strategy = RetryStrategy::default();
        assert_eq!(strategy.backoff_delay(0), Duration::ZERO);
        assert_eq!(strategy.backoff_delay(1), Duration::from_millis(100));
        assert_eq!(strategy.backoff_delay(2), Duration::from_millis(200));
        assert_eq!(strategy.backoff_delay(3), Duration::from_millis(400));
    }

    #[test]
    fn test_backoff_max_cap() {
        let strategy = RetryStrategy {
            initial_backoff_ms: 100,
            max_backoff_ms: 500,
            backoff_multiplier: 10.0,
            ..Default::default()
        };
        assert!(strategy.backoff_delay(10) <= Duration::from_millis(500));
    }

    #[test]
    fn test_retryable_errors() {
        let strategy = RetryStrategy::default();
        assert!(strategy.is_retryable("timeout"));
        assert!(strategy.is_retryable("connection_error"));
        assert!(!strategy.is_retryable("invalid_params"));
    }

    #[test]
    fn test_custom_retryable_errors() {
        let strategy = RetryStrategy {
            retryable_errors: RetryableErrorTypes::Custom(vec!["custom_error".to_string()]),
            ..Default::default()
        };
        assert!(strategy.is_retryable("custom_error"));
        assert!(!strategy.is_retryable("timeout"));
    }

    #[test]
    fn default_retry_constants_unchanged() {
        let strategy = RetryStrategy::default();
        assert_eq!(strategy.max_retries, 3);
        assert_eq!(strategy.initial_backoff_ms, 100);
        assert_eq!(strategy.max_backoff_ms, 10000);
        assert!((strategy.backoff_multiplier - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn backoff_doubles_each_attempt() {
        let strategy = RetryStrategy::default();
        // attempt 1: 100ms, attempt 2: 200ms, attempt 3: 400ms
        let d1 = strategy.backoff_delay(1);
        let d2 = strategy.backoff_delay(2);
        let d3 = strategy.backoff_delay(3);
        assert_eq!(d2, d1 * 2);
        assert_eq!(d3, d2 * 2);
    }

    #[test]
    fn backoff_never_exceeds_max_backoff() {
        let strategy = RetryStrategy::default();
        // With multiplier=2.0 and initial=100, attempt 7 would be 6400ms
        // Attempt 14 would be 819200ms but should be capped at 10000ms
        for attempt in 1..=50 {
            let delay = strategy.backoff_delay(attempt);
            assert!(
                delay <= Duration::from_millis(strategy.max_backoff_ms),
                "backoff_delay({}) = {:?} exceeded max {:?}",
                attempt,
                delay,
                Duration::from_millis(strategy.max_backoff_ms),
            );
        }
    }

    #[test]
    fn backoff_delay_at_attempt_zero_is_zero() {
        let strategy = RetryStrategy::default();
        assert_eq!(strategy.backoff_delay(0), Duration::ZERO);
    }

    #[test]
    fn backoff_delay_custom_initial() {
        let strategy = RetryStrategy {
            initial_backoff_ms: 250,
            backoff_multiplier: 3.0,
            max_backoff_ms: 100000,
            ..Default::default()
        };
        // attempt 1: 250 * 3^0 = 250
        assert_eq!(strategy.backoff_delay(1), Duration::from_millis(250));
        // attempt 2: 250 * 3^1 = 750
        assert_eq!(strategy.backoff_delay(2), Duration::from_millis(750));
        // attempt 3: 250 * 3^2 = 2250
        assert_eq!(strategy.backoff_delay(3), Duration::from_millis(2250));
    }

    #[test]
    fn backoff_max_cap_with_exact_boundary() {
        let strategy = RetryStrategy {
            initial_backoff_ms: 1000,
            max_backoff_ms: 1000,
            backoff_multiplier: 2.0,
            ..Default::default()
        };
        // attempt 1: 1000, attempt 2: 2000 (capped to 1000)
        assert_eq!(strategy.backoff_delay(1), Duration::from_millis(1000));
        assert_eq!(strategy.backoff_delay(2), Duration::from_millis(1000));
        assert_eq!(strategy.backoff_delay(10), Duration::from_millis(1000));
    }

    #[test]
    fn retryable_all_accepts_everything() {
        let strategy = RetryStrategy {
            retryable_errors: RetryableErrorTypes::All,
            ..Default::default()
        };
        assert!(strategy.is_retryable("timeout"));
        assert!(strategy.is_retryable("anything"));
        assert!(strategy.is_retryable(""));
    }

    #[test]
    fn retryable_transient_only_rejects_non_transient() {
        let strategy = RetryStrategy::default();
        assert!(!strategy.is_retryable("validation_error"));
        assert!(!strategy.is_retryable("not_found"));
        assert!(!strategy.is_retryable("rate_limit_exceeded"));
        assert!(!strategy.is_retryable("permission_denied"));
    }

    #[test]
    fn retryable_transient_only_accepts_all_transient_codes() {
        let strategy = RetryStrategy::default();
        assert!(strategy.is_retryable("timeout"));
        assert!(strategy.is_retryable("connection_error"));
        assert!(strategy.is_retryable("service_unavailable"));
        assert!(strategy.is_retryable("internal_error"));
    }

    #[test]
    fn retryable_custom_empty_list_rejects_all() {
        let strategy = RetryStrategy {
            retryable_errors: RetryableErrorTypes::Custom(vec![]),
            ..Default::default()
        };
        assert!(!strategy.is_retryable("timeout"));
        assert!(!strategy.is_retryable("anything"));
    }
}
