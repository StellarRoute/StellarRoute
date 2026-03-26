use super::config::{RegionConfig, RegionId};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicI64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Health status of a regional replica
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Region is healthy and ready to serve reads
    Healthy,
    /// Region had recent failures but may recover (degraded)
    Degraded,
    /// Region is failing, avoid routing reads here
    Unhealthy,
    /// Circuit breaker is open - don't try requests
    CircuitOpen,
}

impl HealthStatus {
    pub fn is_healthy(&self) -> bool {
        matches!(self, HealthStatus::Healthy)
    }

    pub fn is_usable(&self) -> bool {
        matches!(self, HealthStatus::Healthy | HealthStatus::Degraded)
    }
}

/// Snapshot of a region's health at a point in time
#[derive(Debug, Clone)]
pub struct HealthSnapshot {
    /// Region identifier
    pub region_id: RegionId,
    /// Current health status
    pub status: HealthStatus,
    /// Consecutive failed health checks
    pub consecutive_failures: u32,
    /// Last successful health check (Unix timestamp)
    pub last_success_ts: i64,
    /// Last failed health check (Unix timestamp)
    pub last_failure_ts: i64,
    /// Average response time in milliseconds
    pub avg_response_time_ms: u32,
    /// Measured replica lag in seconds
    pub replica_lag_secs: u32,
}

/// Tracks health metrics for a single region
pub struct RegionHealthCheck {
    region_id: RegionId,
    config: RegionConfig,

    // Atomic counters for lock-free updates
    consecutive_failures: Arc<AtomicU32>,
    last_success_ts: Arc<AtomicI64>,
    last_failure_ts: Arc<AtomicI64>,

    // Metrics
    response_times: Arc<parking_lot::RwLock<Vec<u32>>>, // Rolling window
    replica_lag_secs: Arc<AtomicU32>,
}

impl RegionHealthCheck {
    /// Create a new health tracker for a region
    pub fn new(region_id: RegionId, config: RegionConfig) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        RegionHealthCheck {
            region_id,
            config,
            consecutive_failures: Arc::new(AtomicU32::new(0)),
            last_success_ts: Arc::new(AtomicI64::new(now)),
            last_failure_ts: Arc::new(AtomicI64::new(0)),
            response_times: Arc::new(parking_lot::RwLock::new(Vec::with_capacity(100))),
            replica_lag_secs: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Record a successful health check
    pub fn record_success(&self, response_time_ms: u32, replica_lag_secs: u32) {
        self.consecutive_failures.store(0, Ordering::Relaxed);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        self.last_success_ts.store(now, Ordering::Release);

        // Update rolling average
        let mut times = self.response_times.write();
        times.push(response_time_ms);
        if times.len() > 100 {
            times.remove(0);
        }

        self.replica_lag_secs.store(replica_lag_secs, Ordering::Relaxed);
    }

    /// Record a failed health check
    pub fn record_failure(&self) {
        let failures = self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        self.last_failure_ts.store(now, Ordering::Release);

        tracing::warn!(
            region = %self.region_id,
            consecutive_failures = failures,
            "Health check failed for region"
        );
    }

    /// Determine current health status
    pub fn current_status(&self) -> HealthStatus {
        let failures = self.consecutive_failures.load(Ordering::Relaxed);
        let last_failure = self.last_failure_ts.load(Ordering::Acquire);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        // Check circuit breaker recovery window
        if failures >= self.config.circuit_breaker_threshold {
            let recovery_elapsed = (now - last_failure) as u64;
            if recovery_elapsed < self.config.circuit_breaker_timeout_secs {
                return HealthStatus::CircuitOpen;
            }
            // Transition to half-open after timeout
            return HealthStatus::Degraded;
        }

        // Check replica lag
        let lag = self.replica_lag_secs.load(Ordering::Relaxed);
        if lag > self.config.max_replica_lag_secs {
            return HealthStatus::Degraded;
        }

        // Healthy
        HealthStatus::Healthy
    }

    /// Get current health snapshot
    pub fn snapshot(&self) -> HealthSnapshot {
        let times = self.response_times.read();
        let avg_response_time_ms = if times.is_empty() {
            0
        } else {
            let sum: u64 = times.iter().map(|&t| t as u64).sum();
            (sum / times.len() as u64) as u32
        };

        HealthSnapshot {
            region_id: self.region_id,
            status: self.current_status(),
            consecutive_failures: self.consecutive_failures.load(Ordering::Relaxed),
            last_success_ts: self.last_success_ts.load(Ordering::Acquire),
            last_failure_ts: self.last_failure_ts.load(Ordering::Acquire),
            avg_response_time_ms,
            replica_lag_secs: self.replica_lag_secs.load(Ordering::Relaxed),
        }
    }

    /// Check if circuit breaker allows requests
    pub fn circuit_allows_request(&self) -> bool {
        !matches!(self.current_status(), HealthStatus::CircuitOpen)
    }
}

/// Manages health checks across all regions
pub struct RegionalHealthManager {
    checkers: Arc<std::collections::HashMap<RegionId, RegionHealthCheck>>,
}

impl RegionalHealthManager {
    /// Create health manager with checkers for all regions
    pub fn new(configs: Vec<RegionConfig>) -> Self {
        let mut checkers = std::collections::HashMap::new();
        for config in configs {
            let region_id = config.region_id;
            checkers.insert(region_id, RegionHealthCheck::new(region_id, config));
        }

        RegionalHealthManager {
            checkers: Arc::new(checkers),
        }
    }

    /// Get health checker for a region
    pub fn get_checker(&self, region_id: RegionId) -> Option<&RegionHealthCheck> {
        self.checkers.get(&region_id)
    }

    /// Get snapshot of all region health
    pub fn all_snapshots(&self) -> Vec<HealthSnapshot> {
        self.checkers
            .values()
            .map(|checker| checker.snapshot())
            .collect()
    }

    /// Get health snapshot for a specific region
    pub fn snapshot(&self, region_id: RegionId) -> Option<HealthSnapshot> {
        self.checkers.get(&region_id).map(|c| c.snapshot())
    }

    /// Get all healthy regions
    pub fn healthy_regions(&self) -> Vec<RegionId> {
        self.checkers
            .values()
            .filter(|c| c.current_status().is_healthy())
            .map(|c| c.region_id)
            .collect()
    }

    /// Get all usable regions (healthy or degraded)
    pub fn usable_regions(&self) -> Vec<RegionId> {
        self.checkers
            .values()
            .filter(|c| c.current_status().is_usable())
            .map(|c| c.region_id)
            .collect()
    }

    /// Count regions by status
    pub fn count_by_status(&self) -> (usize, usize, usize, usize) {
        let mut healthy = 0;
        let mut degraded = 0;
        let mut unhealthy = 0;
        let mut circuit_open = 0;

        for checker in self.checkers.values() {
            match checker.current_status() {
                HealthStatus::Healthy => healthy += 1,
                HealthStatus::Degraded => degraded += 1,
                HealthStatus::Unhealthy => unhealthy += 1,
                HealthStatus::CircuitOpen => circuit_open += 1,
            }
        }

        (healthy, degraded, unhealthy, circuit_open)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_checks() {
        assert!(HealthStatus::Healthy.is_healthy());
        assert!(!HealthStatus::Unhealthy.is_healthy());
        assert!(HealthStatus::Healthy.is_usable());
        assert!(HealthStatus::Degraded.is_usable());
        assert!(!HealthStatus::CircuitOpen.is_usable());
    }

    #[test]
    fn test_consecutive_failures_trigger_circuit() {
        let config = RegionConfig::new(
            RegionId::UsEast,
            "postgres://test".to_string(),
            0,
        );
        let checker = RegionHealthCheck::new(RegionId::UsEast, config);

        assert_eq!(checker.current_status(), HealthStatus::Healthy);

        checker.record_failure();
        checker.record_failure();
        checker.record_failure();

        assert_eq!(checker.current_status(), HealthStatus::CircuitOpen);
    }

    #[test]
    fn test_successful_check_resets_failures() {
        let config = RegionConfig::new(
            RegionId::UsEast,
            "postgres://test".to_string(),
            0,
        );
        let checker = RegionHealthCheck::new(RegionId::UsEast, config);

        checker.record_failure();
        checker.record_failure();
        assert!(checker.consecutive_failures.load(Ordering::Relaxed) > 0);

        checker.record_success(50, 1);
        assert_eq!(checker.consecutive_failures.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_replica_lag_degradation() {
        let mut config = RegionConfig::new(
            RegionId::UsEast,
            "postgres://test".to_string(),
            0,
        );
        config.max_replica_lag_secs = 5;

        let checker = RegionHealthCheck::new(RegionId::UsEast, config);
        checker.record_success(50, 10); // Lag > threshold

        assert_eq!(checker.current_status(), HealthStatus::Degraded);
    }

    #[test]
    fn test_response_time_averaging() {
        let config = RegionConfig::new(
            RegionId::UsEast,
            "postgres://test".to_string(),
            0,
        );
        let checker = RegionHealthCheck::new(RegionId::UsEast, config);

        checker.record_success(100, 1);
        checker.record_success(50, 1);
        checker.record_success(150, 1);

        let snapshot = checker.snapshot();
        assert_eq!(snapshot.avg_response_time_ms, 100); // (100+50+150)/3
    }
}
