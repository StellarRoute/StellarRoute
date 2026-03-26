use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

/// Represents a version of data across regional replicas
/// Used to ensure consistency - don't mix reads from different data versions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DataVersion {
    /// Timestamp when this version started propagating
    pub timestamp_micros: i64,

    /// Highest ledger sequence seen at this version
    /// (tracks Stellar consensus progression)
    pub ledger_sequence: u32,

    /// Content hash of key datasets (for future validation)
    /// Computed from normalized_liquidity view
    pub content_hash: Option<String>,
}

impl std::fmt::Display for DataVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "v{}@ledger{}",
            self.timestamp_micros, self.ledger_sequence
        )
    }
}

impl DataVersion {
    /// Create new version with current timestamp
    pub fn new(ledger_sequence: u32) -> Self {
        let now = Utc::now();
        let timestamp_micros = now.timestamp_micros();

        DataVersion {
            timestamp_micros,
            ledger_sequence,
            content_hash: None,
        }
    }

    /// Create version with content hash for validation
    pub fn with_hash(ledger_sequence: u32, content_hash: String) -> Self {
        let now = Utc::now();
        let timestamp_micros = now.timestamp_micros();

        DataVersion {
            timestamp_micros,
            ledger_sequence,
            content_hash: Some(content_hash),
        }
    }

    /// Check if this version is too old (stale)
    pub fn is_stale(&self, max_age_secs: u64) -> bool {
        let now = Utc::now();
        let age_micros = (now.timestamp_micros() - self.timestamp_micros) as u64;
        let age_secs = age_micros / 1_000_000;
        age_secs >= max_age_secs
    }

    /// Get age of this version in seconds
    pub fn age_secs(&self) -> u64 {
        let now = Utc::now();
        let age_micros = (now.timestamp_micros() - self.timestamp_micros) as u64;
        age_micros / 1_000_000
    }
}

/// Consistency constraints for multi-region reads
#[derive(Debug, Clone)]
pub struct ConsistencyConstraint {
    /// Maximum acceptable age of data (staleness)
    pub max_age_secs: u64,

    /// Allow reads from degraded regions (replica lag > threshold)
    pub allow_degraded: bool,

    /// Require version match across all regions (strict consistency)
    pub require_version_match: bool,

    /// Max allowed ledger skew when require_version_match is true
    pub max_ledger_skew: Option<u32>,

    /// If true, prefer primary region even if slightly slower
    pub prefer_primary: bool,
}

impl ConsistencyConstraint {
    /// Strong consistency - fresh data from primary only
    pub fn strong() -> Self {
        ConsistencyConstraint {
            max_age_secs: 1,
            allow_degraded: false,
            require_version_match: true,
            max_ledger_skew: Some(0),
            prefer_primary: true,
        }
    }

    /// Eventual consistency - allow stale reads from healthy replicas
    pub fn eventual(max_age_secs: u64) -> Self {
        ConsistencyConstraint {
            max_age_secs,
            allow_degraded: true,
            require_version_match: false,
            max_ledger_skew: None,
            prefer_primary: false,
        }
    }

    /// Session consistency - slightly stale ok, but prefer most recent
    pub fn session(max_age_secs: u64) -> Self {
        ConsistencyConstraint {
            max_age_secs,
            allow_degraded: false,
            require_version_match: true,
            max_ledger_skew: Some(5),
            prefer_primary: true,
        }
    }

    /// Check if version satisfies staleness requirement
    pub fn satisfies(&self, version: &DataVersion) -> bool {
        !version.is_stale(self.max_age_secs)
    }

    /// Check if version satisfies both staleness and version-match requirements
    pub fn satisfies_with_baseline(&self, version: &DataVersion, baseline_ledger: Option<u32>) -> bool {
        if !self.satisfies(version) {
            return false;
        }

        if self.require_version_match {
            if let Some(skew) = self.max_ledger_skew {
                if let Some(base) = baseline_ledger {
                    let diff = if version.ledger_sequence > base {
                        version.ledger_sequence - base
                    } else {
                        base - version.ledger_sequence
                    };
                    return diff <= skew;
                }

                // If we don't know baseline ledger, be conservative and fail
                return false;
            }
        }

        true
    }
}

/// Tracks versions per region for consistency checking
pub struct VersionTracker {
    /// Current data version (shared across regions, should eventually converge)
    current_version: Arc<parking_lot::RwLock<DataVersion>>,

    /// Last time primary region updated
    primary_last_update_ts: Arc<AtomicI64>,

    /// Track observed versions per region for debugging
    region_versions: Arc<parking_lot::RwLock<std::collections::HashMap<String, DataVersion>>>,
}

impl VersionTracker {
    /// Create new version tracker
    pub fn new() -> Self {
        VersionTracker {
            current_version: Arc::new(parking_lot::RwLock::new(DataVersion::new(0))),
            primary_last_update_ts: Arc::new(AtomicI64::new(0)),
            region_versions: Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Update version from primary region
    pub fn update_from_primary(&self, version: DataVersion) {
        let now = Utc::now().timestamp_micros();
        self.primary_last_update_ts.store(now, Ordering::Release);

        let mut current = self.current_version.write();
        *current = version;
    }

    /// Record a version observation from a region
    pub fn observe_version(&self, region: &str, version: DataVersion) {
        let mut versions = self.region_versions.write();
        versions.insert(region.to_string(), version);
    }

    /// Get current version
    pub fn current(&self) -> DataVersion {
        self.current_version.read().clone()
    }

    /// Check if all regions have converged to same version (within tolerance)
    /// Used to detect split-brain scenarios
    pub fn is_converged(&self, ledger_tolerance: u32) -> bool {
        let versions = self.region_versions.read();
        if versions.is_empty() {
            return true;
        }

        let ledgers: Vec<u32> = versions.values().map(|v| v.ledger_sequence).collect();
        let max = ledgers.iter().copied().max().unwrap_or(0);
        let min = ledgers.iter().copied().min().unwrap_or(0);

        max.saturating_sub(min) <= ledger_tolerance
    }

    /// Get version drift (max - min ledger across regions)
    pub fn version_drift(&self) -> u32 {
        let versions = self.region_versions.read();
        if versions.len() < 2 {
            return 0;
        }

        let ledgers: Vec<u32> = versions.values().map(|v| v.ledger_sequence).collect();
        let max = ledgers.iter().max().copied().unwrap_or(0);
        let min = ledgers.iter().min().copied().unwrap_or(0);
        max.saturating_sub(min)
    }

    /// Get all tracked versions for debugging
    pub fn debug_versions(&self) -> std::collections::HashMap<String, DataVersion> {
        self.region_versions.read().clone()
    }
}

impl Default for VersionTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_version_creation() {
        let version = DataVersion::new(12345);
        assert_eq!(version.ledger_sequence, 12345);
        assert!(version.age_secs() < 2); // Should be very fresh
    }

    #[test]
    fn test_staleness_check() {
        let version = DataVersion::new(100);
        assert!(!version.is_stale(10)); // 10 second tolerance (version is <1 sec old)

        // Manually set old timestamp
        let old_version = DataVersion {
            timestamp_micros: (Utc::now().timestamp_micros() - 20_000_000), // 20 seconds ago
            ledger_sequence: 100,
            content_hash: None,
        };
        assert!(old_version.is_stale(10));
    }

    #[test]
    fn test_consistency_constraint_strong() {
        let constraint = ConsistencyConstraint::strong();
        assert_eq!(constraint.max_age_secs, 1);
        assert!(!constraint.allow_degraded);
    }

    #[test]
    fn test_consistency_constraint_eventual() {
        let constraint = ConsistencyConstraint::eventual(60);
        assert_eq!(constraint.max_age_secs, 60);
        assert!(constraint.allow_degraded);
    }

    #[test]
    fn test_version_tracker_convergence() {
        let tracker = VersionTracker::new();
        tracker.observe_version("us-east", DataVersion::new(100));
        tracker.observe_version("eu-west", DataVersion::new(102));
        tracker.observe_version("ap-southeast", DataVersion::new(101));

        assert!(tracker.is_converged(5)); // 5 ledger tolerance
        assert!(!tracker.is_converged(1)); // 1 ledger tolerance
    }

    #[test]
    fn test_version_drift_calculation() {
        let tracker = VersionTracker::new();
        tracker.observe_version("us-east", DataVersion::new(100));
        tracker.observe_version("eu-west", DataVersion::new(110));

        assert_eq!(tracker.version_drift(), 10);
    }
}
