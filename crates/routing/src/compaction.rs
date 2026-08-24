use crate::cross_chain::BridgeEdgeMeta;
use crate::pathfinder::LiquidityEdge;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Compact adjacency-list edge used by the in-memory routes graph.
///
/// Safety metadata (`venue_type`, `provider`, `bridge`) is preserved losslessly
/// through [`CompactedGraph::from_edges`] → [`CompactedGraph::to_edges`] so a
/// bridge edge can never be laundered into `sdex`/`amm`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompactedEdge {
    pub to_idx: u32,
    /// Arbitrary venue kind string (`sdex`, `amm`, `bridge`, …). Source of truth.
    #[serde(default)]
    pub venue_type: String,
    /// Legacy AMM/SDEX index kept for additive serde compatibility.
    /// `0` = sdex, `1` = amm, `2` = bridge, `255` = other/unknown.
    /// Prefer [`Self::resolved_venue_type`] over this field.
    #[serde(default)]
    pub venue_type_idx: u8,
    pub venue_ref: String,
    pub liquidity: i128,
    pub price: f64,
    pub fee_bps: u32,
    pub anomaly_score: f32,
    /// Optional liquidity provider id (kill-switch / policy subject).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Optional bridge metadata (non-executable under default routing policy).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridge: Option<BridgeEdgeMeta>,
}

impl CompactedEdge {
    /// Resolve venue type, preferring the string field over the legacy index.
    pub fn resolved_venue_type(&self) -> &str {
        if !self.venue_type.is_empty() {
            return self.venue_type.as_str();
        }
        match self.venue_type_idx {
            1 => "amm",
            2 => "bridge",
            0 => "sdex",
            _ => "unknown",
        }
    }

    pub fn is_amm(&self) -> bool {
        self.resolved_venue_type() == "amm" || self.venue_type_idx == 1
    }
}

fn venue_type_idx_for(venue_type: &str, bridge: Option<&BridgeEdgeMeta>) -> u8 {
    if venue_type.eq_ignore_ascii_case("amm") {
        1
    } else if venue_type.eq_ignore_ascii_case("bridge") || bridge.is_some() {
        2
    } else if venue_type.eq_ignore_ascii_case("sdex") {
        0
    } else {
        255
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CompactedGraph {
    pub assets: Vec<String>,
    pub asset_map: HashMap<String, u32>,
    pub edges: Vec<CompactedEdge>,
    pub offsets: Vec<usize>, // offsets[i] is the start of edges for assets[i]
}

impl CompactedGraph {
    pub fn from_edges(edges: Vec<LiquidityEdge>) -> Self {
        let mut asset_map = HashMap::new();
        let mut assets = Vec::new();

        let mut get_asset_idx = |asset: &String| {
            if let Some(&idx) = asset_map.get(asset) {
                idx
            } else {
                let idx = assets.len() as u32;
                asset_map.insert(asset.clone(), idx);
                assets.push(asset.clone());
                idx
            }
        };

        // First pass: identify all assets
        for edge in &edges {
            get_asset_idx(&edge.from);
            get_asset_idx(&edge.to);
        }

        // Group edges by from_idx
        let mut grouped_edges: HashMap<u32, Vec<CompactedEdge>> = HashMap::new();
        for edge in edges {
            let from_idx = *asset_map.get(&edge.from).unwrap();
            let to_idx = *asset_map.get(&edge.to).unwrap();

            let c_edge = CompactedEdge {
                to_idx,
                venue_type: edge.venue_type.clone(),
                venue_type_idx: venue_type_idx_for(&edge.venue_type, edge.bridge.as_ref()),
                venue_ref: edge.venue_ref,
                liquidity: edge.liquidity,
                price: edge.price,
                fee_bps: edge.fee_bps,
                anomaly_score: 0.0_f32,
                provider: edge.provider,
                bridge: edge.bridge,
            };
            grouped_edges.entry(from_idx).or_default().push(c_edge);
        }

        let mut final_edges = Vec::new();
        let mut offsets = Vec::with_capacity(assets.len() + 1);

        for i in 0..assets.len() {
            offsets.push(final_edges.len());
            if let Some(mut neighbors) = grouped_edges.remove(&(i as u32)) {
                final_edges.append(&mut neighbors);
            }
        }
        offsets.push(final_edges.len());

        Self {
            assets,
            asset_map,
            edges: final_edges,
            offsets,
        }
    }

    pub fn get_neighbors(&self, asset_idx: u32) -> &[CompactedEdge] {
        let start = self.offsets[asset_idx as usize];
        let end = self.offsets[asset_idx as usize + 1];
        &self.edges[start..end]
    }

    pub fn asset_count(&self) -> usize {
        self.assets.len()
    }

    pub fn update_edge(
        &mut self,
        from: &str,
        venue_ref: &str,
        liquidity: i128,
        price: f64,
    ) -> bool {
        if let Some(&from_idx) = self.asset_map.get(from) {
            let start = self.offsets[from_idx as usize];
            let end = self.offsets[from_idx as usize + 1];
            for edge in &mut self.edges[start..end] {
                if edge.venue_ref == venue_ref {
                    edge.liquidity = liquidity;
                    edge.price = price;
                    return true;
                }
            }
        }
        false
    }

    /// Convert compacted graph back to LiquidityEdge vector (lossless for
    /// venue kind, provider, and bridge metadata).
    pub fn to_edges(&self) -> Vec<LiquidityEdge> {
        let mut edges = Vec::new();
        for (from_idx, from_asset) in self.assets.iter().enumerate() {
            let start = self.offsets[from_idx];
            let end = self.offsets[from_idx + 1];
            for compact_edge in &self.edges[start..end] {
                let to_asset = &self.assets[compact_edge.to_idx as usize];
                edges.push(LiquidityEdge {
                    from: from_asset.clone(),
                    to: to_asset.clone(),
                    venue_type: compact_edge.resolved_venue_type().to_string(),
                    venue_ref: compact_edge.venue_ref.clone(),
                    liquidity: compact_edge.liquidity,
                    price: compact_edge.price,
                    fee_bps: compact_edge.fee_bps,
                    provider: compact_edge.provider.clone(),
                    bridge: compact_edge.bridge.clone(),
                });
            }
        }
        edges
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain_asset::{AssetReference, ChainAsset, ChainId};
    use crate::cross_chain::BridgeEdgeMeta;
    use crate::cross_chain::ProviderPolicy;
    use crate::pathfinder::{Pathfinder, PathfinderConfig};
    use crate::policy::RoutingPolicy;

    const ISSUER: &str = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";

    fn xlm() -> String {
        ChainAsset::stellar_native("pubnet").to_canonical()
    }

    fn usdc() -> String {
        ChainAsset::stellar_credit("pubnet", "USDC", ISSUER)
            .unwrap()
            .to_canonical()
    }

    fn eth_usdc() -> String {
        ChainAsset::new(
            ChainId::ethereum_mainnet(),
            AssetReference::Erc20 {
                address: "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".into(),
            },
        )
        .unwrap()
        .to_canonical()
    }

    #[test]
    fn round_trip_preserves_bridge_identity() {
        let from = usdc();
        let to = eth_usdc();
        let bridge = BridgeEdgeMeta::bridge(
            "example-bridge",
            &ChainId::stellar_pubnet(),
            &ChainId::ethereum_mainnet(),
        );
        let edges = vec![LiquidityEdge {
            from: from.clone(),
            to: to.clone(),
            venue_type: "bridge".into(),
            venue_ref: "example-bridge:lane".into(),
            liquidity: 50_000_000_000,
            price: 1.0,
            fee_bps: 10,
            provider: Some("example-bridge".into()),
            bridge: Some(bridge.clone()),
        }];

        let graph = CompactedGraph::from_edges(edges);
        let restored = graph.to_edges();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].venue_type, "bridge");
        assert_eq!(restored[0].provider.as_deref(), Some("example-bridge"));
        assert_eq!(restored[0].bridge.as_ref(), Some(&bridge));
        assert_ne!(restored[0].venue_type, "sdex");
        assert_ne!(restored[0].venue_type, "amm");
    }

    #[test]
    fn compacted_bridge_still_rejected_by_default_pathfinding() {
        let from = usdc();
        let to = eth_usdc();
        let edges = vec![LiquidityEdge {
            from: from.clone(),
            to: to.clone(),
            venue_type: "bridge".into(),
            venue_ref: "bridge:lane".into(),
            liquidity: 50_000_000_000,
            price: 1.0,
            fee_bps: 10,
            provider: Some("example-bridge".into()),
            bridge: Some(BridgeEdgeMeta::bridge(
                "example-bridge",
                &ChainId::stellar_pubnet(),
                &ChainId::ethereum_mainnet(),
            )),
        }];

        let restored = CompactedGraph::from_edges(edges).to_edges();
        let finder = Pathfinder::new(PathfinderConfig {
            min_liquidity_threshold: 1,
        });
        let err = finder
            .find_paths(&from, &to, &restored, 1_000_000, &RoutingPolicy::default())
            .expect_err("bridge must remain non-executable after compaction");
        assert!(matches!(err, crate::error::RoutingError::NoRoute(_, _)));
    }

    #[test]
    fn provider_tagged_compacted_edge_excluded_after_round_trip() {
        let from = xlm();
        let to = usdc();
        let edges = vec![
            LiquidityEdge {
                from: from.clone(),
                to: to.clone(),
                venue_type: "amm".into(),
                venue_ref: "pool-bad".into(),
                liquidity: 10_000_000_000,
                price: 1.0,
                fee_bps: 30,
                provider: Some("bad-dex".into()),
                bridge: None,
            },
            LiquidityEdge {
                from: from.clone(),
                to: to.clone(),
                venue_type: "amm".into(),
                venue_ref: "pool-good".into(),
                liquidity: 10_000_000_000,
                price: 1.0,
                fee_bps: 30,
                provider: Some("good-dex".into()),
                bridge: None,
            },
        ];

        let restored = CompactedGraph::from_edges(edges).to_edges();
        assert!(restored
            .iter()
            .any(|e| e.provider.as_deref() == Some("bad-dex")));

        let policy = RoutingPolicy::default()
            .with_provider_policy(ProviderPolicy::default().with_kill_switch("bad-dex", true));
        let finder = Pathfinder::new(PathfinderConfig {
            min_liquidity_threshold: 1,
        });
        let paths = finder
            .find_paths(&from, &to, &restored, 1_000_000, &policy)
            .expect("good-dex should remain");
        for path in &paths {
            for hop in &path.hops {
                assert_ne!(hop.provider.as_deref(), Some("bad-dex"));
            }
        }
    }

    #[test]
    fn additive_serde_defaults_for_legacy_edges() {
        // Legacy payload with only venue_type_idx (no venue_type/provider/bridge).
        let json = r#"{
            "to_idx": 1,
            "venue_type_idx": 1,
            "venue_ref": "pool-1",
            "liquidity": 100,
            "price": 1.0,
            "fee_bps": 30,
            "anomaly_score": 0.0
        }"#;
        let edge: CompactedEdge = serde_json::from_str(json).unwrap();
        assert_eq!(edge.venue_type, "");
        assert_eq!(edge.resolved_venue_type(), "amm");
        assert!(edge.provider.is_none());
        assert!(edge.bridge.is_none());
    }
}
