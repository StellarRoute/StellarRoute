//! Cross-chain venue metadata and provider policy hooks.
//!
//! These types are **abstractions only**. They describe bridge/cross-chain
//! venues and provider-level allow/deny/kill-switch controls so future
//! adapters can plug in without implying proprietary bridge settlement or
//! external execution exists today.
//!
//! Bridge edges are **non-executable** under default [`crate::policy::RoutingPolicy`]
//! (`allow_bridge_edges = false`). [`crate::compaction::CompactedGraph`] preserves
//! bridge/provider identity losslessly so compaction cannot launder a bridge into
//! `sdex`/`amm`. Provider kill-switches apply only to provider-tagged edges.

use crate::chain_asset::{ChainAsset, ChainId};
use crate::error::{Result, RoutingError};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Stable provider identifier for DEX venues and bridge adapters.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProviderId(pub String);

impl ProviderId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ProviderId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for ProviderId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

/// Classification of a liquidity venue for routing policy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VenueKind {
    /// Same-chain DEX venue (SDEX, AMM, …).
    Dex { venue_type: String },
    /// Cross-chain bridge adapter (metadata only; no settlement implementation).
    Bridge {
        provider: ProviderId,
        source_chain: ChainId,
        destination_chain: ChainId,
    },
}

impl VenueKind {
    pub fn is_bridge(&self) -> bool {
        matches!(self, Self::Bridge { .. })
    }

    pub fn provider(&self) -> Option<&ProviderId> {
        match self {
            Self::Bridge { provider, .. } => Some(provider),
            Self::Dex { .. } => None,
        }
    }

    pub fn venue_type_label(&self) -> &str {
        match self {
            Self::Dex { venue_type } => venue_type.as_str(),
            Self::Bridge { .. } => "bridge",
        }
    }
}

/// Optional bridge metadata attached to a liquidity edge / path hop.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BridgeEdgeMeta {
    /// Bridge / messaging provider id (adapter name), not a settlement proof.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Chain id string for the source endpoint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_chain: Option<String>,
    /// Chain id string for the destination endpoint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination_chain: Option<String>,
}

impl BridgeEdgeMeta {
    pub fn bridge(
        provider: impl Into<String>,
        source_chain: &ChainId,
        destination_chain: &ChainId,
    ) -> Self {
        Self {
            provider: Some(provider.into()),
            source_chain: Some(source_chain.to_caip2()),
            destination_chain: Some(destination_chain.to_caip2()),
        }
    }

    pub fn is_cross_chain(&self) -> bool {
        match (&self.source_chain, &self.destination_chain) {
            (Some(src), Some(dst)) => src != dst,
            _ => false,
        }
    }

    /// Ensure bridge chain labels match the parsed endpoint `ChainAsset`s.
    ///
    /// Validation runs even though bridge edges are non-executable by default,
    /// so stored metadata cannot contradict edge endpoints.
    pub fn validate_against_endpoints(&self, from_asset: &str, to_asset: &str) -> Result<()> {
        let from = ChainAsset::parse(from_asset)?;
        let to = ChainAsset::parse(to_asset)?;

        let Some(source) = self.source_chain.as_deref() else {
            return Err(RoutingError::InvalidAsset(
                "bridge metadata missing source_chain".to_string(),
            ));
        };
        let Some(dest) = self.destination_chain.as_deref() else {
            return Err(RoutingError::InvalidAsset(
                "bridge metadata missing destination_chain".to_string(),
            ));
        };

        if source != from.chain.to_caip2() {
            return Err(RoutingError::InvalidAsset(format!(
                "bridge source_chain {source} does not match from asset chain {}",
                from.chain.to_caip2()
            )));
        }
        if dest != to.chain.to_caip2() {
            return Err(RoutingError::InvalidAsset(format!(
                "bridge destination_chain {dest} does not match to asset chain {}",
                to.chain.to_caip2()
            )));
        }
        Ok(())
    }
}

/// Returns true when an edge is a bridge edge (venue type or metadata).
pub fn is_bridge_edge(venue_type: &str, bridge: Option<&BridgeEdgeMeta>) -> bool {
    venue_type.eq_ignore_ascii_case("bridge") || bridge.is_some()
}

/// Provider-level routing policy hooks (allow/deny + kill switch).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProviderPolicy {
    /// When non-empty, only listed providers are eligible.
    #[serde(default)]
    pub allowlist: Vec<String>,
    /// Providers that must never appear in a route.
    #[serde(default)]
    pub denylist: Vec<String>,
    /// Hard kill switches keyed by provider id (`true` = disabled).
    #[serde(default)]
    pub kill_switches: HashMap<String, bool>,
}

impl ProviderPolicy {
    pub fn with_allowlist(mut self, allowlist: Vec<String>) -> Self {
        self.allowlist = allowlist;
        self
    }

    pub fn with_denylist(mut self, denylist: Vec<String>) -> Self {
        self.denylist = denylist;
        self
    }

    pub fn with_kill_switch(mut self, provider: impl Into<String>, disabled: bool) -> Self {
        self.kill_switches.insert(provider.into(), disabled);
        self
    }

    /// Merge kill switches from another policy (OR: disabled if either says so).
    pub fn merge_kill_switches(&mut self, other: &ProviderPolicy) {
        for (provider, disabled) in &other.kill_switches {
            if *disabled {
                self.kill_switches.insert(provider.clone(), true);
            }
        }
    }

    /// Returns true when a provider is allowed under allow/deny/kill-switch rules.
    ///
    /// `None` provider is allowed for same-chain DEX edges that do not declare one.
    pub fn is_provider_allowed(&self, provider: Option<&str>) -> bool {
        let Some(provider) = provider else {
            return true;
        };

        if self.kill_switches.get(provider).copied().unwrap_or(false) {
            return false;
        }

        if !self.allowlist.is_empty() && !self.allowlist.iter().any(|p| p == provider) {
            return false;
        }

        !self.denylist.iter().any(|p| p == provider)
    }

    pub fn validate(&self) -> std::result::Result<(), String> {
        if !self.allowlist.is_empty() && !self.denylist.is_empty() {
            let allow: HashSet<&str> = self.allowlist.iter().map(String::as_str).collect();
            let deny: HashSet<&str> = self.denylist.iter().map(String::as_str).collect();
            let overlap: Vec<_> = allow.intersection(&deny).collect();
            if !overlap.is_empty() {
                return Err(format!(
                    "providers appear in both allowlist and denylist: {overlap:?}"
                ));
            }
        }
        Ok(())
    }
}

/// Evaluate whether a venue/provider combination should be excluded.
pub fn should_exclude_provider(
    policy: &ProviderPolicy,
    provider: Option<&str>,
) -> Option<&'static str> {
    if policy.is_provider_allowed(provider) {
        None
    } else {
        Some("provider_policy_excluded")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain_asset::{AssetReference, ChainAsset, ChainId};

    #[test]
    fn provider_kill_switch_blocks_provider() {
        let policy = ProviderPolicy::default().with_kill_switch("wormhole-adapter", true);
        assert!(!policy.is_provider_allowed(Some("wormhole-adapter")));
        assert!(policy.is_provider_allowed(Some("other-bridge")));
        assert!(policy.is_provider_allowed(None));
    }

    #[test]
    fn bridge_meta_must_match_endpoint_chains() {
        let from = ChainAsset::stellar_credit(
            "pubnet",
            "USDC",
            "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5",
        )
        .unwrap()
        .to_canonical();
        let to = ChainAsset::new(
            ChainId::ethereum_mainnet(),
            AssetReference::Erc20 {
                address: "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".into(),
            },
        )
        .unwrap()
        .to_canonical();

        let ok = BridgeEdgeMeta::bridge(
            "example-bridge",
            &ChainId::stellar_pubnet(),
            &ChainId::ethereum_mainnet(),
        );
        assert!(ok.validate_against_endpoints(&from, &to).is_ok());

        let bad = BridgeEdgeMeta::bridge(
            "example-bridge",
            &ChainId::ethereum_mainnet(),
            &ChainId::stellar_pubnet(),
        );
        assert!(bad.validate_against_endpoints(&from, &to).is_err());
    }

    #[test]
    fn is_bridge_edge_detects_type_or_meta() {
        assert!(is_bridge_edge("bridge", None));
        assert!(is_bridge_edge(
            "amm",
            Some(&BridgeEdgeMeta::bridge(
                "x",
                &ChainId::stellar_pubnet(),
                &ChainId::ethereum_mainnet()
            ))
        ));
        assert!(!is_bridge_edge("amm", None));
    }
}
