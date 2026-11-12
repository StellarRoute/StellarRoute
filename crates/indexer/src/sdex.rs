//! SDEX (Stellar Decentralized Exchange) orderbook indexing

use sqlx::PgPool;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

use crate::db::Database;
use crate::error::{IndexerError, Result};
use crate::horizon::HorizonClient;
use crate::models::{asset::Asset, offer::Offer};
use crate::telemetry::TraceContext;

/// Canonical `(asset_type, asset_code, asset_issuer)` lookup key used to cache
/// resolved `assets.id` UUIDs within a single SDEX poll cycle.
pub type AssetKey = (String, Option<String>, Option<String>);

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
}

fn pair_key(selling: &Asset, buying: &Asset) -> String {
    fn asset_label(asset: &Asset) -> String {
        let (asset_type, code, _) = asset.key();
        code.unwrap_or(asset_type)
    }

    format!("{}/{}", asset_label(selling), asset_label(buying))
}

async fn resolve_asset_cached<F, Fut>(
    cache: &mut HashMap<AssetKey, uuid::Uuid>,
    asset: &Asset,
    upsert_fn: F,
) -> Result<uuid::Uuid>
where
    F: FnOnce(Asset) -> Fut,
    Fut: std::future::Future<Output = Result<uuid::Uuid>>,
{
    let key = asset.key();
    if let Some(&existing_id) = cache.get(&key) {
        return Ok(existing_id);
    }
    let id = upsert_fn(asset.clone()).await?;
    cache.insert(key, id);
    Ok(id)
}

/// Deduplicate a page of `(Offer, selling_asset_id, buying_asset_id)` tuples by
/// `offer.id`, keeping the **last** occurrence in page order so `INSERT ...
/// SELECT * FROM UNNEST(...) ON CONFLICT (offer_id) DO UPDATE` matches
/// sequential `upsert_offer` overwrite semantics without triggering PostgreSQL's
/// `"ON CONFLICT DO UPDATE command cannot affect row a second time"` error.
fn dedup_page_offers_keep_last(
    rows: &[(Offer, uuid::Uuid, uuid::Uuid)],
) -> Vec<&(Offer, uuid::Uuid, uuid::Uuid)> {
    let mut last_index_by_id: HashMap<u64, usize> = HashMap::with_capacity(rows.len());
    for (idx, (offer, _, _)) in rows.iter().enumerate() {
        last_index_by_id.insert(offer.id, idx);
    }
    rows.iter()
        .enumerate()
        .filter_map(|(idx, item)| {
            if last_index_by_id.get(&item.0.id) == Some(&idx) {
                Some(item)
            } else {
                None
            }
        })
        .collect()
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
        }
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

    /// Index offers from Horizon API.
    ///
    /// Maintains a per-poll `HashMap<AssetKey, Uuid>` so each distinct asset is
    /// upserted at most once per poll, and writes each Horizon page of offers in
    /// a single `INSERT ... SELECT * FROM UNNEST(...) ON CONFLICT` statement
    /// inside a transaction.
    #[tracing::instrument(skip(self), fields(source = "horizon"))]
    pub async fn index_offers(&self) -> Result<usize> {
        debug!("Fetching offers from Horizon");

        let pool = self.db.pool();
        let mut indexed = 0usize;
        let page_limit = 200u32;
        let mut cursor: Option<String> = None;
        let mut asset_cache: HashMap<AssetKey, uuid::Uuid> = HashMap::new();
        let mut page_offers: Vec<(Offer, uuid::Uuid, uuid::Uuid)> =
            Vec::with_capacity(page_limit as usize);

        // Horizon returns offers in pages; polling only the first page misses most
        // of testnet/mainnet liquidity (including native/XLM sell offers).
        for _ in 0..100 {
            let horizon_offers = self
                .horizon
                .get_offers(Some(page_limit), cursor.as_deref(), None)
                .await?;
            let batch_len = horizon_offers.len();
            if batch_len == 0 {
                break;
            }

            debug!("Fetched {} offers from Horizon", batch_len);
            let next_cursor = horizon_offers
                .last()
                .and_then(|offer| offer.paging_token.clone());

            page_offers.clear();
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

                // Resolve selling and buying asset IDs using the per-poll cache
                let selling_asset_id = match resolve_asset_cached(
                    &mut asset_cache,
                    &offer.selling,
                    |asset| async move { self.upsert_asset(pool, &asset).await },
                )
                .await
                {
                    Ok(id) => id,
                    Err(e) => {
                        warn!("Failed to upsert selling asset: {}", e);
                        continue;
                    }
                };
                let buying_asset_id = match resolve_asset_cached(
                    &mut asset_cache,
                    &offer.buying,
                    |asset| async move { self.upsert_asset(pool, &asset).await },
                )
                .await
                {
                    Ok(id) => id,
                    Err(e) => {
                        warn!("Failed to upsert buying asset: {}", e);
                        continue;
                    }
                };

                page_offers.push((offer, selling_asset_id, buying_asset_id));
            }

            if !page_offers.is_empty() {
                match self.upsert_offers_batch(pool, &page_offers).await {
                    Ok(()) => indexed += page_offers.len(),
                    Err(e) => {
                        warn!(
                            page_offer_count = page_offers.len(),
                            "Failed to batch-upsert page of offers: {}", e
                        );
                    }
                }
            }

            if batch_len < page_limit as usize {
                break;
            }

            cursor = next_cursor;
            if cursor.is_none() {
                break;
            }
        }

        Ok(indexed)
    }

    /// Upsert an entire page of parsed offers in a single `UNNEST` statement
    /// inside a transaction.
    #[tracing::instrument(skip(self, pool, rows), fields(batch_size = rows.len()))]
    async fn upsert_offers_batch(
        &self,
        pool: &PgPool,
        rows: &[(Offer, uuid::Uuid, uuid::Uuid)],
    ) -> Result<()> {
        if rows.is_empty() {
            return Ok(());
        }

        let deduped = dedup_page_offers_keep_last(rows);
        let cap = deduped.len();

        let mut offer_ids: Vec<i64> = Vec::with_capacity(cap);
        let mut sellers: Vec<&str> = Vec::with_capacity(cap);
        let mut selling_asset_ids: Vec<uuid::Uuid> = Vec::with_capacity(cap);
        let mut buying_asset_ids: Vec<uuid::Uuid> = Vec::with_capacity(cap);
        let mut amounts: Vec<&str> = Vec::with_capacity(cap);
        let mut price_ns: Vec<i64> = Vec::with_capacity(cap);
        let mut price_ds: Vec<i64> = Vec::with_capacity(cap);
        let mut prices: Vec<&str> = Vec::with_capacity(cap);
        let mut ledgers: Vec<i64> = Vec::with_capacity(cap);
        let mut paging_tokens: Vec<Option<&str>> = Vec::with_capacity(cap);

        for (offer, selling_id, buying_id) in deduped {
            offer_ids.push(offer.id as i64);
            sellers.push(offer.seller.as_str());
            selling_asset_ids.push(*selling_id);
            buying_asset_ids.push(*buying_id);
            amounts.push(offer.amount.as_str());
            price_ns.push(i64::from(offer.price_n));
            price_ds.push(i64::from(offer.price_d));
            prices.push(offer.price.as_str());
            ledgers.push(offer.last_modified_ledger as i64);
            paging_tokens.push(offer.paging_token.as_deref());
        }

        let trace_context = TraceContext::current();

        let mut tx = pool.begin().await.map_err(IndexerError::DatabaseQuery)?;

        sqlx::query(
            r#"
            INSERT INTO sdex_offers (
                offer_id, seller, selling_asset_id, buying_asset_id,
                amount, price_n, price_d, price, last_modified_ledger, paging_token,
                source_trace_id, source_span_id, updated_at
            )
            SELECT
                u.offer_id,
                u.seller,
                u.selling_asset_id,
                u.buying_asset_id,
                u.amount::numeric,
                u.price_n,
                u.price_d,
                u.price::numeric,
                u.last_modified_ledger,
                u.paging_token,
                $11,
                $12,
                NOW()
            FROM UNNEST(
                $1::bigint[],
                $2::text[],
                $3::uuid[],
                $4::uuid[],
                $5::text[],
                $6::bigint[],
                $7::bigint[],
                $8::text[],
                $9::bigint[],
                $10::text[]
            ) AS u(
                offer_id,
                seller,
                selling_asset_id,
                buying_asset_id,
                amount,
                price_n,
                price_d,
                price,
                last_modified_ledger,
                paging_token
            )
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
        .bind(&offer_ids)
        .bind(&sellers)
        .bind(&selling_asset_ids)
        .bind(&buying_asset_ids)
        .bind(&amounts)
        .bind(&price_ns)
        .bind(&price_ds)
        .bind(&prices)
        .bind(&ledgers)
        .bind(&paging_tokens)
        .bind(&trace_context.trace_id)
        .bind(&trace_context.span_id)
        .execute(&mut *tx)
        .await
        .map_err(IndexerError::DatabaseQuery)?;

        tx.commit().await.map_err(IndexerError::DatabaseQuery)?;

        Ok(())
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
    use std::sync::atomic::{AtomicUsize, Ordering};

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
    // Per-poll AssetKey cache & per-page deduplication (#1654)
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_asset_cache_upserts_each_distinct_asset_at_most_once_per_poll() {
        let issuer = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";
        let distinct_assets = [
            Asset::Native,
            Asset::CreditAlphanum4 {
                asset_code: "USDC".to_string(),
                asset_issuer: issuer.to_string(),
            },
            Asset::CreditAlphanum4 {
                asset_code: "EURC".to_string(),
                asset_issuer: issuer.to_string(),
            },
            Asset::CreditAlphanum12 {
                asset_code: "YIELDXLM0001".to_string(),
                asset_issuer: issuer.to_string(),
            },
        ];

        let mut cache: HashMap<AssetKey, uuid::Uuid> = HashMap::new();
        let db_calls = AtomicUsize::new(0);

        // Simulate resolving selling & buying assets across 200 offers referencing the 4 assets.
        for i in 0..200usize {
            let selling = &distinct_assets[i % distinct_assets.len()];
            let buying = &distinct_assets[(i + 1) % distinct_assets.len()];

            let s_id = resolve_asset_cached(&mut cache, selling, |_| {
                let n = db_calls.fetch_add(1, Ordering::SeqCst) + 1;
                async move { Ok(uuid::Uuid::from_u128(n as u128)) }
            })
            .await
            .unwrap();

            let b_id = resolve_asset_cached(&mut cache, buying, |_| {
                let n = db_calls.fetch_add(1, Ordering::SeqCst) + 1;
                async move { Ok(uuid::Uuid::from_u128(n as u128)) }
            })
            .await
            .unwrap();

            assert_ne!(s_id, b_id);
        }

        assert_eq!(cache.len(), 4);
        assert_eq!(
            db_calls.load(Ordering::SeqCst),
            4,
            "400 asset lookups across 4 distinct assets must execute exactly 4 DB upserts"
        );
    }

    #[test]
    fn test_dedup_page_offers_keeps_last_occurrence() {
        let seller = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";
        let o1_old = Offer::try_from(
            serde_json::from_value::<HorizonOffer>(make_horizon_offer_json("77", seller)).unwrap(),
        )
        .unwrap();
        let mut o1_new = o1_old.clone();
        o1_new.amount = "250.5".to_string();
        let o2 = Offer::try_from(
            serde_json::from_value::<HorizonOffer>(make_horizon_offer_json("88", seller)).unwrap(),
        )
        .unwrap();

        let a = uuid::Uuid::from_u128(1);
        let b = uuid::Uuid::from_u128(2);
        let rows = vec![(o1_old, a, b), (o2, a, b), (o1_new, a, b)];
        let deduped = dedup_page_offers_keep_last(&rows);

        assert_eq!(deduped.len(), 2);
        assert_eq!(deduped[0].0.id, 88);
        assert_eq!(deduped[1].0.id, 77);
        assert_eq!(deduped[1].0.amount, "250.5");
    }

    // -----------------------------------------------------------------------
    // Mock Horizon API response deserialization
    // -----------------------------------------------------------------------

    fn make_horizon_offer_json(id: &str, seller: &str) -> serde_json::Value {
        json!({
            "id": id,
            "paging_token": format!("pt_{}", id),
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

    // -----------------------------------------------------------------------
    // PostgreSQL parity & 5,000-offer benchmark tests (#1654)
    // Run with: TEST_DATABASE_URL=postgres://... cargo test -p stellarroute-indexer --lib sdex::tests -- --ignored --nocapture
    // -----------------------------------------------------------------------

    fn replace_db_name(url: &str, db_name: &str) -> String {
        let (without_query, query) = match url.split_once('?') {
            Some((prefix, q)) => (prefix, Some(q)),
            None => (url, None),
        };
        let slash_idx = without_query.rfind('/').expect("valid postgres URL");
        let prefix = &without_query[..=slash_idx];
        match query {
            Some(q) => format!("{}{}?{}", prefix, db_name, q),
            None => format!("{}{}", prefix, db_name),
        }
    }

    async fn create_migrated_temp_db(base_url: &str, prefix: &str) -> (PgPool, String, Database) {
        let admin_url = replace_db_name(base_url, "postgres");
        let admin_pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&admin_url)
            .await
            .expect("connect admin db");
        let db_name = format!("{}_{}", prefix, uuid::Uuid::new_v4().simple());
        sqlx::query(&format!("CREATE DATABASE {}", db_name))
            .execute(&admin_pool)
            .await
            .expect("create temp db");

        let cfg = crate::config::IndexerConfig {
            database_url: replace_db_name(base_url, &db_name),
            max_connections: 4,
            min_connections: 1,
            ..crate::config::IndexerConfig::default()
        };
        let db = Database::new(&cfg).await.expect("connect temp db");
        db.migrate().await.expect("migrate temp db");
        (admin_pool, db_name, db)
    }

    async fn drop_temp_db(admin_pool: PgPool, db_name: &str, db: Database) {
        db.pool().close().await;
        let _ = sqlx::query(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = $1 AND pid <> pg_backend_pid()",
        )
        .bind(db_name)
        .execute(&admin_pool)
        .await;
        let _ = sqlx::query(&format!("DROP DATABASE IF EXISTS {}", db_name))
            .execute(&admin_pool)
            .await;
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL via TEST_DATABASE_URL or DATABASE_URL"]
    async fn test_batched_page_matches_legacy_sdex_offers_and_normalized_liquidity() {
        use wiremock::matchers::{method, path, query_param_is_missing};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let base_url = std::env::var("TEST_DATABASE_URL")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .expect("TEST_DATABASE_URL or DATABASE_URL required");

        let seller = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";
        let records = vec![
            json!({
                "id": "101",
                "paging_token": "pt_101",
                "seller": seller,
                "selling": {"asset_type": "native"},
                "buying": {"asset_type": "credit_alphanum4", "asset_code": "USDC", "asset_issuer": seller},
                "amount": "250.1250000",
                "price": "0.1125000",
                "price_r": {"n": 9, "d": 80},
                "last_modified_ledger": 50001
            }),
            json!({
                "id": "102",
                "paging_token": null,
                "seller": seller,
                "selling": {"asset_type": "credit_alphanum4", "asset_code": "USDC", "asset_issuer": seller},
                "buying": {"asset_type": "credit_alphanum12", "asset_code": "YIELDXLM0001", "asset_issuer": seller},
                "amount": "99.5000000",
                "price": "2.5000000",
                "price_r": {"n": 5, "d": 2},
                "last_modified_ledger": 50002
            }),
            // Malformed offer should be skipped identically by both paths
            json!({
                "id": "BAD_ID",
                "seller": seller,
                "selling": {"asset_type": "native"},
                "buying": {"asset_type": "native"},
                "amount": "1.0",
                "price": "1.0",
                "last_modified_ledger": 50003
            }),
            json!({
                "id": "103",
                "paging_token": "pt_103",
                "seller": seller,
                "selling": {"asset_type": "credit_alphanum12", "asset_code": "YIELDXLM0001", "asset_issuer": seller},
                "buying": {"asset_type": "native"},
                "amount": "1234.0000000",
                "price": "8.8888889",
                "price_r": {"n": 80, "d": 9},
                "last_modified_ledger": 50004
            }),
        ];

        let mock_horizon = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/offers"))
            .and(query_param_is_missing("cursor"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "_embedded": { "records": records },
                "_links": {}
            })))
            .mount(&mock_horizon)
            .await;

        let (admin_a, name_a, db_legacy) = create_migrated_temp_db(&base_url, "sr_1654_leg").await;
        let (admin_b, name_b, db_batch) = create_migrated_temp_db(&base_url, "sr_1654_bat").await;

        let pm = crate::partition::PartitionManager::from_config(
            &crate::config::IndexerConfig::default(),
        );
        let legacy_indexer = SdexIndexer::new(
            HorizonClient::new(&mock_horizon.uri()),
            db_legacy.clone(),
            pm.clone(),
        );
        let batch_indexer = SdexIndexer::new(
            HorizonClient::new(&mock_horizon.uri()),
            db_batch.clone(),
            pm,
        );

        // Run legacy per-offer path on db_legacy
        for raw in &records {
            let h: HorizonOffer = serde_json::from_value(raw.clone()).unwrap();
            if let Ok(offer) = Offer::try_from(h) {
                let s_id = legacy_indexer
                    .upsert_asset(db_legacy.pool(), &offer.selling)
                    .await
                    .unwrap();
                let b_id = legacy_indexer
                    .upsert_asset(db_legacy.pool(), &offer.buying)
                    .await
                    .unwrap();
                legacy_indexer
                    .upsert_offer(db_legacy.pool(), &offer, s_id, b_id)
                    .await
                    .unwrap();
            }
        }

        // Run new batched poll path on db_batch
        let count = batch_indexer.index_offers().await.unwrap();
        assert_eq!(count, 3);

        type OfferSnapshot = (
            i64,
            String,
            String,
            Option<String>,
            String,
            Option<String>,
            String,
            String,
            Option<i64>,
            Option<i64>,
            i64,
            Option<String>,
        );

        let offer_query = r#"
            SELECT
                o.offer_id,
                o.seller,
                sa.asset_type,
                sa.asset_code,
                ba.asset_type,
                ba.asset_code,
                o.amount::text,
                o.price::text,
                o.price_n,
                o.price_d,
                o.last_modified_ledger,
                o.paging_token
            FROM sdex_offers o
            JOIN assets sa ON sa.id = o.selling_asset_id
            JOIN assets ba ON ba.id = o.buying_asset_id
            ORDER BY o.offer_id ASC
        "#;

        let legacy_offers: Vec<OfferSnapshot> = sqlx::query_as(offer_query)
            .fetch_all(db_legacy.pool())
            .await
            .unwrap();
        let batch_offers: Vec<OfferSnapshot> = sqlx::query_as(offer_query)
            .fetch_all(db_batch.pool())
            .await
            .unwrap();
        assert_eq!(batch_offers, legacy_offers);

        type NormSnapshot = (
            String,
            String,
            String,
            Option<String>,
            String,
            Option<String>,
            String,
            String,
            i64,
            i64,
            i64,
        );

        let norm_query = r#"
            SELECT
                nl.venue_type,
                nl.venue_ref,
                sa.asset_type,
                sa.asset_code,
                ba.asset_type,
                ba.asset_code,
                nl.price::text,
                nl.available_amount::text,
                nl.price_e7,
                nl.available_amount_e7,
                nl.source_ledger
            FROM normalized_liquidity nl
            JOIN assets sa ON sa.id = nl.selling_asset_id
            JOIN assets ba ON ba.id = nl.buying_asset_id
            ORDER BY nl.venue_ref ASC
        "#;

        let legacy_norm: Vec<NormSnapshot> = sqlx::query_as(norm_query)
            .fetch_all(db_legacy.pool())
            .await
            .unwrap();
        let batch_norm: Vec<NormSnapshot> = sqlx::query_as(norm_query)
            .fetch_all(db_batch.pool())
            .await
            .unwrap();
        assert_eq!(batch_norm.len(), 3);
        assert_eq!(batch_norm, legacy_norm);

        drop_temp_db(admin_a, &name_a, db_legacy).await;
        drop_temp_db(admin_b, &name_b, db_batch).await;
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL via TEST_DATABASE_URL or DATABASE_URL"]
    async fn bench_5000_offer_poll_duration_batched_vs_legacy() {
        use wiremock::matchers::{method, path, query_param, query_param_is_missing};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let base_url = std::env::var("TEST_DATABASE_URL")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .expect("TEST_DATABASE_URL or DATABASE_URL required");

        let seller = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";
        let codes = [
            "USDC", "EURC", "AQUA", "YBX", "BTC", "ETH", "ARS", "BRL", "NGNT", "TRYB",
        ];

        let mock_horizon = MockServer::start().await;
        let mut all_parsed_offers: Vec<Offer> = Vec::with_capacity(5_000);

        // Mount 25 pages x 200 offers = 5,000 offers, plus a final empty page.
        for page_idx in 0..25usize {
            let mut page_records = Vec::with_capacity(200);
            for offset in 0..200usize {
                let id_num = page_idx * 200 + offset + 1;
                let code_a = codes[id_num % codes.len()];
                let selling_json = if id_num % 3 == 0 {
                    json!({"asset_type": "native"})
                } else {
                    json!({"asset_type": "credit_alphanum4", "asset_code": code_a, "asset_issuer": seller})
                };
                let code_b = codes[(id_num + 3) % codes.len()];
                let buying_json = json!({
                    "asset_type": "credit_alphanum4",
                    "asset_code": code_b,
                    "asset_issuer": seller
                });

                let rec = json!({
                    "id": id_num.to_string(),
                    "paging_token": format!("cursor_{}", id_num),
                    "seller": seller,
                    "selling": selling_json,
                    "buying": buying_json,
                    "amount": format!("{}.5000000", (id_num % 500) + 1),
                    "price": format!("1.{:04}", (id_num % 9000) + 100),
                    "price_r": {"n": (id_num % 9000) + 10100, "d": 10000},
                    "last_modified_ledger": 100_000 + id_num as u64
                });
                let ho: HorizonOffer = serde_json::from_value(rec.clone()).unwrap();
                all_parsed_offers.push(Offer::try_from(ho).unwrap());
                page_records.push(rec);
            }

            let body = json!({
                "_embedded": { "records": page_records },
                "_links": {}
            });

            if page_idx == 0 {
                Mock::given(method("GET"))
                    .and(path("/offers"))
                    .and(query_param_is_missing("cursor"))
                    .respond_with(ResponseTemplate::new(200).set_body_json(body))
                    .mount(&mock_horizon)
                    .await;
            } else {
                let prev_cursor = format!("cursor_{}", page_idx * 200);
                Mock::given(method("GET"))
                    .and(path("/offers"))
                    .and(query_param("cursor", prev_cursor))
                    .respond_with(ResponseTemplate::new(200).set_body_json(body))
                    .mount(&mock_horizon)
                    .await;
            }
        }

        Mock::given(method("GET"))
            .and(path("/offers"))
            .and(query_param("cursor", "cursor_5000"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "_embedded": { "records": [] },
                "_links": {}
            })))
            .mount(&mock_horizon)
            .await;

        let (admin_leg, name_leg, db_leg) =
            create_migrated_temp_db(&base_url, "sr_bench_leg").await;
        let (admin_bat, name_bat, db_bat) =
            create_migrated_temp_db(&base_url, "sr_bench_bat").await;

        let pm = crate::partition::PartitionManager::from_config(
            &crate::config::IndexerConfig::default(),
        );
        let leg_indexer = SdexIndexer::new(
            HorizonClient::new(&mock_horizon.uri()),
            db_leg.clone(),
            pm.clone(),
        );
        let bat_indexer =
            SdexIndexer::new(HorizonClient::new(&mock_horizon.uri()), db_bat.clone(), pm);

        let t_leg = std::time::Instant::now();
        for offer in &all_parsed_offers {
            let s_id = leg_indexer
                .upsert_asset(db_leg.pool(), &offer.selling)
                .await
                .unwrap();
            let b_id = leg_indexer
                .upsert_asset(db_leg.pool(), &offer.buying)
                .await
                .unwrap();
            leg_indexer
                .upsert_offer(db_leg.pool(), offer, s_id, b_id)
                .await
                .unwrap();
        }
        let legacy_ms = t_leg.elapsed().as_secs_f64() * 1_000.0;

        let t_bat = std::time::Instant::now();
        let indexed = bat_indexer.index_offers().await.unwrap();
        let batched_ms = t_bat.elapsed().as_secs_f64() * 1_000.0;

        assert_eq!(indexed, 5_000);
        println!(
            "[#1654 5,000-offer poll benchmark] legacy_db_only={:.1}ms vs batched_full_poll_with_http={:.1}ms ({:.1}x speedup)",
            legacy_ms,
            batched_ms,
            legacy_ms / batched_ms
        );
        assert!(
            batched_ms < legacy_ms,
            "Expected batched 5,000-offer poll ({:.1}ms) to beat sequential per-offer DB writes ({:.1}ms)",
            batched_ms,
            legacy_ms
        );

        drop_temp_db(admin_leg, &name_leg, db_leg).await;
        drop_temp_db(admin_bat, &name_bat, db_bat).await;
    }
}
