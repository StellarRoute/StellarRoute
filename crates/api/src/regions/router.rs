use super::config::{RegionConfig, RegionId, RegionRegistry};
use super::consistency::{ConsistencyConstraint, DataVersion, VersionTracker};
use super::health::{HealthStatus, RegionalHealthManager};
use crate::error::{ApiError, Result};
use sqlx::{PgPool, Pool, Postgres};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Routing decision explains why a region was selected
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    /// Selected region
    pub region_id: RegionId,
    /// Reason for selection
    pub reason: String,
    /// Response time in microseconds
    pub response_time_us: u64,
    /// Data version returned
    pub data_version: DataVersion,
    /// Number of regions evaluated
    pub regions_evaluated: usize,
    /// Was this a fallback selection
    pub is_fallback: bool,
}

/// Metrics for routing decisions
#[derive(Debug, Clone)]
pub struct RoutingMetrics {
    /// Total routing decisions made
    pub total_decisions: u64,
    /// Decisions where primary was used
    pub primary_used: u64,
    /// Decisions where fallback was needed
    pub fallback_used: u64,
    /// Decisions where all healthy regions tried
    pub all_healthy_exhausted: u64,
    /// Decisions where all regions tried (full degradation)
    pub all_regions_exhausted: u64,
    /// Decisions where circuit breaker blocked a region
    pub circuit_breaker_blocks: u64,
}

impl Default for RoutingMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl RoutingMetrics {
    pub fn new() -> Self {
        RoutingMetrics {
            total_decisions: 0,
            primary_used: 0,
            fallback_used: 0,
            all_healthy_exhausted: 0,
            all_regions_exhausted: 0,
            circuit_breaker_blocks: 0,
        }
    }

    pub fn primary_percentage(&self) -> f64 {
        if self.total_decisions == 0 {
            0.0
        } else {
            (self.primary_used as f64 / self.total_decisions as f64) * 100.0
        }
    }

    pub fn fallback_percentage(&self) -> f64 {
        if self.total_decisions == 0 {
            0.0
        } else {
            (self.fallback_used as f64 / self.total_decisions as f64) * 100.0
        }
    }
}

/// Routes reads across multiple regions with failover support
pub struct MultiRegionRouter {
    /// Regional configurations
    registry: RegionRegistry,

    /// Health tracking for each region
    health: Arc<RegionalHealthManager>,

    /// Database connection pools per region
    pools: Arc<HashMap<RegionId, Pool<Postgres>>>,

    /// Data version tracking for consistency
    versions: Arc<VersionTracker>,

    /// Routing metrics
    metrics: Arc<parking_lot::RwLock<RoutingMetrics>>,

    // Atomic counters for lock-free metric updates
    total_decisions: Arc<AtomicU64>,
    primary_used_count: Arc<AtomicU64>,
    fallback_count: Arc<AtomicU64>,
}

impl MultiRegionRouter {
    /// Create a new multi-region router
    pub async fn new(registry: RegionRegistry) -> Result<Self> {
        let configs: Vec<RegionConfig> = registry
            .enabled_regions()
            .into_iter()
            .filter_map(|r| registry.get_config(r))
            .collect();

        // Create connection pools for each region
        let mut pools = HashMap::new();
        for config in &configs {
            tracing::info!(
                region = %config.region_id,
                pool_size = config.pool_size,
                "Creating connection pool"
            );

            let pool = PgPool::connect_with(
                sqlx::postgres::PgConnectOptions::new()
                    .host(&config.database_url)
                    .to_owned(),
            )
            .await?;

            pools.insert(config.region_id, pool);
        }

        let health = Arc::new(RegionalHealthManager::new(configs));

        Ok(MultiRegionRouter {
            registry,
            health,
            pools: Arc::new(pools),
            versions: Arc::new(VersionTracker::new()),
            metrics: Arc::new(parking_lot::RwLock::new(RoutingMetrics::new())),
            total_decisions: Arc::new(AtomicU64::new(0)),
            primary_used_count: Arc::new(AtomicU64::new(0)),
            fallback_count: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Execute a read function across regions with failover
    ///
    /// # Arguments
    /// * `constraint` - Consistency requirements for this read
    /// * `read_fn` - Async function that performs the actual read
    ///
    /// # Returns
    /// * `Result<(T, RoutingDecision)>` - Data and routing info
    pub async fn read_with_failover<T, F>(
        &self,
        constraint: &ConsistencyConstraint,
        read_fn: impl Fn(Self, RegionId) -> F,
    ) -> Result<(T, RoutingDecision)>
    where
        F: std::future::Future<Output = Result<(T, DataVersion)>>,
    {
        self.total_decisions.fetch_add(1, Ordering::Relaxed);

        let all_regions = self.registry.all_regions();
        let mut last_error: Option<String> = None;

        // Try regions in priority order
        for region_id in &all_regions {
            // Check health status and circuit breaker
            let status = self
                .health
                .get_checker(*region_id)
                .map(|c| c.current_status())
                .unwrap_or(HealthStatus::Unhealthy);

            // Skip unhealthy regions unless allow_degraded
            if !constraint.allow_degraded && status == HealthStatus::Degraded {
                continue;
            }

            // Never route through open circuits
            if status == HealthStatus::CircuitOpen {
                self.metrics.write().circuit_breaker_blocks += 1;
                continue;
            }

            // Try this region
            let start = Instant::now();
            match read_fn(self.clone(), *region_id).await {
                Ok((data, version)) => {
                    let elapsed_us = start.elapsed().as_micros() as u64;

                    // Check consistency (staleness + optional ledger skew)
                    let baseline_ledgers = self.versions.current().ledger_sequence;
                    if !constraint.satisfies_with_baseline(&version, Some(baseline_ledgers)) {
                        last_error = Some(format!(
                            "Data violates consistency policy from region {}",
                            region_id
                        ));
                        continue;
                    }

                    // Record success if primary
                    if Some(*region_id) == self.registry.primary_region() {
                        self.primary_used_count.fetch_add(1, Ordering::Relaxed);
                        self.versions.update_from_primary(version.clone());

                        return Ok((
                            data,
                            RoutingDecision {
                                region_id: *region_id,
                                reason: "Primary region successful".to_string(),
                                response_time_us: elapsed_us,
                                data_version: version,
                                regions_evaluated: 1,
                                is_fallback: false,
                            },
                        ));
                    }

                    // Fallback region succeeded
                    self.fallback_count.fetch_add(1, Ordering::Relaxed);
                    return Ok((
                        data,
                        RoutingDecision {
                            region_id: *region_id,
                            reason: format!(
                                "Fallback to {} due to primary unavailability",
                                region_id
                            ),
                            response_time_us: elapsed_us,
                            data_version: version,
                            regions_evaluated: all_regions.len(),
                            is_fallback: true,
                        },
                    ));
                }
                Err(e) => {
                    last_error = Some(e.to_string());
                    // Record failure in health check
                    if let Some(checker) = self.health.get_checker(*region_id) {
                        checker.record_failure();
                    }
                    continue;
                }
            }
        }

        // All regions failed
        Err(ApiError::Internal(Arc::new(anyhow::anyhow!(
            "All regions exhausted: {}",
            last_error.unwrap_or_else(|| "Unknown error".to_string())
        ))))
    }

    /// Get routing metrics
    pub fn metrics(&self) -> RoutingMetrics {
        self.metrics.read().clone()
    }

    /// Get health status of all regions
    pub fn health_snapshots(&self) -> Vec<super::health::HealthSnapshot> {
        self.health.all_snapshots()
    }

    /// Get current version state
    pub fn current_version(&self) -> DataVersion {
        self.versions.current()
    }

    /// Check if all regions are converged
    pub fn is_converged(&self, ledger_tolerance: u32) -> bool {
        self.versions.is_converged(ledger_tolerance)
    }

    /// Get version drift across regions
    pub fn version_drift(&self) -> u32 {
        self.versions.version_drift()
    }

    /// Get all enabled regions
    pub fn enabled_regions(&self) -> Vec<RegionId> {
        self.registry.enabled_regions()
    }

    /// Test health of all regions (for background monitoring)
    pub async fn run_health_checks(&self) -> Result<()> {
        for region_id in self.enabled_regions() {
            if let Some(pool) = self.pools.get(&region_id) {
                let start = Instant::now();

                match sqlx::query_scalar::<_, i64>("SELECT EXTRACT(EPOCH FROM NOW())::INT")
                    .fetch_one(pool)
                    .await
                {
                    Ok(_) => {
                        let response_time = start.elapsed().as_millis() as u32;
                        let lag_secs = 0; // TODO: Query replication lag

                        if let Some(checker) = self.health.get_checker(region_id) {
                            checker.record_success(response_time, lag_secs);
                        }
                    }
                    Err(e) => {
                        tracing::warn!(region = %region_id, error = %e, "Health check failed");
                        if let Some(checker) = self.health.get_checker(region_id) {
                            checker.record_failure();
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl Clone for MultiRegionRouter {
    fn clone(&self) -> Self {
        MultiRegionRouter {
            registry: self.registry.clone(),
            health: Arc::clone(&self.health),
            pools: Arc::clone(&self.pools),
            versions: Arc::clone(&self.versions),
            metrics: Arc::clone(&self.metrics),
            total_decisions: Arc::clone(&self.total_decisions),
            primary_used_count: Arc::clone(&self.primary_used_count),
            fallback_count: Arc::clone(&self.fallback_count),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routing_metrics() {
        let metrics = RoutingMetrics::new();
        assert_eq!(metrics.primary_percentage(), 0.0);
        assert_eq!(metrics.fallback_percentage(), 0.0);
    }

    #[test]
    fn test_routing_decision_display() {
        let decision = RoutingDecision {
            region_id: RegionId::UsEast,
            reason: "Primary selected".to_string(),
            response_time_us: 1000,
            data_version: DataVersion::new(100),
            regions_evaluated: 1,
            is_fallback: false,
        };

        assert!(!decision.is_fallback);
        assert_eq!(decision.regions_evaluated, 1);
    }
}
