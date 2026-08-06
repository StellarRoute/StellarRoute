//! Circle CCTP v2 bridge routes — production HTTP gate (fail-closed by default).

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

use crate::cctp::access::{
    generate_ephemeral_access_token, hash_access_token, TRANSFER_ACCESS_HEADER,
};
use crate::cctp::gate::{ensure_public_gate, map_reattest_denied};
use crate::cctp::gate::{
    hash_presented_access_token, map_service_error, to_prepare_burn_response,
    to_prepare_mint_response, to_quote_response, to_reattest_response, to_status_response,
    to_submit_burn_response, to_submit_mint_response, uniform_transfer_not_found, POLL_LEASE_SECS,
    REATTEST_COOLDOWN_SECS, REATTEST_LEASE_SECS, REATTEST_MAX_ATTEMPTS,
};
use crate::cctp::idempotency::{
    canonical_quote_request_hash, lease_owner_hash_from_nonce, new_lease_owner_nonce,
    normalize_idempotency_key, CctpIdempotencyError, IdempotencyState, IDEMPOTENCY_HEADER,
};
use crate::error::{ApiError, Result};
use crate::metrics;
use crate::middleware::RequestId;
use crate::models::v2_cctp::{
    is_valid_tx_hash, parse_transfer_id, CctpPrepareBurnResponse, CctpPrepareMintResponse,
    CctpQuoteRequest, CctpQuoteResponse, CctpReattestResponse, CctpSubmitBurnRequest,
    CctpSubmitBurnResponse, CctpSubmitMintRequest, CctpSubmitMintResponse, CctpTransferStatus,
    CctpTransferStatusResponse, CctpValidationError,
};
use crate::models::ApiResponse;
use crate::state::AppState;

fn map_validation(err: CctpValidationError) -> ApiError {
    match err {
        CctpValidationError::UnsupportedCorridor => ApiError::UnsupportedCorridor,
        CctpValidationError::InvalidFinality => ApiError::InvalidFinality,
        CctpValidationError::InvalidRecipient => ApiError::InvalidRecipient,
        CctpValidationError::InvalidAmount => {
            ApiError::InvalidAmount("amount must be a positive decimal string".to_string())
        }
        CctpValidationError::InvalidSender => ApiError::Validation(
            "sender must be a valid G-address for Stellar or 0x address for EVM source".to_string(),
        ),
        CctpValidationError::InvalidMintSubmitter => ApiError::Validation(
            "mint_submitter must be a valid Stellar G-address for evm_to_stellar".to_string(),
        ),
        CctpValidationError::StellarRemainder => ApiError::Validation(
            "Stellar outbound amount must have zero 7th-decimal remainder".to_string(),
        ),
    }
}

fn cctp_not_enabled() -> ApiError {
    ApiError::CctpNotEnabled(
        "Circle CCTP bridge settlement is not enabled on this deployment".to_string(),
    )
}

fn require_cctp(state: &AppState) -> Result<Arc<crate::cctp::bootstrap::CctpHttpContext>> {
    let ctx = state.cctp.clone().ok_or_else(cctp_not_enabled)?;
    if !ctx.config.enabled {
        return Err(cctp_not_enabled());
    }
    Ok(ctx)
}

fn parse_transfer_id_param(transfer_id: &str) -> Result<Uuid> {
    parse_transfer_id(transfer_id).map_err(ApiError::Validation)
}

fn validate_submit_tx_hash(tx_hash: &str) -> Result<()> {
    if is_valid_tx_hash(tx_hash) {
        Ok(())
    } else {
        Err(ApiError::Validation(
            "tx_hash must be a 64-hex Stellar hash or 0x-prefixed 32-byte EVM hash".to_string(),
        ))
    }
}

fn access_token_from_headers(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(TRANSFER_ACCESS_HEADER)
        .and_then(|v| v.to_str().ok())
}

fn idempotency_key_from_headers(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(IDEMPOTENCY_HEADER)
        .and_then(|v| v.to_str().ok())
}

async fn load_authorized_transfer(
    state: &AppState,
    transfer_id: Uuid,
    headers: &HeaderMap,
) -> Result<crate::cctp::store::CctpTransfer> {
    let ctx = require_cctp(state)?;
    let token_hash = hash_presented_access_token(transfer_id, access_token_from_headers(headers))?;
    let transfer = ctx
        .service
        .store
        .get_authorized(transfer_id, &token_hash)
        .await
        .map_err(|e| {
            map_service_error(
                crate::cctp::service::CctpServiceError::Store(e),
                Some(transfer_id),
            )
        })?
        .ok_or_else(|| uniform_transfer_not_found(transfer_id))?;
    Ok(transfer)
}

async fn gate_for_direction(
    state: &AppState,
    direction: crate::models::v2_cctp::CctpDirection,
) -> Result<()> {
    let ctx = require_cctp(state)?;
    ensure_public_gate(
        &ctx.service,
        direction,
        &state.kill_switch,
        &state.external_dependency_health,
    )
    .await
}

/// `POST /api/v2/bridge/cctp/quote`
#[utoipa::path(
    post,
    path = "/api/v2/bridge/cctp/quote",
    tag = "cctp",
    params(
        ("Idempotency-Key" = Option<String>, Header, description = "Optional idempotency key (1-128 UTF-8 chars). Same key with byte-identical body replays the original quote and transfer access token."),
    ),
    request_body = CctpQuoteRequest,
    responses(
        (status = 200, description = "CCTP fee quote", body = CctpQuoteResponse),
        (status = 400, description = "Invalid request"),
        (status = 409, description = "Idempotency key reused with different quote body"),
        (status = 425, description = "Idempotent quote still in progress; retry with same key"),
        (status = 429, description = "Rate limit exceeded"),
        (status = 503, description = "CCTP bridge not enabled or dependencies not ready"),
    )
)]
pub async fn cctp_quote(
    State(state): State<Arc<AppState>>,
    request_id: RequestId,
    headers: HeaderMap,
    Json(body): Json<CctpQuoteRequest>,
) -> Result<Json<ApiResponse<CctpQuoteResponse>>> {
    let started = Instant::now();
    body.validate().map_err(map_validation)?;

    let ctx = require_cctp(&state)?;
    gate_for_direction(&state, body.direction).await?;

    let request_hash =
        canonical_quote_request_hash(&serde_json::to_value(&body).map_err(|e| {
            ApiError::Internal(std::sync::Arc::new(anyhow::anyhow!(e.to_string())))
        })?)
        .map_err(|e| match e {
            CctpIdempotencyError::RequestTooLarge => {
                ApiError::Validation("quote request body too large".into())
            }
            other => ApiError::Internal(std::sync::Arc::new(anyhow::anyhow!(other.to_string()))),
        })?;

    let _ = ctx.idempotency.cleanup_expired(32).await;

    if let Some(raw_key) = idempotency_key_from_headers(&headers) {
        let key = normalize_idempotency_key(raw_key)
            .map_err(|_| ApiError::Validation("idempotency key exceeds maximum length".into()))?;
        let lease_owner = lease_owner_hash_from_nonce(&new_lease_owner_nonce());
        let expires_at =
            chrono::Utc::now() + chrono::Duration::seconds(ctx.config.quote_ttl_secs as i64);

        let claim = match ctx
            .idempotency
            .claim_quote(&key, &request_hash, &lease_owner, expires_at)
            .await
        {
            Ok(c) => c,
            Err(CctpIdempotencyError::Conflict) => {
                metrics::record_cctp_endpoint_outcome("quote", "idempotency_conflict");
                return Err(ApiError::Conflict {
                    message: "Idempotency key reused with different quote request".into(),
                    quote_id: String::new(),
                    tx_hash: String::new(),
                    status: "idempotency_conflict".into(),
                });
            }
            Err(CctpIdempotencyError::PendingInProgress) => {
                metrics::record_cctp_endpoint_outcome("quote", "idempotency_pending");
                return Err(ApiError::TooEarly(
                    "An idempotent quote with this key is still in progress".into(),
                ));
            }
            Err(e) => {
                return Err(ApiError::Internal(std::sync::Arc::new(anyhow::anyhow!(
                    e.to_string()
                ))));
            }
        };

        if claim.state == IdempotencyState::Completed {
            let transfer = ctx
                .service
                .get_transfer(claim.transfer_id)
                .await
                .map_err(|e| map_service_error(e, Some(claim.transfer_id)))?;
            let stored_hash = transfer.access_token_hash.as_deref().ok_or_else(|| {
                ApiError::Internal(std::sync::Arc::new(anyhow::anyhow!(
                    "completed idempotency without access binding"
                )))
            })?;
            let access_token = ctx
                .access_token_keys
                .recover_idempotent_token(&key, &request_hash, claim.transfer_id, stored_hash)
                .ok_or_else(|| {
                    ApiError::CctpNotEnabled(
                        "Idempotent replay unavailable after access key rotation; request a new quote"
                            .into(),
                    )
                })?;
            metrics::record_cctp_endpoint_outcome("quote", "idempotent_hit");
            return Ok(Json(ApiResponse::with_version(
                2,
                to_quote_response(&transfer, &access_token),
                request_id.as_str(),
            )));
        }

        let access_token =
            ctx.access_token_keys
                .derive_idempotent_token(&key, &request_hash, claim.transfer_id);
        let access_hash = hash_access_token(&access_token);
        let transfer = ctx
            .service
            .build_quote_transfer(&body, claim.transfer_id, access_hash)
            .await
            .map_err(|e| map_service_error(e, Some(claim.transfer_id)))?;

        ctx.idempotency
            .finalize_quote(&key, &lease_owner, &transfer)
            .await
            .map_err(|e| match e {
                CctpIdempotencyError::PendingInProgress => ApiError::TooEarly(
                    "An idempotent quote with this key is still in progress".into(),
                ),
                CctpIdempotencyError::Conflict => ApiError::Conflict {
                    message: "Idempotency key reused with different quote request".into(),
                    quote_id: claim.transfer_id.to_string(),
                    tx_hash: String::new(),
                    status: "idempotency_conflict".into(),
                },
                other => {
                    ApiError::Internal(std::sync::Arc::new(anyhow::anyhow!(other.to_string())))
                }
            })?;

        metrics::record_cctp_endpoint_outcome("quote", "success");
        metrics::record_cctp_iris_latency(started.elapsed(), "quote");
        return Ok(Json(ApiResponse::with_version(
            2,
            to_quote_response(&transfer, &access_token),
            request_id.as_str(),
        )));
    }

    let (access_token, access_hash) = generate_ephemeral_access_token();
    let transfer = ctx
        .service
        .quote_core(&body, access_hash)
        .await
        .map_err(|e| map_service_error(e, None))?;

    metrics::record_cctp_endpoint_outcome("quote", "success");
    metrics::record_cctp_iris_latency(started.elapsed(), "quote");
    Ok(Json(ApiResponse::with_version(
        2,
        to_quote_response(&transfer, &access_token),
        request_id.as_str(),
    )))
}

/// `POST /api/v2/bridge/cctp/{transfer_id}/prepare-burn`
#[utoipa::path(
    post,
    path = "/api/v2/bridge/cctp/{transfer_id}/prepare-burn",
    tag = "cctp",
    params(
        ("transfer_id" = String, Path, description = "Transfer UUID"),
        ("x-cctp-transfer-access" = String, Header, description = "Transfer capability token from quote (required)"),
    ),
    responses(
        (status = 200, description = "Prepared burn wallet payload", body = CctpPrepareBurnResponse),
        (status = 400, description = "Invalid transfer ID or transfer state"),
        (status = 404, description = "Transfer not found or access token invalid (uniform response)"),
        (status = 429, description = "Rate limit exceeded"),
        (status = 503, description = "CCTP bridge not enabled or dependencies not ready"),
    )
)]
pub async fn cctp_prepare_burn(
    State(state): State<Arc<AppState>>,
    Path(transfer_id): Path<String>,
    request_id: RequestId,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<CctpPrepareBurnResponse>>> {
    let transfer_id = parse_transfer_id_param(&transfer_id)?;
    let transfer = load_authorized_transfer(&state, transfer_id, &headers).await?;
    gate_for_direction(&state, transfer.direction).await?;

    let ctx = require_cctp(&state)?;
    let _ = ctx
        .service
        .prepare_burn(transfer_id)
        .await
        .map_err(|e| map_service_error(e, Some(transfer_id)))?;
    let bundle = ctx
        .service
        .prepare_burn_wallet(transfer_id)
        .await
        .map_err(|e| map_service_error(e, Some(transfer_id)))?;
    let updated = ctx
        .service
        .get_transfer(transfer_id)
        .await
        .map_err(|e| map_service_error(e, Some(transfer_id)))?;

    metrics::record_cctp_endpoint_outcome("prepare_burn", "success");
    Ok(Json(ApiResponse::with_version(
        2,
        to_prepare_burn_response(&updated, &bundle),
        request_id.as_str(),
    )))
}

/// `POST /api/v2/bridge/cctp/{transfer_id}/submit-burn`
#[utoipa::path(
    post,
    path = "/api/v2/bridge/cctp/{transfer_id}/submit-burn",
    tag = "cctp",
    params(
        ("transfer_id" = String, Path, description = "Transfer UUID"),
        ("x-cctp-transfer-access" = String, Header, description = "Transfer capability token from quote (required)"),
    ),
    request_body = CctpSubmitBurnRequest,
    responses(
        (status = 200, description = "Burn or approval tx hash recorded", body = CctpSubmitBurnResponse),
        (status = 400, description = "Invalid transfer ID, tx_hash, or transfer state"),
        (status = 404, description = "Transfer not found or access token invalid (uniform response)"),
        (status = 429, description = "Rate limit exceeded"),
        (status = 503, description = "CCTP bridge not enabled or dependencies not ready"),
    )
)]
pub async fn cctp_submit_burn(
    State(state): State<Arc<AppState>>,
    Path(transfer_id): Path<String>,
    request_id: RequestId,
    headers: HeaderMap,
    Json(body): Json<CctpSubmitBurnRequest>,
) -> Result<Json<ApiResponse<CctpSubmitBurnResponse>>> {
    let transfer_id = parse_transfer_id_param(&transfer_id)?;
    validate_submit_tx_hash(&body.tx_hash)?;
    let transfer = load_authorized_transfer(&state, transfer_id, &headers).await?;
    gate_for_direction(&state, transfer.direction).await?;

    let ctx = require_cctp(&state)?;
    let updated = ctx
        .service
        .record_source_submission(transfer_id, &body.tx_hash)
        .await
        .map_err(|e| map_service_error(e, Some(transfer_id)))?;

    metrics::record_cctp_endpoint_outcome("submit_burn", "success");
    Ok(Json(ApiResponse::with_version(
        2,
        to_submit_burn_response(&updated),
        request_id.as_str(),
    )))
}

/// `GET /api/v2/bridge/cctp/{transfer_id}`
#[utoipa::path(
    get,
    path = "/api/v2/bridge/cctp/{transfer_id}",
    tag = "cctp",
    params(
        ("transfer_id" = String, Path, description = "Transfer UUID"),
        ("x-cctp-transfer-access" = String, Header, description = "Transfer capability token from quote (required)"),
    ),
    responses(
        (status = 200, description = "Transfer saga status", body = CctpTransferStatusResponse),
        (status = 400, description = "Invalid transfer ID"),
        (status = 404, description = "Transfer not found or access token invalid (uniform response)"),
        (status = 429, description = "Rate limit exceeded"),
        (status = 503, description = "CCTP bridge not enabled or dependencies not ready"),
    )
)]
pub async fn cctp_get_transfer(
    State(state): State<Arc<AppState>>,
    Path(transfer_id): Path<String>,
    request_id: RequestId,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<CctpTransferStatusResponse>>> {
    let transfer_id = parse_transfer_id_param(&transfer_id)?;
    let transfer = load_authorized_transfer(&state, transfer_id, &headers).await?;
    gate_for_direction(&state, transfer.direction).await?;

    let ctx = require_cctp(&state)?;
    let polled = ctx
        .service
        .poll_one_transfer_with_lease(
            transfer_id,
            POLL_LEASE_SECS,
            ctx.config.poll_interval_secs as i64,
        )
        .await
        .map_err(|e| map_service_error(e, Some(transfer_id)))?;

    metrics::record_cctp_endpoint_outcome("get_transfer", "success");
    Ok(Json(ApiResponse::with_version(
        2,
        to_status_response(&polled),
        request_id.as_str(),
    )))
}

/// `POST /api/v2/bridge/cctp/{transfer_id}/prepare-mint`
#[utoipa::path(
    post,
    path = "/api/v2/bridge/cctp/{transfer_id}/prepare-mint",
    tag = "cctp",
    params(
        ("transfer_id" = String, Path, description = "Transfer UUID"),
        ("x-cctp-transfer-access" = String, Header, description = "Transfer capability token from quote (required)"),
    ),
    responses(
        (status = 200, description = "Prepared mint wallet payload", body = CctpPrepareMintResponse),
        (status = 400, description = "Invalid transfer ID or transfer state"),
        (status = 404, description = "Transfer not found or access token invalid (uniform response)"),
        (status = 429, description = "Rate limit exceeded"),
        (status = 503, description = "CCTP bridge not enabled or dependencies not ready"),
    )
)]
pub async fn cctp_prepare_mint(
    State(state): State<Arc<AppState>>,
    Path(transfer_id): Path<String>,
    request_id: RequestId,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<CctpPrepareMintResponse>>> {
    let transfer_id = parse_transfer_id_param(&transfer_id)?;
    let transfer = load_authorized_transfer(&state, transfer_id, &headers).await?;
    gate_for_direction(&state, transfer.direction).await?;

    if transfer.status != CctpTransferStatus::AttestationReady
        && transfer.status != CctpTransferStatus::MintFailedRetryable
    {
        return Err(map_service_error(
            CctpServiceError::InvalidState,
            Some(transfer_id),
        ));
    }

    let ctx = require_cctp(&state)?;
    let bundle = ctx
        .service
        .prepare_mint(transfer_id)
        .await
        .map_err(|e| map_service_error(e, Some(transfer_id)))?;
    let updated = ctx
        .service
        .get_transfer(transfer_id)
        .await
        .map_err(|e| map_service_error(e, Some(transfer_id)))?;

    metrics::record_cctp_endpoint_outcome("prepare_mint", "success");
    Ok(Json(ApiResponse::with_version(
        2,
        to_prepare_mint_response(&updated, &bundle),
        request_id.as_str(),
    )))
}

/// `POST /api/v2/bridge/cctp/{transfer_id}/submit-mint`
#[utoipa::path(
    post,
    path = "/api/v2/bridge/cctp/{transfer_id}/submit-mint",
    tag = "cctp",
    params(
        ("transfer_id" = String, Path, description = "Transfer UUID"),
        ("x-cctp-transfer-access" = String, Header, description = "Transfer capability token from quote (required)"),
    ),
    request_body = CctpSubmitMintRequest,
    responses(
        (status = 200, description = "Mint tx hash recorded", body = CctpSubmitMintResponse),
        (status = 400, description = "Invalid transfer ID, tx_hash, or transfer state"),
        (status = 404, description = "Transfer not found or access token invalid (uniform response)"),
        (status = 429, description = "Rate limit exceeded"),
        (status = 503, description = "CCTP bridge not enabled or dependencies not ready"),
    )
)]
pub async fn cctp_submit_mint(
    State(state): State<Arc<AppState>>,
    Path(transfer_id): Path<String>,
    request_id: RequestId,
    headers: HeaderMap,
    Json(body): Json<CctpSubmitMintRequest>,
) -> Result<Json<ApiResponse<CctpSubmitMintResponse>>> {
    let transfer_id = parse_transfer_id_param(&transfer_id)?;
    validate_submit_tx_hash(&body.tx_hash)?;
    let transfer = load_authorized_transfer(&state, transfer_id, &headers).await?;
    gate_for_direction(&state, transfer.direction).await?;

    let ctx = require_cctp(&state)?;
    let updated = ctx
        .service
        .record_mint_submission(transfer_id, &body.tx_hash)
        .await
        .map_err(|e| map_service_error(e, Some(transfer_id)))?;

    metrics::record_cctp_endpoint_outcome("submit_mint", "success");
    Ok(Json(ApiResponse::with_version(
        2,
        to_submit_mint_response(&updated),
        request_id.as_str(),
    )))
}

/// `POST /api/v2/bridge/cctp/{transfer_id}/reattest`
#[utoipa::path(
    post,
    path = "/api/v2/bridge/cctp/{transfer_id}/reattest",
    tag = "cctp",
    params(
        ("transfer_id" = String, Path, description = "Transfer UUID"),
        ("x-cctp-transfer-access" = String, Header, description = "Transfer capability token from quote (required)"),
    ),
    responses(
        (status = 200, description = "Attestation re-poll requested", body = CctpReattestResponse),
        (status = 400, description = "Invalid transfer ID, cooldown, or transfer state"),
        (status = 404, description = "Transfer not found or access token invalid (uniform response)"),
        (status = 409, description = "Re-attest claim conflict (cooldown or concurrent claim)"),
        (status = 429, description = "Rate limit exceeded"),
        (status = 503, description = "CCTP bridge not enabled or dependencies not ready"),
    )
)]
pub async fn cctp_reattest(
    State(state): State<Arc<AppState>>,
    Path(transfer_id): Path<String>,
    request_id: RequestId,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<CctpReattestResponse>>> {
    let transfer_id = parse_transfer_id_param(&transfer_id)?;
    let _transfer = load_authorized_transfer(&state, transfer_id, &headers).await?;
    gate_for_direction(&state, _transfer.direction).await?;

    let ctx = require_cctp(&state)?;
    let updated = match ctx
        .service
        .reattest_with_claim(
            transfer_id,
            REATTEST_MAX_ATTEMPTS,
            REATTEST_COOLDOWN_SECS,
            REATTEST_LEASE_SECS,
        )
        .await
    {
        Ok(t) => t,
        Err(crate::cctp::service::CctpServiceError::InvalidState) => {
            return Err(map_reattest_denied(transfer_id));
        }
        Err(e) => return Err(map_service_error(e, Some(transfer_id))),
    };

    metrics::record_cctp_endpoint_outcome("reattest", "success");
    Ok(Json(ApiResponse::with_version(
        2,
        to_reattest_response(&updated),
        request_id.as_str(),
    )))
}

use crate::cctp::service::CctpServiceError;
