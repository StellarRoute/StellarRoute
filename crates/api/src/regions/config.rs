use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Ordered geographical regions for read replica distribution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum RegionId {
    /// Primary region - N. Virginia
    UsEast,
    /// Secondary region - Ireland
    EuWest,
    /// Tertiary region - Sydney
    ApSoutheast,
}

impl RegionId {
    pub fn as_str(&self) -> &'static str {
        match self {
            RegionId::UsEast => "us-east",
            RegionId::EuWest => "eu-west",
            RegionId::ApSoutheast => "ap-southeast",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "us-east" => Some(RegionId::UsEast),
            "eu-west" => Some(RegionId::EuWest),
            "ap-southeast" => Some(RegionId::ApSoutheast),
            _ => None,
        }
    }

    /// All regions in priority order (primary → fallback)
    pub fn all_ordered() -> Vec<RegionId> {
        vec![RegionId::UsEast, RegionId::EuWest, RegionId::ApSoutheast]
    }
}

impl std::fmt::Display for RegionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Configuration for a single region's read replica
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionConfig {
    /// Region identifier
    pub region_id: RegionId,

    /// Database URL for this region's read replica
    pub database_url: String,

    /// Priority order (0 = highest priority / primary)
    pub priority: u8,

    /// Maximum acceptable replica lag in seconds
    /// Before this region is considered "stale"
    pub max_replica_lag_secs: u32,

    /// Maximum accepted staleness per read in seconds
    /// Queries return error if data older than this
    pub max_staleness_secs: u32,

    /// Connection pool size for this region
    pub pool_size: u32,

    /// Health check interval in seconds
    pub health_check_interval_secs: u64,

    /// Circuit breaker: fail threshold (consecutive failures before open)
    pub circuit_breaker_threshold: u32,

    /// Circuit breaker: recovery timeout in seconds before half-open
    pub circuit_breaker_timeout_secs: u64,

    /// Enable this region for reads
    pub enabled: bool,
}

impl RegionConfig {
    /// Create a default config for a region
    pub fn new(region_id: RegionId, database_url: String, priority: u8) -> Self {
        RegionConfig {
            region_id,
            database_url,
            priority,
            max_replica_lag_secs: 5,          // 5 second acceptable lag
            max_staleness_secs: 10,           // 10 second freshness requirement
            pool_size: 5,                     // Smaller per-region pool
            health_check_interval_secs: 3,    // Check every 3 seconds
            circuit_breaker_threshold: 3,     // Fail after 3 bad checks
            circuit_breaker_timeout_secs: 30, // Try recovery after 30 seconds
            enabled: true,
        }
    }
}

/// Registry managing all regional read replicas
#[derive(Debug, Clone)]
pub struct RegionRegistry {
    /// Regional configurations ordered by priority
    configurations: Arc<HashMap<RegionId, RegionConfig>>,
}

impl RegionRegistry {
    /// Create registry from environment or config
    /// Supports format: REGIONS_CONFIG=us-east:postgres://...,eu-west:postgres://...
    pub fn from_env() -> Result<Self, String> {
        let mut configs = HashMap::new();

        // Primary region (mandatory)
        let primary_url = std::env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL not set".to_string())?;
        configs.insert(
            RegionId::UsEast,
            RegionConfig::new(RegionId::UsEast, primary_url, 0),
        );

        // Secondary regions (optional)
        if let Ok(secondary_url) = std::env::var("DATABASE_URL_EU_WEST") {
            configs.insert(
                RegionId::EuWest,
                RegionConfig::new(RegionId::EuWest, secondary_url, 1),
            );
        }

        if let Ok(tertiary_url) = std::env::var("DATABASE_URL_AP_SOUTHEAST") {
            configs.insert(
                RegionId::ApSoutheast,
                RegionConfig::new(RegionId::ApSoutheast, tertiary_url, 2),
            );
        }

        Ok(RegionRegistry {
            configurations: Arc::new(configs),
        })
    }

    /// Create with custom configurations
    pub fn with_configs(configs: Vec<RegionConfig>) -> Self {
        let mut map = HashMap::new();
        for config in configs {
            map.insert(config.region_id, config);
        }
        RegionRegistry {
            configurations: Arc::new(map),
        }
    }

    /// Get all regions in priority order
    pub fn all_regions(&self) -> Vec<RegionId> {
        let mut regions: Vec<_> = self.configurations.keys().copied().collect();
        regions.sort_by_key(|r| {
            self.configurations
                .get(r)
                .map(|c| c.priority)
                .unwrap_or(u8::MAX)
        });
        regions
    }

    /// Get configuration for a region
    pub fn get_config(&self, region_id: RegionId) -> Option<RegionConfig> {
        self.configurations.get(&region_id).cloned()
    }

    /// Get primary region (lowest priority number)
    pub fn primary_region(&self) -> Option<RegionId> {
        self.all_regions().into_iter().next()
    }

    /// Get all enabled regions in priority order
    pub fn enabled_regions(&self) -> Vec<RegionId> {
        self.all_regions()
            .into_iter()
            .filter(|r| {
                self.configurations
                    .get(r)
                    .map(|c| c.enabled)
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Total number of regions
    pub fn region_count(&self) -> usize {
        self.configurations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_id_ordering() {
        let regions = RegionId::all_ordered();
        assert_eq!(regions[0], RegionId::UsEast);
        assert_eq!(regions[1], RegionId::EuWest);
        assert_eq!(regions[2], RegionId::ApSoutheast);
    }

    #[test]
    fn test_region_registry_priority() {
        let configs = vec![
            RegionConfig::new(RegionId::UsEast, "postgres://us".to_string(), 0),
            RegionConfig::new(RegionId::EuWest, "postgres://eu".to_string(), 1),
        ];
        let registry = RegionRegistry::with_configs(configs);
        let regions = registry.all_regions();
        assert_eq!(regions[0], RegionId::UsEast);
        assert_eq!(regions[1], RegionId::EuWest);
    }

    #[test]
    fn test_region_id_string_conversion() {
        assert_eq!(RegionId::UsEast.as_str(), "us-east");
        assert_eq!(RegionId::from_str("us-east"), Some(RegionId::UsEast));
        assert_eq!(RegionId::from_str("invalid"), None);
    }

    #[test]
    fn test_region_config_defaults() {
        let config = RegionConfig::new(
            RegionId::EuWest,
            "postgres://eu".to_string(),
            1,
        );
        assert_eq!(config.max_replica_lag_secs, 5);
        assert_eq!(config.max_staleness_secs, 10);
        assert!(config.enabled);
    }
}
