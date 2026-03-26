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

/// Indexer service
pub struct Indexer {
    // TODO: implement long-running orchestration (polling + eventual streaming)
}

impl Indexer {
    /// Create a new indexer instance
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for Indexer {
    fn default() -> Self {
        Self::new()
    }
}
