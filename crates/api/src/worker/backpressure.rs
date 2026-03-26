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
        let threshold = (policy.max_queue_depth * policy.rejection_threshold_percent as usize) / 100;
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
}
