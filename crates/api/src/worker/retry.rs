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

        let delay_ms = (self.initial_backoff_ms as f64 * self.backoff_multiplier.powi(attempt as i32 - 1)) as u64;
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
}
