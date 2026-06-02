//! Partitioning support for the indexer.
//!
//! This module provides a lightweight, deterministic partitioning strategy that
//! distributes market workload across multiple indexer instances. It also
//! implements hot‑pair detection based on a configurable allow‑list (future
//! extensions can use volume‑based detection).

use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::debug;

use crate::config::IndexerConfig;
use crate::metrics;

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
    volume_map: std::sync::Arc<parking_lot::RwLock<std::collections::HashMap<String, (u64, i64)>>>,
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
        set.contains(pair)
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
}
