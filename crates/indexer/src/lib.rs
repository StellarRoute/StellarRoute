//! StellarRoute Indexer
//!
//! This crate provides the indexing service for SDEX orderbooks and Soroban AMM pools.

pub mod amm;
pub mod config;
pub mod db;
pub mod deduplication;
pub mod error;
pub mod horizon;
pub mod models;
pub mod reconciliation;
pub mod telemetry;

pub mod sdex;
pub mod soroban;

use crate::reconciliation::BackfillManager;
use sqlx::PgPool;

pub use deduplication::{
    DeduplicationConfig, DeduplicationResult, DeduplicatorState, DeduplicatorStats,
    EventDeduplicator, EventStatus, IdempotencyKey, OrderingStrategy, ProcessedEvent,
    SequenceError, StreamState,
};

/// Indexer service
pub struct Indexer {
    backfill_manager: Option<BackfillManager>,
}

impl Indexer {
    /// Create a new indexer instance
    pub fn new(pool: PgPool) -> Self {
        Self {
            backfill_manager: Some(BackfillManager::new(pool)),
        }
    }

    /// Access the backfill manager
    pub fn backfill(&self) -> Option<&BackfillManager> {
        self.backfill_manager.as_ref()
    }
}
