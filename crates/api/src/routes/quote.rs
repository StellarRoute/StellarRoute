//! Quote endpoint
//!
//! # Dashboard-Ready Metrics
//!
//! The quote pipeline emits structured tracing logs with the following metric fields:
//! - `metric`: Always "stellarroute.quote.request" for request summaries.
//! - `latency_ms`: Duration of the quote request in milliseconds.
//! - `cache_hit`: Boolean indicating if the quote was served from cache.
//! - `error_class`: Outcome category ("validation", "not_found", "stale_market_data", "internal", "none").
//!
//! Request logs and decision stages include matching `request_id` values.

use axum::{extract::State, Json};
use sqlx::Row;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, info_span, warn, Instrument};

use stellarroute_routing::health::filter::GraphFilter;
use stellarroute_routing::health::freshness::{FreshnessGuard, FreshnessOutcome};
use stellarroute_routing::health::policy::{ExclusionPolicy, OverrideRegistry};
use stellarroute_routing::health::scorer::{
    AmmScorer, HealthScorer, HealthScoringConfig, SdexScorer, VenueScorerInput, VenueType,
};

use crate::{
    cache,
    error::{ApiError, Result},
    middleware::{validation::ValidatedQuoteRequest, RequestId},
    models::{
        request::{AssetPath, QuoteParams},
        AssetInfo, ExcludedVenueInfo as ApiExcludedVenueInfo,
        ExclusionDiagnostics as ApiExclusionDiagnostics, ExclusionReason as ApiExclusionReason,
        PathStep, QuoteRationaleMetadata, QuoteResponse, VenueEvaluation,
    },
    state::AppState,
};

/// Get price quote for a trading pair
///
/// Returns the best available price for trading the specified amount
#[utoipa::path(
    get,
    path = "/api/v1/quote/{base}/{quote}",
    tag = "trading",
    params(
        ("base" = String, Path, description = "Base asset (e.g., 'native', 'USDC', or 'USDC:ISSUER')"),
        ("quote" = String, Path, description = "Quote asset (e.g., 'native', 'USDC', or 'USDC:ISSUER')"),
        ("amount" = Option<String>, Query, description = "Amount to trade (default: 1)"),
        ("slippage_bps" = Option<u32>, Query, description = "Slippage tolerance in basis points (default: 50)"),
        ("quote_type" = Option<String>, Query, description = "Type of quote: 'sell' or 'buy' (default: sell)"),
    ),
    responses(
        (status = 200, description = "Price quote", body = QuoteResponse),
        (status = 400, description = "Invalid parameters", body = ErrorResponse),
        (status = 404, description = "No route found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
)]
pub async fn get_quote(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    request_id: RequestId,
    request: crate::middleware::validation::ValidatedQuoteRequest,
) -> Result<Json<QuoteResponse>> {
    let ValidatedQuoteRequest {
        base: base_asset,
        quote: quote_asset,
        params,
    } = request;

    let base = base_asset.to_canonical();
    let quote = quote_asset.to_canonical();

    let explain_header = headers
        .get("x-explain")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let explain = explain_header || params.explain.unwrap_or(false);

    let start_time = std::time::Instant::now();

    let span = info_span!(
        "quote_pipeline",
        request_id = %request_id,
        %base,
        %quote,
        cache_hit = false,
        error_class = tracing::field::Empty,
        latency_ms = tracing::field::Empty,
    );

    async move {
        match get_quote_inner(state, base_asset, quote_asset, params, explain).await {
            Ok((quote, cache_hit)) => {
                let error_class = "none";
                let latency_ms = start_time.elapsed().as_millis() as u64;

                let span = tracing::Span::current();
                span.record("error_class", error_class);
                span.record("latency_ms", latency_ms);

                // Record Prometheus metrics
                crate::metrics::record_quote_latency(
                    std::time::Duration::from_millis(latency_ms),
                    error_class,
                    cache_hit,
                );

                tracing::info!(
                    metric = "stellarroute.quote.request",
                    "Quote pipeline completed"
                );

                Ok(Json(quote))
            }
            Err(e) => {
                let error_class = match &e {
                    ApiError::Validation(_) | ApiError::InvalidAsset(_) => "validation",
                    ApiError::NotFound(_) | ApiError::NoRouteFound => "not_found",
                    ApiError::StaleMarketData { .. } => "stale_market_data",
                    _ => "internal",
                };
                let latency_ms = start_time.elapsed().as_millis() as u64;

                let span = tracing::Span::current();
                span.record("error_class", error_class);
                span.record("latency_ms", latency_ms);

                // Record Prometheus metrics (errors always count as cache_hit=false)
                crate::metrics::record_quote_latency(
                    std::time::Duration::from_millis(latency_ms),
                    error_class,
                    false,
                );

                tracing::info!(
                    metric = "stellarroute.quote.request",
                    "Quote pipeline failed"
                );

                Err(e)
            }
        }
    }
    .instrument(span)
    .await
}

/// POST /api/v1/batch/quote
pub async fn get_batch_quotes(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<crate::models::request::BatchQuoteRequest>,
) -> Result<Json<crate::models::response::BatchQuoteResponse>> {
    debug!("Getting {} batch quotes", payload.quotes.len());

    if payload.quotes.is_empty() {
        return Err(ApiError::Validation("Empty batch request".to_string()));
    }

    if payload.quotes.len() > 25 {
        return Err(ApiError::Validation(
            "Batch size cannot exceed 25 items".to_string(),
        ));
    }

    let mut quotes = Vec::with_capacity(payload.quotes.len());
    for item in payload.quotes {
        let params = QuoteParams {
            amount: item.amount,
            slippage_bps: item.slippage_bps,
            quote_type: item
                .quote_type
                .unwrap_or(crate::models::request::QuoteType::Sell),
            explain: None,
        };

        let base_asset = AssetPath::parse(&item.base)
            .map_err(|e| ApiError::InvalidAsset(format!("Invalid base asset: {}", e)))?;
        let quote_asset = AssetPath::parse(&item.quote)
            .map_err(|e| ApiError::InvalidAsset(format!("Invalid quote asset: {}", e)))?;

        let (quote, _) =
            get_quote_inner(state.clone(), base_asset, quote_asset, params, false).await?;
        quotes.push(quote);
    }

    let total = quotes.len();
    Ok(Json(crate::models::response::BatchQuoteResponse {
        quotes,
        total,
    }))
}

async fn get_quote_inner(
    state: Arc<AppState>,
    base_asset: AssetPath,
    quote_asset: AssetPath,
    params: QuoteParams,
    explain: bool,
) -> Result<(QuoteResponse, bool)> {
    let base = base_asset.to_canonical();
    let quote = quote_asset.to_canonical();

    debug!(
        "Getting data quote for {}/{} (amount: {:?}, type: {:?})",
        base, quote, params.amount, params.quote_type
    );

    // Parse amount (default to 1)
    let amount: f64 = params
        .amount
        .as_deref()
        .unwrap_or("1")
        .parse()
        .unwrap_or(1.0); // Already validated in extractor

    let slippage_bps = params.slippage_bps();
    let quote_type_str = match params.quote_type {
        crate::models::request::QuoteType::Sell => "sell",
        crate::models::request::QuoteType::Buy => "buy",
    };

    let base_id = find_asset_id(&state, &base_asset).await?;
    let quote_id = find_asset_id(&state, &quote_asset).await?;

    maybe_invalidate_quote_cache(&state, &base, &quote, base_id, quote_id).await?;

    // Use single flight for quote computation
    let amount_str = format!("{:.7}", amount);
    let quote_cache_key = cache::keys::quote(
        &base,
        &quote,
        &amount_str,
        slippage_bps,
        quote_type_str,
        explain,
    );

    let state_c = state.clone();
    let base_c = base.clone();
    let quote_c = quote.clone();
    let quote_cache_key_c = quote_cache_key.clone();

    // Use single-flight to coalesce identical concurrent requests
    let result_arc: Arc<crate::error::Result<(QuoteResponse, bool)>> = state
        .quote_single_flight
        .execute(&quote_cache_key, || async move {
            let state = state_c;
            let base = base_c;
            let quote = quote_c;
            let quote_cache_key = quote_cache_key_c;

            // Try to get from cache first (inside single-flight so we only compute once if miss)
            if let Some(cache) = &state.cache {
                if let Ok(mut cache) = cache.try_lock() {
                    if let Some(cached) = cache.get::<QuoteResponse>(&quote_cache_key).await {
                        state.cache_metrics.inc_quote_hit();
                        crate::metrics::record_cache_hit("quote");
                        tracing::Span::current().record("cache_hit", true);
                        debug!("Returning cached quote for {}/{}", base, quote);
                        // SingleFlight expects Arc<Result<(QuoteResponse, bool)>>
                        return Arc::new(Ok((cached, true)));
                    }
                }
            }

            // Cache miss
            crate::metrics::record_cache_miss("quote");

            // Compute best price with freshness scoring
            let compute_res =
                find_best_price(&state, &base_asset, &quote_asset, base_id, quote_id, amount).await;

            let (
                price,
                path,
                rationale,
                api_diagnostics,
                freshness_outcome,
                fresh_timestamps,
                liquidity_snapshot,
            ) = match compute_res {
                Ok(res) => res,
                Err(e) => return Arc::new(Err(e)),
            };

            // Increment stale inputs metrics
            let stale_count = freshness_outcome.stale.len();
            if stale_count > 0 {
                state
                    .cache_metrics
                    .add_stale_inputs_excluded(stale_count as u64);
            }

            let total = amount * price;
            let timestamp = chrono::Utc::now().timestamp_millis();
            let ttl_seconds = u32::try_from(state.cache_policy.quote_ttl.as_secs()).ok();
            let expires_at = i64::try_from(state.cache_policy.quote_ttl.as_millis())
                .ok()
                .map(|ttl_ms| timestamp + ttl_ms);

            let source_timestamp = fresh_timestamps
                .iter()
                .min()
                .map(|ts| ts.timestamp_millis());

            let data_freshness = Some(crate::models::DataFreshness {
                fresh_count: freshness_outcome.fresh.len(),
                stale_count: freshness_outcome.stale.len(),
                max_staleness_secs: freshness_outcome.max_staleness_secs,
            });

            let response = QuoteResponse {
                base_asset: asset_path_to_info(&base_asset),
                quote_asset: asset_path_to_info(&quote_asset),
                amount: format!("{:.7}", amount),
                price: format!("{:.7}", price),
                total: format!("{:.7}", total),
                quote_type: quote_type_str.to_string(),
                path,
                timestamp,
                expires_at,
                source_timestamp,
                ttl_seconds,
                rationale: Some(rationale),
                exclusion_diagnostics: Some(api_diagnostics),
                data_freshness,
                price_impact: None, // Will be computed in Phase 3
            };

            // Cache the response
            if let Some(cache) = &state.cache {
                if let Ok(mut cache) = cache.try_lock() {
                    let _ = cache
                        .set(&quote_cache_key, &response, state.cache_policy.quote_ttl)
                        .await;
                }
            }

            // [Replay] Non-blocking capture — fire-and-forget, zero latency impact
            if let Some(hook) = &state.replay_capture {
                use stellarroute_routing::health::scorer::HealthScoringConfig;
                let hc = HealthScoringConfig::default();
                let health_config = crate::replay::artifact::HealthConfigSnapshot {
                    freshness_threshold_secs_sdex: hc.freshness_threshold_secs.sdex,
                    freshness_threshold_secs_amm: hc.freshness_threshold_secs.amm,
                    staleness_threshold_secs: hc.staleness_threshold_secs,
                    min_tvl_threshold_e7: hc.min_tvl_threshold_e7,
                };
                hook.capture(
                    &base,
                    &quote,
                    &format!("{:.7}", amount),
                    slippage_bps,
                    quote_type_str,
                    liquidity_snapshot,
                    health_config,
                    &response,
                    None,
                );
            }

            Arc::new(Ok((response, false)))
        })
        .await;

    match Arc::try_unwrap(result_arc) {
        Ok(res) => res,
        Err(arc_res) => arc_res.as_ref().clone(),
    }
}

/// Get routing path for a trading pair
///
/// Returns only the optimal execution path without detailed pricing
#[utoipa::path(
    get,
    path = "/api/v1/route/{base}/{quote}",
    tag = "trading",
    params(
        ("base" = String, Path, description = "Base asset (e.g., 'native', 'USDC', or 'USDC:ISSUER')"),
        ("quote" = String, Path, description = "Quote asset (e.g., 'native', 'USDC', or 'USDC:ISSUER')"),
        ("amount" = Option<String>, Query, description = "Amount to trade (default: 1)"),
        ("slippage_bps" = Option<u32>, Query, description = "Slippage tolerance in basis points (default: 50)"),
        ("quote_type" = Option<String>, Query, description = "Type of quote: 'sell' or 'buy' (default: sell)"),
    ),
    responses(
        (status = 200, description = "Trading route", body = crate::models::RouteResponse),
        (status = 400, description = "Invalid parameters", body = ErrorResponse),
        (status = 404, description = "No route found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
)]
pub async fn get_route(
    State(state): State<Arc<AppState>>,
    request: crate::middleware::validation::ValidatedQuoteRequest,
) -> Result<Json<crate::models::RouteResponse>> {
    let ValidatedQuoteRequest {
        base: base_asset,
        quote: quote_asset,
        params,
    } = request;

    debug!(
        "Getting route for {}/{} with params: {:?}",
        base_asset.asset_code, quote_asset.asset_code, params
    );

    // Parse amount (default to 1)
    let amount: f64 = params
        .amount
        .as_deref()
        .unwrap_or("1")
        .parse()
        .unwrap_or(1.0); // Already validated in extractor

    let slippage_bps = params.slippage_bps();

    let base_id = find_asset_id(&state, &base_asset).await?;
    let quote_id = find_asset_id(&state, &quote_asset).await?;

    // For route endpoint, we reuse the same logic but return a simplified response
    let (_, path, _, _, _, _, _) =
        find_best_price(&state, &base_asset, &quote_asset, base_id, quote_id, amount).await?;

    let response = crate::models::RouteResponse {
        base_asset: asset_path_to_info(&base_asset),
        quote_asset: asset_path_to_info(&quote_asset),
        amount: format!("{:.7}", amount),
        path,
        slippage_bps,
        timestamp: chrono::Utc::now().timestamp_millis(),
    };

    Ok(Json(response))
}

/// Find best price for a trading pair
type FindBestPriceResult = (
    f64,
    Vec<PathStep>,
    QuoteRationaleMetadata,
    ApiExclusionDiagnostics,
    FreshnessOutcome,
    Vec<chrono::DateTime<chrono::Utc>>,
    Vec<crate::replay::artifact::LiquidityCandidate>, // snapshot for replay capture
);

#[tracing::instrument(
    name = "find_best_price",
    skip(state, base_id, quote_id),
    fields(
        candidates_count = tracing::field::Empty,
        stale_count = tracing::field::Empty,
        fresh_count = tracing::field::Empty,
        scored_count = tracing::field::Empty
    )
)]
async fn find_best_price(
    state: &AppState,
    base: &AssetPath,
    quote: &AssetPath,
    base_id: uuid::Uuid,
    quote_id: uuid::Uuid,
    amount: f64,
) -> Result<FindBestPriceResult> {
    // Parallel multi-source quote computation
    let sdex_timeout = Duration::from_millis(500);
    let amm_timeout = Duration::from_millis(500);

    let sdex_task = fetch_source_candidates(state, base_id, quote_id, "sdex");
    let amm_task = fetch_source_candidates(state, base_id, quote_id, "amm");

    let (sdex_res, amm_res) = tokio::join!(
        timeout(sdex_timeout, sdex_task),
        timeout(amm_timeout, amm_task)
    );

    let mut candidates = Vec::new();

    match sdex_res {
        Ok(Ok(mut res)) => candidates.append(&mut res),
        Ok(Err(e)) => warn!("SDEX source error: {:?}", e),
        Err(_) => warn!("SDEX source timed out"),
    }

    match amm_res {
        Ok(Ok(mut res)) => candidates.append(&mut res),
        Ok(Err(e)) => warn!("AMM source error: {:?}", e),
        Err(_) => warn!("AMM source timed out"),
    }

    // Deterministic merge: sort by price, then venue type, then ref
    candidates.sort_by(|a, b| {
        a.price
            .partial_cmp(&b.price)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.venue_type.cmp(&b.venue_type))
            .then_with(|| a.venue_ref.cmp(&b.venue_ref))
    });

    // Capture a single wall-clock instant for both scorer_inputs construction and freshness eval
    let now = chrono::Utc::now();

    // Build VenueScorerInput from candidates
    let scorer_inputs: Vec<VenueScorerInput> = candidates
        .iter()
        .map(|c| {
            if c.venue_type == "amm" {
                VenueScorerInput {
                    venue_ref: c.venue_ref.clone(),
                    venue_type: VenueType::Amm,
                    best_bid_e7: None,
                    best_ask_e7: None,
                    depth_top_n_e7: None,
                    reserve_a_e7: Some(c.available_amount_e7 as i128),
                    reserve_b_e7: Some(c.available_amount_e7 as i128),
                    tvl_e7: Some((c.available_amount_e7 * 2) as i128),
                    last_updated_at: Some(now),
                }
            } else {
                VenueScorerInput {
                    venue_ref: c.venue_ref.clone(),
                    venue_type: VenueType::Sdex,
                    best_bid_e7: None,
                    best_ask_e7: Some(c.price_e7 as i128),
                    depth_top_n_e7: Some(c.available_amount_e7 as i128),
                    reserve_a_e7: None,
                    reserve_b_e7: None,
                    tvl_e7: None,
                    last_updated_at: Some(now),
                }
            }
        })
        .collect();

    // Health scoring / exclusion policy (defaults match routing `HealthScoringConfig`)
    let health_config = HealthScoringConfig::default();
    let freshness_outcome =
        FreshnessGuard::evaluate(&scorer_inputs, &health_config.freshness_threshold_secs, now);

    tracing::Span::current().record("stale_count", freshness_outcome.stale.len());
    tracing::Span::current().record("fresh_count", freshness_outcome.fresh.len());

    if freshness_outcome.fresh.is_empty() {
        state.cache_metrics.inc_stale_rejection();
        return Err(ApiError::StaleMarketData {
            stale_count: freshness_outcome.stale.len(),
            fresh_count: 0,
            threshold_secs_sdex: health_config.freshness_threshold_secs.sdex,
            threshold_secs_amm: health_config.freshness_threshold_secs.amm,
        });
    }

    let fresh_candidates: Vec<DirectVenueCandidate> = freshness_outcome
        .fresh
        .iter()
        .filter_map(|&idx| candidates.get(idx).cloned())
        .collect();
    let fresh_scorer_inputs: Vec<&VenueScorerInput> = freshness_outcome
        .fresh
        .iter()
        .filter_map(|&idx| scorer_inputs.get(idx))
        .collect();
    let mut stale_exclusion_entries: Vec<ApiExcludedVenueInfo> = freshness_outcome
        .stale
        .iter()
        .filter_map(|&idx| candidates.get(idx))
        .map(|candidate| ApiExcludedVenueInfo {
            venue_ref: candidate.venue_ref.clone(),
            reason: ApiExclusionReason::StaleData,
        })
        .collect();

    let scorer = HealthScorer {
        sdex: SdexScorer {
            staleness_threshold_secs: health_config.staleness_threshold_secs,
            max_spread: 0.05,
            target_depth_e7: 10_000_000_000,
            depth_levels: health_config.depth_levels,
        },
        amm: AmmScorer {
            staleness_threshold_secs: health_config.staleness_threshold_secs,
            min_tvl_threshold_e7: health_config.min_tvl_threshold_e7,
        },
    };

    // Score only fresh candidates (Req 6.4)
    let fresh_inputs_owned: Vec<VenueScorerInput> = fresh_scorer_inputs
        .iter()
        .map(|&input| VenueScorerInput {
            venue_ref: input.venue_ref.clone(),
            venue_type: input.venue_type.clone(),
            best_bid_e7: input.best_bid_e7,
            best_ask_e7: input.best_ask_e7,
            depth_top_n_e7: input.depth_top_n_e7,
            reserve_a_e7: input.reserve_a_e7,
            reserve_b_e7: input.reserve_b_e7,
            tvl_e7: input.tvl_e7,
            last_updated_at: input.last_updated_at,
        })
        .collect();
    let scored = scorer.score_venues(&fresh_inputs_owned);

    let mut overrides = state.kill_switch.get_override_registry().await;
    // Merge static config overrides into dynamic ones
    for entry in health_config.overrides.clone() {
        overrides.venue_entries.insert(entry.venue_ref, entry.directive);
    }

    let policy = ExclusionPolicy {
        thresholds: health_config.thresholds.clone(),
        overrides,
        circuit_breaker: Some(state.circuit_breaker.clone()),
    };

    // Apply filter (pass empty edges — we just need diagnostics for this single-hop path)
    let filter = GraphFilter::new(&policy);
    let (_, routing_diagnostics) = filter.filter_edges(&[], &scored);

    tracing::info!(
        stage = "policy_filter",
        excluded = routing_diagnostics.excluded_venues.len(),
        "Applied policy and threshold filters"
    );

    // Convert routing diagnostics to API types, then prepend stale exclusions (Req 6.2)
    let mut health_exclusion_entries: Vec<ApiExcludedVenueInfo> = routing_diagnostics
        .excluded_venues
        .iter()
        .map(|v| ApiExcludedVenueInfo {
            venue_ref: v.venue_ref.clone(),
            reason: match &v.reason {
                stellarroute_routing::health::policy::ExclusionReason::PolicyThreshold {
                    threshold,
                } => ApiExclusionReason::PolicyThreshold {
                    threshold: *threshold,
                },
                stellarroute_routing::health::policy::ExclusionReason::Override => {
                    ApiExclusionReason::Override
                }
                stellarroute_routing::health::policy::ExclusionReason::StaleData => {
                    ApiExclusionReason::StaleData
                }
                stellarroute_routing::health::policy::ExclusionReason::CircuitBreakerOpen => {
                    ApiExclusionReason::CircuitBreakerOpen
                }
            },
        })
        .collect();

    stale_exclusion_entries.append(&mut health_exclusion_entries);
    let api_diagnostics = ApiExclusionDiagnostics {
        excluded_venues: stale_exclusion_entries,
    };

    // Pass only fresh candidates to price evaluation (Req 2.2, 6.1)
    let (selected, rationale) = evaluate_single_hop_direct_venues(fresh_candidates, amount)?;

    // Collect last_updated_at timestamps for fresh scorer inputs (for source_timestamp, Req 3.1)
    let fresh_timestamps: Vec<chrono::DateTime<chrono::Utc>> = freshness_outcome
        .fresh
        .iter()
        .filter_map(|&idx| scorer_inputs[idx].last_updated_at)
        .collect();

    // Build liquidity snapshot for replay capture (all candidates, not just fresh)
    let liquidity_snapshot: Vec<crate::replay::artifact::LiquidityCandidate> = candidates
        .iter()
        .map(|c| crate::replay::artifact::LiquidityCandidate {
            venue_type: c.venue_type.clone(),
            venue_ref: c.venue_ref.clone(),
            price: format!("{:.7}", c.price),
            available_amount: format!("{:.7}", c.available_amount),
        })
        .collect();

    let path = vec![PathStep {
        from_asset: asset_path_to_info(base),
        to_asset: asset_path_to_info(quote),
        price: format!("{:.7}", selected.price),
        source: selected.path_source(),
    }];

    Ok((
        selected.price,
        path,
        rationale,
        api_diagnostics,
        freshness_outcome,
        fresh_timestamps,
        liquidity_snapshot,
    ))
}

#[derive(Debug, Clone)]
struct DirectVenueCandidate {
    venue_type: String,
    venue_ref: String,
    price: f64,
    available_amount: f64,
    price_e7: i64,
    available_amount_e7: i64,
}

impl DirectVenueCandidate {
    fn comparison_source(&self) -> String {
        format!("{}:{}", self.venue_type, self.venue_ref)
    }

    fn path_source(&self) -> String {
        if self.venue_type == "amm" {
            format!("amm:{}", self.venue_ref)
        } else {
            "sdex".to_string()
        }
    }
}

fn evaluate_single_hop_direct_venues(
    mut candidates: Vec<DirectVenueCandidate>,
    amount: f64,
) -> Result<(DirectVenueCandidate, QuoteRationaleMetadata)> {
    if candidates.is_empty() {
        return Err(ApiError::NoRouteFound);
    }

    candidates.sort_by(|a, b| {
        a.price
            .partial_cmp(&b.price)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.venue_type.cmp(&b.venue_type))
            .then_with(|| a.venue_ref.cmp(&b.venue_ref))
    });

    let compared_venues = candidates
        .iter()
        .map(|candidate| VenueEvaluation {
            source: candidate.comparison_source(),
            price: format!("{:.7}", candidate.price),
            available_amount: format!("{:.7}", candidate.available_amount),
            executable: candidate.available_amount >= amount && candidate.price > 0.0,
        })
        .collect::<Vec<_>>();

    let selected = candidates
        .iter()
        .find(|candidate| candidate.available_amount >= amount && candidate.price > 0.0)
        .cloned()
        .ok_or(ApiError::NoRouteFound)?;

    Ok((
        selected.clone(),
        QuoteRationaleMetadata {
            strategy: "single_hop_direct_venue_comparison".to_string(),
            selected_source: selected.comparison_source(),
            compared_venues,
        },
    ))
}

async fn maybe_invalidate_quote_cache(
    state: &AppState,
    base: &str,
    quote: &str,
    base_id: uuid::Uuid,
    quote_id: uuid::Uuid,
) -> Result<()> {
    let liquidity_revision = get_liquidity_revision(state, base_id, quote_id).await?;

    if let Some(cache) = &state.cache {
        if let Ok(mut cache) = cache.try_lock() {
            let revision_key = cache::keys::liquidity_revision(base, quote);
            let cached_revision = cache.get::<String>(&revision_key).await;

            if cached_revision.as_deref() != Some(liquidity_revision.as_str()) {
                if cached_revision.is_some() {
                    let pattern = cache::keys::quote_pair_pattern(base, quote);
                    let deleted = cache.delete_by_pattern(&pattern).await.unwrap_or(0);
                    debug!(
                        "Liquidity revision changed for {}/{}; invalidated {} quote cache keys",
                        base, quote, deleted
                    );
                }

                let _ = cache
                    .set(
                        &revision_key,
                        &liquidity_revision,
                        std::time::Duration::from_secs(3600),
                    )
                    .await;
            }
        }
    }

    Ok(())
}

/// Fetch candidates from a specific source
async fn fetch_source_candidates(
    state: &AppState,
    base_id: uuid::Uuid,
    quote_id: uuid::Uuid,
    venue_type: &str,
) -> Result<Vec<DirectVenueCandidate>> {
    let rows = sqlx::query(
        r#"
                select
                    venue_type,
                    venue_ref,
                    price::text as price,
                    available_amount::text as available_amount,
                    price_e7,
                    available_amount_e7
                from normalized_liquidity
        where selling_asset_id = $1
          and buying_asset_id = $2
          and venue_type = $3
        "#,
    )
    .bind(base_id)
    .bind(quote_id)
    .bind(venue_type)
    .fetch_all(state.db.read_pool())
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let venue_type: String = row.get("venue_type");
            let venue_ref: String = row.get("venue_ref");
            let price: f64 = row.get::<String, _>("price").parse().unwrap_or(0.0);
            let available_amount: f64 = row
                .get::<String, _>("available_amount")
                .parse()
                .unwrap_or(0.0);
            let price_e7: i64 = row.get("price_e7");
            let available_amount_e7: i64 = row.get("available_amount_e7");
            DirectVenueCandidate {
                venue_type,
                venue_ref,
                price,
                available_amount,
                price_e7,
                available_amount_e7,
            }
        })
        .collect())
}

async fn get_liquidity_revision(
    state: &AppState,
    base_id: uuid::Uuid,
    quote_id: uuid::Uuid,
) -> Result<String> {
    let row = sqlx::query(
        r#"
        select coalesce(max(source_ledger), 0)::bigint as revision
        from normalized_liquidity
        where (selling_asset_id = $1 and buying_asset_id = $2)
           or (selling_asset_id = $2 and buying_asset_id = $1)
        "#,
    )
    .bind(base_id)
    .bind(quote_id)
    .fetch_one(state.db.read_pool())
    .await?;

    let revision: i64 = row.get("revision");
    Ok(revision.to_string())
}

/// Find asset ID in database
async fn find_asset_id(state: &AppState, asset: &AssetPath) -> Result<uuid::Uuid> {
    use sqlx::Row;

    let asset_type = asset.to_asset_type();

    let row = if asset.asset_code == "native" {
        sqlx::query(
            r#"
            select id from assets
            where asset_type = $1
            limit 1
            "#,
        )
        .bind(&asset_type)
        .fetch_optional(state.db.read_pool())
        .await?
    } else {
        sqlx::query(
            r#"
            select id from assets
            where asset_type = $1
              and asset_code = $2
              and ($3::text is null or asset_issuer = $3)
            limit 1
            "#,
        )
        .bind(&asset_type)
        .bind(&asset.asset_code)
        .bind(&asset.asset_issuer)
        .fetch_optional(state.db.read_pool())
        .await?
    };

    match row {
        Some(row) => Ok(row.get("id")),
        None => Err(ApiError::NotFound(format!(
            "Asset not found: {}",
            asset.asset_code
        ))),
    }
}

/// Convert AssetPath to AssetInfo
fn asset_path_to_info(asset: &AssetPath) -> AssetInfo {
    if asset.asset_code == "native" {
        AssetInfo::native()
    } else {
        AssetInfo::credit(asset.asset_code.clone(), asset.asset_issuer.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::CacheMetrics;

    fn candidate(
        venue_type: &str,
        venue_ref: &str,
        price: f64,
        available_amount: f64,
    ) -> DirectVenueCandidate {
        DirectVenueCandidate {
            venue_type: venue_type.to_string(),
            venue_ref: venue_ref.to_string(),
            price,
            available_amount,
            price_e7: (price * 1e7) as i64,
            available_amount_e7: (available_amount * 1e7) as i64,
        }
    }

    #[test]
    fn selects_best_executable_direct_venue() {
        let candidates = vec![
            candidate("amm", "pool1", 1.02, 100.0),
            candidate("sdex", "offer2", 1.01, 25.0),
            candidate("sdex", "offer1", 1.00, 75.0),
        ];

        let (selected, rationale) =
            evaluate_single_hop_direct_venues(candidates, 50.0).expect("must select a venue");

        assert_eq!(selected.venue_type, "sdex");
        assert_eq!(selected.venue_ref, "offer1");
        assert_eq!(rationale.selected_source, "sdex:offer1");
        assert_eq!(rationale.compared_venues.len(), 3);
    }

    #[test]
    fn tie_break_is_deterministic_by_venue_then_ref() {
        let candidates = vec![
            candidate("sdex", "offer2", 1.0, 100.0),
            candidate("amm", "pool1", 1.0, 100.0),
            candidate("sdex", "offer1", 1.0, 100.0),
        ];

        let (selected, rationale) =
            evaluate_single_hop_direct_venues(candidates, 10.0).expect("must select a venue");

        assert_eq!(selected.comparison_source(), "amm:pool1");
        assert_eq!(
            rationale
                .compared_venues
                .iter()
                .map(|v| v.source.clone())
                .collect::<Vec<_>>(),
            vec![
                "amm:pool1".to_string(),
                "sdex:offer1".to_string(),
                "sdex:offer2".to_string(),
            ]
        );
    }

    #[test]
    fn insufficient_liquidity_returns_no_route() {
        let candidates = vec![
            candidate("amm", "pool1", 1.0, 5.0),
            candidate("sdex", "offer1", 0.99, 2.0),
        ];

        let result = evaluate_single_hop_direct_venues(candidates, 10.0);
        assert!(matches!(result, Err(ApiError::NoRouteFound)));
    }

    // --- Req 4.1: stale_quote_rejections counter ---

    #[test]
    fn stale_rejection_counter_increments_on_all_stale() {
        let metrics = CacheMetrics::default();
        let (rejections_before, _) = metrics.snapshot_staleness();
        assert_eq!(rejections_before, 0);

        // Simulate what find_best_price does when all inputs are stale
        metrics.inc_stale_rejection();

        let (rejections_after, _) = metrics.snapshot_staleness();
        assert_eq!(rejections_after, 1);
    }

    #[test]
    fn stale_rejection_counter_accumulates_across_calls() {
        let metrics = CacheMetrics::default();
        metrics.inc_stale_rejection();
        metrics.inc_stale_rejection();
        metrics.inc_stale_rejection();

        let (rejections, _) = metrics.snapshot_staleness();
        assert_eq!(rejections, 3);
    }

    // --- Req 4.2: stale_inputs_excluded counter ---

    #[test]
    fn stale_inputs_excluded_counter_increments_by_stale_count() {
        let metrics = CacheMetrics::default();
        let (_, excluded_before) = metrics.snapshot_staleness();
        assert_eq!(excluded_before, 0);

        // Simulate what get_quote does when 2 stale inputs were excluded
        let stale_count: u64 = 2;
        metrics.add_stale_inputs_excluded(stale_count);

        let (_, excluded_after) = metrics.snapshot_staleness();
        assert_eq!(excluded_after, 2);
    }

    #[test]
    fn stale_inputs_excluded_counter_accumulates_across_quotes() {
        let metrics = CacheMetrics::default();

        // First quote excludes 1 stale input
        metrics.add_stale_inputs_excluded(1);
        // Second quote excludes 3 stale inputs
        metrics.add_stale_inputs_excluded(3);

        let (_, excluded) = metrics.snapshot_staleness();
        assert_eq!(excluded, 4);
    }

    #[test]
    fn stale_inputs_excluded_not_incremented_when_all_fresh() {
        let metrics = CacheMetrics::default();

        // Simulate get_quote with stale_count == 0 (no increment should happen)
        let stale_count = 0usize;
        if stale_count > 0 {
            metrics.add_stale_inputs_excluded(stale_count as u64);
        }

        let (_, excluded) = metrics.snapshot_staleness();
        assert_eq!(excluded, 0);
    }

    #[test]
    fn rejection_and_excluded_counters_are_independent() {
        let metrics = CacheMetrics::default();

        metrics.inc_stale_rejection();
        metrics.add_stale_inputs_excluded(5);

        let (rejections, excluded) = metrics.snapshot_staleness();
        assert_eq!(rejections, 1);
        assert_eq!(excluded, 5);
    }

    // --- Req 6.3: mixed-freshness — NoRouteFound when fresh candidates lack liquidity ---

    /// When there is one fresh candidate with insufficient liquidity and one stale candidate
    /// (already excluded before reaching evaluate_single_hop_direct_venues), the result must be
    /// ApiError::NoRouteFound, not ApiError::StaleMarketData.
    #[test]
    fn mixed_freshness_insufficient_liquidity_returns_no_route() {
        // The stale candidate has been excluded by freshness filtering before this call.
        // Only the fresh-but-low-liquidity candidate reaches evaluate_single_hop_direct_venues.
        let fresh_candidates = vec![
            candidate("sdex", "offer_fresh", 1.0, 5.0), // fresh but only 5 units available
        ];
        // Request 100 units — exceeds the fresh candidate's available_amount.
        let result = evaluate_single_hop_direct_venues(fresh_candidates, 100.0);

        // Must be NoRouteFound, not StaleMarketData.
        assert!(
            matches!(result, Err(ApiError::NoRouteFound)),
            "expected NoRouteFound but got: {:?}",
            result
        );
    }

    // --- Req 2.2 / 6.1: mixed-freshness happy path ---

    /// When stale candidates have been excluded upstream by FreshnessGuard and the remaining
    /// fresh candidates have sufficient liquidity, evaluate_single_hop_direct_venues succeeds
    /// and selects the best-priced fresh candidate.
    #[test]
    fn mixed_freshness_with_sufficient_fresh_liquidity_succeeds() {
        // Stale candidate already filtered out; only these fresh candidates remain.
        let fresh_candidates = vec![
            candidate("amm", "pool_fresh", 1.05, 200.0),
            candidate("sdex", "offer_fresh", 1.02, 150.0),
        ];
        let amount = 100.0;

        let (selected, rationale) = evaluate_single_hop_direct_venues(fresh_candidates, amount)
            .expect("must select a venue when fresh candidates have sufficient liquidity");

        // Best price (lowest) with sufficient liquidity is selected.
        assert_eq!(
            selected.venue_ref, "offer_fresh",
            "sdex offer should win on price"
        );
        assert_eq!(selected.venue_type, "sdex");
        assert_eq!(rationale.strategy, "single_hop_direct_venue_comparison");
        assert_eq!(rationale.compared_venues.len(), 2);
    }

    // --- Req 3.2 / 3.3: data_freshness fields map from FreshnessOutcome ---

    /// Verifies that the DataFreshness struct is populated with correct counts and max staleness
    /// from a FreshnessOutcome — mirrors the exact mapping performed in get_quote().
    #[test]
    fn data_freshness_fields_map_from_freshness_outcome() {
        use stellarroute_routing::health::freshness::FreshnessOutcome;

        // Simulate FreshnessOutcome: indices 0,2 are fresh; index 1 is stale; max staleness 45s.
        let outcome = FreshnessOutcome {
            fresh: vec![0, 2],
            stale: vec![1],
            max_staleness_secs: 45,
        };

        let data_freshness = crate::models::DataFreshness {
            fresh_count: outcome.fresh.len(),
            stale_count: outcome.stale.len(),
            max_staleness_secs: outcome.max_staleness_secs,
        };

        assert_eq!(
            data_freshness.fresh_count, 2,
            "fresh_count must match fresh indices"
        );
        assert_eq!(
            data_freshness.stale_count, 1,
            "stale_count must match stale indices"
        );
        assert_eq!(data_freshness.max_staleness_secs, 45);
    }

    /// All-fresh FreshnessOutcome produces DataFreshness with stale_count == 0.
    #[test]
    fn data_freshness_stale_count_zero_when_all_inputs_are_fresh() {
        use stellarroute_routing::health::freshness::FreshnessOutcome;

        let outcome = FreshnessOutcome {
            fresh: vec![0, 1, 2],
            stale: vec![],
            max_staleness_secs: 12,
        };

        let data_freshness = crate::models::DataFreshness {
            fresh_count: outcome.fresh.len(),
            stale_count: outcome.stale.len(),
            max_staleness_secs: outcome.max_staleness_secs,
        };

        assert_eq!(
            data_freshness.stale_count, 0,
            "stale_count must be zero when all inputs are fresh"
        );
        assert_eq!(data_freshness.fresh_count, 3);
    }

    /// Multiple stale FreshnessOutcome produces DataFreshness with matching stale_count.
    #[test]
    fn data_freshness_stale_count_matches_number_of_stale_inputs() {
        use stellarroute_routing::health::freshness::FreshnessOutcome;

        let outcome = FreshnessOutcome {
            fresh: vec![2],
            stale: vec![0, 1, 3, 4],
            max_staleness_secs: 300,
        };

        let data_freshness = crate::models::DataFreshness {
            fresh_count: outcome.fresh.len(),
            stale_count: outcome.stale.len(),
            max_staleness_secs: outcome.max_staleness_secs,
        };

        assert_eq!(
            data_freshness.stale_count, 4,
            "stale_count must track all stale indices"
        );
        assert_eq!(data_freshness.fresh_count, 1);
        assert_eq!(data_freshness.max_staleness_secs, 300);
    }
    #[tokio::test]
    async fn test_parallel_execution_latency() {
        use std::time::{Duration, Instant};
        use tokio::time::sleep;

        async fn simulated_source(delay_ms: u64) -> Result<Vec<u32>> {
            sleep(Duration::from_millis(delay_ms)).await;
            Ok(vec![1, 2, 3])
        }

        let delay = 100;

        // Sequential
        let start = Instant::now();
        let _ = (simulated_source(delay).await, simulated_source(delay).await);
        let seq_duration = start.elapsed();

        // Parallel
        let start = Instant::now();
        let _ = tokio::join!(simulated_source(delay), simulated_source(delay));
        let par_duration = start.elapsed();

        println!(
            "Sequential: {:?}, Parallel: {:?}",
            seq_duration, par_duration
        );
        assert!(par_duration < seq_duration);
        assert!(par_duration >= Duration::from_millis(delay));
        assert!(par_duration < Duration::from_millis(delay * 2));
    }
}
