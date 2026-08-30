//! Configurable routing policy controls
//!
//! Provides `RoutingPolicy` for controlling route discovery behaviour:
//! - **max_hops**: caps the depth of multi-hop paths (default: 4).
//! - **venue_allowlist / venue_denylist**
//! - **asset_denylist**: excludes routes containing specific assets
//!
//! Includes route-level exclusion evaluation and diagnostics.

use crate::cross_chain::ProviderPolicy;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Default slippage tolerance in basis points (0.50%).
pub const DEFAULT_SLIPPAGE_BPS: u32 = 50;

/// Diagnostic information for excluded routes
#[derive(Clone, Debug)]
pub struct RouteDiagnostic {
    pub route_id: String,
    pub reason: String,
}

/// Configurable routing policy for controlling route discovery
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoutingPolicy {
    pub max_hops: usize,

    pub venue_allowlist: Vec<String>,
    pub venue_denylist: Vec<String>,

    /// Assets that should never appear in a route
    pub asset_denylist: Vec<String>,

    /// Default slippage tolerance in basis points for route simulation.
    #[serde(default = "default_slippage_bps")]
    pub default_slippage_bps: u32,

    /// Per-venue slippage overrides keyed by `venue_ref`.
    #[serde(default)]
    pub venue_slippage_overrides: HashMap<String, u32>,

    /// Provider-level allow/deny and kill-switch hooks (bridge adapters, etc.).
    #[serde(default)]
    pub provider_policy: ProviderPolicy,

    /// When `false` (default / production), bridge edges are hard-disabled from
    /// pathfinding. Settlement is metadata-only; current API quote paths must
    /// never set this to `true`.
    #[serde(default)]
    pub allow_bridge_edges: bool,
}

fn default_slippage_bps() -> u32 {
    DEFAULT_SLIPPAGE_BPS
}

impl Default for RoutingPolicy {
    fn default() -> Self {
        Self {
            max_hops: 4,
            venue_allowlist: Vec::new(),
            venue_denylist: Vec::new(),
            asset_denylist: Vec::new(),
            default_slippage_bps: DEFAULT_SLIPPAGE_BPS,
            venue_slippage_overrides: HashMap::new(),
            provider_policy: ProviderPolicy::default(),
            // Bridges are non-executable until settlement exists.
            allow_bridge_edges: false,
        }
    }
}

impl RoutingPolicy {
    pub fn new(max_hops: usize) -> Self {
        Self {
            max_hops,
            ..Default::default()
        }
    }

    pub fn with_max_hops(mut self, max_hops: usize) -> Self {
        self.max_hops = max_hops;
        self
    }

    pub fn with_venue_allowlist(mut self, allowlist: Vec<String>) -> Self {
        self.venue_allowlist = allowlist;
        self
    }

    pub fn with_venue_denylist(mut self, denylist: Vec<String>) -> Self {
        self.venue_denylist = denylist;
        self
    }

    /// NEW: Builder for asset denylist
    pub fn with_asset_denylist(mut self, denylist: Vec<String>) -> Self {
        self.asset_denylist = denylist;
        self
    }

    pub fn with_provider_policy(mut self, provider_policy: ProviderPolicy) -> Self {
        self.provider_policy = provider_policy;
        self
    }

    /// Explicit future opt-in for bridge edges. Defaults to `false`.
    ///
    /// Must not be enabled by `/api/v1` quote/route handlers.
    pub fn with_allow_bridge_edges(mut self, allow: bool) -> Self {
        self.allow_bridge_edges = allow;
        self
    }

    pub fn with_default_slippage_bps(mut self, slippage_bps: u32) -> Self {
        self.default_slippage_bps = slippage_bps;
        self
    }

    /// Merge per-venue slippage overrides into this policy.
    pub fn apply_venue_slippage_overrides(
        &mut self,
        overrides: impl IntoIterator<Item = (String, u32)>,
    ) {
        for (venue_ref, slippage_bps) in overrides {
            self.venue_slippage_overrides
                .insert(venue_ref, slippage_bps);
        }
    }

    /// Resolve slippage tolerance for a hop, falling back to the policy default.
    pub fn slippage_bps_for_venue(&self, venue_ref: Option<&str>) -> u32 {
        if let Some(venue_ref) = venue_ref {
            if let Some(&slippage_bps) = self.venue_slippage_overrides.get(venue_ref) {
                return slippage_bps;
            }
        }
        self.default_slippage_bps
    }

    pub fn is_venue_allowed(&self, venue_type: &str) -> bool {
        if !self.venue_allowlist.is_empty() && !self.venue_allowlist.iter().any(|v| v == venue_type)
        {
            return false;
        }

        !self.venue_denylist.iter().any(|v| v == venue_type)
    }

    /// NEW: Check if asset is allowed
    pub fn is_asset_allowed(&self, asset: &str) -> bool {
        !self.asset_denylist.iter().any(|a| a == asset)
    }

    /// Provider allow/deny + kill-switch check.
    pub fn is_provider_allowed(&self, provider: Option<&str>) -> bool {
        self.provider_policy.is_provider_allowed(provider)
    }

    /// 🔥 CORE FUNCTION (Acceptance Criteria)
    ///
    /// Evaluates whether a route should be excluded.
    /// Returns:
    /// - None → route allowed
    /// - Some(reason) → route excluded
    pub fn should_exclude_route(
        &self,
        route_id: &str,
        hops: &[RouteHop],
    ) -> Option<RouteDiagnostic> {
        for hop in hops {
            if crate::cross_chain::is_bridge_edge(&hop.venue_type, hop.bridge.as_ref())
                && !self.allow_bridge_edges
            {
                return Some(RouteDiagnostic {
                    route_id: route_id.to_string(),
                    reason: "Excluded bridge: settlement not executable".to_string(),
                });
            }

            if !self.is_venue_allowed(&hop.venue_type) {
                return Some(RouteDiagnostic {
                    route_id: route_id.to_string(),
                    reason: format!("Excluded venue: {}", hop.venue_type),
                });
            }

            if !self.is_asset_allowed(&hop.asset) {
                return Some(RouteDiagnostic {
                    route_id: route_id.to_string(),
                    reason: format!("Excluded asset: {}", hop.asset),
                });
            }

            if !self.is_provider_allowed(hop.provider.as_deref()) {
                return Some(RouteDiagnostic {
                    route_id: route_id.to_string(),
                    reason: format!(
                        "Excluded provider: {}",
                        hop.provider.as_deref().unwrap_or("<none>")
                    ),
                });
            }
        }

        None
    }

    pub fn from_env() -> Self {
        let max_hops: usize = std::env::var("ROUTING_MAX_HOPS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(4);

        let venue_allowlist =
            parse_comma_list(&std::env::var("ROUTING_VENUE_ALLOWLIST").unwrap_or_default());

        let venue_denylist =
            parse_comma_list(&std::env::var("ROUTING_VENUE_DENYLIST").unwrap_or_default());

        let asset_denylist =
            parse_comma_list(&std::env::var("ROUTING_ASSET_DENYLIST").unwrap_or_default());

        Self {
            max_hops,
            venue_allowlist,
            venue_denylist,
            asset_denylist,
            default_slippage_bps: DEFAULT_SLIPPAGE_BPS,
            venue_slippage_overrides: HashMap::new(),
            provider_policy: ProviderPolicy::default(),
            allow_bridge_edges: false,
        }
    }

    pub fn validate(&self) -> std::result::Result<(), String> {
        if self.max_hops == 0 {
            return Err("max_hops must be at least 1".to_string());
        }

        if !self.venue_allowlist.is_empty() && !self.venue_denylist.is_empty() {
            let allow_set: HashSet<&str> =
                self.venue_allowlist.iter().map(|s| s.as_str()).collect();
            let deny_set: HashSet<&str> = self.venue_denylist.iter().map(|s| s.as_str()).collect();

            let overlap: Vec<&&str> = allow_set.intersection(&deny_set).collect();

            if !overlap.is_empty() {
                return Err(format!(
                    "venue types appear in both allowlist and denylist: {:?}",
                    overlap
                ));
            }
        }

        self.provider_policy.validate()?;

        Ok(())
    }
}

/// Minimal representation of a route hop (adapt if already defined elsewhere)
#[derive(Clone, Debug)]
pub struct RouteHop {
    pub venue_type: String,
    pub asset: String,
    pub provider: Option<String>,
    pub bridge: Option<crate::cross_chain::BridgeEdgeMeta>,
}

fn parse_comma_list(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slippage_bps_for_venue_uses_default_when_no_override() {
        let policy = RoutingPolicy::default().with_default_slippage_bps(75);
        assert_eq!(policy.slippage_bps_for_venue(None), 75);
        assert_eq!(policy.slippage_bps_for_venue(Some("pool-a")), 75);
    }

    #[test]
    fn slippage_bps_for_venue_uses_override_when_present() {
        let mut policy = RoutingPolicy::default().with_default_slippage_bps(50);
        policy.apply_venue_slippage_overrides(vec![("pool-a".to_string(), 200)]);
        assert_eq!(policy.slippage_bps_for_venue(Some("pool-a")), 200);
        assert_eq!(policy.slippage_bps_for_venue(Some("pool-b")), 50);
    }

    #[test]
    fn apply_venue_slippage_overrides_merges_entries() {
        let mut policy = RoutingPolicy::default();
        policy.apply_venue_slippage_overrides(vec![
            ("pool-a".to_string(), 100),
            ("pool-b".to_string(), 150),
        ]);
        policy.apply_venue_slippage_overrides(vec![("pool-a".to_string(), 125)]);

        assert_eq!(policy.slippage_bps_for_venue(Some("pool-a")), 125);
        assert_eq!(policy.slippage_bps_for_venue(Some("pool-b")), 150);
    }
}
