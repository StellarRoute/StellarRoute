//! Route graph snapshot version endpoint.

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use stellarroute_routing::compaction::{CompactedEdge, CompactedGraph};
use utoipa::ToSchema;

use crate::{middleware::RequestId, models::ApiResponse, state::AppState};

const HASH_VERSION: u8 = 1;

/// Current in-memory route graph snapshot token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct RouteGraphVersionResponse {
    /// Stable client cache-busting token for the current graph snapshot.
    pub version: String,
    /// Hex-encoded content hash used to build `version`.
    pub snapshot_hash: String,
    /// Number of assets present in the compacted route graph.
    pub asset_count: usize,
    /// Number of directed liquidity edges present in the compacted route graph.
    pub edge_count: usize,
    /// Unix timestamp in milliseconds when this version response was generated.
    pub generated_at: i64,
}

/// GET /api/v1/route-graph/version
///
/// Returns a cheap token derived from the current in-memory route graph
/// snapshot so clients can decide whether cached route data should be busted
/// without running route discovery.
#[utoipa::path(
    get,
    path = "/api/v1/route-graph/version",
    tag = "trading",
    responses(
        (status = 200, description = "Current in-memory route graph snapshot version", body = RouteGraphVersionResponse),
    )
)]
pub async fn get_route_graph_version(
    State(state): State<Arc<AppState>>,
    request_id: RequestId,
) -> Json<ApiResponse<RouteGraphVersionResponse>> {
    let graph = state.graph_manager.get_edges();
    let snapshot_hash = graph_snapshot_hash(&graph);

    Json(ApiResponse::new(
        RouteGraphVersionResponse {
            version: format!("route-graph-v{HASH_VERSION}-{snapshot_hash}"),
            snapshot_hash,
            asset_count: graph.asset_count(),
            edge_count: graph.edges.len(),
            generated_at: now_millis(),
        },
        request_id.as_str(),
    ))
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn graph_snapshot_hash(graph: &CompactedGraph) -> String {
    let mut hasher = Sha256::new();
    write_str(&mut hasher, "stellarroute-route-graph");
    write_u64(&mut hasher, HASH_VERSION as u64);

    write_u64(&mut hasher, graph.assets.len() as u64);
    for asset in &graph.assets {
        write_str(&mut hasher, asset);
    }

    write_u64(&mut hasher, graph.offsets.len() as u64);
    for offset in &graph.offsets {
        write_u64(&mut hasher, *offset as u64);
    }

    write_u64(&mut hasher, graph.edges.len() as u64);
    for edge in &graph.edges {
        hash_edge(&mut hasher, edge);
    }

    hex::encode(hasher.finalize())
}

fn hash_edge(hasher: &mut Sha256, edge: &CompactedEdge) {
    write_u64(hasher, edge.to_idx as u64);
    write_str(hasher, &edge.venue_type);
    write_str(hasher, edge.resolved_venue_type());
    write_u64(hasher, edge.venue_type_idx as u64);
    write_str(hasher, &edge.venue_ref);
    write_i128(hasher, edge.liquidity);
    write_f64(hasher, edge.price);
    write_u64(hasher, edge.fee_bps as u64);
    write_u64(hasher, edge.anomaly_score.to_bits() as u64);
    write_option_str(hasher, edge.provider.as_deref());

    let bridge = serde_json::to_vec(&edge.bridge).unwrap_or_default();
    write_bytes_with_len(hasher, &bridge);
}

fn write_bytes_with_len(hasher: &mut Sha256, bytes: &[u8]) {
    write_u64(hasher, bytes.len() as u64);
    hasher.update(bytes);
}

fn write_str(hasher: &mut Sha256, value: &str) {
    write_bytes_with_len(hasher, value.as_bytes());
}

fn write_option_str(hasher: &mut Sha256, value: Option<&str>) {
    match value {
        Some(value) => {
            write_u64(hasher, 1);
            write_str(hasher, value);
        }
        None => write_u64(hasher, 0),
    }
}

fn write_u64(hasher: &mut Sha256, value: u64) {
    hasher.update(value.to_le_bytes());
}

fn write_i128(hasher: &mut Sha256, value: i128) {
    hasher.update(value.to_le_bytes());
}

fn write_f64(hasher: &mut Sha256, value: f64) {
    write_u64(hasher, value.to_bits());
}

#[cfg(test)]
mod tests {
    use super::*;
    use stellarroute_routing::pathfinder::LiquidityEdge;

    fn graph_from_liquidity(price: f64) -> CompactedGraph {
        CompactedGraph::from_edges(vec![LiquidityEdge {
            from: "native".to_string(),
            to: "USDC:GISSUER".to_string(),
            venue_type: "sdex".to_string(),
            venue_ref: "offer-1".to_string(),
            liquidity: 1_000_000,
            price,
            fee_bps: 20,
            ..Default::default()
        }])
    }

    #[test]
    fn graph_snapshot_hash_is_stable_for_same_snapshot() {
        let graph = graph_from_liquidity(1.25);

        assert_eq!(graph_snapshot_hash(&graph), graph_snapshot_hash(&graph));
    }

    #[test]
    fn graph_snapshot_hash_changes_when_snapshot_content_changes() {
        let first = graph_from_liquidity(1.25);
        let second = graph_from_liquidity(1.26);

        assert_ne!(graph_snapshot_hash(&first), graph_snapshot_hash(&second));
    }
}
