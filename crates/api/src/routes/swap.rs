//! Swap prepare/submit HTTP handlers (classic PathPaymentStrictSend only).

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use chrono::{Duration, Utc};
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

use stellarroute_routing::health::policy::OverrideDirective;

use crate::{
    audit::{AuditRedactor, SwapSubmitOutcome},
    broadcast::{BroadcastError, BroadcastResult},
    error::{ApiError, Result},
    metrics::{record_swap_prepare, record_swap_submit, swap_inflight_dec, swap_inflight_inc},
    models::{
        request::SwapPrepareRequest, ApiResponse, SwapPrepareResponse, SwapSubmitRequest,
        SwapSubmitResponse,
    },
    state::AppState,
    swap::price::{resolve_min_output, AuthoritativePrice, LiveQuotePriceSource, SwapPriceSource},
    swap::route::validate_classic_route,
    swap::store::{
        hash_xdr, ClaimSubmitOutcome, PreparedSwapQuote, SubmissionStatus, SwapStoreError,
    },
    swap::tx::{
        base_fee_from_env, build_unsigned_swap_tx, network_passphrase_from_env,
        prepare_timeout_secs_from_env, validate_signed_against_prepared, EnvelopeValidationError,
        ExecutionMode, PrepareTxInput, TxBuildError,
    },
    swap::venue::classify_venue,
};

const DEFAULT_PREPARE_TTL_SECS: i64 = 120;
const DEFAULT_SLIPPAGE_BPS: u32 = 50;

// Quote submission status codes
const STATUS_ALREADY_SUBMITTED: &str = "already_submitted";
const STATUS_PERMANENTLY_FAILED: &str = "permanently_failed";
const STATUS_SUBMITTING_WITHOUT_HASH: &str = "submitting_without_hash";
const STATUS_MISSING_NETWORK_PASSPHRASE: &str = "missing_network_passphrase";
const STATUS_ACTIVE_PREPARE_EXISTS: &str = "active_prepare_exists";
const STATUS_INVALID_TRANSITION: &str = "invalid_transition";
const STATUS_BAD_SEQUENCE: &str = "bad_sequence";

/// POST /api/v1/swap/prepare
#[utoipa::path(
    post,
    path = "/api/v1/swap/prepare",
    tag = "swap",
    request_body(content = SwapPrepareRequest, description = "Build an unsigned classic PathPaymentStrictSend"),
    responses(
        (status = 200, description = "Unsigned transaction envelope", body = ApiResponse<SwapPrepareResponse>),
        (status = 400, description = "Validation error", body = crate::models::ErrorResponse),
        (status = 404, description = "Route not executable", body = crate::models::ErrorResponse),
        (status = 409, description = "Active prepare exists for sender", body = crate::models::ErrorResponse),
        (status = 422, description = "Unsupported mode / not executable / stale", body = crate::models::ErrorResponse),
    )
)]
pub async fn prepare_swap(
    State(state): State<Arc<AppState>>,
    request_id: crate::middleware::RequestId,
    Json(body): Json<SwapPrepareRequest>,
) -> Result<impl IntoResponse> {
    swap_inflight_inc("prepare");
    let started = Instant::now();
    let trace_id = String::new();

    let result = prepare_swap_inner(&state, &body).await;

    let elapsed = started.elapsed();
    match &result {
        Ok(_) => record_swap_prepare(elapsed, "none"),
        Err(e) => {
            record_swap_prepare(elapsed, prepare_error_class(e));
            emit_prepare_rejected(
                &state,
                &AuditRedactor::redact_account(body.sender.trim()),
                request_id.as_str(),
                &trace_id,
                started,
                prepare_error_class(e),
            );
        }
    }
    swap_inflight_dec("prepare");

    let prepared = result?;

    emit_prepare_success(
        &state,
        &prepared.quote_id,
        &AuditRedactor::redact_account(body.sender.trim()),
        request_id.as_str(),
        &trace_id,
        started,
        &prepared.expected_output,
        &prepared.min_output,
    );

    Ok((
        StatusCode::OK,
        Json(ApiResponse::new(prepared, request_id.as_str())),
    ))
}

async fn prepare_swap_inner(
    state: &Arc<AppState>,
    body: &SwapPrepareRequest,
) -> Result<SwapPrepareResponse> {
    validate_stellar_account(&body.sender)?;

    let amount: f64 = body
        .amount
        .parse()
        .map_err(|_| ApiError::Validation("amount must be a valid number".to_string()))?;
    if !amount.is_finite() || amount <= 0.0 {
        return Err(ApiError::Validation(
            "amount must be greater than zero".to_string(),
        ));
    }

    let validated = validate_classic_route(&body.route)?;
    reject_paused_venues(state, &validated.hops).await?;

    let slippage_bps = body.slippage_bps.unwrap_or(DEFAULT_SLIPPAGE_BPS);
    if slippage_bps > 10_000 {
        return Err(ApiError::InvalidSlippage(
            "slippage_bps must be <= 10000".to_string(),
        ));
    }

    let priced = authoritative_price(state, &validated, amount).await?;
    let client_min = match &body.min_output {
        Some(raw) => Some(
            raw.parse::<f64>()
                .map_err(|_| ApiError::Validation("min_output must be a valid number".into()))?,
        ),
        None => None,
    };
    let min_output = resolve_min_output(priced.expected_output, slippage_bps, client_min)?;

    let network_passphrase = network_passphrase_from_env();
    let base_fee = base_fee_from_env();
    let timeout_secs = prepare_timeout_secs_from_env();

    let sequence = state
        .account_sequences
        .current_sequence(body.sender.trim())
        .await
        .map_err(map_tx_build_error)?;

    let built = build_unsigned_swap_tx(PrepareTxInput {
        sender: body.sender.trim(),
        validated: &validated,
        amount,
        min_output,
        sequence,
        timeout_secs,
        base_fee,
        network_passphrase: &network_passphrase,
    })
    .map_err(map_tx_build_error)?;

    debug_assert_eq!(built.execution_mode, ExecutionMode::ClassicPathPayment);

    let quote_id = Uuid::new_v4().to_string();
    let expires_at =
        Utc::now() + Duration::seconds(DEFAULT_PREPARE_TTL_SECS.min(timeout_secs as i64));
    let expected_output_str = format!("{:.7}", priced.expected_output);
    let min_output_str = format!("{:.7}", min_output);
    let amount_in_str = format!("{:.7}", amount);

    let _ = state
        .swap_quote_store
        .expire_stale_for_sender(body.sender.trim())
        .await;

    let prepared = PreparedSwapQuote {
        quote_id: quote_id.clone(),
        sender_account: body.sender.trim().to_string(),
        sender_account_hash: AuditRedactor::redact_account(body.sender.trim()),
        unsigned_xdr_hash: built.unsigned_xdr_hash,
        expires_at,
        estimated_output: expected_output_str.clone(),
        min_output: min_output_str.clone(),
        amount_in: amount_in_str,
        execution_mode: built.execution_mode.as_str().to_string(),
        network_passphrase: built.network_passphrase.clone(),
        route_digest: validated.route_digest.clone(),
        price_digest: priced.price_digest,
        source_sequence: Some(built.source_sequence),
        timebounds_max: Some(built.timebounds_max as i64),
        base_fee: Some(base_fee as i32),
        valid_until_ledger: None,
        submission_status: SubmissionStatus::Prepared,
        tx_hash: None,
    };

    state
        .swap_quote_store
        .insert_prepared(&prepared)
        .await
        .map_err(map_store_error)?;

    Ok(SwapPrepareResponse {
        quote_id,
        xdr_envelope: built.xdr_envelope,
        expected_output: expected_output_str,
        min_output: Some(min_output_str),
        expires_at: expires_at.timestamp_millis(),
        execution_mode: ExecutionMode::ClassicPathPayment.as_str().to_string(),
        network_passphrase: built.network_passphrase,
    })
}

async fn authoritative_price(
    state: &Arc<AppState>,
    route: &crate::swap::route::ValidatedClassicRoute,
    amount: f64,
) -> Result<AuthoritativePrice> {
    if let Some(src) = &state.swap_price_source {
        return src.price_swap(route, amount).await;
    }
    LiveQuotePriceSource {
        state: Arc::clone(state),
    }
    .price_swap(route, amount)
    .await
}

async fn reject_paused_venues(
    state: &AppState,
    hops: &[crate::routes::simulation_route::RouteDryRunHop],
) -> Result<()> {
    let registry = state.kill_switch.get_override_registry().await;
    for hop in hops {
        let venue_ref = hop.venue_ref.as_deref().unwrap_or(hop.source.as_str());
        if matches!(
            registry.venue_entries.get(venue_ref),
            Some(OverrideDirective::ForceExclude)
        ) {
            return Err(ApiError::NotExecutable(format!(
                "venue '{venue_ref}' is paused by kill switch"
            )));
        }

        let class = classify_venue(&hop.source, hop.venue_ref.as_deref());
        if let Some(venue_type) = class.to_routing_venue_type() {
            if matches!(
                registry.source_entries.get(&venue_type),
                Some(OverrideDirective::ForceExclude)
            ) {
                return Err(ApiError::NotExecutable(format!(
                    "venue source '{:?}' is paused by kill switch",
                    venue_type
                )));
            }
        }
    }
    Ok(())
}

fn prepare_error_class(err: &ApiError) -> &'static str {
    match err {
        ApiError::Validation(_) | ApiError::InvalidAsset(_) | ApiError::InvalidAmount(_) => {
            "validation"
        }
        ApiError::InvalidSlippage(_) => "validation",
        ApiError::UnsupportedExecutionMode(_) => "unsupported_execution_mode",
        ApiError::UnsupportedRoute(_) => "unsupported_route",
        ApiError::NoRouteFound => "simulation_failed",
        ApiError::NotExecutable(_) => "simulation_failed",
        ApiError::StaleMarketData { .. } => "quote_expired",
        ApiError::DependencyUnavailable(_) => "rpc_error",
        ApiError::Conflict { status, .. } if status == STATUS_ACTIVE_PREPARE_EXISTS => {
            STATUS_ACTIVE_PREPARE_EXISTS
        }
        _ => "internal",
    }
}

/// POST /api/v1/swap/submit
#[utoipa::path(
    post,
    path = "/api/v1/swap/submit",
    tag = "swap",
    request_body(content = SwapSubmitRequest, description = "Broadcast a signed classic PathPaymentStrictSend"),
    responses(
        (status = 200, description = "Transaction included", body = ApiResponse<SwapSubmitResponse>),
        (status = 202, description = "Transaction pending", body = ApiResponse<SwapSubmitResponse>),
        (status = 404, description = "Unknown quote_id", body = crate::models::ErrorResponse),
        (status = 409, description = "Conflict (in progress / already submitted / permanently failed)", body = crate::models::ErrorResponse),
        (status = 422, description = "Quote expired / unsupported", body = crate::models::ErrorResponse),
        (status = 400, description = "Validation / auth error", body = crate::models::ErrorResponse),
    )
)]
pub async fn submit_swap(
    State(state): State<Arc<AppState>>,
    request_id: crate::middleware::RequestId,
    Json(body): Json<SwapSubmitRequest>,
) -> Result<impl IntoResponse> {
    swap_inflight_inc("submit");
    let started = Instant::now();
    let trace_id = String::new();

    let outcome = submit_swap_inner(&state, &body, request_id.as_str(), &trace_id, started).await;
    let elapsed = started.elapsed();

    match &outcome {
        Ok((response, status, sender_hash)) => {
            record_swap_submit(elapsed, "none");
            let event = if response.status == "pending" { "broadcast_pending" } else { "broadcast_success" };
            emit_submit_success(
                &state,
                &body.quote_id,
                &response.tx_hash,
                sender_hash,
                request_id.as_str(),
                &trace_id,
                started,
                &response.status,
                event,
            );
            swap_inflight_dec("submit");
            return Ok((
                *status,
                Json(ApiResponse::new(response.clone(), request_id.as_str())),
            ));
        }
        Err(e) => {
            record_swap_submit(elapsed, submit_error_class(e));
            swap_inflight_dec("submit");
        }
    }

    outcome.map(|(response, status, _)| {
        (
            status,
            Json(ApiResponse::new(response, request_id.as_str())),
        )
    })
}

async fn submit_swap_inner(
    state: &AppState,
    body: &SwapSubmitRequest,
    request_id: &str,
    trace_id: &str,
    started: Instant,
) -> Result<(SwapSubmitResponse, StatusCode, String)> {
    if body.quote_id.trim().is_empty() {
        return Err(ApiError::Validation("quote_id is required".to_string()));
    }
    if body.signed_xdr.trim().is_empty() {
        return Err(ApiError::Validation("signed_xdr is required".to_string()));
    }
    if base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        body.signed_xdr.trim(),
    )
    .is_err()
    {
        return Err(ApiError::Validation(
            "signed_xdr must be valid base64".to_string(),
        ));
    }

    let quote = state
        .swap_quote_store
        .get(body.quote_id.trim())
        .await
        .map_err(map_store_error)?
        .ok_or_else(|| ApiError::QuoteNotFound {
            quote_id: body.quote_id.clone(),
        })?;

    if quote.submission_status == SubmissionStatus::Submitted {
        return Err(ApiError::Conflict {
            message: "Quote has already been submitted".to_string(),
            quote_id: body.quote_id.clone(),
            tx_hash: quote.tx_hash.clone().unwrap_or_default(),
            status: STATUS_ALREADY_SUBMITTED.to_string(),
        });
    }

    if quote.submission_status == SubmissionStatus::Failed {
        return Err(ApiError::Conflict {
            message: "Quote permanently failed; request a fresh prepare".to_string(),
            quote_id: body.quote_id.clone(),
            tx_hash: quote.tx_hash.clone().unwrap_or_default(),
            status: STATUS_PERMANENTLY_FAILED.to_string(),
        });
    }

    // Submitting remains reconcilable past prepare TTL — never TTL-fail it
    // unless the on-chain timebounds window has already closed.
    if quote.submission_status == SubmissionStatus::Submitting {
        let Some(stored_hash) = quote.tx_hash.clone().filter(|h| !h.is_empty()) else {
            if quote_timebounds_expired(&quote) {
                let _ = state
                    .swap_quote_store
                    .mark_failed(body.quote_id.trim())
                    .await;
                return Err(ApiError::Conflict {
                    message: "Quote submission window expired; request a fresh prepare".into(),
                    quote_id: body.quote_id.clone(),
                    tx_hash: String::new(),
                    status: STATUS_PERMANENTLY_FAILED.into(),
                });
            }
            return Err(ApiError::Conflict {
                message: "Quote is submitting without a bound transaction hash; request operator reconciliation or a fresh prepare".into(),
                quote_id: body.quote_id.clone(),
                tx_hash: String::new(),
                status: STATUS_SUBMITTING_WITHOUT_HASH.into(),
            });
        };
        return reconcile_or_rebroadcast(
            state,
            body,
            &quote,
            &stored_hash,
            request_id,
            trace_id,
            started,
        )
        .await;
    }

    // Prepared + expired → permanent fail (submitting handled above).
    if Utc::now() > quote.expires_at {
        let _ = state
            .swap_quote_store
            .mark_failed(body.quote_id.trim())
            .await;
        emit_submit_failed(
            &state,
            &body.quote_id,
            None,
            &quote.sender_account_hash,
            request_id,
            trace_id,
            started,
            "quote_expired",
            "quote_expired",
        );
        return Err(ApiError::QuoteExpired {
            quote_id: body.quote_id.clone(),
        });
    }

    let signed_hash = hash_xdr(body.signed_xdr.trim());
    if signed_hash == quote.unsigned_xdr_hash {
        return Err(ApiError::Validation(
            "signed_xdr must include signatures; unsigned prepare envelope was submitted"
                .to_string(),
        ));
    }

    let tx_hash = match verify_submit_envelope(body, &quote, request_id, trace_id, started, state) {
        Ok(h) => h,
        Err(e) => return Err(e),
    };

    let claimed = state
        .swap_quote_store
        .claim_for_submit(body.quote_id.trim(), &tx_hash)
        .await
        .map_err(|e| map_store_error_for_quote(e, body.quote_id.trim()))?;

    let quote = match claimed {
        ClaimSubmitOutcome::Claimed(quote) => *quote,
        ClaimSubmitOutcome::AlreadySubmitted { tx_hash } => {
            return Err(ApiError::Conflict {
                message: "Quote has already been submitted".to_string(),
                quote_id: body.quote_id.clone(),
                tx_hash,
                status: "already_submitted".to_string(),
            });
        }
        ClaimSubmitOutcome::InProgress(current) => {
            // Race: another request already claimed. Reconcile/rebroadcast with
            // the bound hash instead of hard-409ing the client into a retry loop.
            let Some(stored_hash) = current.tx_hash.clone().filter(|h| !h.is_empty()) else {
                if quote_timebounds_expired(&current) {
                    let _ = state
                        .swap_quote_store
                        .mark_failed(body.quote_id.trim())
                        .await;
                    return Err(ApiError::Conflict {
                        message: "Quote submission window expired; request a fresh prepare".into(),
                        quote_id: body.quote_id.clone(),
                        tx_hash: String::new(),
                        status: STATUS_PERMANENTLY_FAILED.into(),
                    });
                }
                return Err(ApiError::Conflict {
                    message: "Quote is submitting without a bound transaction hash; request operator reconciliation or a fresh prepare".into(),
                    quote_id: body.quote_id.clone(),
                    tx_hash: String::new(),
                    status: STATUS_SUBMITTING_WITHOUT_HASH.into(),
                });
            };
            return reconcile_or_rebroadcast(
                state,
                body,
                &current,
                &stored_hash,
                request_id,
                trace_id,
                started,
            )
            .await;
        }
        ClaimSubmitOutcome::PermanentlyFailed => {
            return Err(ApiError::Conflict {
                message: "Quote permanently failed; request a fresh prepare".to_string(),
                quote_id: body.quote_id.clone(),
                tx_hash: String::new(),
                status: STATUS_PERMANENTLY_FAILED.to_string(),
            });
        }
    };

    broadcast_and_finalize(state, body, &quote, &tx_hash, request_id, trace_id, started).await
}

/// Full cryptographic + body + source + network validation. When `expected` is
/// set, the recomputed hash must match it exactly.
fn verify_submit_envelope(
    body: &SwapSubmitRequest,
    quote: &PreparedSwapQuote,
    request_id: &str,
    trace_id: &str,
    started: Instant,
    state: &AppState,
) -> Result<String> {
    if quote.network_passphrase.trim().is_empty() {
        return Err(ApiError::Conflict {
            message:
                "Prepared quote is missing a bound network passphrase; request a fresh prepare"
                    .into(),
            quote_id: body.quote_id.clone(),
            tx_hash: quote.tx_hash.clone().unwrap_or_default(),
            status: STATUS_MISSING_NETWORK_PASSPHRASE.into(),
        });
    }

    match validate_signed_against_prepared(
        body.signed_xdr.trim(),
        &quote.unsigned_xdr_hash,
        &quote.sender_account,
        &quote.network_passphrase,
    ) {
        Ok(computed) => {
            if let Some(expected) = quote.tx_hash.as_deref() {
                if computed != expected {
                    emit_submit_failed(
                        &state,
                        &body.quote_id,
                        Some(expected),
                        &quote.sender_account_hash,
                        request_id,
                        trace_id,
                        started,
                        "tx_hash_mismatch",
                        "submit_tx_hash_mismatch",
                    );
                    return Err(ApiError::Validation(
                        "signed transaction hash does not match the quote bound at claim".into(),
                    ));
                }
            }
            Ok(computed)
        }
        Err(err) => {
            emit_submit_failed(
                &state,
                &body.quote_id,
                None,
                &quote.sender_account_hash,
                request_id,
                trace_id,
                started,
                "auth_failure",
                "submit_auth_failure",
            );
            Err(map_envelope_error(err))
        }
    }
}

async fn reconcile_or_rebroadcast(
    state: &AppState,
    body: &SwapSubmitRequest,
    quote: &PreparedSwapQuote,
    stored_hash: &str,
    request_id: &str,
    trace_id: &str,
    started: Instant,
) -> Result<(SwapSubmitResponse, StatusCode, String)> {
    // Always re-validate the client envelope against the prepared quote + stored hash.
    let _ = verify_submit_envelope(body, quote, request_id, trace_id, started, state)?;

    match state.transaction_broadcaster.lookup(stored_hash).await {
        Ok(Some(found)) => {
            if found.tx_hash != stored_hash {
                return Err(ApiError::Internal(Arc::new(anyhow::anyhow!(
                    "horizon lookup hash mismatch: expected {stored_hash}, got {}",
                    found.tx_hash
                ))));
            }
            state
                .swap_quote_store
                .finalize_submit(body.quote_id.trim(), stored_hash)
                .await
                .map_err(|e| map_store_error_for_quote(e, body.quote_id.trim()))?;
            emit_submit_success(
                &state,
                &body.quote_id,
                stored_hash,
                &quote.sender_account_hash,
                request_id,
                trace_id,
                started,
                &found.status,
                "reconciliation_success",
            );
            let (resp, status) = submit_response(body, quote, found);
            Ok((resp, status, quote.sender_account_hash.clone()))
        }
        Ok(None) => {
            // Horizon has no tx. If timebounds already closed, rebroadcast can
            // never succeed — release the sender lock instead of looping 409s.
            if quote_timebounds_expired(quote) {
                let _ = state
                    .swap_quote_store
                    .mark_failed(body.quote_id.trim())
                    .await;
                emit_submit_failed(
                    &state,
                    &body.quote_id,
                    Some(stored_hash),
                    &quote.sender_account_hash,
                    request_id,
                    trace_id,
                    started,
                    "timebounds_expired",
                    "reconciliation_timebounds_expired",
                );
                return Err(ApiError::Conflict {
                    message: "Transaction timebounds expired before confirmation; request a fresh prepare"
                        .into(),
                    quote_id: body.quote_id.clone(),
                    tx_hash: stored_hash.to_string(),
                    status: STATUS_PERMANENTLY_FAILED.into(),
                });
            }
            emit_submit_failed(
                &state,
                &body.quote_id,
                Some(stored_hash),
                &quote.sender_account_hash,
                request_id,
                trace_id,
                started,
                "reconcile_absent",
                "reconciliation_absent_rebroadcast",
            );
            broadcast_and_finalize(
                state,
                body,
                quote,
                stored_hash,
                request_id,
                trace_id,
                started,
            )
            .await
        }
        Err(e) if e.is_transient_transport() => {
            emit_submit_failed(
                &state,
                &body.quote_id,
                Some(stored_hash),
                &quote.sender_account_hash,
                request_id,
                trace_id,
                started,
                "reconcile_pending",
                "reconciliation_pending",
            );
            Err(ApiError::DependencyUnavailable(
                "Horizon reconciliation pending; retry submit without re-preparing".into(),
            ))
        }
        Err(e) => Err(map_broadcast_error(e)),
    }
}

async fn broadcast_and_finalize(
    state: &AppState,
    body: &SwapSubmitRequest,
    quote: &PreparedSwapQuote,
    expected_tx_hash: &str,
    request_id: &str,
    trace_id: &str,
    started: Instant,
) -> Result<(SwapSubmitResponse, StatusCode, String)> {
    // Re-run full validation immediately before every broadcast/rebroadcast.
    let recomputed = verify_submit_envelope(body, quote, request_id, trace_id, started, state)?;
    if recomputed != expected_tx_hash {
        return Err(ApiError::Validation(
            "signed transaction hash does not match the quote bound at claim".into(),
        ));
    }

    let broadcast = match state
        .transaction_broadcaster
        .submit(body.signed_xdr.trim())
        .await
    {
        Ok(result) => result,
        Err(err) if err.is_transient_transport() => {
            // Hash already bound at claim — reconcile; do not TTL-fail or clear.
            match state.transaction_broadcaster.lookup(expected_tx_hash).await {
                Ok(Some(found)) => {
                    if found.tx_hash != expected_tx_hash {
                        return Err(ApiError::Internal(Arc::new(anyhow::anyhow!(
                            "horizon lookup hash mismatch after timeout"
                        ))));
                    }
                    state
                        .swap_quote_store
                        .finalize_submit(body.quote_id.trim(), expected_tx_hash)
                        .await
                        .map_err(|e| map_store_error_for_quote(e, body.quote_id.trim()))?;
                    emit_submit_success(
                        &state,
                        &body.quote_id,
                        expected_tx_hash,
                        &quote.sender_account_hash,
                        request_id,
                        trace_id,
                        started,
                        "pending",
                        "reconciliation_success_after_timeout",
                    );
                    let (resp, status) = submit_response(body, quote, found);
                    return Ok((resp, status, quote.sender_account_hash.clone()));
                }
                Ok(None) | Err(_) => {
                    emit_submit_failed(
                        &state,
                        &body.quote_id,
                        Some(expected_tx_hash),
                        &quote.sender_account_hash,
                        request_id,
                        trace_id,
                        started,
                        "broadcast_pending_reconcile",
                        "broadcast_pending_reconcile",
                    );
                    return Err(ApiError::DependencyUnavailable(
                        "Broadcast timed out; quote remains submitting for reconciliation — retry submit"
                            .into(),
                    ));
                }
            }
        }
        Err(err) => {
            let _ = state
                .swap_quote_store
                .mark_failed(body.quote_id.trim())
                .await;
            let class = err.metrics_class();
            emit_submit_failed(
                &state,
                &body.quote_id,
                None,
                &quote.sender_account_hash,
                request_id,
                trace_id,
                started,
                class,
                "broadcast_permanent_failure",
            );
            return Err(map_broadcast_error(err));
        }
    };

    if !broadcast.tx_hash.is_empty() && broadcast.tx_hash != expected_tx_hash {
        return Err(ApiError::Internal(Arc::new(anyhow::anyhow!(
            "horizon accepted hash mismatch: expected {expected_tx_hash}, got {}",
            broadcast.tx_hash
        ))));
    }

    state
        .swap_quote_store
        .finalize_submit(body.quote_id.trim(), expected_tx_hash)
        .await
        .map_err(|e| map_store_error_for_quote(e, body.quote_id.trim()))?;

    let mut result = broadcast;
    if result.tx_hash.is_empty() {
        result.tx_hash = expected_tx_hash.to_string();
    }
    let (resp, status) = submit_response(body, quote, result);
    Ok((resp, status, quote.sender_account_hash.clone()))
}

fn submit_response(
    body: &SwapSubmitRequest,
    quote: &PreparedSwapQuote,
    broadcast: BroadcastResult,
) -> (SwapSubmitResponse, StatusCode) {
    let http_status = if broadcast.status == "success" {
        StatusCode::OK
    } else {
        StatusCode::ACCEPTED
    };
    (
        SwapSubmitResponse {
            quote_id: body.quote_id.clone(),
            tx_hash: broadcast.tx_hash,
            status: broadcast.status,
            output_amount: Some(quote.estimated_output.clone()),
            ledger: broadcast.ledger,
        },
        http_status,
    )
}

fn map_store_error(err: SwapStoreError) -> ApiError {
    match err {
        SwapStoreError::NotFound => ApiError::QuoteNotFound {
            quote_id: String::new(),
        },
        SwapStoreError::ActivePrepareExists => ApiError::Conflict {
            message: "An active prepare already exists for this sender; wait for expiry or submit/fail it first".into(),
            quote_id: String::new(),
            tx_hash: String::new(),
            status: STATUS_ACTIVE_PREPARE_EXISTS.into(),
        },
        SwapStoreError::InvalidTransition => ApiError::Conflict {
            message: "Invalid quote state transition".into(),
            quote_id: String::new(),
            tx_hash: String::new(),
            status: STATUS_INVALID_TRANSITION.into(),
        },
        SwapStoreError::Database(e) => ApiError::Internal(Arc::new(anyhow::anyhow!(e))),
    }
}

fn map_store_error_for_quote(err: SwapStoreError, quote_id: &str) -> ApiError {
    match err {
        SwapStoreError::NotFound => ApiError::QuoteNotFound {
            quote_id: quote_id.to_string(),
        },
        other => map_store_error(other),
    }
}

fn map_broadcast_error(err: BroadcastError) -> ApiError {
    match err {
        BroadcastError::Validation(msg) => ApiError::Validation(msg),
        BroadcastError::Timeout => ApiError::DependencyUnavailable(
            "Horizon timed out while submitting transaction".to_string(),
        ),
        BroadcastError::TransientRpc(msg) => ApiError::DependencyUnavailable(msg),
        BroadcastError::InsufficientFee => ApiError::Validation(
            "Transaction fee is insufficient for network submission".to_string(),
        ),
        BroadcastError::InsufficientBalance => ApiError::Validation(
            "Source account has insufficient balance for this swap".to_string(),
        ),
        BroadcastError::SlippageExceeded => {
            ApiError::NotExecutable("On-chain execution would exceed slippage bounds".to_string())
        }
        BroadcastError::BadSignature => {
            ApiError::Validation("Transaction signature is invalid".to_string())
        }
        BroadcastError::BadSequence => ApiError::Conflict {
            message: "Account sequence mismatch (tx_bad_seq); request a fresh prepare".into(),
            quote_id: String::new(),
            tx_hash: String::new(),
            status: STATUS_BAD_SEQUENCE.into(),
        },
        BroadcastError::Malformed => {
            ApiError::Validation("Transaction is malformed and cannot be submitted".into())
        }
        BroadcastError::Permanent(msg) => ApiError::NotExecutable(msg),
    }
}

fn map_tx_build_error(err: TxBuildError) -> ApiError {
    match err {
        TxBuildError::Validation(msg) => ApiError::Validation(msg),
        TxBuildError::AccountLookup(msg) => ApiError::DependencyUnavailable(msg),
        TxBuildError::Xdr(msg) => ApiError::Internal(Arc::new(anyhow::anyhow!(msg))),
    }
}

fn map_envelope_error(err: EnvelopeValidationError) -> ApiError {
    match err {
        EnvelopeValidationError::Malformed(msg) => ApiError::Validation(format!(
            "signed_xdr is not a valid Stellar transaction envelope: {msg}"
        )),
        EnvelopeValidationError::MissingSignatures => {
            ApiError::Validation("signed_xdr must include at least one signature".to_string())
        }
        EnvelopeValidationError::QuoteMismatch => {
            ApiError::Validation("signed transaction does not match the prepared quote".to_string())
        }
        EnvelopeValidationError::SignerMismatch => ApiError::Validation(
            "transaction source account does not match the prepared sender".to_string(),
        ),
        EnvelopeValidationError::BadSignature => {
            ApiError::Validation("Transaction signature is invalid".to_string())
        }
        EnvelopeValidationError::UnsupportedAccount(msg) => ApiError::Validation(msg),
    }
}

/// True when the prepared envelope's max time is in the past (unix seconds).
fn quote_timebounds_expired(quote: &PreparedSwapQuote) -> bool {
    match quote.timebounds_max {
        Some(max) if max > 0 => Utc::now().timestamp() > max,
        _ => false,
    }
}

fn submit_error_class(err: &ApiError) -> &'static str {
    match err {
        ApiError::QuoteExpired { .. } => "quote_expired",
        ApiError::QuoteNotFound { .. } => "quote_not_found",
        ApiError::Conflict { status, .. } if status == STATUS_PERMANENTLY_FAILED => STATUS_PERMANENTLY_FAILED,
        ApiError::Conflict { status, .. } if status == STATUS_BAD_SEQUENCE => STATUS_BAD_SEQUENCE,
        ApiError::Conflict { .. } => "duplicate_quote",
        ApiError::Validation(_) => "validation",
        ApiError::DependencyUnavailable(_) => "rpc_error",
        ApiError::NotExecutable(_) => "permanent",
        _ => "internal",
    }
}

fn validate_stellar_account(sender: &str) -> Result<()> {
    let sender = sender.trim();
    if stellar_strkey::ed25519::PublicKey::from_string(sender).is_err() {
        return Err(ApiError::Validation(
            "sender must be a valid Stellar G-address".to_string(),
        ));
    }
    Ok(())
}

/// Emit unified audit log entry with automatic elapsed time calculation.
fn emit_audit_log(
    state: &AppState,
    quote_id: &str,
    tx_hash: Option<&str>,
    sender_hash: &str,
    request_id: &str,
    trace_id: &str,
    started: Instant,
    outcome: SwapSubmitOutcome,
    error_class: &'static str,
    event: &str,
    details: serde_json::Value,
) {
    state.swap_submit_audit_writer.emit_swap_submit(
        quote_id,
        tx_hash,
        sender_hash,
        request_id,
        trace_id,
        started.elapsed().as_millis() as u64,
        outcome,
        error_class,
        details.into_iter()
            .chain(std::iter::once(("event".to_string(), serde_json::json!(event))))
            .collect::<serde_json::Map<String, serde_json::Value>>()
            .into(),
    );
}

/// Emit prepare rejection audit log.
fn emit_prepare_rejected(
    state: &AppState,
    sender_hash: &str,
    request_id: &str,
    trace_id: &str,
    started: Instant,
    error_class: &'static str,
) {
    emit_audit_log(
        state,
        "none",
        None,
        sender_hash,
        request_id,
        trace_id,
        started,
        SwapSubmitOutcome::Failed,
        error_class,
        "prepare_rejected",
        serde_json::json!({ "execution_mode": "classic_path_payment" }),
    );
}

/// Emit prepare success audit log.
fn emit_prepare_success(
    state: &AppState,
    quote_id: &str,
    sender_hash: &str,
    request_id: &str,
    trace_id: &str,
    started: Instant,
    expected_output: &str,
    min_output: &str,
) {
    emit_audit_log(
        state,
        quote_id,
        None,
        sender_hash,
        request_id,
        trace_id,
        started,
        SwapSubmitOutcome::Prepared,
        "none",
        "prepare_success",
        serde_json::json!({
            "expected_output": expected_output,
            "min_output": min_output,
            "execution_mode": "classic_path_payment",
        }),
    );
}

/// Emit submit result audit log for various failure scenarios.
fn emit_submit_failed(
    state: &AppState,
    quote_id: &str,
    tx_hash: Option<&str>,
    sender_hash: &str,
    request_id: &str,
    trace_id: &str,
    started: Instant,
    error_class: &'static str,
    event: &str,
) {
    emit_audit_log(
        state,
        quote_id,
        tx_hash,
        sender_hash,
        request_id,
        trace_id,
        started,
        SwapSubmitOutcome::Failed,
        error_class,
        event,
        serde_json::json!({}),
    );
}

/// Emit submit success (reconciliation or broadcast accepted).
fn emit_submit_success(
    state: &AppState,
    quote_id: &str,
    tx_hash: &str,
    sender_hash: &str,
    request_id: &str,
    trace_id: &str,
    started: Instant,
    status: &str,
    event: &str,
) {
    emit_audit_log(
        state,
        quote_id,
        Some(tx_hash),
        sender_hash,
        request_id,
        trace_id,
        started,
        SwapSubmitOutcome::Submitted,
        "none",
        event,
        serde_json::json!({ "status": status }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn validate_stellar_account_rejects_short_keys() {
        assert!(validate_stellar_account("GSHORT").is_err());
    }

    #[test]
    fn quote_timebounds_expired_requires_positive_max() {
        let mut quote = PreparedSwapQuote {
            quote_id: "q".into(),
            sender_account: "G".into(),
            sender_account_hash: "h".into(),
            unsigned_xdr_hash: "u".into(),
            expires_at: Utc::now() + Duration::minutes(1),
            estimated_output: "1".into(),
            min_output: "1".into(),
            amount_in: "1".into(),
            execution_mode: "classic_path_payment".into(),
            network_passphrase: "p".into(),
            route_digest: "r".into(),
            price_digest: "pd".into(),
            source_sequence: None,
            timebounds_max: None,
            base_fee: None,
            valid_until_ledger: None,
            submission_status: SubmissionStatus::Submitting,
            tx_hash: Some("abc".into()),
        };
        assert!(!quote_timebounds_expired(&quote));
        quote.timebounds_max = Some(0);
        assert!(!quote_timebounds_expired(&quote));
        quote.timebounds_max = Some(Utc::now().timestamp() - 5);
        assert!(quote_timebounds_expired(&quote));
        quote.timebounds_max = Some(Utc::now().timestamp() + 60);
        assert!(!quote_timebounds_expired(&quote));
    }
}
