//! Partitioning support for the indexer.
//!
//! This module provides a lightweight, deterministic partitioning strategy that
//! distributes market workload across multiple indexer instances. It also
//! implements hot‑pair detection based on a configurable allow‑list (future
//! extensions can use volume‑based detection).

use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::config::IndexerConfig;
use crate::metrics::{FAIRNESS_SCORE, INDEXER_LAG_LEDGERS, PARTITION_QUEUE_DEPTH};

/// Represents a partition manager that decides whether a given market pair
/// should be processed by this instance.
#[derive(Debug, Clone)]
pub struct PartitionManager {
    /// Total number of partitions.
    pub partition_count: usize,
    /// Identifier for this partition (0‑based).
    pub partition_id: usize,
    /// Threshold volume to consider a pair hot (units are the raw amount field).
    hot_volume_threshold: u64,
    /// Time window (seconds) for volume based hot‑pair detection.
    hot_window_secs: u64,
    /// Map of pair -> (volume, last_updated timestamp).
    volume_map: Arc<RwLock<HashMap<String, (u64, i64)>>>,
    /// Set of explicitly configured hot pair identifiers.
    hot_allowlist: Arc<RwLock<HashSet<String>>>,
}

impl PartitionManager {
    /// Create a new manager from the global configuration.
    pub fn from_config(cfg: &IndexerConfig) -> Self {
        let hot_set = cfg
            .hot_pair_allowlist
            .split(',')
            .filter_map(|s| {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            })
            .collect::<HashSet<_>>();
        Self {
            partition_count: cfg.partition_count,
            partition_id: cfg.partition_id,
            hot_volume_threshold: cfg.hot_pair_volume_threshold,
            hot_window_secs: cfg.hot_pair_window_secs,
            volume_map: Arc::new(RwLock::new(HashMap::new())),
            hot_allowlist: Arc::new(RwLock::new(hot_set)),
        }
    }

    /// Determine if `pair` (e.g., "XLM/USD") should be processed by this
    /// partition.
    ///
    /// The algorithm is:
    ///   1. If the pair is in the hot allow‑list, always process.
    ///   2. Otherwise compute `hash(pair) % partition_count` and compare to
    ///      `partition_id`.
    pub fn should_process(&self, pair: &str) -> bool {
        if self.is_hot(pair) {
            return true;
        }
        let hash = Self::hash_pair(pair);
        (hash % self.partition_count) == self.partition_id
    }

    /// Simple deterministic hash using the default SipHasher.
    fn hash_pair(pair: &str) -> usize {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        pair.hash(&mut hasher);
        hasher.finish() as usize
    }

    /// Check if a pair is designated as hot.
    pub fn is_hot(&self, pair: &str) -> bool {
        let set = self.hot_allowlist.read();
        if set.contains(pair) {
            return true;
        }
        drop(set);

        let now = chrono::Utc::now().timestamp();
        let volumes = self.volume_map.read();
        volumes.get(pair).is_some_and(|(volume, updated_at)| {
            *volume >= self.hot_volume_threshold
                && now.saturating_sub(*updated_at) <= self.hot_window_secs as i64
        })
    }

    /// Record metrics for this partition. Call this periodically (e.g. each
    /// indexing loop) to expose utilization and fairness information.
    pub fn record_metrics(&self, lag: i64, queue_depth: i64) {
        // Record lag per partition
        INDEXER_LAG_LEDGERS
            .with_label_values(&["partition"])
            .set(lag);
        // Record queue depth placeholder
        PARTITION_QUEUE_DEPTH
            .with_label_values(&["partition"])
            .set(queue_depth);
        // Record fairness score (e.g., absolute lag as a simple proxy)
        FAIRNESS_SCORE
            .with_label_values(&["partition"])
            .set(lag.abs());
    }

    /// Record observed trading volume for a market pair to support volume-based hot-pair detection.
    pub fn record_volume(&self, pair: &str, volume: u64) {
        let now = chrono::Utc::now().timestamp();
        self.volume_map.write().insert(pair.to_string(), (volume, now));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_config(partition_count: usize, partition_id: usize, hot_allowlist: &str) -> IndexerConfig {
        IndexerConfig {
            partition_count,
            partition_id,
            hot_pair_allowlist: hot_allowlist.to_string(),
            hot_pair_volume_threshold: 1000,
            hot_pair_window_secs: 600,
            ..IndexerConfig::default()
        }
    }



    #[test]
    fn test_partition_assignment_fairness_synthetic() {
        let count = 4;
        let mut managers = Vec::new();
        for id in 0..count {
            let cfg = make_test_config(count, id, "");
            managers.push(PartitionManager::from_config(&cfg));
        }

        let mut processed_counts = vec![0usize; count];
        for i in 0..100 {
            let pair = format!("ASSET_{}/XLM", i);
            let mut handled = 0;
            for (id, mgr) in managers.iter().enumerate() {
                if mgr.should_process(&pair) {
                    handled += 1;
                    processed_counts[id] += 1;
                }
            }
            assert_eq!(handled, 1, "pair {} should be routed to exactly 1 partition", pair);
        }

        for (id, &c) in processed_counts.iter().enumerate() {
            assert!(c > 0, "partition {} should receive at least 1 pair", id);
        }
    }

    #[test]
    fn test_hot_allowlist_overrides_partition_id() {
        let cfg = make_test_config(5, 0, "XLM/USDC,BTC/XLM");

        let mgr = PartitionManager::from_config(&cfg);
        assert!(mgr.is_hot("XLM/USDC"));
        assert!(mgr.is_hot("BTC/XLM"));
        assert!(mgr.should_process("XLM/USDC"));

        let cfg_other = make_test_config(5, 3, "XLM/USDC,BTC/XLM");

        let mgr_other = PartitionManager::from_config(&cfg_other);
        assert!(mgr_other.should_process("XLM/USDC"));
    }

    #[test]
    fn test_volume_based_hot_pair_detection() {
        let cfg = make_test_config(10, 0, "");

        let mgr = PartitionManager::from_config(&cfg);
        let pair = "ETH/USDC";
        assert!(!mgr.is_hot(pair));

        mgr.record_volume(pair, 500);
        assert!(!mgr.is_hot(pair));

        mgr.record_volume(pair, 1500);
        assert!(mgr.is_hot(pair));
        assert!(mgr.should_process(pair));
    }

    #[test]
    fn test_record_metrics_no_panic() {
        let cfg = make_test_config(1, 0, "");
        let mgr = PartitionManager::from_config(&cfg);
        mgr.record_metrics(10, 50);
    }
}


