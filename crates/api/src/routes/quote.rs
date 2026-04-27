//! Quote endpoint

use axum::{extract::State, Json};
use sqlx::Row;
use std::sync::Arc;
use tracing::{debug, info_span, Instrument};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, info_span, warn, Instrument};

use stellarroute_routing::health::filter::GraphFilter;
use stellarroute_routing::health::freshness::{FreshnessGuard, FreshnessOutcome};
use stellarroute_routing::health::policy::ExclusionPolicy;
use stellarroute_routing::health::scorer::{
    AmmScorer, HealthScorer, HealthScoringConfig, SdexScorer, VenueScorerInput, VenueType,
};

use crate::{
    audit::{AuditExclusion, AuditInputs, AuditOutcome, AuditPathStep, AuditSelected},
    cache,
    error::{ApiError, Result},
    middleware::{validation::ValidatedQuoteRequest, RequestId},
    models::{
        request::{AssetPath, QuoteParams},
        AssetInfo, PathStep, QuoteRationaleMetadata, QuoteResponse, VenueEvaluation,
        ExclusionDiagnostics as ApiExclusionDiagnostics,
        ExcludedVenueInfo as ApiExcludedVenueInfo,
        ExclusionReason as ApiExclusionReason,
    },
    state::AppState,
};

pub async fn get_quote(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    request: ValidatedQuoteRequest,
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
        match get_quote_inner(
            state.clone(),
            base_asset.clone(),
            quote_asset.clone(),
            params.clone(),
            explain,
        )
        .await
        {
            Ok((quote_resp, cache_hit)) => {
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

                // ── Audit log ────────────────────────────────────────────
                let trace_id = crate::tracing_config::TraceContext::current().trace_id;
                let audit_inputs = AuditInputs {
                    base: base.clone(),
                    quote: quote.clone(),
                    amount: quote_resp.amount.clone(),
                    slippage_bps: params.slippage_bps(),
                    quote_type: quote_resp.quote_type.clone(),
                };
                let audit_selected = build_audit_selected(&quote_resp);
                let audit_exclusions = build_audit_exclusions(&quote_resp);
                state.audit_writer.emit(
                    request_id.as_str(),
                    &trace_id,
                    latency_ms,
                    AuditOutcome::Success,
                    cache_hit,
                    audit_inputs,
                    Some(audit_selected),
                    audit_exclusions,
                );

                let envelope = crate::models::ApiResponse::new(quote_resp, request_id.to_string());
                Ok(Json(envelope))
            }
            Err(e) => {
                let (error_class, audit_outcome) = match &e {
                    ApiError::Validation(_) | ApiError::InvalidAsset(_) => {
                        ("validation", AuditOutcome::Error)
                    }
                    ApiError::NotFound(_) | ApiError::NoRouteFound => {
                        ("not_found", AuditOutcome::NoRoute)
                    }
                    ApiError::StaleMarketData { .. } => {
                        ("stale_market_data", AuditOutcome::StaleData)
                    }
                    _ => ("internal", AuditOutcome::Error),
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

                // ── Audit log ────────────────────────────────────────────
                let trace_id = crate::tracing_config::TraceContext::current().trace_id;
                let amount_str = params.amount.as_deref().unwrap_or("1").to_string();
                let quote_type_str = match params.quote_type {
                    crate::models::request::QuoteType::Sell => "sell",
                    crate::models::request::QuoteType::Buy => "buy",
                };
                let audit_inputs = AuditInputs {
                    base: base.clone(),
                    quote: quote.clone(),
                    amount: amount_str,
                    slippage_bps: params.slippage_bps(),
                    quote_type: quote_type_str.to_string(),
                };
                state.audit_writer.emit(
                    request_id.as_str(),
                    &trace_id,
                    latency_ms,
                    audit_outcome,
                    false,
                    audit_inputs,
                    None,
                    vec![],
                );

                Err(e)
            }
        }
    }
    .instrument(span)
    .await
}

async fn get_quote_inner(
    state: Arc<AppState>,
    base_asset: AssetPath,
    quote_asset: AssetPath,
    params: QuoteParams,
    explain: bool,
) -> Result<(PreparedQuoteResponse, bool)> {
    let base = base_asset.to_canonical();
    let quote = quote_asset.to_canonical();

    let amount: f64 = params
        .amount
        .as_deref()
        .unwrap_or("1")
        .parse()
        .unwrap_or(1.0);

    let slippage_bps = params.slippage_bps();

    let quote_type_str = match params.quote_type {
        crate::models::request::QuoteType::Sell => "sell",
        crate::models::request::QuoteType::Buy => "buy",
    };

    // IMPORTANT FIX: pass &state (not Arc)
    let base_id = find_asset_id(&state, &base_asset).await?;
    let quote_id = find_asset_id(&state, &quote_asset).await?;

    maybe_invalidate_quote_cache(&state, &base, &quote, base_id, quote_id).await?;

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
    let base_asset_c = base_asset.clone();
    let quote_asset_c = quote_asset.clone();
    let quote_cache_key_c = quote_cache_key.clone();

    let result_arc = state
        .quote_single_flight
        .execute(&quote_cache_key, || async move {
            let state = state_c;

            if let Some(cache) = &state.cache {
                if let Ok(mut cache) = cache.try_lock() {
                    if let Some(cached) = cache.get::<QuoteResponse>(&quote_cache_key_c).await {
                        state.cache_metrics.inc_quote_hit();
                        return Arc::new(Ok(cached));
                    }
                }
            }

            let compute_res = find_best_price(
                &state,
                &base_asset_c,
                &quote_asset_c,
                base_id,
                quote_id,
                amount,
            )
            .await;

            let (
                price,
                path,
                rationale,
                api_diagnostics,
                freshness_outcome,
                _fresh_timestamps,
                _liquidity_snapshot,
            ) = match compute_res {
                Ok(res) => res,
                Err(e) => return Arc::new(Err(e)),
            };

            let stale_count = freshness_outcome.stale.len();
            if stale_count > 0 {
                state
                    .cache_metrics
                    .add_stale_inputs_excluded(stale_count as u64);
            }

            let total = amount * price;
            let timestamp = chrono::Utc::now().timestamp_millis();

            let response = QuoteResponse {
                base_asset: asset_path_to_info(&base_asset_c),
                quote_asset: asset_path_to_info(&quote_asset_c),
                amount: format!("{:.7}", amount),
                price: format!("{:.7}", price),
                total: format!("{:.7}", total),
                quote_type: quote_type_str.to_string(),
                path,
                timestamp,
                expires_at: None,
                source_timestamp: None,
                ttl_seconds: None,
                rationale: Some(rationale),
                exclusion_diagnostics: Some(api_diagnostics),
                data_freshness: Some(crate::models::DataFreshness {
                    fresh_count: freshness_outcome.fresh.len(),
                    stale_count,
                    max_staleness_secs: freshness_outcome.max_staleness_secs,
                }),
                price_impact: None,
            };

            if let Some(cache) = &state.cache {
                if let Ok(mut cache) = cache.try_lock() {
                    let jitter = crate::cache::JitteredTtl::default();
                    let jittered_ttl = jitter.apply(state.cache_policy.quote_ttl);
                    let _ = cache
                        .set(&quote_cache_key_c, &response, state.cache_policy.quote_ttl)
                        .await;
                }
            }

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
                    &amount_str,
                    slippage_bps,
                    quote_type_str,
                    vec![], // snapshot placeholder (already built elsewhere if needed)
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
