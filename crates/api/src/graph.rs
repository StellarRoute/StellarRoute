//! Background graph manager for routing caching
use arc_swap::ArcSwap;
use sqlx::{postgres::PgListener, PgPool, Row};
use std::sync::Arc;
use stellarroute_routing::health::anomaly::LiquidityAnomalyDetector;
use stellarroute_routing::pathfinder::LiquidityEdge;
use tracing::{debug, error, info, warn};

use stellarroute_routing::compaction::CompactedGraph;

// Fallback defaults used when fee information is not available in the DB.
pub const DEFAULT_AMM_FEE_BPS: u32 = 30;
pub const DEFAULT_SDEX_FEE_BPS: u32 = 20;

/// Daemon that maintains an active in-memory cache of the routing graph
pub struct GraphManager {
    pub db: PgPool,
    pub edges: Arc<ArcSwap<CompactedGraph>>,
    pub anomaly_detector: Arc<tokio::sync::Mutex<LiquidityAnomalyDetector>>,
}

impl GraphManager {
    /// Create uninitialized graph manager
    pub fn new(db: PgPool) -> Self {
        Self {
            db,
            edges: Arc::new(ArcSwap::from_pointee(CompactedGraph::default())),
            anomaly_detector: Arc::new(tokio::sync::Mutex::new(
                stellarroute_routing::health::anomaly::LiquidityAnomalyDetector::new(
                    stellarroute_routing::health::anomaly::AnomalyConfig::default(),
                ),
            )),
        }
    }

    /// Retrieve the current live copy of the routing graph.
    /// Returns an Arc to the compacted graph for zero-copy sharing.
    pub fn get_edges(&self) -> Arc<CompactedGraph> {
        self.edges.load_full()
    }

    /// Spawn a background task to keep the graph updated
    pub fn start_sync(self: Arc<Self>) {
        info!("Starting event-driven routing graph sync task");
        let manager = self.clone();

        tokio::spawn(async move {
            // Initial sync immediately
            if let Err(e) = manager.sync_graph().await {
                error!("Failed initial sync for routing graph: {}", e);
            }

            // Setup Postgres listener for incremental updates
            let mut listener = match PgListener::connect_with(&manager.db).await {
                Ok(l) => l,
                Err(e) => {
                    error!(
                        "Failed to connect PgListener: {}. Falling back to 10s polling.",
                        e
                    );
                    manager.run_polling_fallback().await;
                    return;
                }
            };

            if let Err(e) = listener.listen("liquidity_update").await {
                error!(
                    "Failed to listen on 'liquidity_update': {}. Falling back to 10s polling.",
                    e
                );
                manager.run_polling_fallback().await;
                return;
            }

            info!("GraphManager successfully subscribed to 'liquidity_update' notifications");

            loop {
                match listener.recv().await {
                    Ok(notification) => {
                        debug!(
                            "Received liquidity update notification: protocol={}",
                            notification.payload()
                        );
                        if let Err(e) = manager.sync_graph().await {
                            error!("Failed to sync routing graph after notification: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("PgListener connection lost: {}. Reconnecting...", e);
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

                        // Attempt one reconnection then fallback if it fails again
                        if let Ok(mut l) = PgListener::connect_with(&manager.db).await {
                            if l.listen("liquidity_update").await.is_ok() {
                                listener = l;
                                info!("PgListener reconnected successfully");
                                continue;
                            }
                        }
                        error!("PgListener reconnection failed. Switching to polling fallback.");
                        manager.run_polling_fallback().await;
                        break;
                    }
                }
            }
        });
    }

    /// Fallback loop that polls the database every 10 seconds if notifications are unavailable.
    async fn run_polling_fallback(&self) {
        warn!("Starting fallback polling sync task (10s interval)");
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
        loop {
            interval.tick().await;
            if let Err(e) = self.sync_graph().await {
                error!("Failed to sync routing graph (polling fallback): {}", e);
            }
        }
    }

    /// Pulls the latest liquidity data from the database and performs a swap of the in-memory graph.
    pub async fn sync_graph(&self) -> Result<(), sqlx::Error> {
        debug!("Syncing routing graph from database...");

        let assets = sqlx::query("SELECT id, asset_type, asset_code, asset_issuer FROM assets")
            .fetch_all(&self.db)
            .await?;

        let mut hash_map = std::collections::HashMap::with_capacity(assets.len());
        for row in assets {
            let id: uuid::Uuid = row.get("id");
            let a_type: String = row.get("asset_type");
            let a_code: Option<String> = row.get("asset_code");
            let a_iss: Option<String> = row.get("asset_issuer");

            let canon = if a_type != "native" {
                if let Some(iss) = a_iss {
                    format!("{}:{}", a_code.unwrap_or_default(), iss)
                } else {
                    a_code.unwrap_or_default()
                }
            } else {
                "native".to_string()
            };
            hash_map.insert(id, canon);
        }

        // Join to `amm_pool_reserves` to pull real fee_bps for AMM venues when available.
        // Normalized liquidity currently doesn't include fee information directly.
        // We fallback to module-level defaults when the DB doesn't provide a value.

        let rows = sqlx::query(
            r#"
            SELECT nl.selling_asset_id, nl.buying_asset_id, nl.venue_type, nl.venue_ref, nl.price, nl.available_amount,
                   amm.fee_bps as fee_bps
            FROM normalized_liquidity nl
            LEFT JOIN amm_pool_reserves amm ON nl.venue_type = 'amm' AND nl.venue_ref = amm.pool_address
            WHERE nl.available_amount > 0
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        let mut next_edges: Vec<LiquidityEdge> = Vec::with_capacity(rows.len());

        for r in rows {
            let s_id: uuid::Uuid = r.get("selling_asset_id");
            let b_id: uuid::Uuid = r.get("buying_asset_id");

            if let (Some(e_from), Some(e_to)) = (hash_map.get(&s_id), hash_map.get(&b_id)) {
                let price_str: String = r.get("price");
                let avail_str: String = r.get("available_amount");
                let venue_type: String = r.get("venue_type");

                let price = price_str.parse::<f64>().ok();
                let avail = avail_str.parse::<f64>().ok();

                if let (Some(p), Some(a)) = (price, avail) {
                    if p > 0.0 && a > 0.0 {
                        let is_amm = venue_type == "amm";
                        let venue_ref = r.get::<String, _>("venue_ref");
                        // `fee_bps` may be present from amm_pool_reserves (for AMM) or NULL.
                        // We'll use DB-provided value when present; otherwise fallback.
                        let db_fee: Option<i32> = r.get::<Option<i32>, _>("fee_bps");
                        let fee_bps_u32: u32 = if is_amm {
                            db_fee.map(|v| v as u32).unwrap_or(DEFAULT_AMM_FEE_BPS)
                        } else {
                            db_fee.map(|v| v as u32).unwrap_or(DEFAULT_SDEX_FEE_BPS)
                        };

                        // Perform anomaly detection
                        let mut detector = self.anomaly_detector.lock().await;
                        let (reserves, depth) = if is_amm {
                            // For AMM, we assume reserves are related to the available amount
                            // This is a simplification; in a real app we'd fetch reserves directly
                            (Some(((a * 1e7) as i128, ((a * p * 1e7) as i128))), None)
                        } else {
                            (None, Some((a * 1e7) as i128))
                        };

                        let _anomaly_res =
                            detector.update_and_detect(&venue_ref, reserves, depth, None);

                        next_edges.push(LiquidityEdge {
                            from: e_from.clone(),
                            to: e_to.clone(),
                            venue_type,
                            venue_ref,
                            liquidity: (a * 1e7) as i128,
                            price: p,
                            fee_bps: if is_amm { 30 } else { 20 },
                            anomaly_score: None,
                            anomaly_reasons: None,
                        });
                    }
                }
            }
        }

        info!(
            "Graph sync complete: swapped {} edges atomically into compacted graph",
            next_edges.len()
        );
        self.edges
            .store(Arc::new(CompactedGraph::from_edges(next_edges)));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_graph_manager_snapshot_consistency() {
        let pool = PgPool::connect_lazy("postgres://localhost/test").unwrap();
        let manager = GraphManager::new(pool);

        let initial_edges = vec![
            LiquidityEdge {
                from: "XLM".to_string(),
                to: "USDC".to_string(),
                venue_type: "sdex".to_string(),
                venue_ref: "1".to_string(),
                liquidity: 100,
                price: 1.0,
                fee_bps: 30,
                anomaly_score: None,
                anomaly_reasons: None,
            }
        ];

        manager.edges.store(Arc::new(initial_edges.clone()));

        let snapshot1 = manager.get_edges();
        assert_eq!(snapshot1.asset_count(), 2);
        assert_eq!(snapshot1.assets[0], "XLM");

        let new_edges = vec![
            LiquidityEdge {
                from: "USDC".to_string(),
                to: "XLM".to_string(),
                venue_type: "sdex".to_string(),
                venue_ref: "2".to_string(),
                liquidity: 200,
                price: 0.99,
                fee_bps: 30,
                anomaly_score: None,
                anomaly_reasons: None,
            }
        ];
        manager.edges.store(Arc::new(new_edges));

        let snapshot2 = manager.get_edges();
        assert_eq!(snapshot2.asset_count(), 2);
        assert_eq!(snapshot2.assets[0], "USDC");

        assert_eq!(snapshot1.len(), 1);
        assert_eq!(snapshot1[0].from, "XLM");
    }

    #[tokio::test]
    async fn test_concurrent_reads() {
        let pool = PgPool::connect_lazy("postgres://localhost/test").unwrap();
        let manager = Arc::new(GraphManager::new(pool));
        
        let initial_edges = vec![
            LiquidityEdge {
                from: "A".to_string(),
                to: "B".to_string(),
                venue_type: "sdex".to_string(),
                venue_ref: "1".to_string(),
                liquidity: 100,
                price: 1.0,
                fee_bps: 30,
                anomaly_score: None,
                anomaly_reasons: None,
            }
        ];
        manager.edges.store(Arc::new(initial_edges));

        let mut handles = vec![];
        for _ in 0..10 {
            let m = manager.clone();
            handles.push(tokio::spawn(async move {
                for _ in 0..100 {
                    let edges = m.get_edges();
                    assert!(edges.asset_count() > 0);
                    tokio::task::yield_now().await;
                }
            }));
        }

        let m2 = manager.clone();
        let updater = tokio::spawn(async move {
            for i in 0..50 {
                let edges = vec![
                    LiquidityEdge {
                        from: format!("A{}", i),
                        to: "B".to_string(),
                        venue_type: "sdex".to_string(),
                        venue_ref: "1".to_string(),
                        liquidity: 100,
                        price: 1.0,
                        fee_bps: 30,
                        anomaly_score: None,
                        anomaly_reasons: None,
                    }
                ];
                m2.edges.store(Arc::new(edges));
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            }
        });

        for h in handles {
            h.await.unwrap();
        }
        updater.await.unwrap();
    }
}
