use std::sync::Arc;
use chrono::{DateTime, Utc, Duration};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use parking_lot::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BreakerState {
    /// Normal operation: Venue is being included in routing
    Closed,
    /// Tripped: Venue is excluded due to multiple failures
    Open,
    /// Recovery: Venue is being probed with limited traffic
    HalfOpen,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakerConfig {
    /// Number of consecutive failures before tripping the breaker
    pub failure_threshold: u32,
    /// Number of consecutive successes required to close the breaker
    pub success_threshold: u32,
    /// Duration to keep the breaker open before transitioning to HalfOpen
    pub recovery_timeout_secs: i64,
}

impl Default for BreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            recovery_timeout_secs: 30,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VenueBreaker {
    pub state: BreakerState,
    pub consecutive_failures: u32,
    pub consecutive_successes: u32,
    pub last_failure_at: Option<DateTime<Utc>>,
    pub last_transition_at: DateTime<Utc>,
}

impl VenueBreaker {
    pub fn new() -> Self {
        Self {
            state: BreakerState::Closed,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_failure_at: None,
            last_transition_at: Utc::now(),
        }
    }

    pub fn record_failure(&mut self, config: &BreakerConfig) {
        let now = Utc::now();
        self.consecutive_failures += 1;
        self.consecutive_successes = 0;
        self.last_failure_at = Some(now);

        match self.state {
            BreakerState::Closed => {
                if self.consecutive_failures >= config.failure_threshold {
                    self.transition_to(BreakerState::Open);
                }
            }
            BreakerState::HalfOpen => {
                // Any failure in HalfOpen trips it back to Open immediately
                self.transition_to(BreakerState::Open);
            }
            BreakerState::Open => {} // Already open
        }
    }

    pub fn record_success(&mut self, config: &BreakerConfig) {
        self.consecutive_successes += 1;
        self.consecutive_failures = 0;

        match self.state {
            BreakerState::HalfOpen => {
                if self.consecutive_successes >= config.success_threshold {
                    self.transition_to(BreakerState::Closed);
                }
            }
            BreakerState::Open => {
                // Successes while open shouldn't happen if we're excluding, 
                // but if they do (manual probes), we might want to track them.
            }
            BreakerState::Closed => {} // Stays closed
        }
    }

    pub fn check_and_transition(&mut self, config: &BreakerConfig) {
        let now = Utc::now();
        if self.state == BreakerState::Open {
            if let Some(last_failure) = self.last_failure_at {
                if now - last_failure >= Duration::seconds(config.recovery_timeout_secs) {
                    self.transition_to(BreakerState::HalfOpen);
                }
            }
        }
    }

    fn transition_to(&mut self, new_state: BreakerState) {
        if self.state != new_state {
            self.state = new_state;
            self.last_transition_at = Utc::now();
            self.consecutive_failures = 0;
            self.consecutive_successes = 0;
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CircuitBreakerRegistry {
    breakers: Arc<DashMap<String, Arc<Mutex<VenueBreaker>>>>,
    pub config: BreakerConfig,
}

impl CircuitBreakerRegistry {
    pub fn new(config: BreakerConfig) -> Self {
        Self {
            breakers: Arc::new(DashMap::new()),
            config,
        }
    }

    pub fn is_venue_excluded(&self, venue_ref: &str) -> bool {
        let breaker_arc = self.breakers.entry(venue_ref.to_string()).or_insert_with(|| {
            Arc::new(Mutex::new(VenueBreaker::new()))
        });
        
        let mut breaker = breaker_arc.lock();
        breaker.check_and_transition(&self.config);
        
        // Exclude if state is Open. 
        // HalfOpen should probably allow limited traffic, 
        // but for now let's say it's "included" so it can be probed.
        breaker.state == BreakerState::Open
    }

    pub fn record_result(&self, venue_ref: &str, success: bool) {
        let breaker_arc = self.breakers.entry(venue_ref.to_string()).or_insert_with(|| {
            Arc::new(Mutex::new(VenueBreaker::new()))
        });
        
        let mut breaker = breaker_arc.lock();
        if success {
            breaker.record_success(&self.config);
        } else {
            breaker.record_failure(&self.config);
        }
    }

    pub fn get_state(&self, venue_ref: &str) -> Option<BreakerState> {
        self.breakers.get(venue_ref).map(|b| b.lock().state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_transition_path() {
        let config = BreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            recovery_timeout_secs: 1,
        };
        let mut breaker = VenueBreaker::new();
        assert_eq!(breaker.state, BreakerState::Closed);

        // Fail 1, 2
        breaker.record_failure(&config);
        breaker.record_failure(&config);
        assert_eq!(breaker.state, BreakerState::Closed);

        // Fail 3 -> Open
        breaker.record_failure(&config);
        assert_eq!(breaker.state, BreakerState::Open);

        // Success while Open doesn't change state (usually excluded by policy)
        breaker.record_success(&config);
        assert_eq!(breaker.state, BreakerState::Open);

        // Wait for recovery timeout
        std::thread::sleep(std::time::Duration::from_millis(1100));
        breaker.check_and_transition(&config);
        assert_eq!(breaker.state, BreakerState::HalfOpen);

        // Success 1 in HalfOpen
        breaker.record_success(&config);
        assert_eq!(breaker.state, BreakerState::HalfOpen);

        // Success 2 in HalfOpen -> Closed
        breaker.record_success(&config);
        assert_eq!(breaker.state, BreakerState::Closed);
    }

    #[test]
    fn test_half_open_failure_reopens() {
        let config = BreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            recovery_timeout_secs: 1,
        };
        let mut breaker = VenueBreaker::new();
        
        breaker.record_failure(&config);
        breaker.record_failure(&config);
        assert_eq!(breaker.state, BreakerState::Open);

        std::thread::sleep(std::time::Duration::from_millis(1100));
        breaker.check_and_transition(&config);
        assert_eq!(breaker.state, BreakerState::HalfOpen);

        // Failure in HalfOpen -> immediately Open
        breaker.record_failure(&config);
        assert_eq!(breaker.state, BreakerState::Open);
    }

    #[test]
    fn test_registry_exclusion() {
        let registry = CircuitBreakerRegistry::new(BreakerConfig {
            failure_threshold: 2,
            ..Default::default()
        });

        assert!(!registry.is_venue_excluded("v1"));

        registry.record_result("v1", false);
        assert!(!registry.is_venue_excluded("v1"));

        registry.record_result("v1", false);
        assert!(registry.is_venue_excluded("v1"));

        registry.record_result("v1", true); // Should stay excluded until recovery
        assert!(registry.is_venue_excluded("v1"));
    }
}
