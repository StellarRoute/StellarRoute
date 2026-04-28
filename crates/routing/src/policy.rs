//! Configurable routing policy controls
//!
//! Provides `RoutingPolicy` for controlling route discovery behaviour:
//! - **max_hops**: caps the depth of multi-hop paths (default: 4).
//! - **venue_allowlist / venue_denylist**
//! - **asset_denylist**: excludes routes containing specific assets
//!
//! Includes route-level exclusion evaluation and diagnostics.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

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
}

impl Default for RoutingPolicy {
    fn default() -> Self {
        Self {
            max_hops: 4,
            venue_allowlist: Vec::new(),
            venue_denylist: Vec::new(),
            asset_denylist: Vec::new(),
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

    pub fn is_venue_allowed(&self, venue_type: &str) -> bool {
        if !self.venue_allowlist.is_empty()
            && !self.venue_allowlist.iter().any(|v| v == venue_type)
        {
            return false;
        }

        !self.venue_denylist.iter().any(|v| v == venue_type)
    }

    /// NEW: Check if asset is allowed
    pub fn is_asset_allowed(&self, asset: &str) -> bool {
        !self.asset_denylist.iter().any(|a| a == asset)
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
        }
    }

    pub fn validate(&self) -> std::result::Result<(), String> {
        if self.max_hops == 0 {
            return Err("max_hops must be at least 1".to_string());
        }

        if !self.venue_allowlist.is_empty() && !self.venue_denylist.is_empty() {
            let allow_set: HashSet<&str> =
                self.venue_allowlist.iter().map(|s| s.as_str()).collect();
            let deny_set: HashSet<&str> =
                self.venue_denylist.iter().map(|s| s.as_str()).collect();

            let overlap: Vec<&&str> = allow_set.intersection(&deny_set).collect();

            if !overlap.is_empty() {
                return Err(format!(
                    "venue types appear in both allowlist and denylist: {:?}",
                    overlap
                ));
            }
        }

        Ok(())
    }
}

/// Minimal representation of a route hop (adapt if already defined elsewhere)
#[derive(Clone, Debug)]
pub struct RouteHop {
    pub venue_type: String,
    pub asset: String,
}

fn parse_comma_list(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}