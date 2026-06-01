//! Exactly-once liquidity event processing with replay protection
//!
//! Implements idempotent event ingestion with deduplication and ordering strategies
//! to ensure each liquidity event is processed exactly once.
//!
//! # Features
//!
//! - Idempotency keys for duplicate detection
//! - Sequence-based ordering for out-of-order delivery
//! - Persistent replay protection across restarts
//! - Configurable deduplication window

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IdempotencyKey {
    pub source: String,
    pub event_id: String,
}

impl IdempotencyKey {
    pub fn new(source: impl Into<String>, event_id: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            event_id: event_id.into(),
        }
    }

    pub fn from_ledger(ledger_sequence: u32, tx_hash: &str, op_index: u32) -> Self {
        Self {
            source: "ledger".to_string(),
            event_id: format!("{}:{}:{}", ledger_sequence, tx_hash, op_index),
        }
    }

    pub fn from_stream(stream_id: &str, sequence: u64) -> Self {
        Self {
            source: stream_id.to_string(),
            event_id: sequence.to_string(),
        }
    }
}

impl std::fmt::Display for IdempotencyKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.source, self.event_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedEvent {
    pub key: IdempotencyKey,
    pub status: EventStatus,
    pub sequence: u64,
    pub processed_at: DateTime<Utc>,
    pub retry_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeduplicationResult {
    New,
    Duplicate,
    Reprocessing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OrderingStrategy {
    StrictSequence,
    #[default]
    BestEffort,
    Unordered,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicationConfig {
    pub window_seconds: u64,
    pub max_entries: usize,
    pub ordering_strategy: OrderingStrategy,
    pub max_out_of_order_buffer: usize,
    pub reprocess_failed: bool,
}

impl Default for DeduplicationConfig {
    fn default() -> Self {
        Self {
            window_seconds: 3600,
            max_entries: 100_000,
            ordering_strategy: OrderingStrategy::BestEffort,
            max_out_of_order_buffer: 1000,
            reprocess_failed: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamState {
    pub stream_id: String,
    pub last_sequence: u64,
    pub last_processed_at: DateTime<Utc>,
    pub pending_count: usize,
}

pub struct EventDeduplicator {
    config: DeduplicationConfig,
    processed: Arc<RwLock<HashMap<IdempotencyKey, ProcessedEvent>>>,
    stream_states: Arc<RwLock<HashMap<String, StreamState>>>,
    out_of_order_buffer: Arc<RwLock<Vec<(IdempotencyKey, u64)>>>,
}

impl EventDeduplicator {
    pub fn new(config: DeduplicationConfig) -> Self {
        Self {
            config,
            processed: Arc::new(RwLock::new(HashMap::new())),
            stream_states: Arc::new(RwLock::new(HashMap::new())),
            out_of_order_buffer: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn check(&self, key: &IdempotencyKey) -> DeduplicationResult {
        let processed = self.processed.read().await;

        if let Some(event) = processed.get(key) {
            match event.status {
                EventStatus::Completed => DeduplicationResult::Duplicate,
                EventStatus::Failed if self.config.reprocess_failed => {
                    DeduplicationResult::Reprocessing
                }
                EventStatus::Failed => DeduplicationResult::Duplicate,
                EventStatus::Processing | EventStatus::Pending => DeduplicationResult::Duplicate,
            }
        } else {
            DeduplicationResult::New
        }
    }

    pub async fn check_sequence(
        &self,
        stream_id: &str,
        sequence: u64,
    ) -> Result<bool, SequenceError> {
        let states = self.stream_states.read().await;

        if let Some(state) = states.get(stream_id) {
            match self.config.ordering_strategy {
                OrderingStrategy::StrictSequence => {
                    if sequence != state.last_sequence + 1 {
                        if sequence <= state.last_sequence {
                            return Ok(false);
                        }
                        return Err(SequenceError::Gap {
                            expected: state.last_sequence + 1,
                            received: sequence,
                        });
                    }
                }
                OrderingStrategy::BestEffort => {
                    if sequence <= state.last_sequence {
                        return Ok(false);
                    }
                }
                OrderingStrategy::Unordered => {}
            }
        }

        Ok(true)
    }

    pub async fn mark_processing(&self, key: IdempotencyKey, sequence: u64) {
        let mut processed = self.processed.write().await;

        processed.insert(
            key.clone(),
            ProcessedEvent {
                key,
                status: EventStatus::Processing,
                sequence,
                processed_at: Utc::now(),
                retry_count: 0,
            },
        );
    }

    pub async fn mark_completed(&self, key: &IdempotencyKey, stream_id: &str, sequence: u64) {
        let mut processed = self.processed.write().await;

        if let Some(event) = processed.get_mut(key) {
            event.status = EventStatus::Completed;
            event.processed_at = Utc::now();
        }

        drop(processed);

        let mut states = self.stream_states.write().await;
        let state = states.entry(stream_id.to_string()).or_insert(StreamState {
            stream_id: stream_id.to_string(),
            last_sequence: 0,
            last_processed_at: Utc::now(),
            pending_count: 0,
        });

        if sequence > state.last_sequence {
            state.last_sequence = sequence;
        }
        state.last_processed_at = Utc::now();
    }

    pub async fn mark_failed(&self, key: &IdempotencyKey) {
        let mut processed = self.processed.write().await;

        if let Some(event) = processed.get_mut(key) {
            event.status = EventStatus::Failed;
            event.retry_count += 1;
            event.processed_at = Utc::now();
        }
    }

    pub async fn buffer_out_of_order(&self, key: IdempotencyKey, sequence: u64) -> bool {
        let mut buffer = self.out_of_order_buffer.write().await;

        if buffer.len() >= self.config.max_out_of_order_buffer {
            return false;
        }

        buffer.push((key, sequence));
        buffer.sort_by_key(|(_, seq)| *seq);
        true
    }

    pub async fn drain_ready(&self, stream_id: &str) -> Vec<(IdempotencyKey, u64)> {
        let states = self.stream_states.read().await;
        let last_seq = states.get(stream_id).map(|s| s.last_sequence).unwrap_or(0);
        drop(states);

        let mut buffer = self.out_of_order_buffer.write().await;
        let mut ready = Vec::new();
        let mut i = 0;

        while i < buffer.len() {
            let (ref key, seq) = buffer[i];
            if key.source == stream_id && seq == last_seq + 1 + ready.len() as u64 {
                ready.push(buffer.remove(i));
            } else {
                i += 1;
            }
        }

        ready
    }

    pub async fn cleanup_expired(&self) {
        let cutoff = Utc::now() - chrono::Duration::seconds(self.config.window_seconds as i64);
        let mut processed = self.processed.write().await;

        processed.retain(|_, event| event.processed_at > cutoff);

        if processed.len() > self.config.max_entries {
            let mut events: Vec<_> = processed.iter().collect();
            events.sort_by_key(|(_, e)| e.processed_at);

            let to_remove: Vec<_> = events
                .iter()
                .take(processed.len() - self.config.max_entries)
                .map(|(k, _)| (*k).clone())
                .collect();

            for key in to_remove {
                processed.remove(&key);
            }
        }
    }

    pub async fn get_stream_state(&self, stream_id: &str) -> Option<StreamState> {
        let states = self.stream_states.read().await;
        states.get(stream_id).cloned()
    }

    pub async fn get_stats(&self) -> DeduplicatorStats {
        let processed = self.processed.read().await;
        let buffer = self.out_of_order_buffer.read().await;
        let states = self.stream_states.read().await;

        let mut completed = 0;
        let mut failed = 0;
        let mut processing = 0;

        for event in processed.values() {
            match event.status {
                EventStatus::Completed => completed += 1,
                EventStatus::Failed => failed += 1,
                EventStatus::Processing | EventStatus::Pending => processing += 1,
            }
        }

        DeduplicatorStats {
            total_tracked: processed.len(),
            completed,
            failed,
            processing,
            buffered: buffer.len(),
            streams: states.len(),
        }
    }

    pub async fn export_state(&self) -> DeduplicatorState {
        let processed = self.processed.read().await;
        let states = self.stream_states.read().await;

        DeduplicatorState {
            processed: processed.clone(),
            stream_states: states.clone(),
            exported_at: Utc::now(),
        }
    }

    pub async fn import_state(&self, state: DeduplicatorState) {
        let mut processed = self.processed.write().await;
        let mut states = self.stream_states.write().await;

        *processed = state.processed;
        *states = state.stream_states;
    }
}

#[derive(Debug, Clone)]
pub enum SequenceError {
    Gap { expected: u64, received: u64 },
}

impl std::fmt::Display for SequenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SequenceError::Gap { expected, received } => {
                write!(
                    f,
                    "sequence gap: expected {}, received {}",
                    expected, received
                )
            }
        }
    }
}

impl std::error::Error for SequenceError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicatorStats {
    pub total_tracked: usize,
    pub completed: usize,
    pub failed: usize,
    pub processing: usize,
    pub buffered: usize,
    pub streams: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicatorState {
    pub processed: HashMap<IdempotencyKey, ProcessedEvent>,
    pub stream_states: HashMap<String, StreamState>,
    pub exported_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_idempotency_key_creation() {
        let key1 = IdempotencyKey::new("source", "event-123");
        assert_eq!(key1.source, "source");
        assert_eq!(key1.event_id, "event-123");

        let key2 = IdempotencyKey::from_ledger(12345, "abc123", 0);
        assert_eq!(key2.source, "ledger");
        assert_eq!(key2.event_id, "12345:abc123:0");

        let key3 = IdempotencyKey::from_stream("horizon", 999);
        assert_eq!(key3.source, "horizon");
        assert_eq!(key3.event_id, "999");
    }

    #[tokio::test]
    async fn test_new_event_detection() {
        let dedup = EventDeduplicator::new(DeduplicationConfig::default());
        let key = IdempotencyKey::new("test", "event-1");

        let result = dedup.check(&key).await;
        assert_eq!(result, DeduplicationResult::New);
    }

    #[tokio::test]
    async fn test_duplicate_detection() {
        let dedup = EventDeduplicator::new(DeduplicationConfig::default());
        let key = IdempotencyKey::new("test", "event-1");

        dedup.mark_processing(key.clone(), 1).await;
        dedup.mark_completed(&key, "test", 1).await;

        let result = dedup.check(&key).await;
        assert_eq!(result, DeduplicationResult::Duplicate);
    }

    #[tokio::test]
    async fn test_failed_event_reprocessing() {
        let config = DeduplicationConfig {
            reprocess_failed: true,
            ..Default::default()
        };
        let dedup = EventDeduplicator::new(config);
        let key = IdempotencyKey::new("test", "event-1");

        dedup.mark_processing(key.clone(), 1).await;
        dedup.mark_failed(&key).await;

        let result = dedup.check(&key).await;
        assert_eq!(result, DeduplicationResult::Reprocessing);
    }

    #[tokio::test]
    async fn test_sequence_checking_strict() {
        let config = DeduplicationConfig {
            ordering_strategy: OrderingStrategy::StrictSequence,
            ..Default::default()
        };
        let dedup = EventDeduplicator::new(config);

        let key1 = IdempotencyKey::from_stream("stream1", 1);
        dedup.mark_processing(key1.clone(), 1).await;
        dedup.mark_completed(&key1, "stream1", 1).await;

        let ok = dedup.check_sequence("stream1", 2).await;
        assert!(ok.is_ok());
        assert!(ok.unwrap());

        let gap = dedup.check_sequence("stream1", 5).await;
        assert!(gap.is_err());
    }

    #[tokio::test]
    async fn test_sequence_checking_best_effort() {
        let config = DeduplicationConfig {
            ordering_strategy: OrderingStrategy::BestEffort,
            ..Default::default()
        };
        let dedup = EventDeduplicator::new(config);

        let key1 = IdempotencyKey::from_stream("stream1", 1);
        dedup.mark_processing(key1.clone(), 1).await;
        dedup.mark_completed(&key1, "stream1", 1).await;

        let ok = dedup.check_sequence("stream1", 5).await;
        assert!(ok.is_ok());
        assert!(ok.unwrap());

        let old = dedup.check_sequence("stream1", 1).await;
        assert!(old.is_ok());
        assert!(!old.unwrap());
    }

    #[tokio::test]
    async fn test_out_of_order_buffering() {
        let dedup = EventDeduplicator::new(DeduplicationConfig::default());

        let key = IdempotencyKey::from_stream("stream1", 5);
        assert!(dedup.buffer_out_of_order(key, 5).await);

        let stats = dedup.get_stats().await;
        assert_eq!(stats.buffered, 1);
    }

    #[tokio::test]
    async fn test_stats() {
        let dedup = EventDeduplicator::new(DeduplicationConfig::default());

        let key1 = IdempotencyKey::new("test", "1");
        let key2 = IdempotencyKey::new("test", "2");

        dedup.mark_processing(key1.clone(), 1).await;
        dedup.mark_completed(&key1, "test", 1).await;

        dedup.mark_processing(key2.clone(), 2).await;
        dedup.mark_failed(&key2).await;

        let stats = dedup.get_stats().await;
        assert_eq!(stats.total_tracked, 2);
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.failed, 1);
    }

    #[tokio::test]
    async fn test_state_export_import() {
        let dedup1 = EventDeduplicator::new(DeduplicationConfig::default());

        let key = IdempotencyKey::new("test", "1");
        dedup1.mark_processing(key.clone(), 1).await;
        dedup1.mark_completed(&key, "test", 1).await;

        let state = dedup1.export_state().await;

        let dedup2 = EventDeduplicator::new(DeduplicationConfig::default());
        dedup2.import_state(state).await;

        let result = dedup2.check(&key).await;
        assert_eq!(result, DeduplicationResult::Duplicate);
    }

    #[tokio::test]
    async fn test_stream_state() {
        let dedup = EventDeduplicator::new(DeduplicationConfig::default());

        let key = IdempotencyKey::from_stream("stream1", 42);
        dedup.mark_processing(key.clone(), 42).await;
        dedup.mark_completed(&key, "stream1", 42).await;

        let state = dedup.get_stream_state("stream1").await;
        assert!(state.is_some());
        assert_eq!(state.unwrap().last_sequence, 42);
    }
}
