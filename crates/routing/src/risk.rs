//! Per-asset risk limits for route selection
//!
//! Provides configurable risk controls that are enforced during route computation:
//! - **max_exposure**: Maximum position size allowed for an asset
//! - **max_impact_bps**: Maximum acceptable price impact in basis points
//! - **liquidity_floor**: Minimum liquidity required to consider a route
//!
//! Limits can be configured per-asset or use global defaults.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExclusionReason {
    MaxExposureExceeded,
    MaxImpactExceeded,
    LiquidityBelowFloor,
    AssetBlacklisted,
    LiquidityAnomaly,
}

impl std::fmt::Display for ExclusionReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExclusionReason::MaxExposureExceeded => write!(f, "max_exposure_exceeded"),
            ExclusionReason::MaxImpactExceeded => write!(f, "max_impact_exceeded"),
            ExclusionReason::LiquidityBelowFloor => write!(f, "liquidity_below_floor"),
            ExclusionReason::AssetBlacklisted => write!(f, "asset_blacklisted"),
            ExclusionReason::LiquidityAnomaly => write!(f, "liquidity_anomaly"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteExclusion {
    pub asset: String,
    pub reason: ExclusionReason,
    pub limit_value: i128,
    pub actual_value: i128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRiskLimit {
    pub max_exposure: i128,
    pub max_impact_bps: u32,
    pub liquidity_floor: i128,
    pub blacklisted: bool,
}

impl Default for AssetRiskLimit {
    fn default() -> Self {
        Self {
            max_exposure: i128::MAX,
            max_impact_bps: 500,
            liquidity_floor: 1_000_000,
            blacklisted: false,
        }
    }
}

impl AssetRiskLimit {
    pub fn strict() -> Self {
        Self {
            max_exposure: 10_000_000_000,
            max_impact_bps: 100,
            liquidity_floor: 100_000_000,
            blacklisted: false,
        }
    }

    pub fn permissive() -> Self {
        Self {
            max_exposure: i128::MAX,
            max_impact_bps: 1000,
            liquidity_floor: 100_000,
            blacklisted: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RiskLimitConfig {
    pub global_defaults: AssetRiskLimit,
    pub per_asset: HashMap<String, AssetRiskLimit>,
}

impl RiskLimitConfig {
    pub fn new(global_defaults: AssetRiskLimit) -> Self {
        Self {
            global_defaults,
            per_asset: HashMap::new(),
        }
    }

    pub fn with_asset_limit(mut self, asset: impl Into<String>, limit: AssetRiskLimit) -> Self {
        self.per_asset.insert(asset.into(), limit);
        self
    }

    pub fn get_limit(&self, asset: &str) -> &AssetRiskLimit {
        self.per_asset.get(asset).unwrap_or(&self.global_defaults)
    }

    pub fn set_asset_limit(&mut self, asset: impl Into<String>, limit: AssetRiskLimit) {
        self.per_asset.insert(asset.into(), limit);
    }

    pub fn remove_asset_limit(&mut self, asset: &str) -> Option<AssetRiskLimit> {
        self.per_asset.remove(asset)
    }

    pub fn strict_policy() -> Self {
        Self {
            global_defaults: AssetRiskLimit::strict(),
            per_asset: HashMap::new(),
        }
    }

    pub fn permissive_policy() -> Self {
        Self {
            global_defaults: AssetRiskLimit::permissive(),
            per_asset: HashMap::new(),
        }
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

pub struct RiskValidator {
    config: RiskLimitConfig,
}

impl RiskValidator {
    pub fn new(config: RiskLimitConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &RiskLimitConfig {
        &self.config
    }

    pub fn validate_exposure(&self, asset: &str, exposure: i128) -> Result<(), RouteExclusion> {
        let limit = self.config.get_limit(asset);

        if limit.blacklisted {
            return Err(RouteExclusion {
                asset: asset.to_string(),
                reason: ExclusionReason::AssetBlacklisted,
                limit_value: 0,
                actual_value: exposure,
            });
        }

        if exposure > limit.max_exposure {
            return Err(RouteExclusion {
                asset: asset.to_string(),
                reason: ExclusionReason::MaxExposureExceeded,
                limit_value: limit.max_exposure,
                actual_value: exposure,
            });
        }

        Ok(())
    }

    pub fn validate_impact(&self, asset: &str, impact_bps: u32) -> Result<(), RouteExclusion> {
        let limit = self.config.get_limit(asset);

        if limit.blacklisted {
            return Err(RouteExclusion {
                asset: asset.to_string(),
                reason: ExclusionReason::AssetBlacklisted,
                limit_value: 0,
                actual_value: impact_bps as i128,
            });
        }

        if impact_bps > limit.max_impact_bps {
            return Err(RouteExclusion {
                asset: asset.to_string(),
                reason: ExclusionReason::MaxImpactExceeded,
                limit_value: limit.max_impact_bps as i128,
                actual_value: impact_bps as i128,
            });
        }

        Ok(())
    }

    pub fn validate_liquidity(&self, asset: &str, liquidity: i128) -> Result<(), RouteExclusion> {
        let limit = self.config.get_limit(asset);

        if limit.blacklisted {
            return Err(RouteExclusion {
                asset: asset.to_string(),
                reason: ExclusionReason::AssetBlacklisted,
                limit_value: 0,
                actual_value: liquidity,
            });
        }

        if liquidity < limit.liquidity_floor {
            return Err(RouteExclusion {
                asset: asset.to_string(),
                reason: ExclusionReason::LiquidityBelowFloor,
                limit_value: limit.liquidity_floor,
                actual_value: liquidity,
            });
        }

        Ok(())
    }

    pub fn validate_route(
        &self,
        asset: &str,
        exposure: i128,
        impact_bps: u32,
        liquidity: i128,
    ) -> Result<(), Vec<RouteExclusion>> {
        let mut exclusions = Vec::new();

        if let Err(e) = self.validate_exposure(asset, exposure) {
            exclusions.push(e);
        }

        if let Err(e) = self.validate_impact(asset, impact_bps) {
            exclusions.push(e);
        }

        if let Err(e) = self.validate_liquidity(asset, liquidity) {
            exclusions.push(e);
        }

        if exclusions.is_empty() {
            Ok(())
        } else {
            Err(exclusions)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_limits() {
        let limit = AssetRiskLimit::default();
        assert_eq!(limit.max_exposure, i128::MAX);
        assert_eq!(limit.max_impact_bps, 500);
        assert_eq!(limit.liquidity_floor, 1_000_000);
        assert!(!limit.blacklisted);
    }

    #[test]
    fn test_strict_limits() {
        let limit = AssetRiskLimit::strict();
        assert_eq!(limit.max_exposure, 10_000_000_000);
        assert_eq!(limit.max_impact_bps, 100);
        assert_eq!(limit.liquidity_floor, 100_000_000);
    }

    #[test]
    fn test_permissive_limits() {
        let limit = AssetRiskLimit::permissive();
        assert_eq!(limit.max_exposure, i128::MAX);
        assert_eq!(limit.max_impact_bps, 1000);
        assert_eq!(limit.liquidity_floor, 100_000);
    }

    #[test]
    fn test_config_per_asset_override() {
        let config = RiskLimitConfig::default().with_asset_limit("USDC", AssetRiskLimit::strict());

        let xlm_limit = config.get_limit("XLM");
        assert_eq!(xlm_limit.max_impact_bps, 500);

        let usdc_limit = config.get_limit("USDC");
        assert_eq!(usdc_limit.max_impact_bps, 100);
    }

    #[test]
    fn test_validator_exposure_pass() {
        let config = RiskLimitConfig::default();
        let validator = RiskValidator::new(config);

        assert!(validator.validate_exposure("XLM", 1_000_000).is_ok());
    }

    #[test]
    fn test_validator_exposure_fail() {
        let config = RiskLimitConfig::default().with_asset_limit(
            "XLM",
            AssetRiskLimit {
                max_exposure: 1_000_000,
                ..Default::default()
            },
        );
        let validator = RiskValidator::new(config);

        let result = validator.validate_exposure("XLM", 2_000_000);
        assert!(result.is_err());
        let exclusion = result.unwrap_err();
        assert_eq!(exclusion.reason, ExclusionReason::MaxExposureExceeded);
    }

    #[test]
    fn test_validator_impact_fail() {
        let config = RiskLimitConfig::strict_policy();
        let validator = RiskValidator::new(config);

        let result = validator.validate_impact("XLM", 200);
        assert!(result.is_err());
        let exclusion = result.unwrap_err();
        assert_eq!(exclusion.reason, ExclusionReason::MaxImpactExceeded);
    }

    #[test]
    fn test_validator_liquidity_fail() {
        let config = RiskLimitConfig::strict_policy();
        let validator = RiskValidator::new(config);

        let result = validator.validate_liquidity("XLM", 50_000_000);
        assert!(result.is_err());
        let exclusion = result.unwrap_err();
        assert_eq!(exclusion.reason, ExclusionReason::LiquidityBelowFloor);
    }

    #[test]
    fn test_validator_blacklisted_asset() {
        let config = RiskLimitConfig::default().with_asset_limit(
            "SCAM",
            AssetRiskLimit {
                blacklisted: true,
                ..Default::default()
            },
        );
        let validator = RiskValidator::new(config);

        let result = validator.validate_exposure("SCAM", 100);
        assert!(result.is_err());
        let exclusion = result.unwrap_err();
        assert_eq!(exclusion.reason, ExclusionReason::AssetBlacklisted);
    }

    #[test]
    fn test_validate_route_multiple_failures() {
        let config = RiskLimitConfig::strict_policy();
        let validator = RiskValidator::new(config);

        let result = validator.validate_route("XLM", 100_000_000_000, 500, 10_000);
        assert!(result.is_err());
        let exclusions = result.unwrap_err();
        assert!(exclusions.len() >= 2);
    }

    #[test]
    fn test_config_json_roundtrip() {
        let config = RiskLimitConfig::default().with_asset_limit("USDC", AssetRiskLimit::strict());

        let json = config.to_json().unwrap();
        let parsed = RiskLimitConfig::from_json(&json).unwrap();

        assert_eq!(
            parsed.get_limit("USDC").max_impact_bps,
            config.get_limit("USDC").max_impact_bps
        );
    }

    #[test]
    fn test_exclusion_reason_display() {
        assert_eq!(
            ExclusionReason::MaxExposureExceeded.to_string(),
            "max_exposure_exceeded"
        );
        assert_eq!(
            ExclusionReason::MaxImpactExceeded.to_string(),
            "max_impact_exceeded"
        );
        assert_eq!(
            ExclusionReason::LiquidityBelowFloor.to_string(),
            "liquidity_below_floor"
        );
        assert_eq!(
            ExclusionReason::AssetBlacklisted.to_string(),
            "asset_blacklisted"
        );
    }
}
