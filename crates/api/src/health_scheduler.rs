//! Background job scheduler for liquidity health score recomputation.
//!
//! Periodically computes venue health scores from the current liquidity
//! state, persists them to `venue_health_scores`, and emits metrics.
//! Decoupled from the hot quote path — runs on its own configurable cadence
//! with jitter, retries, and dead-letter logging.

use std::time::{Duration, Instant};

use rand::Rng;
use sqlx::{PgPool, Row};
use stellarroute_indexer::db::HealthScoreWriter;
use stellarroute_routing::health::scorer::{
    AmmScorer, HealthScorer, SdexScorer, VenueScorerInput, VenueType,
};
use tracing::{error, info, warn};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the health score background scheduler.
#[derive(Debug, Clone)]
pub struct HealthSchedulerConfig {
    /// Base interval between recomputation cycles.
    pub interval: Duration,
    /// Maximum random jitter added to the interval (uniform 0..jitter).
    pub jitter: Duration,
    /// Maximum number of retry attempts per cycle before dead-letter logging.
    pub max_retries: u32,
    /// Fixed delay between retry attempts.
    pub retry_delay: Duration,
}

impl Default for HealthSchedulerConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(60),
            jitter: Duration::from_secs(10),
            max_retries: 3,
            retry_delay: Duration::from_secs(5),
        }
    }
}

impl HealthSchedulerConfig {
    /// Read from environment variables (with defaults).
    pub fn from_env() -> Self {
        let interval = std::env::var("HEALTH_SCORE_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(60));

        let jitter = std::env::var("HEALTH_SCORE_JITTER_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(10));

        let max_retries = std::env::var("HEALTH_SCORE_MAX_RETRIES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3);

        let retry_delay = std::env::var("HEALTH_SCORE_RETRY_DELAY_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(5));

        Self {
            interval,
            jitter,
            max_retries,
            retry_delay,
        }
    }
}

// ---------------------------------------------------------------------------
// Scheduler
// ---------------------------------------------------------------------------

/// Background scheduler that periodically recomputes and persists venue
/// health scores.
pub struct HealthScheduler;

impl HealthScheduler {
    /// Spawn the background recomputation loop.
    ///
    /// The loop sleeps for `interval + random jitter` between cycles.
    pub fn start(pool: PgPool, config: HealthSchedulerConfig) {
        info!(
            interval_secs = %config.interval.as_secs(),
            jitter_secs = %config.jitter.as_secs(),
            max_retries = %config.max_retries,
            "Starting health score background scheduler",
        );

        tokio::spawn(async move {
            Self::run_loop(pool, config).await;
        });
    }

    // ── main loop ──────────────────────────────────────────────────────────

    async fn run_loop(pool: PgPool, config: HealthSchedulerConfig) {
        loop {
            // 1. Sleep with jitter so that multiple API instances desynchronise.
            let wait = config.interval + Self::jitter(&config.jitter);
            tokio::time::sleep(wait).await;

            // 2. Run one computation cycle with retries.
            if let Err(dead_letter) = Self::run_cycle_with_retry(pool.clone(), &config).await {
                error!(
                    error = %dead_letter,
                    max_retries = %config.max_retries,
                    "Health score cycle failed after all retries (dead-letter)",
                );
                crate::metrics::record_health_score_failure();
            }
        }
    }

    // ── retry wrapper ───────────────────────────────────────────────────────

    async fn run_cycle_with_retry(
        pool: PgPool,
        config: &HealthSchedulerConfig,
    ) -> Result<(), String> {
        let mut last_err = String::new();
        for attempt in 1..=config.max_retries {
            match Self::run_cycle(pool.clone()).await {
                Ok(count) => {
                    info!(
                        venues_computed = count,
                        "Health score recomputation complete",
                    );
                    return Ok(());
                }
                Err(e) => {
                    warn!(
                        attempt,
                        max_retries = %config.max_retries,
                        error = %e,
                        "Health score cycle failed",
                    );
                    last_err = e;
                    tokio::time::sleep(config.retry_delay).await;
                }
            }
        }
        Err(last_err)
    }

    // ── single cycle ────────────────────────────────────────────────────────

    async fn run_cycle(pool: PgPool) -> Result<usize, String> {
        let start = Instant::now();

        // Build the scorer with sensible defaults.
        let scorer = HealthScorer {
            sdex: SdexScorer {
                staleness_threshold_secs: 60,
                max_spread: 0.05,
                target_depth_e7: 10_000_000_000,
                depth_levels: 5,
            },
            amm: AmmScorer {
                staleness_threshold_secs: 60,
                min_tvl_threshold_e7: 1_000_000_000,
            },
        };

        // Fetch all venue inputs.
        let inputs = Self::fetch_venue_inputs(&pool)
            .await
            .map_err(|e| format!("fetch venue inputs: {e}"))?;

        if inputs.is_empty() {
            info!("No venues found for health scoring — skipping cycle");
            return Ok(0);
        }

        // Score all venues.
        let scored = scorer.score_venues(&inputs);

        // Persist via HealthScoreWriter (indexer crate).
        let writer = HealthScoreWriter::new(pool);
        let now = chrono::Utc::now();
        let mut persisted = 0usize;

        for sv in &scored {
            let venue_type_str = match sv.record.venue_type {
                VenueType::Sdex => "sdex",
                VenueType::Amm => "amm",
            };
            let record = stellarroute_indexer::db::HealthScoreRecord {
                venue_ref: sv.record.venue_ref.clone(),
                venue_type: venue_type_str.to_string(),
                score: sv.record.score,
                signals: sv.record.signals.clone(),
                computed_at: now,
            };
            // writer.write logs on error internally; we count silently dropped.
            if writer.write(&record).await.is_ok() {
                persisted += 1;
            }
        }

        let elapsed = start.elapsed();
        info!(
            venues_scored = scored.len(),
            persisted,
            duration_ms = elapsed.as_millis(),
            "Health score recomputation complete",
        );

        // Record metrics.
        crate::metrics::record_health_score_duration(elapsed);

        Ok(scored.len())
    }

    // ── data fetching ───────────────────────────────────────────────────────

    /// Queries source tables and builds `VenueScorerInput`s for every venue.
    async fn fetch_venue_inputs(pool: &PgPool) -> Result<Vec<VenueScorerInput>, sqlx::Error> {
        let mut inputs = Vec::new();

        // --- SDEX offers ---
        // Each offer is an individual venue. We pair-level aggregate to
        // determine the market-wide best bid / best ask for the pair so that
        // the spread component of the health score is meaningful.
        //
        // offer_id ─── selling_asset ─── buying_asset ─── price
        //   (A)           X                  Y             P_A
        //   (B)           Y                  X             P_B
        //
        // For pair (X, Y):
        //   best_ask = MIN(price_e7) among offers selling X for Y
        //   best_bid = 1 / MIN(price_e7) among offers selling Y for X
        //              ( = MAX price in terms of Y per X )
        //
        // We fetch all SDEX rows once, then in Rust group by pair.

        let sdex_rows = sqlx::query(
            r#"
            SELECT
                venue_ref,
                selling_asset_id,
                buying_asset_id,
                price_e7,
                available_amount_e7,
                updated_at
            FROM normalized_liquidity
            WHERE venue_type = 'sdex'
              AND available_amount_e7 > 0
            "#,
        )
        .fetch_all(pool)
        .await?;

        if sdex_rows.is_empty() {
            // No SDEX data — skip.
        } else {
            // Build per-pair best-ask map: (selling, buying) -> min price_e7
            let mut best_ask: std::collections::HashMap<(uuid::Uuid, uuid::Uuid), i64> =
                std::collections::HashMap::new();
            for row in &sdex_rows {
                let selling: uuid::Uuid = row.get("selling_asset_id");
                let buying: uuid::Uuid = row.get("buying_asset_id");
                let price: i64 = row.get("price_e7");
                let entry = best_ask.entry((selling, buying)).or_insert(i64::MAX);
                if price < *entry {
                    *entry = price;
                }
            }

            // Build per-pair best-bid map: (selling, buying) -> min price_e7
            // This captures the cheapest offer for the REVERSE direction,
            // which we later invert to get the bid for the forward direction.
            let mut best_bid_reverse: std::collections::HashMap<(uuid::Uuid, uuid::Uuid), i64> =
                std::collections::HashMap::new();
            for row in &sdex_rows {
                let selling: uuid::Uuid = row.get("selling_asset_id");
                let buying: uuid::Uuid = row.get("buying_asset_id");
                let price: i64 = row.get("price_e7");
                let entry = best_bid_reverse
                    .entry((buying, selling))
                    .or_insert(i64::MAX);
                if price < *entry {
                    *entry = price;
                }
            }

            // Build VenueScorerInput for each SDEX offer.
            for row in &sdex_rows {
                let venue_ref: String = row.get("venue_ref");
                let selling: uuid::Uuid = row.get("selling_asset_id");
                let buying: uuid::Uuid = row.get("buying_asset_id");
                let price_e7: i64 = row.get("price_e7");
                let amount_e7: i64 = row.get("available_amount_e7");
                let updated_at: chrono::DateTime<chrono::Utc> = row.get("updated_at");

                let pair_ask = best_ask
                    .get(&(selling, buying))
                    .copied()
                    .unwrap_or(price_e7);
                let pair_bid_rev = best_bid_reverse
                    .get(&(selling, buying))
                    .copied()
                    .unwrap_or(price_e7);

                // Invert the reverse-direction price to express it as
                // Y-per-X.  Since both are i64 e7 values we compute as f64.
                let best_bid_e7 = if pair_bid_rev > 0 {
                    // best_bid_rev is the best ask in the reverse direction
                    // (min price of selling Y for X). To get Y-per-X:
                    // We have X per Y = pair_bid_rev / 1e7
                    // We want Y per X = 1 / (X per Y) = 1e7 / pair_bid_rev
                    // In e7: (1e7 / pair_bid_rev) * 1e7 = 1e14 / pair_bid_rev
                    (10_000_000_000_000_000_i128 / pair_bid_rev as i128) as i64
                } else {
                    0
                };

                inputs.push(VenueScorerInput {
                    venue_ref,
                    venue_type: VenueType::Sdex,
                    best_bid_e7: Some(best_bid_e7 as i128),
                    best_ask_e7: Some(pair_ask as i128),
                    depth_top_n_e7: Some(amount_e7 as i128),
                    reserve_a_e7: None,
                    reserve_b_e7: None,
                    tvl_e7: None,
                    last_updated_at: Some(updated_at),
                });
            }
        }

        // --- AMM pools ---
        // Each pool is a venue. We read reserves directly from the source
        // table because the e7 columns in normalized_liquidity only expose
        // the selling-side amount, not both reserves.

        let amm_rows = sqlx::query(
            r#"
            SELECT
                pool_address,
                reserve_selling::text,
                reserve_buying::text,
                updated_at
            FROM amm_pool_reserves
            "#,
        )
        .fetch_all(pool)
        .await?;

        for row in &amm_rows {
            let pool_address: String = row.get("pool_address");
            let reserve_selling: String = row.get("reserve_selling");
            let reserve_buying: String = row.get("reserve_buying");
            let updated_at: chrono::DateTime<chrono::Utc> = row.get("updated_at");

            // Parse numeric(38,18) → i128 e7.
            let r_selling: i128 = numeric_to_e7(&reserve_selling);
            let r_buying: i128 = numeric_to_e7(&reserve_buying);

            // Approximate TVL in e7: reserve_selling + reserve_buying in a
            // common numeraire is hard without a price feed; use product as
            // a proxy (constant-product invariant).
            let tvl_e7 = if r_selling > 0 && r_buying > 0 {
                // Rough indicator: min(reserve_selling, reserve_buying) * price
                // Simplified to just the smaller reserve (weakest link).
                r_selling.min(r_buying) * 2
            } else {
                0
            };

            inputs.push(VenueScorerInput {
                venue_ref: pool_address,
                venue_type: VenueType::Amm,
                best_bid_e7: None,
                best_ask_e7: None,
                depth_top_n_e7: None,
                reserve_a_e7: Some(r_selling),
                reserve_b_e7: Some(r_buying),
                tvl_e7: Some(tvl_e7),
                last_updated_at: Some(updated_at),
            });
        }

        Ok(inputs)
    }

    // ── helpers ─────────────────────────────────────────────────────────────

    /// Returns a random duration in `[0, jitter)`.
    fn jitter(jitter: &Duration) -> Duration {
        let ms = jitter.as_millis() as u64;
        if ms == 0 {
            return Duration::ZERO;
        }
        Duration::from_millis(rand::thread_rng().gen_range(0..ms))
    }
}

/// Parse a `numeric(38,18)` PG text value into an i128 scaled by 10^7.
/// Drops sub‑e7 precision.
fn numeric_to_e7(val: &str) -> i128 {
    if let Some(dot) = val.find('.') {
        let int_part = &val[..dot];
        let frac_part = &val[dot + 1..];
        let int_val: i128 = int_part.parse().unwrap_or(0);
        let digits = &frac_part[..frac_part.len().min(7)];
        let frac_str = format!("{:0<7}", digits);
        let frac_val: i128 = frac_str[..7].parse().unwrap_or(0);
        int_val * 10_000_000 + frac_val
    } else {
        val.parse::<i128>().unwrap_or(0) * 10_000_000
    }
}
