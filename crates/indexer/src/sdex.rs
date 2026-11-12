//! SDEX (Stellar Decentralized Exchange) orderbook indexing

use lazy_static::lazy_static;
use prometheus::{register_int_counter, register_int_gauge, IntCounter, IntGauge};
use sqlx::PgPool;
use tracing::{debug, error, info, warn};

use crate::db::Database;
use crate::error::{IndexerError, Result};
use crate::horizon::HorizonClient;
use crate::models::{asset::Asset, offer::Offer};
use crate::telemetry::TraceContext;

/// Default Horizon page size for SDEX offer polling.
pub const DEFAULT_SDEX_PAGE_LIMIT: u32 = 200;

/// Default maximum number of Horizon pages fetched per SDEX poll cycle.
pub const DEFAULT_SDEX_MAX_PAGES_PER_POLL: usize = 100;

lazy_static! {
    /// Total number of disappeared SDEX offers deleted across complete polling sweeps.
    pub static ref SDEX_OFFERS_SWEPT_TOTAL: IntCounter = register_int_counter!(
        "stellarroute_indexer_sdex_offers_swept_total",
        "Total number of disappeared SDEX offers swept after complete Horizon polls"
    )
    .expect("Can't create SDEX_OFFERS_SWEPT_TOTAL counter");

    /// Number of disappeared SDEX offers deleted in the most recent complete poll sweep.
    pub static ref SDEX_OFFERS_SWEPT_LAST_POLL: IntGauge = register_int_gauge!(
        "stellarroute_indexer_sdex_offers_swept_last_poll",
        "Number of disappeared SDEX offers swept in the most recent complete Horizon poll"
    )
    .expect("Can't create SDEX_OFFERS_SWEPT_LAST_POLL gauge");
}

/// Total number of disappeared SDEX offers swept since process start.
pub fn offers_swept_total() -> u64 {
    SDEX_OFFERS_SWEPT_TOTAL.get()
}

/// Number of disappeared SDEX offers swept during the latest complete poll.
pub fn offers_swept_last_poll() -> i64 {
    SDEX_OFFERS_SWEPT_LAST_POLL.get()
}

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
    partition_manager: crate::partition::PartitionManager,
    page_limit: u32,
    max_pages_per_poll: usize,
}

fn pair_key(selling: &Asset, buying: &Asset) -> String {
    fn asset_label(asset: &Asset) -> String {
        let (asset_type, code, _) = asset.key();
        code.unwrap_or(asset_type)
    }

    format!("{}/{}", asset_label(selling), asset_label(buying))
}

impl SdexIndexer {
    /// Create a new SDEX indexer with polling mode
    pub fn new(
        horizon: HorizonClient,
        db: Database,
        partition_manager: crate::partition::PartitionManager,
    ) -> Self {
        Self {
            horizon,
            db,
            mode: IndexingMode::Polling,
            partition_manager,
            page_limit: DEFAULT_SDEX_PAGE_LIMIT,
            max_pages_per_poll: DEFAULT_SDEX_MAX_PAGES_PER_POLL,
        }
    }

    /// Create a new SDEX indexer with specified mode
    pub fn with_mode(
        horizon: HorizonClient,
        db: Database,
        mode: IndexingMode,
        partition_manager: crate::partition::PartitionManager,
    ) -> Self {
        Self {
            horizon,
            db,
            mode,
            partition_manager,
            page_limit: DEFAULT_SDEX_PAGE_LIMIT,
            max_pages_per_poll: DEFAULT_SDEX_MAX_PAGES_PER_POLL,
        }
    }

    /// Override pagination bounds (page size and max pages per poll).
    pub fn with_pagination(mut self, page_limit: u32, max_pages_per_poll: usize) -> Self {
        self.page_limit = page_limit.max(1);
        self.max_pages_per_poll = max_pages_per_poll.max(1);
        self
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
                    crate::metrics::record_offers_indexed("sdex", count as u64);
                    crate::metrics::record_throttle_success("sdex");
                    if let Err(e) = self.record_poll_heartbeat().await {
                        warn!("Failed to record SDEX poll heartbeat: {}", e);
                    }
                }
                Err(IndexerError::RateLimitExceeded { retry_after }) => {
                    // Cursor is NOT advanced on rate-limit — we retry the same page.
                    let wait_secs = retry_after.unwrap_or(5);
                    warn!(
                        retry_after_secs = wait_secs,
                        "SDEX polling rate-limited; preserving cursor and waiting"
                    );
                    let consecutive = self.horizon.throttle.consecutive_429s();
                    crate::metrics::record_throttle_event(wait_secs * 1_000, consecutive, "sdex");
                    tokio::time::sleep(tokio::time::Duration::from_secs(wait_secs)).await;
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

    /// Record the Horizon tip so API lag monitors treat a fresh poll as caught up.
    ///
    /// Orderbook polling rewrites current offers whose `last_modified_ledger` may
    /// be millions of ledgers old; without this heartbeat `/health` stays 503.
    async fn record_poll_heartbeat(&self) -> Result<()> {
        let ledger = self.horizon.get_latest_ledger().await?;
        sqlx::query(
            r#"
            INSERT INTO ingestion_state (key, value, updated_at)
            VALUES ('sdex_last_horizon_ledger', $1, now())
            ON CONFLICT (key)
            DO UPDATE SET value = EXCLUDED.value, updated_at = now()
            "#,
        )
        .bind(ledger.to_string())
        .execute(self.db.pool())
        .await?;
        Ok(())
    }

    /// Start streaming mode indexing
    async fn start_streaming(&self) -> Result<()> {
        use futures::StreamExt;

        info!("Starting SDEX offer indexing (streaming mode)");

        let mut cursor = self.fetch_latest_paging_token().await.ok();
        let mut error_count = 0;

        loop {
            info!(
                cursor = ?cursor,
                "Connecting to Horizon offer stream"
            );

            let stream_result = self.horizon.stream_offers(cursor.as_deref()).await;

            match stream_result {
                Ok(stream) => {
                    error_count = 0;
                    futures::pin_mut!(stream);

                    while let Some(result) = stream.next().await {
                        match result {
                            Ok(horizon_offer) => {
                                crate::metrics::record_sse_event("sdex");
                                // Update local cursor if provided in the offer
                                if let Some(ref token) = horizon_offer.paging_token {
                                    cursor = Some(token.clone());
                                }

                                // Convert to our Offer model
                                match Offer::try_from(horizon_offer) {
                                    Ok(offer) => {
                                        // Determine market pair identifier for partitioning
                                        let pair_key = pair_key(&offer.selling, &offer.buying);
                                        if !self.partition_manager.should_process(&pair_key) {
                                            debug!(
                                                "Skipping pair {} on partition {}",
                                                pair_key, self.partition_manager.partition_id
                                            );
                                            continue;
                                        }

                                        // Index the offer
                                        let pool = self.db.pool();
                                        let selling_asset_id =
                                            match self.upsert_asset(pool, &offer.selling).await {
                                                Ok(id) => id,
                                                Err(e) => {
                                                    warn!("Failed to upsert selling asset: {}", e);
                                                    continue;
                                                }
                                            };
                                        let buying_asset_id =
                                            match self.upsert_asset(pool, &offer.buying).await {
                                                Ok(id) => id,
                                                Err(e) => {
                                                    warn!("Failed to upsert buying asset: {}", e);
                                                    continue;
                                                }
                                            };
                                        if let Err(e) = self
                                            .upsert_offer(
                                                pool,
                                                &offer,
                                                selling_asset_id,
                                                buying_asset_id,
                                            )
                                            .await
                                        {
                                            warn!("Failed to upsert offer {}: {}", offer.id, e);
                                        } else {
                                            debug!("Indexed offer {} via streaming", offer.id);
                                            crate::metrics::record_offers_indexed("sdex", 1);
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Failed to parse streamed offer: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Stream event error: {}", e);
                            }
                        }
                    }
                    warn!("Offer stream ended unexpectedly; reconnecting");
                    crate::metrics::record_sse_disconnect("sdex");
                }
                Err(e) => {
                    error_count += 1;
                    error!(
                        "Failed to connect to SSE stream (attempt {}): {}",
                        error_count, e
                    );

                    if error_count >= 3 {
                        warn!("SSE connection failed consistently; falling back to polling");
                        return self.start_polling().await;
                    }

                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
    }

    /// Fetch the latest paging token from the database
    async fn fetch_latest_paging_token(&self) -> Result<String> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT paging_token FROM sdex_offers WHERE paging_token IS NOT NULL ORDER BY last_modified_ledger DESC LIMIT 1"
        )
        .fetch_optional(self.db.pool())
        .await
        .map_err(IndexerError::DatabaseQuery)?;

        row.map(|r| r.0).ok_or(IndexerError::NotFound {
            entity: "paging_token".to_string(),
            id: "latest".to_string(),
        })
    }

    /// Execute one Horizon SDEX poll cycle and, when the poll reads every page
    /// cleanly without hitting the page cap, sweep offers whose `updated_at`
    /// precedes the poll's database start timestamp.
    #[tracing::instrument(skip(self), fields(source = "horizon"))]
    pub async fn index_offers(&self) -> Result<usize> {
        debug!("Fetching offers from Horizon");

        let pool = self.db.pool();
        let poll_started_at: chrono::DateTime<chrono::Utc> =
            sqlx::query_scalar("SELECT clock_timestamp()")
                .fetch_one(pool)
                .await
                .map_err(IndexerError::DatabaseQuery)?;

        let mut indexed = 0;
        let page_limit = self.page_limit;
        let mut cursor: Option<String> = None;
        let mut reached_end_of_book = false;
        let mut had_upsert_error = false;

        // Horizon returns offers in pages; polling only the first page misses most
        // of testnet/mainnet liquidity (including native/XLM sell offers).
        for _ in 0..self.max_pages_per_poll {
            let horizon_offers = self
                .horizon
                .get_offers(Some(page_limit), cursor.as_deref(), None)
                .await?;
            let batch_len = horizon_offers.len();
            if batch_len == 0 {
                reached_end_of_book = true;
                break;
            }

            debug!("Fetched {} offers from Horizon", batch_len);
            let next_cursor = horizon_offers
                .last()
                .and_then(|offer| offer.paging_token.clone());

            for horizon_offer in horizon_offers {
                // Convert Horizon offer to our Offer model
                let offer = match Offer::try_from(horizon_offer) {
                    Ok(o) => o,
                    Err(e) => {
                        warn!("Failed to parse offer: {}", e);
                        continue;
                    }
                };

                // Determine market pair identifier for partitioning
                let pair_key = pair_key(&offer.selling, &offer.buying);
                if !self.partition_manager.should_process(&pair_key) {
                    debug!(
                        "Skipping pair {} on partition {}",
                        pair_key, self.partition_manager.partition_id
                    );
                    continue;
                }

                // Extract and upsert assets
                let selling_asset_id = match self.upsert_asset(pool, &offer.selling).await {
                    Ok(id) => id,
                    Err(e) => {
                        had_upsert_error = true;
                        warn!("Failed to upsert selling asset: {}", e);
                        continue;
                    }
                };
                let buying_asset_id = match self.upsert_asset(pool, &offer.buying).await {
                    Ok(id) => id,
                    Err(e) => {
                        had_upsert_error = true;
                        warn!("Failed to upsert buying asset: {}", e);
                        continue;
                    }
                };

                // Upsert offer
                match self
                    .upsert_offer(pool, &offer, selling_asset_id, buying_asset_id)
                    .await
                {
                    Ok(_) => indexed += 1,
                    Err(e) => {
                        had_upsert_error = true;
                        warn!("Failed to upsert offer {}: {}", offer.id, e);
                    }
                }
            }

            if batch_len < page_limit as usize {
                reached_end_of_book = true;
                break;
            }

            cursor = next_cursor;
            if cursor.is_none() {
                warn!(
                    "Horizon returned a full SDEX offer page without paging_token; skipping sweep"
                );
                break;
            }
        }

        if reached_end_of_book && !had_upsert_error {
            let swept = self.sweep_disappeared_offers(pool, poll_started_at).await?;
            SDEX_OFFERS_SWEPT_TOTAL.inc_by(swept);
            SDEX_OFFERS_SWEPT_LAST_POLL.set(swept as i64);
            if swept > 0 {
                info!(
                    swept_offers = swept,
                    "Swept disappeared SDEX offers after complete poll"
                );
            } else {
                debug!("Complete SDEX poll finished with zero disappeared offers");
            }
        } else {
            debug!(
                reached_end_of_book = reached_end_of_book,
                had_upsert_error = had_upsert_error,
                "Skipping SDEX offer sweep because poll did not complete cleanly or hit page cap"
            );
        }

        Ok(indexed)
    }

    /// Delete SDEX offers that were not touched during a complete poll in a single
    /// `DELETE FROM sdex_offers` statement so `trg_sync_normalized_sdex` removes
    /// their corresponding `normalized_liquidity` rows.
    async fn sweep_disappeared_offers(
        &self,
        pool: &PgPool,
        poll_started_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64> {
        if self.partition_manager.partition_count <= 1 {
            let result = sqlx::query("DELETE FROM sdex_offers WHERE updated_at < $1")
                .bind(poll_started_at)
                .execute(pool)
                .await
                .map_err(IndexerError::DatabaseQuery)?;
            return Ok(result.rows_affected());
        }

        // Partitioned replicas share `sdex_offers`: only sweep pairs owned by this partition.
        type StalePairRow = (
            uuid::Uuid,
            uuid::Uuid,
            String,
            Option<String>,
            Option<String>,
            String,
            Option<String>,
            Option<String>,
        );

        let stale_pairs: Vec<StalePairRow> = sqlx::query_as(
            r#"
            SELECT DISTINCT
                o.selling_asset_id,
                o.buying_asset_id,
                sa.asset_type,
                sa.asset_code,
                sa.asset_issuer,
                ba.asset_type,
                ba.asset_code,
                ba.asset_issuer
            FROM sdex_offers o
            JOIN assets sa ON sa.id = o.selling_asset_id
            JOIN assets ba ON ba.id = o.buying_asset_id
            WHERE o.updated_at < $1
            "#,
        )
        .bind(poll_started_at)
        .fetch_all(pool)
        .await
        .map_err(IndexerError::DatabaseQuery)?;

        let mut owned_selling = Vec::new();
        let mut owned_buying = Vec::new();
        for (s_id, b_id, s_type, s_code, _s_iss, b_type, b_code, _b_iss) in stale_pairs {
            let s_label = s_code.unwrap_or(s_type);
            let b_label = b_code.unwrap_or(b_type);
            let p_key = format!("{}/{}", s_label, b_label);
            if self.partition_manager.should_process(&p_key) {
                owned_selling.push(s_id);
                owned_buying.push(b_id);
            }
        }

        if owned_selling.is_empty() {
            return Ok(0);
        }

        let result = sqlx::query(
            r#"
            DELETE FROM sdex_offers o
            USING UNNEST($2::uuid[], $3::uuid[]) AS owned(selling_id, buying_id)
            WHERE o.updated_at < $1
              AND o.selling_asset_id = owned.selling_id
              AND o.buying_asset_id = owned.buying_id
            "#,
        )
        .bind(poll_started_at)
        .bind(&owned_selling)
        .bind(&owned_buying)
        .execute(pool)
        .await
        .map_err(IndexerError::DatabaseQuery)?;

        Ok(result.rows_affected())
    }

    /// Upsert an asset into the database
    #[tracing::instrument(skip(self, pool, asset), fields(asset_type = %asset.key().0))]
    async fn upsert_asset(&self, pool: &PgPool, asset: &Asset) -> Result<uuid::Uuid> {
        let (asset_type, asset_code, asset_issuer) = asset.key();

        if asset_type == "native" {
            if let Some(id) = sqlx::query_scalar::<_, uuid::Uuid>(
                r#"
                select id from assets
                where asset_type = 'native'
                order by created_at asc
                limit 1
                "#,
            )
            .fetch_optional(pool)
            .await
            .map_err(IndexerError::DatabaseQuery)?
            {
                return Ok(id);
            }
        }

        sqlx::query_scalar(
            r#"
            INSERT INTO assets (asset_type, asset_code, asset_issuer, created_at)
            VALUES ($1, $2, $3, NOW())
            ON CONFLICT (asset_type, asset_code, asset_issuer)
            DO UPDATE SET asset_type = EXCLUDED.asset_type
            RETURNING id
            "#,
        )
        .bind(asset_type)
        .bind(asset_code)
        .bind(asset_issuer)
        .fetch_one(pool)
        .await
        .map_err(IndexerError::DatabaseQuery)
    }

    /// Upsert an offer into the database
    #[tracing::instrument(skip(self, pool, offer), fields(offer_id = offer.id))]
    async fn upsert_offer(
        &self,
        pool: &PgPool,
        offer: &Offer,
        selling_asset_id: uuid::Uuid,
        buying_asset_id: uuid::Uuid,
    ) -> Result<()> {
        let trace_context = TraceContext::current();

        sqlx::query(
            r#"
            INSERT INTO sdex_offers (
                offer_id, seller, selling_asset_id, buying_asset_id,
                amount, price_n, price_d, price, last_modified_ledger, paging_token,
                source_trace_id, source_span_id, updated_at
            )
            VALUES ($1, $2, $3, $4, $5::numeric, $6, $7, $8::numeric, $9, $10, $11, $12, NOW())
            ON CONFLICT (offer_id)
            DO UPDATE SET
                seller = EXCLUDED.seller,
                selling_asset_id = EXCLUDED.selling_asset_id,
                buying_asset_id = EXCLUDED.buying_asset_id,
                amount = EXCLUDED.amount,
                price_n = EXCLUDED.price_n,
                price_d = EXCLUDED.price_d,
                price = EXCLUDED.price,
                last_modified_ledger = EXCLUDED.last_modified_ledger,
                paging_token = EXCLUDED.paging_token,
                source_trace_id = EXCLUDED.source_trace_id,
                source_span_id = EXCLUDED.source_span_id,
                updated_at = NOW()
            "#,
        )
        .bind(offer.id as i64)
        .bind(offer.seller.as_str())
        .bind(selling_asset_id)
        .bind(buying_asset_id)
        .bind(offer.amount.as_str())
        .bind(offer.price_n)
        .bind(offer.price_d)
        .bind(offer.price.as_str())
        .bind(offer.last_modified_ledger as i64)
        .bind(offer.paging_token.as_deref())
        .bind(trace_context.trace_id)
        .bind(trace_context.span_id)
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
