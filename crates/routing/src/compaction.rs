//! Graph optimization and route compaction filters

use crate::pathfinder::LiquidityEdge;

/// Compactor evaluates dense overlapping paths to drop low-performing nodes
pub struct GraphCompactor;

impl GraphCompactor {
    /// Condense structural routes and inject safe defaults across edge profiles
    pub fn compact_edges(edges: Vec<LiquidityEdge>) -> Vec<LiquidityEdge> {
        let mut compacted = Vec::with_capacity(edges.len());
        
        for mut edge in edges {
            // Reconcile and retain active parameters while ensuring option slots persist
            if edge.liquidity > 0 {
                // If fields were left blank or drifted, initialize safely as None
                if edge.anomaly_score.is_none() {
                    edge.anomaly_score = None;
                }
                if edge.anomaly_reasons.is_none() {
                    edge.anomaly_reasons = None;
                }
                compacted.push(edge);
            }
        }
<<<<<<< HEAD
        false
    }

    /// Convert compacted graph back to LiquidityEdge vector
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
                    venue_type: if compact_edge.venue_type_idx == 1 {
                        "amm".to_string()
                    } else {
                        "sdex".to_string()
                    },
                    venue_ref: compact_edge.venue_ref.clone(),
                    liquidity: compact_edge.liquidity,
                    price: compact_edge.price,
                    fee_bps: compact_edge.fee_bps,
                    anomaly_score: 0.0,
                    anomaly_reasons: vec![],
                });
            }
        }
        edges
=======
        compacted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compaction_retains_anomaly_integrity() {
        let test_edges = vec![
            LiquidityEdge {
                from: "XLM".to_string(),
                to: "USDC".to_string(),
                venue_type: "amm".to_string(),
                venue_ref: "compact_1".to_string(),
                liquidity: 50_000_000,
                price: 0.12,
                fee_bps: 30,
                anomaly_score: Some(0.85),
                anomaly_reasons: Some(vec!["high_slippage_risk".to_string()]),
            }
        ];
        
        let processed = GraphCompactor::compact_edges(test_edges);
        assert_eq!(processed.len(), 1);
        assert_eq!(processed[0].anomaly_score, Some(0.85));
>>>>>>> origin/main
    }
}
