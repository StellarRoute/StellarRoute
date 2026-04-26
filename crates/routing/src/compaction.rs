use crate::pathfinder::LiquidityEdge;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompactedEdge {
    pub to_idx: u32,
    pub venue_type_idx: u8, // Index into a small static table or enum
    pub venue_ref: String,
    pub liquidity: i128,
    pub price: f64,
    pub fee_bps: u32,
    pub anomaly_score: f32,
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
                venue_type_idx: if edge.venue_type == "amm" { 1 } else { 0 },
                venue_ref: edge.venue_ref,
                liquidity: edge.liquidity,
                price: edge.price,
                fee_bps: edge.fee_bps,
                anomaly_score: edge.anomaly_score as f32,
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
}
