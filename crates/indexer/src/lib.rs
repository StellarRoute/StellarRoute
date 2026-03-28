//! StellarRoute Indexer
//!
//! This crate provides the indexing service for SDEX orderbooks and Soroban AMM pools.

pub mod amm;
pub mod config;
pub mod db;
pub mod error;
pub mod horizon;
pub mod models;
pub mod reconciliation;
pub mod telemetry;

// Legacy placeholders (kept for now; will be replaced as Phase 1.2 progresses)
pub mod sdex;
pub mod soroban;

use sqlx::PgPool;
use crate::reconciliation::BackfillManager;

/// Indexer service
pub struct Indexer {
    pool: PgPool,
    backfill_manager: Option<BackfillManager>,
}

impl Indexer {
    /// Create a new indexer instance
    pub fn new(pool: PgPool) -> Self {
        Self {
            backfill_manager: Some(BackfillManager::new(pool.clone())),
            pool,
        }
    }

    /// Access the backfill manager
    pub fn backfill(&self) -> Option<&BackfillManager> {
        self.backfill_manager.as_ref()
    }
}
