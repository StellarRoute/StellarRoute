/// Multi-region read replica support for the quote API
///
/// This module implements a sophisticated multi-region routing system with:
/// - Automatic health discovery and circuit breaking
/// - Consistency constraints (strong, eventual, session)
/// - Intelligent failover to secondary regions
/// - Version tracking to prevent split-brain scenarios
/// - Comprehensive metrics for operational visibility
///
/// # Architecture
///
/// The multi-region system consists of:
///
/// 1. **RegionRegistry** - Central registration of all regions and configurations
/// 2. **RegionalHealthManager** - Tracks health of each region with atomic counters
/// 3. **MultiRegionRouter** - Routes reads with intelligent failover
/// 4. **VersionTracker** - Ensures consistency across regions
///
/// # Example Usage
///
/// ```rust,ignore
/// // Create router from environment configuration
/// let registry = RegionRegistry::from_env()?;
/// let router = MultiRegionRouter::new(registry).await?;
///
/// // Read with eventual consistency (10 second tolerance)
/// let constraint = ConsistencyConstraint::eventual(10);
/// let (data, decision) = router.read_with_failover(
///     &constraint,
///     |router, region| async {
///         // Your read operation here
///         Ok((data, version))
///     }
/// ).await?;
///
/// println!("Routed to {}, took {}us", decision.region_id, decision.response_time_us);
/// ```
///
/// # Consistency Models
///
/// - **Strong**: Fresh data from primary only (max_age: 1s, no degraded)
/// - **Session**: Slightly stale ok, prefer primary (adjustable max_age)
/// - **Eventual**: Accept stale data from any healthy region (adjustable max_age)
///
/// # Failover Strategy
///
/// Reads are attempted in priority order:
/// 1. Primary region (lowest priority number)
/// 2. Secondary regions (higher priority numbers)
/// 3. All enabled regions if allow_degraded
///
/// Each region has a circuit breaker that opens after N consecutive failures.
/// Open circuits won't accept requests for a configured timeout period.

pub mod config;
pub mod consistency;
pub mod health;
pub mod router;

pub use config::{RegionConfig, RegionId, RegionRegistry};
pub use consistency::{ConsistencyConstraint, DataVersion, VersionTracker};
pub use health::{HealthSnapshot, HealthStatus, RegionalHealthManager};
pub use router::{MultiRegionRouter, RoutingDecision, RoutingMetrics};
