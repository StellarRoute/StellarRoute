//! StellarRoute Indexer
//!
//! This crate provides the indexing service for SDEX orderbooks and Soroban AMM pools.

pub mod error;
pub mod config;
pub mod db;
pub mod horizon;
pub mod models;

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
