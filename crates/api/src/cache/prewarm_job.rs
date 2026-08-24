use crate::cache::keys;
use crate::cache::JitteredTtl;
// crate::error::Result not needed here
use crate::models::request::{AssetPath, QuoteParams, QuoteType};
use crate::models::PreparedQuoteResponse;
use crate::state::AppState;
use lazy_static::lazy_static;
use prometheus::{IntCounter, IntCounterVec};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

lazy_static! {
    pub static ref PREWARM_RUNS: IntCounter = prometheus::register_int_counter!(
        "stellarroute_prewarm_runs_total",
        "Total number of prewarm runs"
    )
    .expect("Can't create PREWARM_RUNS");
    pub static ref PREWARM_SKIPPED: IntCounterVec = prometheus::register_int_counter_vec!(
        "stellarroute_prewarm_skipped_total",
        "Number of prewarm runs skipped",
        &["reason"]
    )
    .expect("Can't create PREWARM_SKIPPED");
    pub static ref PREWARM_SUCCESS: IntCounter = prometheus::register_int_counter!(
        "stellarroute_prewarm_success_total",
        "Number of successfully prewarmed entries"
    )
    .expect("Can't create PREWARM_SUCCESS");
    pub static ref PREWARM_ERRORS: IntCounter = prometheus::register_int_counter!(
        "stellarroute_prewarm_errors_total",
        "Number of errors during prewarm runs"
    )
    .expect("Can't create PREWARM_ERRORS");
}

/// Configuration for prewarm job
#[derive(Debug, Clone)]
pub struct PrewarmConfig {
    pub pairs: Vec<(String, String)>,
    pub interval_secs: u64,
    pub amount: String,
    pub slippage_bps: u32,
}

impl Default for PrewarmConfig {
    fn default() -> Self {
        Self {
            pairs: Vec::new(),
            interval_secs: 60,
            amount: "1".to_string(),
            slippage_bps: 50,
        }
    }
}

pub struct PrewarmJob {
    config: PrewarmConfig,
    state: Arc<AppState>,
}

impl PrewarmJob {
    pub fn new(config: PrewarmConfig, state: Arc<AppState>) -> Self {
        Self { config, state }
    }

    pub fn start(self: Arc<Self>) {
        info!(
            interval_secs = self.config.interval_secs,
            "Starting cache prewarm job"
        );

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(Duration::from_secs(self.config.interval_secs));

            loop {
                interval.tick().await;

                PREWARM_RUNS.inc();

                // Skip if indexer lag is unhealthy
                let snaps = self.state.indexer_lag.snapshots().await;
                let mut skip = false;
                for s in snaps.iter() {
                    if s.status != crate::indexer_lag::SyncStatus::Ok {
                        warn!(
                            source = s.source,
                            status = s.status.as_str(),
                            "Skipping prewarm due to indexer lag"
                        );
                        PREWARM_SKIPPED.with_label_values(&["indexer_lag"]).inc();
                        skip = true;
                        break;
                    }
                }

                if skip {
                    continue;
                }

                // For each configured pair, compute a live quote and populate cache
                for (base, quote) in self.config.pairs.iter() {
                    let state = self.state.clone();
                    let base_s = base.clone();
                    let quote_s = quote.clone();
                    let amount = self.config.amount.clone();
                    let slippage = self.config.slippage_bps;

                    // Run each prewarm concurrently but do not flood the system
                    let task = tokio::spawn(async move {
                        match AssetPath::parse(&base_s) {
                            Ok(base_ap) => match AssetPath::parse(&quote_s) {
                                Ok(quote_ap) => {
                                    let params = QuoteParams {
                                        amount: Some(amount.clone()),
                                        slippage_bps: Some(slippage),
                                        quote_type: QuoteType::Sell,
                                        explain: Some(false),
                                        fields: None,
                                    };

                                    match crate::routes::quote::compute_quote_response(
                                        state.clone(),
                                        base_ap.clone(),
                                        quote_ap.clone(),
                                        params.clone(),
                                        false,
                                    )
                                    .await
                                    {
                                        Ok(qr) => {
                                            // Serialize prepared response and set cache
                                            if let Ok(prepared) =
                                                PreparedQuoteResponse::from_quote(qr)
                                            {
                                                if let Some(cache) = &state.cache {
                                                    if let Ok(mut guard) = cache.try_lock() {
                                                        let jitter = JitteredTtl::default();
                                                        let ttl = jitter
                                                            .apply(state.cache_policy.quote_ttl);
                                                        let key = keys::quote(
                                                            &base_s,
                                                            &quote_s,
                                                            &params
                                                                .amount
                                                                .clone()
                                                                .unwrap_or_else(|| "1".to_string()),
                                                            params.slippage_bps(),
                                                            match params.quote_type {
                                                                QuoteType::Sell => "sell",
                                                                _ => "buy",
                                                            },
                                                            params.explain.unwrap_or(false),
                                                        );

                                                        let _ = guard
                                                            .set_json(
                                                                &key,
                                                                std::str::from_utf8(
                                                                    prepared.json_bytes(),
                                                                )
                                                                .expect("valid utf8"),
                                                                ttl,
                                                            )
                                                            .await;
                                                        PREWARM_SUCCESS.inc();
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            PREWARM_ERRORS.inc();
                                            debug!(error = ?e, "Prewarm compute failed for pair");
                                        }
                                    }
                                }
                                Err(e) => {
                                    PREWARM_ERRORS.inc();
                                    warn!(error = %e, quote = %quote_s, "Invalid quote asset for prewarm");
                                }
                            },
                            Err(e) => {
                                PREWARM_ERRORS.inc();
                                warn!(error = %e, base = %base_s, "Invalid base asset for prewarm");
                            }
                        }
                    });

                    // Limit concurrency slightly by awaiting a short time between spawns
                    let _ = task.await;
                }
            }
        });
    }
}
