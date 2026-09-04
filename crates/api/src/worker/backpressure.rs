//! Backpressure protection for API under load spikes

use crate::error::{ApiError, Result};

/// Backpressure policy configuration
#[derive(Clone, Debug)]
pub struct BackpressurePolicy {
    /// Maximum number of jobs in queue before rejecting new requests
    pub max_queue_depth: usize,
    /// Maximum number of concurrent workers
    pub max_workers: usize,
    /// Reject requests when backlog exceeds this threshold (0-100%)
    pub rejection_threshold_percent: u32,
}

impl Default for BackpressurePolicy {
    fn default() -> Self {
        Self {
            max_queue_depth: 10000,
            max_workers: 100,
            rejection_threshold_percent: 80, // Reject when 80% full
        }
    }
}

impl BackpressurePolicy {
    /// Check if we should accept a new job based on current queue and load
    pub fn should_accept(&self, pending_jobs: usize, processing_jobs: usize) -> Result<()> {
        let total_backlog = pending_jobs + processing_jobs;

        // Hard limit check
        if total_backlog >= self.max_queue_depth {
            return Err(ApiError::Overloaded(
                "Job queue at capacity, please retry later".to_string(),
            ));
        }

        // Soft threshold check (percentage-based rejection)
        let threshold = (self.max_queue_depth * self.rejection_threshold_percent as usize) / 100;
        if total_backlog >= threshold {
            return Err(ApiError::Overloaded(
                "System under heavy load, please retry later".to_string(),
            ));
        }

        Ok(())
    }

    /// Calculate weighted score for load estimation (0-100)
    pub fn load_score(&self, pending_jobs: usize, processing_jobs: usize) -> u32 {
        let total_backlog = pending_jobs + processing_jobs;
        ((total_backlog * 100) / self.max_queue_depth).min(100) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backpressure_accept() {
        let policy = BackpressurePolicy::default();
        assert!(policy.should_accept(100, 50).is_ok());
    }

    #[test]
    fn test_backpressure_soft_reject() {
        let policy = BackpressurePolicy::default();
        let threshold =
            (policy.max_queue_depth * policy.rejection_threshold_percent as usize) / 100;
        assert!(policy.should_accept(threshold + 100, 0).is_err());
    }

    #[test]
    fn test_backpressure_hard_reject() {
        let policy = BackpressurePolicy::default();
        assert!(policy.should_accept(policy.max_queue_depth, 0).is_err());
    }

    #[test]
    fn test_load_score() {
        let policy = BackpressurePolicy::default();
        assert_eq!(policy.load_score(0, 0), 0);
        assert_eq!(policy.load_score(policy.max_queue_depth / 2, 0), 50);
        assert_eq!(policy.load_score(policy.max_queue_depth, 0), 100);
    }

    #[test]
    fn default_policy_constants_unchanged() {
        let policy = BackpressurePolicy::default();
        assert_eq!(policy.max_queue_depth, 10000);
        assert_eq!(policy.max_workers, 100);
        assert_eq!(policy.rejection_threshold_percent, 80);
    }

    #[test]
    fn reject_when_backlog_at_80_percent_of_max_queue_depth() {
        let policy = BackpressurePolicy::default();
        let threshold_80 = (policy.max_queue_depth * 80) / 100;
        assert_eq!(threshold_80, 8000);

        // Exactly at 80% threshold - should reject (>=)
        let result = policy.should_accept(threshold_80, 0);
        assert!(result.is_err(), "should reject at exactly 80% backlog");

        // Just below 80% - should accept
        let result = policy.should_accept(threshold_80 - 1, 0);
        assert!(result.is_ok(), "should accept below 80% backlog");
    }

    #[test]
    fn backlog_combines_pending_and_processing() {
        let policy = BackpressurePolicy::default();

        // pending=7999 + processing=1 = 8000 - at 80%, should reject
        let result = policy.should_accept(7999, 1);
        assert!(result.is_err(), "combined backlog at 80% should reject");

        // pending=7998 + processing=1 = 7999 - just below, should accept
        let result = policy.should_accept(7998, 1);
        assert!(result.is_ok(), "combined backlog below 80% should accept");
    }

    #[test]
    fn accept_at_zero_load() {
        let policy = BackpressurePolicy::default();
        assert!(policy.should_accept(0, 0).is_ok());
    }

    #[test]
    fn reject_at_full_capacity() {
        let policy = BackpressurePolicy::default();
        // max_queue_depth = 10000, reject at >= 10000
        assert!(policy.should_accept(10000, 0).is_err());
        assert!(policy.should_accept(5000, 5000).is_err());
        assert!(policy.should_accept(9999, 1).is_err());
    }

    #[test]
    fn custom_policy_threshold_respected() {
        let policy = BackpressurePolicy {
            max_queue_depth: 500,
            max_workers: 10,
            rejection_threshold_percent: 50,
        };

        // 50% of 500 = 250
        assert!(
            policy.should_accept(250, 0).is_err(),
            "should reject at 50% threshold"
        );
        assert!(
            policy.should_accept(249, 0).is_ok(),
            "should accept below 50% threshold"
        );
        assert!(
            policy.should_accept(500, 0).is_err(),
            "should reject at full capacity"
        );
    }

    #[test]
    fn load_score_clamps_at_100() {
        let policy = BackpressurePolicy::default();
        assert_eq!(policy.load_score(policy.max_queue_depth * 2, 0), 100);
    }

    #[test]
    fn load_score_includes_processing_jobs() {
        let policy = BackpressurePolicy::default();
        let score_pending_only = policy.load_score(5000, 0);
        let score_with_processing = policy.load_score(3000, 2000);
        assert_eq!(score_pending_only, score_with_processing);
    }
}
