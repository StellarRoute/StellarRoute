//! SDEX (Stellar Decentralized Exchange) orderbook indexing

use sqlx::PgPool;
use tracing::{debug, error, info, warn};

use crate::db::Database;
use crate::error::{IndexerError, Result};
use crate::horizon::HorizonClient;
use crate::models::{asset::Asset, horizon::HorizonOffer, offer::Offer};

/// Indexing mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexingMode {
    /// Poll for offers at regular intervals
    Polling,
    /// Stream offers in real-time (SSE)
    Streaming,
}

/// SDEX orderbook indexer
pub struct SdexIndexer {
    horizon: HorizonClient,
    db: Database,
    mode: IndexingMode,
}

impl SdexIndexer {
    /// Create a new SDEX indexer with polling mode
    pub fn new(horizon: HorizonClient, db: Database) -> Self {
        Self {
            horizon,
            db,
            mode: IndexingMode::Polling,
        }
    }

    /// Create a new SDEX indexer with specified mode
    pub fn with_mode(horizon: HorizonClient, db: Database, mode: IndexingMode) -> Self {
        Self { horizon, db, mode }
    }

    /// Start indexing offers from Horizon
    pub async fn start_indexing(&self) -> Result<()> {
        match self.mode {
            IndexingMode::Polling => self.start_polling().await,
            IndexingMode::Streaming => self.start_streaming().await,
        }
    }

    /// Start polling mode indexing
    async fn start_polling(&self) -> Result<()> {
        info!("Starting SDEX offer indexing (polling mode)");

        loop {
            match self.index_offers().await {
                Ok(count) => {
                    info!("Indexed {} offers", count);
                }
                Err(e) => {
                    error!("Error indexing offers: {}", e);
                    // Continue indexing despite errors
                }
            }

            // Poll every 5 seconds
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }

    /// Start streaming mode indexing
    async fn start_streaming(&self) -> Result<()> {
        use futures::StreamExt;

        info!("Starting SDEX offer indexing (streaming mode)");

        let stream = self.horizon.stream_offers().await?;
        futures::pin_mut!(stream);

        while let Some(result) = stream.next().await {
            match result {
                Ok(horizon_offer) => {
                    // Convert to our Offer model
                    match Offer::try_from(horizon_offer) {
                        Ok(offer) => {
                            // Index the offer
                            let pool = self.db.pool();
                            if let Err(e) = self.upsert_asset(pool, &offer.selling).await {
                                warn!("Failed to upsert selling asset: {}", e);
                            }
                            if let Err(e) = self.upsert_asset(pool, &offer.buying).await {
                                warn!("Failed to upsert buying asset: {}", e);
                            }
                            if let Err(e) = self.upsert_offer(pool, &offer).await {
                                warn!("Failed to upsert offer {}: {}", offer.id, e);
                            } else {
                                debug!("Indexed offer {} via streaming", offer.id);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse streamed offer: {}", e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Stream error: {}", e);
                }
            }
        }

        warn!("Offer stream ended unexpectedly");
        Ok(())
    }

    /// Index offers from Horizon API
    async fn index_offers(&self) -> Result<usize> {
        debug!("Fetching offers from Horizon");

        let horizon_offers: Vec<HorizonOffer> = self.horizon.get_offers(None, None, None).await?;
        debug!("Fetched {} offers from Horizon", horizon_offers.len());

        let pool = self.db.pool();
        let mut indexed = 0;

        for horizon_offer in horizon_offers {
            // Convert Horizon offer to our Offer model
            let offer = match Offer::try_from(horizon_offer) {
                Ok(o) => o,
                Err(e) => {
                    warn!("Failed to parse offer: {}", e);
                    continue;
                }
            };

            // Extract and upsert assets
            if let Err(e) = self.upsert_asset(pool, &offer.selling).await {
                warn!("Failed to upsert selling asset: {}", e);
            }
            if let Err(e) = self.upsert_asset(pool, &offer.buying).await {
                warn!("Failed to upsert buying asset: {}", e);
            }

            // Upsert offer
            match self.upsert_offer(pool, &offer).await {
                Ok(_) => indexed += 1,
                Err(e) => {
                    warn!("Failed to upsert offer {}: {}", offer.id, e);
                }
            }
        }

        Ok(indexed)
    }

    /// Upsert an asset into the database
    async fn upsert_asset(&self, pool: &PgPool, asset: &Asset) -> Result<()> {
        let (asset_type, asset_code, asset_issuer) = asset.key();

        sqlx::query(
            r#"
            INSERT INTO assets (asset_type, asset_code, asset_issuer, created_at, updated_at)
            VALUES ($1, $2, $3, NOW(), NOW())
            ON CONFLICT (asset_type, asset_code, asset_issuer)
            DO UPDATE SET updated_at = NOW()
            "#,
        )
        .bind(asset_type)
        .bind(asset_code)
        .bind(asset_issuer)
        .execute(pool)
        .await
        .map_err(IndexerError::Database)?;

        Ok(())
    }

    /// Upsert an offer into the database
    async fn upsert_offer(&self, pool: &PgPool, offer: &Offer) -> Result<()> {
        let (selling_type, selling_code, selling_issuer) = offer.selling.key();
        let (buying_type, buying_code, buying_issuer) = offer.buying.key();

        sqlx::query(
            r#"
            INSERT INTO sdex_offers (
                offer_id, seller_id, selling_asset_type, selling_asset_code, selling_asset_issuer,
                buying_asset_type, buying_asset_code, buying_asset_issuer,
                amount, price_n, price_d, price, last_modified_ledger, last_modified_time,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, NOW(), NOW())
            ON CONFLICT (offer_id)
            DO UPDATE SET
                seller_id = EXCLUDED.seller_id,
                amount = EXCLUDED.amount,
                price_n = EXCLUDED.price_n,
                price_d = EXCLUDED.price_d,
                price = EXCLUDED.price,
                last_modified_ledger = EXCLUDED.last_modified_ledger,
                last_modified_time = EXCLUDED.last_modified_time,
                updated_at = NOW()
            "#,
        )
        .bind(offer.id as i64)
        .bind(offer.seller.as_str())
        .bind(selling_type)
        .bind(selling_code)
        .bind(selling_issuer)
        .bind(buying_type)
        .bind(buying_code)
        .bind(buying_issuer)
        .bind(offer.amount.as_str())
        .bind(offer.price_n)
        .bind(offer.price_d)
        .bind(offer.price.as_str())
        .bind(offer.last_modified_ledger as i64)
        .bind(offer.last_modified_time)
        .execute(pool)
        .await
        .map_err(IndexerError::Database)?;

        Ok(())
    }
}
