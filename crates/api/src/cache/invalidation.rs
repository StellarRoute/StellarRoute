//! Cache invalidation hooks for liquidity update events
//!
//! This module provides targeted cache invalidation when SDEX or AMM liquidity
//! updates arrive, preventing stale outputs while maintaining cache efficiency.

use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use super::CacheManager;

/// Liquidity update event types
#[derive(Debug, Clone)]
pub enum LiquidityUpdateEvent {
    /// SDEX orderbook update for a specific pair
    SdexUpdate {
        base_asset: String,
        counter_asset: String,
        ledger_sequence: u64,
    },
    /// AMM pool reserve update
    AmmUpdate {
        pool_address: String,
        asset_a: String,
        asset_b: String,
        ledger_sequence: u64,
    },
    /// General liquidity revision update
    RevisionUpdate {
        base_asset: String,
        counter_asset: String,
        revision: u64,
    },
}

/// Cache invalidation manager for liquidity-aware cache control
pub struct CacheInvalidationManager {
    cache: Arc<Mutex<CacheManager>>,
    /// Track pending invalidations to batch them
    pending_invalidations: Arc<Mutex<Vec<LiquidityUpdateEvent>>>,
}

impl CacheInvalidationManager {
    /// Create a new cache invalidation manager
    pub fn new(cache: Arc<Mutex<CacheManager>>) -> Self {
        Self {
            cache,
            pending_invalidations: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Process a liquidity update event with targeted invalidation
    ///
    /// This method ensures:
    /// - Invalidation scope is pair-aware and amount-safe
    /// - Race conditions between reads and invalidation are handled
    /// - Only relevant cached data is invalidated
    pub async fn handle_liquidity_update(
        &self,
        event: LiquidityUpdateEvent,
    ) -> Result<u64, String> {
        match &event {
            LiquidityUpdateEvent::SdexUpdate {
                base_asset,
                counter_asset,
                ledger_sequence,
            } => {
                info!(
                    base = %base_asset,
                    counter = %counter_asset,
                    ledger = ledger_sequence,
                    "Processing SDEX liquidity update"
                );
                self.invalidate_pair_caches(base_asset, counter_asset).await
            }
            LiquidityUpdateEvent::AmmUpdate {
                pool_address,
                asset_a,
                asset_b,
                ledger_sequence,
            } => {
                info!(
                    pool = %pool_address,
                    asset_a = %asset_a,
                    asset_b = %asset_b,
                    ledger = ledger_sequence,
                    "Processing AMM liquidity update"
                );
                self.invalidate_pair_caches(asset_a, asset_b).await
            }
            LiquidityUpdateEvent::RevisionUpdate {
                base_asset,
                counter_asset,
                revision,
            } => {
                debug!(
                    base = %base_asset,
                    counter = %counter_asset,
                    revision = revision,
                    "Processing liquidity revision update"
                );
                self.update_revision_and_invalidate(base_asset, counter_asset, *revision)
                    .await
            }
        }
    }

    /// Invalidate all cached quotes for a specific trading pair
    ///
    /// Uses pair-aware pattern matching to only invalidate relevant caches,
    /// ensuring amount-safe invalidation (all amounts for the pair are cleared).
    async fn invalidate_pair_caches(
        &self,
        base_asset: &str,
        counter_asset: &str,
    ) -> Result<u64, String> {
        let pattern = super::keys::quote_pair_pattern(base_asset, counter_asset);

        let mut cache = self.cache.lock().await;
        match cache.delete_by_pattern(&pattern).await {
            Ok(count) => {
                info!(
                    base = %base_asset,
                    counter = %counter_asset,
                    deleted = count,
                    "Invalidated quote caches for pair"
                );
                Ok(count)
            }
            Err(e) => {
                warn!(
                    base = %base_asset,
                    counter = %counter_asset,
                    error = %e,
                    "Failed to invalidate quote caches"
                );
                Err(format!("Cache invalidation failed: {}", e))
            }
        }
    }

    /// Update liquidity revision and invalidate stale caches
    ///
    /// This handles race conditions by:
    /// 1. First updating the revision marker
    /// 2. Then invalidating quotes that may be based on older revisions
    async fn update_revision_and_invalidate(
        &self,
        base_asset: &str,
        counter_asset: &str,
        revision: u64,
    ) -> Result<u64, String> {
        let revision_key = super::keys::liquidity_revision(base_asset, counter_asset);

        let mut cache = self.cache.lock().await;

        // Store the new revision (with a reasonable TTL)
        if let Err(e) = cache
            .set(
                &revision_key,
                &revision,
                std::time::Duration::from_secs(300),
            )
            .await
        {
            warn!(
                base = %base_asset,
                counter = %counter_asset,
                error = %e,
                "Failed to update liquidity revision"
            );
        }

        // Invalidate all quotes for this pair
        let pattern = super::keys::quote_pair_pattern(base_asset, counter_asset);
        match cache.delete_by_pattern(&pattern).await {
            Ok(count) => {
                debug!(
                    base = %base_asset,
                    counter = %counter_asset,
                    revision = revision,
                    deleted = count,
                    "Updated revision and invalidated caches"
                );
                Ok(count)
            }
            Err(e) => {
                warn!(
                    base = %base_asset,
                    counter = %counter_asset,
                    error = %e,
                    "Failed to invalidate after revision update"
                );
                Err(format!("Cache invalidation failed: {}", e))
            }
        }
    }

    /// Batch process multiple liquidity updates
    ///
    /// Collects updates and processes them together to reduce lock contention
    pub async fn batch_invalidate(&self, events: Vec<LiquidityUpdateEvent>) -> Result<u64, String> {
        let mut total_invalidated = 0u64;

        for event in events {
            match self.handle_liquidity_update(event).await {
                Ok(count) => total_invalidated += count,
                Err(e) => warn!("Batch invalidation error: {}", e),
            }
        }

        info!(
            total_invalidated = total_invalidated,
            "Completed batch cache invalidation"
        );

        Ok(total_invalidated)
    }

    /// Queue an invalidation event for later processing
    pub async fn queue_invalidation(&self, event: LiquidityUpdateEvent) {
        let mut pending = self.pending_invalidations.lock().await;
        pending.push(event);
    }

    /// Process all queued invalidation events
    pub async fn flush_pending(&self) -> Result<u64, String> {
        let events = {
            let mut pending = self.pending_invalidations.lock().await;
            std::mem::take(&mut *pending)
        };

        if events.is_empty() {
            return Ok(0);
        }

        self.batch_invalidate(events).await
    }
}

/// Helper to check if a cached quote is stale relative to current revision
pub async fn is_quote_stale(
    cache: &mut CacheManager,
    base_asset: &str,
    counter_asset: &str,
    quote_timestamp: u64,
) -> bool {
    let revision_key = super::keys::liquidity_revision(base_asset, counter_asset);

    match cache.get::<u64>(&revision_key).await {
        Some(current_revision) => {
            // If current revision is newer than quote timestamp, quote is stale
            current_revision > quote_timestamp
        }
        None => {
            // No revision tracked, assume quote is fresh
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_liquidity_update_event_variants() {
        let sdex_event = LiquidityUpdateEvent::SdexUpdate {
            base_asset: "XLM".to_string(),
            counter_asset: "USDC".to_string(),
            ledger_sequence: 12345,
        };

        let amm_event = LiquidityUpdateEvent::AmmUpdate {
            pool_address: "CAAAAAAA".to_string(),
            asset_a: "XLM".to_string(),
            asset_b: "USDC".to_string(),
            ledger_sequence: 12345,
        };

        let revision_event = LiquidityUpdateEvent::RevisionUpdate {
            base_asset: "XLM".to_string(),
            counter_asset: "USDC".to_string(),
            revision: 100,
        };

        // Events should be constructable and debug-printable
        assert!(format!("{:?}", sdex_event).contains("SdexUpdate"));
        assert!(format!("{:?}", amm_event).contains("AmmUpdate"));
        assert!(format!("{:?}", revision_event).contains("RevisionUpdate"));
    }
}
