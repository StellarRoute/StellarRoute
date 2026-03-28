use std::collections::VecDeque;
use std::time::{Duration, Instant};

const WINDOW: Duration = Duration::from_secs(60);
const MAX_MESSAGES: usize = 60;

/// Per-connection sliding-window message rate limiter.
///
/// Allows up to 60 messages per 60-second window.
pub struct MessageRateLimiter {
    timestamps: VecDeque<Instant>,
}

impl MessageRateLimiter {
    pub fn new() -> Self {
        Self {
            timestamps: VecDeque::new(),
        }
    }

    /// Returns `true` if the message is within the rate limit and records it.
    /// Returns `false` if the limit has been exceeded (message should be rejected).
    pub fn check_and_increment(&mut self) -> bool {
        let now = Instant::now();

        // Remove entries older than the 60-second window
        while let Some(&front) = self.timestamps.front() {
            if now.duration_since(front) >= WINDOW {
                self.timestamps.pop_front();
            } else {
                break;
            }
        }

        if self.timestamps.len() < MAX_MESSAGES {
            self.timestamps.push_back(now);
            true
        } else {
            false
        }
    }
}

impl Default for MessageRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_60_messages() {
        let mut limiter = MessageRateLimiter::new();
        for _ in 0..60 {
            assert!(limiter.check_and_increment());
        }
    }

    #[test]
    fn rejects_61st_message() {
        let mut limiter = MessageRateLimiter::new();
        for _ in 0..60 {
            limiter.check_and_increment();
        }
        assert!(!limiter.check_and_increment());
    }

    #[test]
    fn starts_empty() {
        let mut limiter = MessageRateLimiter::new();
        assert!(limiter.check_and_increment());
    }
}
