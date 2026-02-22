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
        .map_err(IndexerError::DatabaseQuery)?;

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
        .map_err(IndexerError::DatabaseQuery)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::horizon::{
        HorizonEmbedded, HorizonLinks, HorizonOffer, HorizonPage, HorizonPriceR,
    };
    use serde_json::json;

    // -----------------------------------------------------------------------
    // IndexingMode
    // -----------------------------------------------------------------------

    #[test]
    fn test_indexing_mode_polling_eq() {
        assert_eq!(IndexingMode::Polling, IndexingMode::Polling);
    }

    #[test]
    fn test_indexing_mode_streaming_eq() {
        assert_eq!(IndexingMode::Streaming, IndexingMode::Streaming);
    }

    #[test]
    fn test_indexing_mode_polling_ne_streaming() {
        assert_ne!(IndexingMode::Polling, IndexingMode::Streaming);
    }

    #[test]
    fn test_indexing_mode_is_copy() {
        let mode = IndexingMode::Polling;
        let mode2 = mode; // Copy must work without clone()
        assert_eq!(mode, mode2);
    }

    #[test]
    fn test_indexing_mode_clone() {
        let mode = IndexingMode::Streaming;
        let cloned = mode;
        assert_eq!(mode, cloned);
    }

    #[test]
    fn test_indexing_mode_debug() {
        let s = format!("{:?}", IndexingMode::Polling);
        assert!(s.contains("Polling"));
        let s2 = format!("{:?}", IndexingMode::Streaming);
        assert!(s2.contains("Streaming"));
    }

    // -----------------------------------------------------------------------
    // Mock Horizon API response deserialization
    // -----------------------------------------------------------------------

    fn make_horizon_offer_json(id: &str, seller: &str) -> serde_json::Value {
        json!({
            "id": id,
            "paging_token": "token",
            "seller": seller,
            "selling": {"asset_type": "native"},
            "buying": {
                "asset_type": "credit_alphanum4",
                "asset_code": "USDC",
                "asset_issuer": seller
            },
            "amount": "100.0",
            "price": "1.5",
            "price_r": {"n": 3, "d": 2},
            "last_modified_ledger": 12345
        })
    }

    #[test]
    fn test_horizon_offer_deserializes_from_json() {
        let value = make_horizon_offer_json(
            "99",
            "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
        );
        let offer: HorizonOffer = serde_json::from_value(value).unwrap();
        assert_eq!(offer.id, "99");
        assert_eq!(offer.last_modified_ledger, 12345);
        assert!(offer.price_r.is_some());
    }

    #[test]
    fn test_horizon_offer_without_optional_fields() {
        let value = json!({
            "id": "1",
            "seller": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
            "selling": {"asset_type": "native"},
            "buying": {"asset_type": "native"},
            "amount": "1.0",
            "price": "1.0",
            "last_modified_ledger": 1
        });
        let offer: HorizonOffer = serde_json::from_value(value).unwrap();
        assert!(offer.paging_token.is_none());
        assert!(offer.price_r.is_none());
    }

    #[test]
    fn test_horizon_price_r_fields() {
        let pr = HorizonPriceR { n: 7, d: 3 };
        assert_eq!(pr.n, 7);
        assert_eq!(pr.d, 3);
    }

    #[test]
    fn test_horizon_page_with_records_deserializes() {
        let seller = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";
        let page_json = json!({
            "_embedded": {
                "records": [
                    make_horizon_offer_json("1", seller),
                    make_horizon_offer_json("2", seller)
                ]
            },
            "_links": {
                "next": {"href": "https://horizon.stellar.org/offers?cursor=2"}
            }
        });

        let page: HorizonPage<HorizonOffer> = serde_json::from_value(page_json).unwrap();
        assert_eq!(page.embedded.records.len(), 2);
        assert_eq!(page.embedded.records[0].id, "1");
        assert_eq!(page.embedded.records[1].id, "2");
        assert!(page.links.is_some());
    }

    #[test]
    fn test_horizon_page_empty_records() {
        let page_json = json!({
            "_embedded": {"records": []},
            "_links": null
        });
        let page: HorizonPage<HorizonOffer> = serde_json::from_value(page_json).unwrap();
        assert!(page.embedded.records.is_empty());
    }

    #[test]
    fn test_horizon_page_without_next_link() {
        let seller = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";
        let page_json = json!({
            "_embedded": {
                "records": [make_horizon_offer_json("1", seller)]
            }
        });
        let page: HorizonPage<HorizonOffer> = serde_json::from_value(page_json).unwrap();
        assert_eq!(page.embedded.records.len(), 1);
        assert!(page.links.is_none());
    }

    #[test]
    fn test_horizon_embedded_records_count() {
        let seller = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";
        let embedded: HorizonEmbedded<HorizonOffer> = HorizonEmbedded {
            records: vec![
                serde_json::from_value(make_horizon_offer_json("10", seller)).unwrap(),
                serde_json::from_value(make_horizon_offer_json("20", seller)).unwrap(),
                serde_json::from_value(make_horizon_offer_json("30", seller)).unwrap(),
            ],
        };
        assert_eq!(embedded.records.len(), 3);
    }

    // -----------------------------------------------------------------------
    // Integration: HorizonOffer → Offer round-trip via mock data
    // -----------------------------------------------------------------------

    #[test]
    fn test_offer_parsed_from_mock_horizon_response() {
        use crate::models::offer::Offer;

        let seller = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";
        let value = make_horizon_offer_json("42", seller);
        let horizon_offer: HorizonOffer = serde_json::from_value(value).unwrap();
        let offer = Offer::try_from(horizon_offer).unwrap();

        assert_eq!(offer.id, 42);
        assert_eq!(offer.seller, seller);
        assert_eq!(offer.price_n, 3);
        assert_eq!(offer.price_d, 2);
    }

    #[test]
    fn test_multiple_offers_from_horizon_page() {
        use crate::models::offer::Offer;

        let seller = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";
        let page_json = json!({
            "_embedded": {
                "records": [
                    make_horizon_offer_json("1", seller),
                    make_horizon_offer_json("2", seller),
                    make_horizon_offer_json("3", seller),
                ]
            }
        });

        let page: HorizonPage<HorizonOffer> = serde_json::from_value(page_json).unwrap();

        let offers: Vec<Offer> = page
            .embedded
            .records
            .into_iter()
            .filter_map(|h| Offer::try_from(h).ok())
            .collect();

        assert_eq!(offers.len(), 3);
        assert_eq!(offers[0].id, 1);
        assert_eq!(offers[1].id, 2);
        assert_eq!(offers[2].id, 3);
    }

    #[test]
    fn test_empty_orderbook_page_produces_zero_offers() {
        use crate::models::offer::Offer;

        let page_json = json!({"_embedded": {"records": []}});
        let page: HorizonPage<HorizonOffer> = serde_json::from_value(page_json).unwrap();

        let offers: Vec<Offer> = page
            .embedded
            .records
            .into_iter()
            .filter_map(|h| Offer::try_from(h).ok())
            .collect();

        assert!(offers.is_empty());
    }

    #[test]
    fn test_malformed_offer_in_page_is_skipped() {
        use crate::models::offer::Offer;

        let seller = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";
        let page_json = json!({
            "_embedded": {
                "records": [
                    make_horizon_offer_json("1", seller),
                    // Malformed: id is not a number
                    {
                        "id": "NOTANUMBER",
                        "seller": seller,
                        "selling": {"asset_type": "native"},
                        "buying": {"asset_type": "native"},
                        "amount": "1.0",
                        "price": "1.0",
                        "last_modified_ledger": 1
                    },
                    make_horizon_offer_json("3", seller),
                ]
            }
        });

        let page: HorizonPage<HorizonOffer> = serde_json::from_value(page_json).unwrap();
        let offers: Vec<Offer> = page
            .embedded
            .records
            .into_iter()
            .filter_map(|h| Offer::try_from(h).ok())
            .collect();

        // Only offer id=1 and id=3 parse; id=NOTANUMBER is skipped
        // id=3 also fails because same asset selling==buying, so only id=1 succeeds
        assert!(!offers.is_empty());
        assert!(offers.iter().any(|o| o.id == 1));
    }

    #[test]
    fn test_horizon_links_next_href() {
        let next_href = "https://horizon.stellar.org/offers?cursor=100&limit=200&order=asc";
        let links = HorizonLinks {
            next: Some(crate::models::horizon::HorizonLink {
                href: next_href.to_string(),
            }),
        };
        assert_eq!(links.next.unwrap().href, next_href);
    }
}
