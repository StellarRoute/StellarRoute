//! HTTP execution gate, redacted wire mapping, and service error translation.

use uuid::Uuid;

use crate::cctp::access::{hash_access_token, validate_access_token_format};
use crate::cctp::builders::{BuilderError, BurnPrepareStep};
use crate::cctp::config::CctpConfig;
use crate::cctp::readiness::CctpRuntime;
use crate::cctp::service::{CctpService, CctpServiceError};
use crate::cctp::store::CctpTransfer;
use crate::cctp::transitions::is_recoverable_failure;
use crate::cctp::verifiers::VerifierError;
use crate::dependency_health::ExternalDependencyHealth;
use crate::error::ApiError;
use crate::kill_switch::KillSwitchManager;
use crate::metrics;
use crate::models::v2_cctp::SupportedCorridor;
use crate::models::v2_cctp::{
    CctpDirection, CctpFeeQuote, CctpPrepareBurnResponse, CctpPrepareMintResponse,
    CctpQuoteResponse, CctpReattestResponse, CctpStatusDetails, CctpSubmitBurnResponse,
    CctpSubmitMintResponse, CctpTransferStatus, CctpTransferStatusResponse, CctpValidationError,
    CCTP_PROVIDER_ID, CCTP_TESTNET_CORRIDOR_ID, SEPOLIA_CHAIN_ID, SEPOLIA_USDC_ASSET,
    SEPOLIA_USDC_CANONICAL, STELLAR_TESTNET_CHAIN_ID, STELLAR_TESTNET_USDC_ASSET,
    STELLAR_TESTNET_USDC_CANONICAL,
};
use stellarroute_routing::health::policy::OverrideDirective;

pub const REATTEST_MAX_ATTEMPTS: u32 = 5;
pub const REATTEST_COOLDOWN_SECS: i64 = 60;
pub const REATTEST_LEASE_SECS: i64 = 30;
pub const CCTP_CHAIN_KILL_STELLAR: &str = "cctp:chain:stellar-testnet";
pub const CCTP_CHAIN_KILL_SEPOLIA: &str = "cctp:chain:sepolia";
pub const POLL_LEASE_SECS: i64 = 15;

pub fn reattest_lease_secs() -> i64 {
    std::env::var("CCTP_REATTEST_LEASE_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&v| v > 0)
        .unwrap_or(REATTEST_LEASE_SECS)
}

/// Direction-specific executability — all mandatory prepare+verify components ready.
pub fn direction_executable(
    runtime: &CctpRuntime,
    config: &CctpConfig,
    direction: CctpDirection,
) -> bool {
    config.enabled && config.is_configured() && runtime.assess(direction).is_ready()
}

pub fn any_direction_executable(runtime: &CctpRuntime, config: &CctpConfig) -> bool {
    direction_executable(runtime, config, CctpDirection::StellarToEvm)
        || direction_executable(runtime, config, CctpDirection::EvmToStellar)
}

/// Runtime public executability including kill switches and dependency health.
pub async fn bridge_settlement_publicly_executable(
    runtime: &CctpRuntime,
    config: &CctpConfig,
    kill_switch: &KillSwitchManager,
    dependency_health: &ExternalDependencyHealth,
    provider_killed: bool,
) -> bool {
    if provider_killed || !config.enabled || !config.is_configured() {
        return false;
    }
    if dependency_health.guard_live_path().is_err() {
        return false;
    }
    for direction in [CctpDirection::StellarToEvm, CctpDirection::EvmToStellar] {
        if direction_publicly_executable(
            runtime,
            config,
            kill_switch,
            dependency_health,
            direction,
            provider_killed,
        )
        .await
        {
            return true;
        }
    }
    false
}

pub async fn supported_corridors_with_gates(
    runtime: &CctpRuntime,
    config: &CctpConfig,
    kill_switch: &KillSwitchManager,
    dependency_health: &ExternalDependencyHealth,
    provider_killed: bool,
) -> Vec<SupportedCorridor> {
    let mut out = Vec::new();
    for direction in [CctpDirection::StellarToEvm, CctpDirection::EvmToStellar] {
        let mut corridor = corridor_descriptor(direction, runtime, config);
        corridor.executable = direction_publicly_executable(
            runtime,
            config,
            kill_switch,
            dependency_health,
            direction,
            provider_killed,
        )
        .await;
        out.push(corridor);
    }
    out
}

/// Single bounded snapshot for `/api/v2` metadata (one provider-policy read).
pub async fn cctp_public_executability_snapshot(
    service: &CctpService,
    kill_switch: &KillSwitchManager,
    dependency_health: &ExternalDependencyHealth,
) -> (Vec<SupportedCorridor>, bool) {
    let config = &service.config;
    if !config.enabled || !config.is_configured() {
        return (vec![], false);
    }
    let provider_killed = service.provider_killed().await;
    let corridors = supported_corridors_with_gates(
        &service.runtime,
        config,
        kill_switch,
        dependency_health,
        provider_killed,
    )
    .await;
    let executable = bridge_settlement_publicly_executable(
        &service.runtime,
        config,
        kill_switch,
        dependency_health,
        provider_killed,
    )
    .await;
    (corridors, executable)
}

/// Per-direction public executability: readiness + symmetric source/dest chain kills + chain health.
pub async fn direction_publicly_executable(
    runtime: &CctpRuntime,
    config: &CctpConfig,
    kill_switch: &KillSwitchManager,
    dependency_health: &ExternalDependencyHealth,
    direction: CctpDirection,
    provider_killed: bool,
) -> bool {
    if provider_killed {
        return false;
    }
    direction_executable(runtime, config, direction)
        && !corridor_chain_killed(kill_switch, direction).await
        && dependency_health.guard_cctp_direction(direction).is_ok()
}

pub fn supported_corridors(runtime: &CctpRuntime, config: &CctpConfig) -> Vec<SupportedCorridor> {
    vec![
        corridor_descriptor(CctpDirection::StellarToEvm, runtime, config),
        corridor_descriptor(CctpDirection::EvmToStellar, runtime, config),
    ]
}

fn corridor_descriptor(
    direction: CctpDirection,
    runtime: &CctpRuntime,
    config: &CctpConfig,
) -> SupportedCorridor {
    let (source_chain_id, destination_chain_id, source_asset, destination_asset) = match direction {
        CctpDirection::StellarToEvm => (
            STELLAR_TESTNET_CHAIN_ID,
            SEPOLIA_CHAIN_ID,
            stellar_usdc_asset(),
            sepolia_usdc_asset(),
        ),
        CctpDirection::EvmToStellar => (
            SEPOLIA_CHAIN_ID,
            STELLAR_TESTNET_CHAIN_ID,
            sepolia_usdc_asset(),
            stellar_usdc_asset(),
        ),
    };

    SupportedCorridor {
        corridor_id: CCTP_TESTNET_CORRIDOR_ID.into(),
        provider: CCTP_PROVIDER_ID.into(),
        direction,
        source_chain_id: source_chain_id.into(),
        destination_chain_id: destination_chain_id.into(),
        source_asset,
        destination_asset,
        executable: direction_executable(runtime, config, direction),
    }
}

fn stellar_usdc_asset() -> crate::models::v2_cctp::CctpChainAsset {
    crate::models::v2_cctp::CctpChainAsset {
        chain_id: STELLAR_TESTNET_CHAIN_ID.into(),
        asset: STELLAR_TESTNET_USDC_ASSET.into(),
        canonical: STELLAR_TESTNET_USDC_CANONICAL.into(),
        symbol: Some("USDC".into()),
    }
}

fn sepolia_usdc_asset() -> crate::models::v2_cctp::CctpChainAsset {
    crate::models::v2_cctp::CctpChainAsset {
        chain_id: SEPOLIA_CHAIN_ID.into(),
        asset: SEPOLIA_USDC_ASSET.into(),
        canonical: SEPOLIA_USDC_CANONICAL.into(),
        symbol: Some("USDC".into()),
    }
}

/// Fail-closed gate before mutating saga state or returning executable payloads.
pub async fn ensure_public_gate(
    service: &CctpService,
    direction: CctpDirection,
    kill_switch: &KillSwitchManager,
    dependency_health: &ExternalDependencyHealth,
) -> Result<(), ApiError> {
    let config = &service.config;
    if !config.enabled {
        metrics::record_cctp_endpoint_outcome("gate", "not_enabled");
        return Err(ApiError::CctpNotEnabled(
            "Circle CCTP bridge settlement is not enabled on this deployment".into(),
        ));
    }
    if !config.is_configured() {
        metrics::record_cctp_endpoint_outcome("gate", "not_configured");
        return Err(ApiError::CctpNotEnabled(
            "Circle CCTP bridge is not fully configured on this deployment".into(),
        ));
    }
    if service.provider_killed().await {
        metrics::record_cctp_endpoint_outcome("gate", "provider_killed");
        return Err(ApiError::ProviderKilled(
            "Circle CCTP provider is temporarily unavailable".into(),
        ));
    }
    if corridor_chain_killed(kill_switch, direction).await {
        metrics::record_cctp_endpoint_outcome("gate", "chain_killed");
        return Err(ApiError::ProviderKilled(
            "CCTP corridor chain is temporarily unavailable".into(),
        ));
    }
    if let Err(err) = dependency_health.guard_live_path() {
        metrics::record_cctp_endpoint_outcome("gate", "dependency_unhealthy");
        return Err(err);
    }
    if let Err(err) = dependency_health.guard_cctp_direction(direction) {
        metrics::record_cctp_endpoint_outcome("gate", "chain_dependency_unhealthy");
        return Err(err);
    }
    if !direction_executable(&service.runtime, config, direction) {
        metrics::record_cctp_endpoint_outcome("gate", "direction_not_ready");
        return Err(ApiError::CctpNotEnabled(
            "Circle CCTP corridor dependencies are not ready for this direction".into(),
        ));
    }
    Ok(())
}

pub(crate) async fn corridor_chain_killed(
    kill_switch: &KillSwitchManager,
    direction: CctpDirection,
) -> bool {
    let state = kill_switch.get_state().await;
    let stellar_killed = matches!(
        state.venues.get(CCTP_CHAIN_KILL_STELLAR),
        Some(OverrideDirective::ForceExclude)
    );
    let sepolia_killed = matches!(
        state.venues.get(CCTP_CHAIN_KILL_SEPOLIA),
        Some(OverrideDirective::ForceExclude)
    );
    match direction {
        // Stellar source + Sepolia destination must both be live.
        CctpDirection::StellarToEvm => stellar_killed || sepolia_killed,
        // Sepolia source + Stellar destination must both be live.
        CctpDirection::EvmToStellar => sepolia_killed || stellar_killed,
    }
}

pub fn uniform_transfer_not_found(transfer_id: Uuid) -> ApiError {
    metrics::record_cctp_endpoint_outcome("access", "not_found");
    ApiError::TransferNotFound {
        transfer_id: transfer_id.to_string(),
    }
}

pub fn hash_presented_access_token(
    transfer_id: Uuid,
    token: Option<&str>,
) -> Result<String, ApiError> {
    let token = token.ok_or_else(|| uniform_transfer_not_found(transfer_id))?;
    validate_access_token_format(token).map_err(|_| uniform_transfer_not_found(transfer_id))?;
    Ok(hash_access_token(token))
}

pub fn map_reattest_denied(transfer_id: Uuid) -> ApiError {
    let _ = transfer_id;
    ApiError::Validation(
        "Re-attestation is not allowed (cooldown, attempt limit, or invalid state)".into(),
    )
}

pub fn map_service_error(err: CctpServiceError, transfer_id: Option<Uuid>) -> ApiError {
    match err {
        CctpServiceError::NotEnabled => ApiError::CctpNotEnabled(
            "Circle CCTP bridge settlement is not enabled on this deployment".into(),
        ),
        CctpServiceError::ProviderKilled => {
            ApiError::ProviderKilled("Circle CCTP provider is temporarily unavailable".into())
        }
        CctpServiceError::Validation(v) => map_validation(v),
        CctpServiceError::FeeQuoteUnavailable => {
            ApiError::FeeQuoteUnavailable("Runtime CCTP fee quote is unavailable".into())
        }
        CctpServiceError::VerifiersNotReady => ApiError::CctpNotEnabled(
            "Circle CCTP verifiers are not ready on this deployment".into(),
        ),
        CctpServiceError::NotFound => ApiError::TransferNotFound {
            transfer_id: transfer_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "unknown".into()),
        },
        CctpServiceError::InvalidState => {
            ApiError::Validation("Transfer is not in a valid state for this operation".into())
        }
        CctpServiceError::QuoteExpired => ApiError::QuoteExpired {
            quote_id: transfer_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "unknown".into()),
        },
        CctpServiceError::FeeExpired => {
            ApiError::FeeQuoteUnavailable("CCTP fee quote has expired; request a new quote".into())
        }
        CctpServiceError::MintPayloadExpired => {
            ApiError::Validation("Mint payload has expired; call prepare-mint again".into())
        }
        CctpServiceError::AttestationTimeout => ApiError::AttestationExpired {
            transfer_id: transfer_id.map(|id| id.to_string()).unwrap_or_default(),
        },
        CctpServiceError::MissingAttestation | CctpServiceError::Attestation(_) => {
            ApiError::AttestationPending {
                transfer_id: transfer_id.map(|id| id.to_string()).unwrap_or_default(),
            }
        }
        CctpServiceError::MintRetryable => ApiError::MintRetryable {
            transfer_id: transfer_id.map(|id| id.to_string()).unwrap_or_default(),
        },
        CctpServiceError::ActivePrepareExists => {
            ApiError::Validation("An active prepare already exists for this source account".into())
        }
        CctpServiceError::AmountExceedsCap => {
            ApiError::InvalidAmount("amount exceeds configured cap".into())
        }
        CctpServiceError::FastNotSupported => ApiError::InvalidFinality,
        CctpServiceError::StellarRemainder => ApiError::Validation(
            "Stellar outbound amount must have zero 7th-decimal remainder".into(),
        ),
        CctpServiceError::Iris(_) => ApiError::DependencyUnavailable(
            "Circle attestation service is temporarily unavailable".into(),
        ),
        CctpServiceError::Verifier(VerifierError::Transient(_))
        | CctpServiceError::Verifier(VerifierError::TxNotFound) => ApiError::DependencyUnavailable(
            "Submitted transaction is not yet available for verification; retry shortly".into(),
        ),
        CctpServiceError::Builder(err) => map_builder_error(err),
        CctpServiceError::Verifier(VerifierError::Failed(msg))
            if msg.contains("wrong function") || msg.contains("unsupported stellar invoke") =>
        {
            ApiError::Validation(
                "Submitted transaction is a USDC approval, not a burn. Click Prepare source transaction, then sign the burn."
                    .into(),
            )
        }
        CctpServiceError::InvalidMessage(reason) => {
            ApiError::Validation(format!("CCTP message validation failed: {reason}"))
        }
        CctpServiceError::Verifier(_)
        | CctpServiceError::IrisTxHashMismatch
        | CctpServiceError::MintPayloadHashMismatch => {
            ApiError::Validation("On-chain verification failed for submitted transaction".into())
        }
        CctpServiceError::Store(_) => ApiError::Internal(std::sync::Arc::new(anyhow::anyhow!(
            "CCTP persistence error"
        ))),
    }
}

fn map_builder_error(err: BuilderError) -> ApiError {
    match err {
        BuilderError::NotReady => ApiError::CctpNotEnabled(
            "Circle CCTP transaction builder is not ready on this deployment".into(),
        ),
        BuilderError::QuoteExpired => ApiError::QuoteExpired {
            quote_id: "cctp-transfer".into(),
        },
        BuilderError::FeeExpired => ApiError::FeeQuoteUnavailable(
            "CCTP fee quote has expired; request a new quote".into(),
        ),
        BuilderError::Validation(msg) => {
            if msg.contains("sender required") {
                ApiError::Validation(
                    "Connect your source wallet and request a new quote before preparing the burn"
                        .into(),
                )
            } else {
                ApiError::Validation(msg)
            }
        }
        BuilderError::SimulationFailed(msg) => {
            let lower = msg.to_ascii_lowercase();
            if lower.contains("insufficient") || lower.contains("balance") {
                ApiError::Validation(
                    "Insufficient USDC balance on Stellar for this burn amount".into(),
                )
            } else {
                ApiError::Validation(format!(
                    "Could not prepare source transaction: Soroban simulation failed ({msg})"
                ))
            }
        }
        BuilderError::AccountLookup(msg) => ApiError::DependencyUnavailable(format!(
            "Stellar account lookup failed while preparing the burn: {msg}"
        )),
        BuilderError::Encoding(msg) => ApiError::Validation(format!(
            "Could not encode CCTP source transaction: {msg}"
        )),
    }
}

fn map_validation(err: CctpValidationError) -> ApiError {
    match err {
        CctpValidationError::UnsupportedCorridor => ApiError::UnsupportedCorridor,
        CctpValidationError::InvalidFinality => ApiError::InvalidFinality,
        CctpValidationError::InvalidRecipient => ApiError::InvalidRecipient,
        CctpValidationError::InvalidAmount => {
            ApiError::InvalidAmount("amount must be a positive decimal string".into())
        }
        CctpValidationError::InvalidSender => ApiError::Validation(
            "sender must be a valid G-address for Stellar or 0x address for EVM source".into(),
        ),
        CctpValidationError::InvalidMintSubmitter => ApiError::Validation(
            "mint_submitter must be a valid Stellar G-address for evm_to_stellar".into(),
        ),
        CctpValidationError::StellarRemainder => ApiError::Validation(
            "Stellar outbound amount must have zero 7th-decimal remainder".into(),
        ),
    }
}

pub fn to_quote_response(transfer: &CctpTransfer, access_token: &str) -> CctpQuoteResponse {
    CctpQuoteResponse {
        transfer_id: transfer.transfer_id.to_string(),
        corridor_id: transfer.corridor_id.clone(),
        provider: transfer.provider.clone(),
        direction: transfer.direction,
        source_amount: transfer.amount.clone(),
        destination_amount: transfer.destination_amount.clone(),
        fee_quote: fee_quote_from_transfer(transfer),
        expires_at: transfer.quote_expires_at.timestamp(),
        finality: transfer.finality,
        access_token: access_token.to_string(),
    }
}

pub fn to_status_response(transfer: &CctpTransfer) -> CctpTransferStatusResponse {
    let retryable = matches!(
        transfer.status,
        CctpTransferStatus::AttestationFailed
            | CctpTransferStatus::MintFailedRetryable
            | CctpTransferStatus::AwaitingAttestation
            | CctpTransferStatus::MintSubmitted
    );

    CctpTransferStatusResponse {
        transfer_id: transfer.transfer_id.to_string(),
        corridor_id: transfer.corridor_id.clone(),
        provider: transfer.provider.clone(),
        direction: transfer.direction,
        status: transfer.status,
        source_tx_hash: transfer.source_tx_hash.clone(),
        destination_tx_hash: transfer.destination_tx_hash.clone(),
        support_reference_id: Some(transfer.support_reference_id.clone()),
        retryable,
        error: redacted_status_error(transfer),
        reattest_cooldown_until: transfer.reattest_cooldown_until.map(|t| t.timestamp()),
    }
}

fn redacted_status_error(transfer: &CctpTransfer) -> Option<CctpStatusDetails> {
    let code = transfer.last_provider_code.as_deref()?;
    let safe_codes = [
        "poll_timeout",
        "429",
        "mint_retryable",
        "mint_reconciliation_nonce",
        "attestation_pending",
        "iris_reattest_failed",
    ];
    if !safe_codes.contains(&code) {
        return None;
    }
    let retryable = matches!(
        transfer.status,
        CctpTransferStatus::AttestationFailed
            | CctpTransferStatus::MintFailedRetryable
            | CctpTransferStatus::AwaitingAttestation
    );
    Some(CctpStatusDetails {
        code: code.to_string(),
        message: sanitized_provider_message(transfer.last_provider_error.as_deref()),
        retryable: Some(retryable),
    })
}

fn sanitized_provider_message(raw: Option<&str>) -> String {
    match raw {
        Some(msg) if !msg.contains("http") && !msg.contains("0x") && msg.len() <= 200 => {
            msg.to_string()
        }
        _ => "Provider operation pending or failed".into(),
    }
}

fn fee_quote_from_transfer(transfer: &CctpTransfer) -> CctpFeeQuote {
    let fee_asset = match transfer.direction {
        CctpDirection::StellarToEvm => Some(stellar_usdc_asset()),
        CctpDirection::EvmToStellar => Some(sepolia_usdc_asset()),
    };
    CctpFeeQuote {
        source_fee: transfer.runtime_fee_quote.clone(),
        destination_fee: None,
        bridge_fee: transfer.max_fee.clone(),
        fee_asset,
    }
}

pub fn to_prepare_burn_response(
    transfer: &CctpTransfer,
    bundle: &crate::cctp::builders::PreparedBurnBundle,
) -> CctpPrepareBurnResponse {
    CctpPrepareBurnResponse {
        transfer_id: transfer.transfer_id.to_string(),
        status: transfer.status,
        payload: bundle.primary.clone(),
        expires_at: bundle.expires_at,
        approval_required: bundle.step == BurnPrepareStep::Approval,
    }
}

pub fn to_prepare_mint_response(
    transfer: &CctpTransfer,
    bundle: &crate::cctp::builders::PreparedMintBundle,
) -> CctpPrepareMintResponse {
    CctpPrepareMintResponse {
        transfer_id: transfer.transfer_id.to_string(),
        status: transfer.status,
        payload: bundle.primary.clone(),
        expires_at: bundle.expires_at,
        trustline_required: bundle.trustline_required
            || bundle.step == crate::cctp::builders::MintPrepareStep::Trustline,
    }
}

pub fn to_submit_burn_response(transfer: &CctpTransfer) -> CctpSubmitBurnResponse {
    CctpSubmitBurnResponse {
        transfer_id: transfer.transfer_id.to_string(),
        status: transfer.status,
        source_tx_hash: transfer
            .source_tx_hash
            .clone()
            .or_else(|| transfer.source_approval_tx_hash.clone())
            .unwrap_or_default(),
    }
}

pub fn to_submit_mint_response(transfer: &CctpTransfer) -> CctpSubmitMintResponse {
    CctpSubmitMintResponse {
        transfer_id: transfer.transfer_id.to_string(),
        status: transfer.status,
        destination_tx_hash: transfer.destination_tx_hash.clone().unwrap_or_default(),
    }
}

pub fn to_reattest_response(transfer: &CctpTransfer) -> CctpReattestResponse {
    CctpReattestResponse {
        transfer_id: transfer.transfer_id.to_string(),
        status: transfer.status,
        retryable: is_recoverable_failure(transfer.status)
            || transfer.status == CctpTransferStatus::AwaitingAttestation,
    }
}

/// Legacy helper for `/api/v2` info when only runtime is available (tests).
pub fn bridge_settlement_executable(runtime: &CctpRuntime, config: &CctpConfig) -> bool {
    any_direction_executable(runtime, config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cctp::access::generate_access_token;
    use chrono::Utc;
    use stellarroute_routing::health::policy::OverrideDirective;

    async fn set_chain_kill(kill_switch: &KillSwitchManager, venue: &str) {
        let mut state = kill_switch.get_state().await;
        state
            .venues
            .insert(venue.to_string(), OverrideDirective::ForceExclude);
        kill_switch.update_state(state).await.unwrap();
    }

    async fn set_provider_kill(kill_switch: &KillSwitchManager, provider: &str) {
        let mut state = kill_switch.get_state().await;
        state
            .providers
            .insert(provider.to_string(), OverrideDirective::ForceExclude);
        kill_switch.update_state(state).await.unwrap();
    }

    fn sample_transfer(status: CctpTransferStatus) -> CctpTransfer {
        let (token, hash) = generate_access_token();
        let _ = token;
        CctpTransfer {
            transfer_id: Uuid::new_v4(),
            support_reference_id: "cctp-test".into(),
            corridor_id: CCTP_TESTNET_CORRIDOR_ID.into(),
            provider: CCTP_PROVIDER_ID.into(),
            direction: CctpDirection::StellarToEvm,
            source_chain_id: STELLAR_TESTNET_CHAIN_ID.into(),
            destination_chain_id: SEPOLIA_CHAIN_ID.into(),
            source_asset: STELLAR_TESTNET_USDC_ASSET.into(),
            source_asset_canonical: STELLAR_TESTNET_USDC_CANONICAL.into(),
            destination_asset: SEPOLIA_USDC_ASSET.into(),
            destination_asset_canonical: SEPOLIA_USDC_CANONICAL.into(),
            sender: "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF".into(),
            recipient: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0".into(),
            mint_submitter: None,
            amount: "1.0".into(),
            destination_amount: "1.0".into(),
            finality: crate::models::v2_cctp::CctpFinality::Standard,
            runtime_fee_quote: Some("1".into()),
            max_fee: Some("1".into()),
            fee_expires_at: Some(Utc::now()),
            quote_expires_at: Utc::now(),
            status,
            source_tx_hash: None,
            source_approval_tx_hash: None,
            source_approval_verified_at: None,
            destination_tx_hash: None,
            iris_message_hash: None,
            message_nonce: None,
            raw_message: Some(vec![1, 2, 3]),
            attestation: Some(vec![4, 5, 6]),
            retry_count: 0,
            last_provider_error: Some("secret http://evil".into()),
            last_provider_code: Some("poll_timeout".into()),
            version: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            terminal_at: None,
            mint_payload_hash: None,
            mint_payload_expires_at: None,
            approval_payload_hash: None,
            approval_expiration_ledger: None,
            burn_payload_hash: None,
            burn_prepare_step: None,
            access_token_hash: Some(hash),
            last_polled_at: None,
            poll_lease_until: None,
            reattest_lease_owner_hash: None,
            reattest_lease_until: None,
            reattest_attempt_count: 0,
            reattest_cooldown_until: None,
        }
    }

    #[test]
    fn status_response_includes_reattest_cooldown_when_set() {
        let mut transfer = sample_transfer(CctpTransferStatus::AttestationFailed);
        transfer.reattest_cooldown_until = Some(Utc::now() + chrono::Duration::seconds(120));
        let json = serde_json::to_string(&to_status_response(&transfer)).unwrap();
        assert!(json.contains("reattest_cooldown_until"));
        assert!(!json.contains("reattest_lease_owner"));
        assert!(!json.contains("access_token"));
    }

    #[test]
    fn status_response_never_leaks_raw_message_or_urls() {
        let json = serde_json::to_string(&to_status_response(&sample_transfer(
            CctpTransferStatus::AwaitingAttestation,
        )))
        .unwrap();
        assert!(!json.contains("raw_message"));
        assert!(!json.contains("\"attestation\""));
        assert!(!json.contains("http://"));
        assert!(!json.contains("access_token_hash"));
    }

    #[tokio::test]
    async fn corridor_chain_kill_blocks_both_directions_symmetrically() {
        let kill = KillSwitchManager::new(None);
        assert!(!corridor_chain_killed(&kill, CctpDirection::StellarToEvm).await);
        assert!(!corridor_chain_killed(&kill, CctpDirection::EvmToStellar).await);

        set_chain_kill(&kill, CCTP_CHAIN_KILL_STELLAR).await;
        assert!(corridor_chain_killed(&kill, CctpDirection::StellarToEvm).await);
        assert!(corridor_chain_killed(&kill, CctpDirection::EvmToStellar).await);

        let kill2 = KillSwitchManager::new(None);
        set_chain_kill(&kill2, CCTP_CHAIN_KILL_SEPOLIA).await;
        assert!(corridor_chain_killed(&kill2, CctpDirection::StellarToEvm).await);
        assert!(corridor_chain_killed(&kill2, CctpDirection::EvmToStellar).await);
    }

    #[tokio::test]
    async fn direction_dependency_guard_blocks_each_corridor_side() {
        use crate::models::v2_cctp::CctpDirection;

        let health = ExternalDependencyHealth::new(vec![], vec![]);
        assert!(health
            .guard_cctp_direction(CctpDirection::StellarToEvm)
            .is_ok());

        for _ in 0..3 {
            health.record_soroban_result(false);
        }
        assert!(health
            .guard_cctp_direction(CctpDirection::StellarToEvm)
            .is_err());
        assert!(health
            .guard_cctp_direction(CctpDirection::EvmToStellar)
            .is_err());

        let health2 = ExternalDependencyHealth::new(vec![], vec![]);
        for _ in 0..3 {
            health2.record_evm_rpc_result(false);
        }
        assert!(health2
            .guard_cctp_direction(CctpDirection::StellarToEvm)
            .is_err());
        assert!(health2
            .guard_cctp_direction(CctpDirection::EvmToStellar)
            .is_err());
    }

    #[tokio::test]
    async fn provider_kill_snapshot_marks_all_corridors_non_executable() {
        use std::sync::Arc;

        use crate::cctp::idempotency::InMemoryCctpQuoteIdempotencyStore;
        use crate::cctp::iris::{IrisClient, IrisFeeQuote, IrisPollOutcome};
        use crate::cctp::prepare_lock::InMemoryCctpPrepareLockStore;
        use crate::cctp::store::InMemoryCctpTransferStore;

        struct MockIris;
        #[async_trait::async_trait]
        impl IrisClient for MockIris {
            async fn fetch_burn_fees(
                &self,
                _: u32,
                _: u32,
            ) -> Result<IrisFeeQuote, crate::cctp::iris::IrisError> {
                Ok(IrisFeeQuote {
                    standard_fee: "1".into(),
                    fast_fee: None,
                })
            }
            async fn poll_messages_by_tx(
                &self,
                _: u32,
                _: &str,
            ) -> Result<IrisPollOutcome, crate::cctp::iris::IrisError> {
                Ok(IrisPollOutcome::Pending)
            }
            async fn reattest(&self, _: &str) -> Result<(), crate::cctp::iris::IrisError> {
                Ok(())
            }
        }

        let kill = Arc::new(KillSwitchManager::new(None));
        let mut cfg = CctpConfig::default_testnet();
        cfg.enabled = true;
        cfg.sepolia_rpc_url = "https://sepolia.drpc.org".into();
        let mut runtime = CctpRuntime::from_config(&cfg);
        runtime.attestation_verifier =
            Arc::new(crate::cctp::attestation::FakeAttestationVerifier { ready: true });
        struct ReadyStellarMintBuilder;
        #[async_trait::async_trait]
        impl crate::cctp::builders::StellarCctpMintBuilder for ReadyStellarMintBuilder {
            fn is_ready(&self) -> bool {
                true
            }
            async fn prepare_mint(
                &self,
                transfer: &CctpTransfer,
                config: &CctpConfig,
            ) -> Result<
                crate::cctp::builders::PreparedMintBundle,
                crate::cctp::builders::BuilderError,
            > {
                Ok(crate::cctp::builders::PreparedMintBundle {
                    step: crate::cctp::builders::MintPrepareStep::Mint,
                    trustline_required: false,
                    primary: crate::models::v2_cctp::PreparedWalletPayload::StellarXdr {
                        network_passphrase: config.stellar_network_passphrase.clone(),
                        xdr_envelope: "AAAA".into(),
                        source: None,
                    },
                    expires_at: transfer.quote_expires_at.timestamp(),
                    payload_hash: "test".into(),
                })
            }
        }
        runtime.stellar_mint_builder = Arc::new(ReadyStellarMintBuilder);
        runtime.stellar_mint_verifier = Arc::new(crate::cctp::verifiers::FakeMintVerifier {
            facts: crate::cctp::verifiers::VerifiedMintFacts {
                tx_hash: "mint".into(),
                destination_chain_id: STELLAR_TESTNET_CHAIN_ID.into(),
                contract_address: "c".into(),
                function_selector: "mint".into(),
                message_hash: [0; 32],
                attestation_hash: [0; 32],
                nonce: "n".into(),
                payload_hash: "p".into(),
                outcome: crate::cctp::verifiers::MintVerifyOutcome::Pending,
                recipient_evidence: None,
            },
            completion: crate::cctp::verifiers::MintVerifyOutcome::Pending,
            ready: true,
        });
        struct ReadyEvmBurnBuilder;
        #[async_trait::async_trait]
        impl crate::cctp::builders::EvmCctpBurnBuilder for ReadyEvmBurnBuilder {
            fn is_ready(&self) -> bool {
                true
            }
            async fn prepare_burn(
                &self,
                _: &CctpTransfer,
                _: &CctpConfig,
            ) -> Result<
                crate::cctp::builders::PreparedBurnBundle,
                crate::cctp::builders::BuilderError,
            > {
                Err(crate::cctp::builders::BuilderError::NotReady)
            }
        }
        runtime.evm_burn_builder = Arc::new(ReadyEvmBurnBuilder);
        runtime.evm_burn_verifier = Arc::new(crate::cctp::verifiers::FakeBurnVerifier {
            facts: crate::cctp::verifiers::VerifiedBurnFacts {
                tx_hash: "burn".into(),
                source_chain_id: SEPOLIA_CHAIN_ID.into(),
                source_domain: 0,
                destination_domain: 27,
                sender: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0".into(),
                amount_cctp_subunits: 1,
                burn_token_bytes32: [0; 32],
                mint_recipient_bytes32: [0; 32],
                destination_caller_bytes32: [0; 32],
                min_finality_threshold: 2000,
                hook_data: None,
                token_messenger_bytes32: [0; 32],
                block_or_ledger: None,
            },
            ready: true,
        });
        runtime.evm_approval_verifier = Arc::new(crate::cctp::approval::FakeApprovalVerifier {
            facts: crate::cctp::approval::VerifiedApprovalFacts {
                tx_hash: "approve".into(),
                owner: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0".into(),
                token_contract: cfg.contracts.sepolia_usdc.clone(),
                spender_contract: cfg.contracts.sepolia_token_messenger.clone(),
                amount: 1,
                chain_id: SEPOLIA_CHAIN_ID.into(),
            },
            ready: true,
        });
        let service = CctpService {
            config: cfg.clone(),
            store: Arc::new(InMemoryCctpTransferStore::default()),
            prepare_lock: Arc::new(InMemoryCctpPrepareLockStore::default()),
            iris: Arc::new(MockIris),
            kill_switch: kill.clone(),
            runtime,
        };
        let health = ExternalDependencyHealth::new(vec![], vec![]);

        let (_, exec_before) = cctp_public_executability_snapshot(&service, &kill, &health).await;
        assert!(exec_before);

        set_provider_kill(&kill, CCTP_PROVIDER_ID).await;
        let (corridors, exec_after) =
            cctp_public_executability_snapshot(&service, &kill, &health).await;
        assert!(!exec_after);
        assert_eq!(corridors.len(), 2);
        assert!(corridors.iter().all(|c| !c.executable));
    }

    #[test]
    fn builder_simulation_failure_maps_to_prepare_message() {
        let err = map_service_error(
            CctpServiceError::Builder(BuilderError::SimulationFailed(
                "insufficient balance".into(),
            )),
            None,
        );
        match err {
            ApiError::Validation(msg) => {
                assert!(msg.contains("Insufficient USDC balance"));
            }
            other => panic!("expected validation, got {other:?}"),
        }
    }

    #[test]
    fn verifier_tx_not_found_is_retryable_dependency() {
        let err = map_service_error(
            CctpServiceError::Verifier(VerifierError::TxNotFound),
            None,
        );
        match err {
            ApiError::DependencyUnavailable(msg) => {
                assert!(msg.contains("not yet available"));
            }
            other => panic!("expected dependency_unavailable, got {other:?}"),
        }
    }
}
