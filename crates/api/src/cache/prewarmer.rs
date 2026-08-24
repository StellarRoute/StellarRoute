//! Predictive cache prewarming based on route-demand forecasting

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{instrument, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrewarmMetrics {
    pub total_prewarmed: u64,
    pub hits_from_prewarmed: u64,
    pub cache_hits_saved: u64,
    pub compute_savings_ms: u64,
}

impl PrewarmMetrics {
    pub fn hit_ratio(&self) -> f64 {
        if self.total_prewarmed == 0 {
            return 0.0;
        }
        self.hits_from_prewarmed as f64 / self.total_prewarmed as f64
    }
}

#[derive(Debug, Clone)]
pub struct KeyDemandEntry {
    pub key: String,
    pub access_count: u64,
    pub last_accessed_at: u64,
}

#[derive(Debug)]
pub struct DemandForecaster {
    top_keys: Arc<RwLock<Vec<KeyDemandEntry>>>,
    max_keys: usize,
    window_secs: u64,
}

impl DemandForecaster {
    pub fn new(max_keys: usize, window_secs: u64) -> Self {
        Self {
            top_keys: Arc::new(RwLock::new(Vec::new())),
            max_keys,
            window_secs,
        }
    }

    #[instrument(skip(self))]
    pub async fn record_access(&self, key: String) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut top = self.top_keys.write().await;
        if let Some(entry) = top.iter_mut().find(|e| e.key == key) {
            entry.access_count += 1;
            entry.last_accessed_at = now;
        } else {
            top.push(KeyDemandEntry {
                key,
                access_count: 1,
                last_accessed_at: now,
            });
        }

        top.sort_by_key(|b| std::cmp::Reverse(b.access_count));
        top.truncate(self.max_keys);
    }

    pub async fn forecast_top_keys(&self) -> Vec<String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let top = self.top_keys.read().await;
        top.iter()
            .filter(|e| now.saturating_sub(e.last_accessed_at) < self.window_secs)
            .map(|e| e.key.clone())
            .take(self.max_keys / 2) // Prewarm top 50%
            .collect()
    }
}

#[derive(Error, Debug)]
pub enum PrewarmError {
    #[error("Prewarming disabled due to resource limits")]
    ResourceLimitExceeded,
    #[error("Prewarmer error: {0}")]
    Error(String),
}

pub struct CachePrewarmer {
    forecaster: Arc<DemandForecaster>,
    max_prewarm_entries: u64,
    prewarm_entries: Arc<AtomicU64>,
    metrics: Arc<RwLock<PrewarmMetrics>>,
    enabled: Arc<tokio::sync::Semaphore>,
}

impl CachePrewarmer {
    pub fn new(forecaster: Arc<DemandForecaster>, max_prewarm_entries: u64) -> Self {
        Self {
            forecaster,
            max_prewarm_entries,
            prewarm_entries: Arc::new(AtomicU64::new(0)),
            metrics: Arc::new(RwLock::new(PrewarmMetrics {
                total_prewarmed: 0,
                hits_from_prewarmed: 0,
                cache_hits_saved: 0,
                compute_savings_ms: 0,
            })),
            enabled: Arc::new(tokio::sync::Semaphore::new(1)),
        }
    }

    #[instrument(skip(self, compute_fn))]
    pub async fn prewarm<F, T>(&self, compute_fn: F) -> Result<(), PrewarmError>
    where
        F: Fn(
                String,
            )
                -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, String>> + Send>>
            + Send
            + Sync,
    {
        if self.prewarm_entries.load(Ordering::Acquire) > self.max_prewarm_entries {
            warn!("Prewarm resource limit exceeded");
            return Err(PrewarmError::ResourceLimitExceeded);
        }

        let _permit = self
            .enabled
            .acquire()
            .await
            .map_err(|_| PrewarmError::Error("Semaphore error".to_string()))?;

        let keys = self.forecaster.forecast_top_keys().await;
        let start = std::time::Instant::now();

        for key in keys {
            if (compute_fn)(key.clone()).await.is_ok() {
                self.prewarm_entries.fetch_add(1, Ordering::Release);
                let mut metrics = self.metrics.write().await;
                metrics.total_prewarmed += 1;
            }
        }

        let elapsed = start.elapsed().as_millis() as u64;
        let mut metrics = self.metrics.write().await;
        metrics.compute_savings_ms += elapsed;

        Ok(())
    }

    pub async fn record_hit(&self, _key: &str) {
        let mut metrics = self.metrics.write().await;
        metrics.hits_from_prewarmed += 1;
        metrics.cache_hits_saved += 1;
    }

    pub async fn get_metrics(&self) -> PrewarmMetrics {
        self.metrics.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_demand_forecasting() {
        let forecaster = DemandForecaster::new(10, 60);

        for _ in 0..5 {
            forecaster.record_access("key_hot".to_string()).await;
        }
        for _ in 0..3 {
            forecaster.record_access("key_warm".to_string()).await;
        }
        forecaster.record_access("key_cold".to_string()).await;

        let top = forecaster.forecast_top_keys().await;
        assert!(top[0] == "key_hot");
        assert!(top.len() <= 5); // max_keys / 2
    }

    #[tokio::test]
    async fn test_prewarm_metrics() {
        let forecaster = Arc::new(DemandForecaster::new(100, 60));
        let prewarmer = CachePrewarmer::new(forecaster.clone(), 1000);

        forecaster.record_access("test_key".to_string()).await;

        prewarmer
            .prewarm(|_key| Box::pin(async { Ok::<(), String>(()) }))
            .await
            .ok();

        prewarmer.record_hit("test_key").await;

        let metrics = prewarmer.get_metrics().await;
        assert!(metrics.total_prewarmed > 0);
        assert!(metrics.hits_from_prewarmed > 0);
    }

    #[tokio::test]
    async fn test_resource_exhaustion_protection() {
        let forecaster = Arc::new(DemandForecaster::new(100, 60));
        let prewarmer = CachePrewarmer::new(forecaster, 5);

        prewarmer.prewarm_entries.store(10, Ordering::Release);

        let result = prewarmer
            .prewarm(|_| Box::pin(async { Ok::<(), String>(()) }))
            .await;

        assert!(matches!(result, Err(PrewarmError::ResourceLimitExceeded)));
    }
}
