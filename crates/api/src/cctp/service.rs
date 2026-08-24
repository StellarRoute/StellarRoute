//! CCTP core service — quote, burn/mint preparation, attestation polling, verification.

use std::sync::Arc;

use chrono::{Duration, Utc};
use thiserror::Error;
use uuid::Uuid;

use crate::cctp::bounds::{check_byte_len, MAX_ATTESTATION_BYTES, MAX_RAW_MESSAGE_BYTES};
use crate::cctp::builders::{
    BuilderError, BurnPrepareStep, PreparedBurnBundle, PreparedMintBundle,
};
use crate::cctp::config::{CctpConfig, SEPOLIA_DOMAIN, STELLAR_TESTNET_DOMAIN};
use crate::cctp::encoding::{
    cctp_subunits_to_stellar_subunits, decimal_to_cctp_subunits,
    stellar_outbound_cctp_amount_strict,
};
use crate::cctp::expectations::{build_corridor_expectations, build_expected_burn_facts};
use crate::cctp::iris::{IrisClient, IrisMessage, IrisMessageStatus, IrisPollOutcome};
use crate::cctp::message::{decode_hex_message, validate_message_for_corridor};
use crate::cctp::prepare_lock::{
    CctpActivePrepare, CctpPrepareKind, CctpPrepareLockError, CctpPrepareLockStore,
    PrepareAcquireResult,
};
use crate::cctp::prepare_payload_cache::{
    deserialize_burn_bundle, deserialize_mint_bundle, serialize_burn_bundle, serialize_mint_bundle,
    PreparePayloadCacheError,
};
use crate::cctp::readiness::CctpRuntime;
use crate::cctp::stellar_payload::payload_hash_from_envelope_xdr;
use crate::cctp::stellar_rpc::StellarRpcClient;
use crate::cctp::stellar_tx::{parse_invoke_envelope, TxStatus};
use crate::cctp::store::{CctpStoreError, CctpTransfer, CctpTransferStore, TransferPatch};
use crate::cctp::transitions::can_cancel;
use crate::cctp::verifiers::{facts_match, MintVerifyOutcome, VerifierError};
use crate::kill_switch::KillSwitchManager;
use crate::metrics;
use crate::models::v2_cctp::{
    CctpDirection, CctpFinality, CctpQuoteRequest, CctpTransferStatus, CctpValidationError,
    PreparedWalletPayload,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StellarSourceSubmissionKind {
    Approval,
    Burn,
}

#[derive(Debug, Error)]
pub enum CctpServiceError {
    #[error("not enabled")]
    NotEnabled,
    #[error("provider killed")]
    ProviderKilled,
    #[error("validation: {0:?}")]
    Validation(CctpValidationError),
    #[error("fee quote unavailable")]
    FeeQuoteUnavailable,
    #[error("store: {0}")]
    Store(CctpStoreError),
    #[error("iris: {0}")]
    Iris(String),
    #[error("verifier: {0}")]
    Verifier(VerifierError),
    #[error("attestation: {0}")]
    Attestation(crate::cctp::attestation::AttestationVerifyError),
    #[error("builder: {0}")]
    Builder(BuilderError),
    #[error("verifiers not ready")]
    VerifiersNotReady,
    #[error("message validation failed: {0}")]
    InvalidMessage(String),
    #[error("amount exceeds cap")]
    AmountExceedsCap,
    #[error("fast finality not supported")]
    FastNotSupported,
    #[error("not found")]
    NotFound,
    #[error("invalid state")]
    InvalidState,
    #[error("attestation poll timeout")]
    AttestationTimeout,
    #[error("iris source tx hash mismatch")]
    IrisTxHashMismatch,
    #[error("missing attestation")]
    MissingAttestation,
    #[error("quote expired")]
    QuoteExpired,
    #[error("fee quote expired")]
    FeeExpired,
    #[error("mint payload expired")]
    MintPayloadExpired,
    #[error("mint payload hash mismatch")]
    MintPayloadHashMismatch,
    #[error("mint retryable")]
    MintRetryable,
    #[error("active prepare exists for source")]
    ActivePrepareExists,
    #[error("stellar amount remainder")]
    StellarRemainder,
}

pub struct CctpService {
    pub config: CctpConfig,
    pub store: Arc<dyn CctpTransferStore>,
    pub prepare_lock: Arc<dyn CctpPrepareLockStore>,
    pub iris: Arc<dyn IrisClient>,
    pub kill_switch: Arc<KillSwitchManager>,
    pub runtime: CctpRuntime,
}

impl CctpService {
    pub fn is_public_executable(&self) -> bool {
        self.runtime.is_public_executable(&self.config)
    }

    pub fn burn_verifier_ready(&self, direction: CctpDirection) -> bool {
        match direction {
            CctpDirection::StellarToEvm => self.runtime.stellar_burn_verifier.is_ready(),
            CctpDirection::EvmToStellar => self.runtime.evm_burn_verifier.is_ready(),
        }
    }

    pub fn attestation_verifier_ready(&self) -> bool {
        self.runtime.attestation_verifier.is_ready()
    }

    pub fn core_verifiers_ready(&self, direction: CctpDirection) -> bool {
        self.burn_verifier_ready(direction) && self.attestation_verifier_ready()
    }

    fn ensure_burn_verifier_ready(&self, direction: CctpDirection) -> Result<(), CctpServiceError> {
        if !self.burn_verifier_ready(direction) {
            return Err(CctpServiceError::VerifiersNotReady);
        }
        Ok(())
    }

    fn ensure_attestation_verifier_ready(&self) -> Result<(), CctpServiceError> {
        if !self.attestation_verifier_ready() {
            return Err(CctpServiceError::VerifiersNotReady);
        }
        Ok(())
    }

    fn ensure_quote_not_expired(transfer: &CctpTransfer) -> Result<(), CctpServiceError> {
        if Utc::now() > transfer.quote_expires_at {
            return Err(CctpServiceError::QuoteExpired);
        }
        Ok(())
    }

    fn ensure_mint_verifier_ready(&self, direction: CctpDirection) -> Result<(), CctpServiceError> {
        let ready = match direction {
            CctpDirection::StellarToEvm => self.runtime.evm_mint_verifier.is_ready(),
            CctpDirection::EvmToStellar => self.runtime.stellar_mint_verifier.is_ready(),
        };
        if !ready {
            return Err(CctpServiceError::VerifiersNotReady);
        }
        Ok(())
    }

    fn ensure_mint_payload_valid(transfer: &CctpTransfer) -> Result<(), CctpServiceError> {
        if let Some(exp) = transfer.mint_payload_expires_at {
            if Utc::now() > exp {
                return Err(CctpServiceError::MintPayloadExpired);
            }
        }
        Ok(())
    }

    fn ensure_fee_not_expired(transfer: &CctpTransfer) -> Result<(), CctpServiceError> {
        if let Some(exp) = transfer.fee_expires_at {
            if Utc::now() > exp {
                return Err(CctpServiceError::FeeExpired);
            }
        }
        Ok(())
    }

    async fn release_transfer_prepare_locks(&self, transfer: &CctpTransfer) {
        if !transfer.sender.is_empty() {
            let _ = self
                .prepare_lock
                .release(&transfer.sender, transfer.transfer_id)
                .await;
        }
        if let Some(submitter) = &transfer.mint_submitter {
            let _ = self
                .prepare_lock
                .release(submitter, transfer.transfer_id)
                .await;
        }
    }

    fn map_prepare_lock_error(err: CctpPrepareLockError) -> CctpServiceError {
        match err {
            CctpPrepareLockError::PayloadHashMismatch
            | CctpPrepareLockError::ActivePrepareExists => CctpServiceError::ActivePrepareExists,
            CctpPrepareLockError::PayloadTooLarge => {
                CctpServiceError::Store(CctpStoreError::PayloadTooLarge)
            }
            CctpPrepareLockError::Database(msg) => {
                CctpServiceError::Store(CctpStoreError::Database(sqlx::Error::Protocol(msg)))
            }
        }
    }

    fn map_payload_cache_error(err: PreparePayloadCacheError) -> CctpServiceError {
        match err {
            PreparePayloadCacheError::TooLarge => {
                CctpServiceError::Store(CctpStoreError::PayloadTooLarge)
            }
            PreparePayloadCacheError::Serialization(msg) => CctpServiceError::Verifier(
                VerifierError::Failed(format!("prepare payload cache: {msg}")),
            ),
        }
    }

    async fn try_acquire_prepare_lock(
        &self,
        reservation: CctpActivePrepare,
    ) -> Result<PrepareAcquireResult, CctpServiceError> {
        self.prepare_lock
            .try_acquire(&reservation)
            .await
            .map_err(Self::map_prepare_lock_error)
    }

    async fn active_mint_bundle_for_transfer(
        &self,
        transfer_id: Uuid,
        submitter: &str,
    ) -> Result<Option<PreparedMintBundle>, CctpServiceError> {
        let active = self
            .prepare_lock
            .get_for_transfer(transfer_id)
            .await
            .map_err(Self::map_prepare_lock_error)?;
        let Some(active) = active else {
            return Ok(None);
        };
        if active.source_account != submitter || active.expires_at <= Utc::now() {
            return Ok(None);
        }
        let Some(payload) = active.prepared_payload else {
            return Ok(None);
        };
        let bundle =
            deserialize_mint_bundle(&payload).map_err(Self::map_payload_cache_error)?;
        // Never return a cached trustline step — re-probe Horizon each prepare.
        if bundle.trustline_required
            || bundle.step == crate::cctp::builders::MintPrepareStep::Trustline
        {
            return Ok(None);
        }
        Ok(Some(bundle))
    }

    fn burn_reservation(
        source: &str,
        transfer_id: Uuid,
        kind: CctpPrepareKind,
        payload_hash: String,
        prepared_payload: String,
        expires_at: chrono::DateTime<Utc>,
    ) -> CctpActivePrepare {
        CctpActivePrepare {
            source_account: source.to_string(),
            transfer_id,
            kind,
            payload_hash,
            prepared_payload: Some(prepared_payload),
            expires_at,
            updated_at: Utc::now(),
        }
    }

    pub async fn provider_killed(&self) -> bool {
        let policy = self.kill_switch.get_provider_policy().await;
        !policy.is_provider_allowed(Some(self.config.provider_id()))
    }

    /// `created` → `burn_prepared` when config/verifiers allow (no wallet payload yet).
    pub async fn prepare_burn(&self, transfer_id: Uuid) -> Result<CctpTransfer, CctpServiceError> {
        if !self.config.enabled || !self.config.is_configured() {
            return Err(CctpServiceError::NotEnabled);
        }
        if self.provider_killed().await {
            metrics::record_cctp_provider_killed_new_transfer();
            return Err(CctpServiceError::ProviderKilled);
        }

        let transfer = self
            .store
            .get(transfer_id)
            .await
            .map_err(CctpServiceError::Store)?
            .ok_or(CctpServiceError::NotFound)?;

        if transfer.status == CctpTransferStatus::BurnPrepared {
            return Ok(transfer);
        }
        if transfer.status != CctpTransferStatus::Created {
            return Err(CctpServiceError::InvalidState);
        }

        Self::ensure_quote_not_expired(&transfer)?;
        Self::ensure_fee_not_expired(&transfer)?;
        self.ensure_burn_verifier_ready(transfer.direction)?;

        let updated = self
            .store
            .transition(
                transfer_id,
                transfer.version,
                CctpTransferStatus::BurnPrepared,
                TransferPatch::default(),
            )
            .await
            .map_err(CctpServiceError::Store)?;

        metrics::record_cctp_transition("burn_prepared");
        Ok(updated)
    }

    /// Internal quote-core — creates durable transfer with access token binding.
    pub async fn quote_core(
        &self,
        request: &CctpQuoteRequest,
        access_token_hash: String,
    ) -> Result<CctpTransfer, CctpServiceError> {
        let transfer = self
            .build_quote_transfer(request, Uuid::new_v4(), access_token_hash)
            .await?;
        self.store
            .insert(&transfer)
            .await
            .map_err(CctpServiceError::Store)?;
        metrics::record_cctp_transition("created");
        Ok(transfer)
    }

    /// Build transfer row without persisting (idempotent finalize path).
    pub async fn build_quote_transfer(
        &self,
        request: &CctpQuoteRequest,
        transfer_id: Uuid,
        access_token_hash: String,
    ) -> Result<CctpTransfer, CctpServiceError> {
        if !self.config.enabled {
            return Err(CctpServiceError::NotEnabled);
        }
        if self.provider_killed().await {
            metrics::record_cctp_provider_killed_new_transfer();
            return Err(CctpServiceError::ProviderKilled);
        }
        request.validate().map_err(CctpServiceError::Validation)?;

        // Both corridor directions may quote Fast; Iris supplies the fee tier.
        // (Builder encodes corridor_min_finality(transfer.finality) on the burn.)

        let cctp_amount = match request.direction {
            CctpDirection::StellarToEvm => stellar_outbound_cctp_amount_strict(&request.amount)
                .map_err(|_| CctpServiceError::StellarRemainder)?,
            CctpDirection::EvmToStellar => decimal_to_cctp_subunits(&request.amount)
                .map_err(|_| CctpServiceError::Validation(CctpValidationError::InvalidAmount))?,
        };

        if request.direction == CctpDirection::EvmToStellar && request.mint_submitter.is_none() {
            return Err(CctpServiceError::Validation(
                CctpValidationError::InvalidMintSubmitter,
            ));
        }

        let cap = decimal_to_cctp_subunits(&self.config.amount_cap).unwrap_or(u128::MAX);
        if cctp_amount > cap {
            return Err(CctpServiceError::AmountExceedsCap);
        }

        let (source_domain, dest_domain) = match request.direction {
            CctpDirection::StellarToEvm => (STELLAR_TESTNET_DOMAIN, SEPOLIA_DOMAIN),
            CctpDirection::EvmToStellar => (SEPOLIA_DOMAIN, STELLAR_TESTNET_DOMAIN),
        };

        let fees = self
            .iris
            .fetch_burn_fees(source_domain, dest_domain)
            .await
            .map_err(|e| CctpServiceError::Iris(e.to_string()))?;

        let max_fee = match request.finality {
            CctpFinality::Standard => fees.standard_fee.clone(),
            CctpFinality::Fast => fees
                .fast_fee
                .clone()
                .ok_or(CctpServiceError::FastNotSupported)?,
        };
        let now = Utc::now();

        let support_id = format!("cctp-{}", transfer_id);

        let destination_amount = match request.direction {
            CctpDirection::StellarToEvm => request.amount.clone(),
            CctpDirection::EvmToStellar => {
                let stellar_sub = cctp_subunits_to_stellar_subunits(cctp_amount).map_err(|_| {
                    CctpServiceError::Validation(CctpValidationError::InvalidAmount)
                })?;
                format_stellar_amount(stellar_sub)
            }
        };

        let transfer = CctpTransfer {
            transfer_id,
            support_reference_id: support_id,
            corridor_id: request.corridor_id.clone(),
            provider: request.provider.clone(),
            direction: request.direction,
            source_chain_id: request.source_chain_id.clone(),
            destination_chain_id: request.destination_chain_id.clone(),
            source_asset: request.source_asset.asset.clone(),
            source_asset_canonical: request.source_asset.canonical.clone(),
            destination_asset: request.destination_asset.asset.clone(),
            destination_asset_canonical: request.destination_asset.canonical.clone(),
            sender: request.sender.clone().unwrap_or_default(),
            recipient: request.recipient.clone(),
            mint_submitter: request.mint_submitter.clone(),
            amount: request.amount.clone(),
            destination_amount,
            finality: request.finality,
            runtime_fee_quote: Some(max_fee.clone()),
            max_fee: Some(max_fee),
            fee_expires_at: Some(now + Duration::minutes(10)),
            quote_expires_at: now + Duration::seconds(self.config.quote_ttl_secs as i64),
            status: CctpTransferStatus::Created,
            source_tx_hash: None,
            source_approval_tx_hash: None,
            source_approval_verified_at: None,
            destination_tx_hash: None,
            iris_message_hash: None,
            message_nonce: None,
            raw_message: None,
            attestation: None,
            retry_count: 0,
            last_provider_error: None,
            last_provider_code: None,
            version: 1,
            created_at: now,
            updated_at: now,
            terminal_at: None,
            mint_payload_hash: None,
            mint_payload_expires_at: None,
            approval_payload_hash: None,
            approval_expiration_ledger: None,
            burn_payload_hash: None,
            burn_prepare_step: None,
            access_token_hash: Some(access_token_hash),
            last_polled_at: None,
            poll_lease_until: None,
            reattest_lease_owner_hash: None,
            reattest_lease_until: None,
            reattest_attempt_count: 0,
            reattest_cooldown_until: None,
        };

        Ok(transfer)
    }

    pub async fn poll_one_transfer_with_lease(
        &self,
        transfer_id: Uuid,
        lease_secs: i64,
        min_interval_secs: i64,
    ) -> Result<CctpTransfer, CctpServiceError> {
        let Some((transfer, outcome)) = self
            .store
            .try_acquire_poll_lease(transfer_id, lease_secs, min_interval_secs)
            .await
            .map_err(CctpServiceError::Store)?
        else {
            return Err(CctpServiceError::NotFound);
        };
        crate::metrics::record_cctp_poll_lease(match outcome {
            crate::cctp::store::PollLeaseOutcome::Acquired => "acquired",
            crate::cctp::store::PollLeaseOutcome::Skipped => "skipped",
        });
        if outcome == crate::cctp::store::PollLeaseOutcome::Skipped {
            return Ok(transfer);
        }
        self.poll_one_transfer(transfer_id).await
    }

    pub async fn reattest_with_claim(
        &self,
        transfer_id: Uuid,
        max_attempts: u32,
        cooldown_secs: i64,
        lease_secs: i64,
    ) -> Result<CctpTransfer, CctpServiceError> {
        use crate::cctp::idempotency::{lease_owner_hash_from_nonce, new_lease_owner_nonce};

        let lease_owner = lease_owner_hash_from_nonce(&new_lease_owner_nonce());
        let Some((transfer, outcome)) = self
            .store
            .claim_reattest_lease(transfer_id, &lease_owner, lease_secs, max_attempts)
            .await
            .map_err(CctpServiceError::Store)?
        else {
            return Err(CctpServiceError::NotFound);
        };
        match outcome {
            crate::cctp::store::ReattestClaimOutcome::Claimed => {}
            crate::cctp::store::ReattestClaimOutcome::InProgress => {
                return Err(CctpServiceError::InvalidState);
            }
            crate::cctp::store::ReattestClaimOutcome::NotAllowed => {
                return Err(CctpServiceError::InvalidState);
            }
        }

        let iris_result = if let Some(nonce) = transfer.message_nonce.as_ref() {
            self.iris.reattest(nonce).await
        } else if transfer.source_tx_hash.is_some() {
            Ok(())
        } else {
            let _ = self
                .store
                .finalize_reattest_failure(
                    transfer_id,
                    &lease_owner,
                    "invalid_state",
                    "reattest requires message nonce or source tx hash",
                    cooldown_secs,
                )
                .await;
            return Err(CctpServiceError::InvalidState);
        };

        match iris_result {
            Ok(()) => {
                let updated = self
                    .store
                    .finalize_reattest_success(transfer_id, &lease_owner)
                    .await
                    .map_err(CctpServiceError::Store)?
                    .ok_or(CctpServiceError::InvalidState)?;
                metrics::record_cctp_transition("reattest_awaiting");
                Ok(updated)
            }
            Err(e) => {
                let code = "iris_reattest_failed";
                let detail = e.to_string();
                let redacted = if detail.len() <= 120 && !detail.contains("http") {
                    detail
                } else {
                    "Circle attestation reattest failed".into()
                };
                let _ = self
                    .store
                    .finalize_reattest_failure(
                        transfer_id,
                        &lease_owner,
                        code,
                        &redacted,
                        cooldown_secs,
                    )
                    .await;
                Err(CctpServiceError::Iris(redacted))
            }
        }
    }

    pub async fn record_burn_submission(
        &self,
        transfer_id: Uuid,
        tx_hash: &str,
    ) -> Result<CctpTransfer, CctpServiceError> {
        let transfer = self
            .store
            .get(transfer_id)
            .await
            .map_err(CctpServiceError::Store)?
            .ok_or(CctpServiceError::NotFound)?;

        if transfer.status != CctpTransferStatus::BurnPrepared {
            return Err(CctpServiceError::InvalidState);
        }

        Self::ensure_quote_not_expired(&transfer)?;
        Self::ensure_fee_not_expired(&transfer)?;
        self.ensure_burn_verifier_ready(transfer.direction)?;

        let facts = match transfer.direction {
            CctpDirection::StellarToEvm => self
                .runtime
                .stellar_burn_verifier
                .verify_burn(tx_hash)
                .await
                .map_err(CctpServiceError::Verifier)?,
            CctpDirection::EvmToStellar => self
                .runtime
                .evm_burn_verifier
                .verify_burn(tx_hash)
                .await
                .map_err(CctpServiceError::Verifier)?,
        };

        let expected =
            build_expected_burn_facts(&transfer, &self.config, tx_hash).map_err(|_| {
                CctpServiceError::Verifier(VerifierError::Failed("expectations".into()))
            })?;

        if facts_match(&expected, &facts).is_err() {
            metrics::record_cctp_verifier_mismatch();
            return Err(CctpServiceError::Verifier(VerifierError::Failed(
                "burn facts mismatch".into(),
            )));
        }

        let awaiting = self
            .store
            .record_verified_burn(transfer_id, transfer.version, tx_hash)
            .await
            .map_err(CctpServiceError::Store)?;

        self.release_transfer_prepare_locks(&transfer).await;

        metrics::record_cctp_transition("awaiting_attestation");
        Ok(awaiting)
    }

    pub async fn poll_one_transfer(
        &self,
        transfer_id: Uuid,
    ) -> Result<CctpTransfer, CctpServiceError> {
        let transfer = self
            .store
            .get(transfer_id)
            .await
            .map_err(CctpServiceError::Store)?
            .ok_or(CctpServiceError::NotFound)?;

        if transfer.status == CctpTransferStatus::MintSubmitted {
            return self.poll_mint_completion(transfer).await;
        }

        if transfer.status != CctpTransferStatus::AwaitingAttestation {
            return Ok(transfer);
        }

        if self.attestation_timed_out(&transfer) {
            return self.fail_attestation_timeout(transfer).await;
        }

        let source_domain = match transfer.direction {
            CctpDirection::StellarToEvm => STELLAR_TESTNET_DOMAIN,
            CctpDirection::EvmToStellar => SEPOLIA_DOMAIN,
        };

        let tx_hash = transfer
            .source_tx_hash
            .as_ref()
            .ok_or(CctpServiceError::InvalidState)?;

        let start = std::time::Instant::now();
        let outcome = self
            .iris
            .poll_messages_by_tx(source_domain, tx_hash)
            .await
            .map_err(|e| CctpServiceError::Iris(e.to_string()))?;
        metrics::record_cctp_iris_latency(start.elapsed(), "poll");

        match outcome {
            IrisPollOutcome::Pending | IrisPollOutcome::NotFound => Ok(transfer),
            IrisPollOutcome::RateLimited { retry_after_secs } => {
                metrics::record_cctp_rate_limited();
                let _ = self
                    .store
                    .transition(
                        transfer_id,
                        transfer.version,
                        CctpTransferStatus::AwaitingAttestation,
                        TransferPatch {
                            last_provider_error: Some(format!(
                                "rate limited; retry after {retry_after_secs}s"
                            )),
                            last_provider_code: Some("429".into()),
                            ..Default::default()
                        },
                    )
                    .await;
                Ok(transfer)
            }
            IrisPollOutcome::Complete(msg) => {
                self.validate_and_mark_attestation_ready(&transfer, &msg)
                    .await
            }
        }
    }

    fn attestation_timed_out(&self, transfer: &CctpTransfer) -> bool {
        let deadline =
            transfer.updated_at + Duration::seconds(self.config.poll_timeout_secs as i64);
        Utc::now() > deadline
    }

    async fn fail_attestation_timeout(
        &self,
        transfer: CctpTransfer,
    ) -> Result<CctpTransfer, CctpServiceError> {
        let updated = self
            .store
            .transition(
                transfer.transfer_id,
                transfer.version,
                CctpTransferStatus::AttestationFailed,
                TransferPatch {
                    last_provider_error: Some("attestation poll timeout".into()),
                    last_provider_code: Some("poll_timeout".into()),
                    ..Default::default()
                },
            )
            .await
            .map_err(CctpServiceError::Store)?;
        metrics::record_cctp_transition("attestation_failed");
        self.release_transfer_prepare_locks(&transfer).await;
        Ok(updated)
    }

    async fn validate_and_mark_attestation_ready(
        &self,
        transfer: &CctpTransfer,
        iris_msg: &IrisMessage,
    ) -> Result<CctpTransfer, CctpServiceError> {
        self.ensure_attestation_verifier_ready()?;

        if iris_msg.status != IrisMessageStatus::Complete {
            return Err(CctpServiceError::InvalidMessage(
                "iris status is not complete".into(),
            ));
        }

        let persisted = transfer
            .source_tx_hash
            .as_ref()
            .ok_or(CctpServiceError::InvalidState)?;
        let iris_hash = iris_msg
            .source_tx_hash
            .as_ref()
            .ok_or(CctpServiceError::IrisTxHashMismatch)?;
        if !tx_hashes_equal(persisted, iris_hash) {
            metrics::record_cctp_invalid_message();
            return Err(CctpServiceError::IrisTxHashMismatch);
        }

        let attestation_hex = iris_msg
            .attestation_hex
            .as_ref()
            .ok_or(CctpServiceError::MissingAttestation)?;
        if attestation_hex.is_empty() || attestation_hex.eq_ignore_ascii_case("PENDING") {
            return Err(CctpServiceError::MissingAttestation);
        }

        let expectations = build_corridor_expectations(transfer, &self.config).map_err(|e| {
            metrics::record_cctp_invalid_message();
            CctpServiceError::InvalidMessage(format!("corridor expectations: {e}"))
        })?;

        if let Err(e) = validate_message_for_corridor(&iris_msg.message_hex, &expectations) {
            metrics::record_cctp_invalid_message();
            return Err(CctpServiceError::InvalidMessage(format!(
                "corridor message: {e}"
            )));
        }

        let raw = decode_hex_message(&iris_msg.message_hex).map_err(|e| {
            CctpServiceError::InvalidMessage(format!("message hex: {e}"))
        })?;
        if check_byte_len("raw_message", &raw, MAX_RAW_MESSAGE_BYTES).is_err() {
            return Err(CctpServiceError::InvalidMessage(
                "raw message exceeds bound".into(),
            ));
        }

        let attestation = decode_hex_message(attestation_hex).map_err(|e| {
            CctpServiceError::InvalidMessage(format!("attestation hex: {e}"))
        })?;
        if check_byte_len("attestation", &attestation, MAX_ATTESTATION_BYTES).is_err() {
            return Err(CctpServiceError::InvalidMessage(
                "attestation exceeds bound".into(),
            ));
        }

        self.runtime
            .attestation_verifier
            .verify_attestation(&raw, &attestation)
            .await
            .map_err(|e| {
                tracing::warn!(
                    transfer_id = %transfer.transfer_id,
                    error = %e,
                    "CCTP attestation signature verification failed"
                );
                CctpServiceError::Attestation(e)
            })?;

        if iris_msg.event_nonce.is_empty() {
            return Err(CctpServiceError::InvalidMessage(
                "iris event nonce empty".into(),
            ));
        }

        let updated = self
            .store
            .transition(
                transfer.transfer_id,
                transfer.version,
                CctpTransferStatus::AttestationReady,
                TransferPatch {
                    raw_message: Some(raw),
                    attestation: Some(attestation),
                    message_nonce: Some(iris_msg.event_nonce.clone()),
                    ..Default::default()
                },
            )
            .await
            .map_err(CctpServiceError::Store)?;

        metrics::record_cctp_transition("attestation_ready");
        Ok(updated)
    }

    pub async fn reattest(&self, transfer_id: Uuid) -> Result<CctpTransfer, CctpServiceError> {
        use crate::cctp::gate::{
            REATTEST_COOLDOWN_SECS, REATTEST_LEASE_SECS, REATTEST_MAX_ATTEMPTS,
        };

        self.reattest_with_claim(
            transfer_id,
            REATTEST_MAX_ATTEMPTS,
            REATTEST_COOLDOWN_SECS,
            REATTEST_LEASE_SECS,
        )
        .await
    }

    /// Classify a Stellar source tx by on-chain invoke target (approval vs burn).
    async fn classify_stellar_source_submission(
        &self,
        tx_hash: &str,
    ) -> Result<StellarSourceSubmissionKind, VerifierError> {
        let rpc = StellarRpcClient::new(&self.config)?;
        let tx = rpc.get_finalized_transaction(tx_hash).await?;
        if tx.status != TxStatus::Success {
            return Err(VerifierError::Failed("tx failed".into()));
        }
        let invoke = parse_invoke_envelope(&tx.envelope_xdr)?;
        match invoke.function.as_str() {
            "approve" => Ok(StellarSourceSubmissionKind::Approval),
            "deposit_for_burn" | "deposit_for_burn_with_hook" => {
                Ok(StellarSourceSubmissionKind::Burn)
            }
            other => Err(VerifierError::Failed(format!(
                "unsupported stellar invoke: {other}"
            ))),
        }
    }

    /// Route `submit-burn` to approval or burn verification using on-chain evidence.
    pub async fn record_source_submission(
        &self,
        transfer_id: Uuid,
        tx_hash: &str,
    ) -> Result<CctpTransfer, CctpServiceError> {
        let transfer = self
            .store
            .get(transfer_id)
            .await
            .map_err(CctpServiceError::Store)?
            .ok_or(CctpServiceError::NotFound)?;

        match transfer.direction {
            CctpDirection::StellarToEvm => {
                let kind = self
                    .classify_stellar_source_submission(tx_hash)
                    .await
                    .map_err(CctpServiceError::Verifier)?;
                match kind {
                    StellarSourceSubmissionKind::Approval => {
                        self.record_approval_submission(transfer_id, tx_hash).await
                    }
                    StellarSourceSubmissionKind::Burn => {
                        match self.record_burn_submission(transfer_id, tx_hash).await {
                            // Belt-and-suspenders: if RPC decode races classify, never leave
                            // traders on the generic "on-chain verification failed" path for
                            // an approve tx submitted to submit-burn.
                            Err(CctpServiceError::Verifier(VerifierError::Failed(msg)))
                                if msg.contains("wrong function") =>
                            {
                                self.record_approval_submission(transfer_id, tx_hash).await
                            }
                            other => other,
                        }
                    }
                }
            }
            CctpDirection::EvmToStellar => {
                if transfer.burn_prepare_step.as_deref() == Some("approval")
                    && transfer.source_approval_verified_at.is_none()
                {
                    self.record_approval_submission(transfer_id, tx_hash).await
                } else {
                    self.record_burn_submission(transfer_id, tx_hash).await
                }
            }
        }
    }

    pub async fn record_approval_submission(
        &self,
        transfer_id: Uuid,
        tx_hash: &str,
    ) -> Result<CctpTransfer, CctpServiceError> {
        let transfer = self
            .store
            .get(transfer_id)
            .await
            .map_err(CctpServiceError::Store)?
            .ok_or(CctpServiceError::NotFound)?;

        if transfer.status != CctpTransferStatus::BurnPrepared {
            return Err(CctpServiceError::InvalidState);
        }

        Self::ensure_quote_not_expired(&transfer)?;
        Self::ensure_fee_not_expired(&transfer)?;

        if let Some(existing) = transfer.source_approval_tx_hash.as_deref() {
            if !tx_hashes_equal(existing, tx_hash) {
                return Err(CctpServiceError::Verifier(VerifierError::Failed(
                    "conflicting approval tx hash".into(),
                )));
            }
            if transfer.source_approval_verified_at.is_some() {
                return Ok(transfer);
            }
        }

        let verified_at = Utc::now();
        match transfer.direction {
            CctpDirection::StellarToEvm => {
                if !self.runtime.stellar_approval_verifier.is_ready() {
                    return Err(CctpServiceError::VerifiersNotReady);
                }
                let cctp_amount = stellar_outbound_cctp_amount_strict(&transfer.amount)
                    .map_err(|_| CctpServiceError::Verifier(VerifierError::Failed("amount".into())))?;
                let required_i128: i128 = cctp_subunits_to_stellar_subunits(cctp_amount)
                    .map_err(|_| CctpServiceError::Verifier(VerifierError::Failed("amount".into())))?
                    .try_into()
                    .map_err(|_| CctpServiceError::Verifier(VerifierError::Failed("amount".into())))?;
                self.runtime
                    .stellar_approval_verifier
                    .verify_approval(&transfer, tx_hash, required_i128)
                    .await
                    .map_err(CctpServiceError::Verifier)?;
            }
            CctpDirection::EvmToStellar => {
                if !self.runtime.evm_approval_verifier.is_ready() {
                    return Err(CctpServiceError::VerifiersNotReady);
                }
                let required_subunits = decimal_to_cctp_subunits(&transfer.amount)
                    .map_err(|_| CctpServiceError::Verifier(VerifierError::Failed("amount".into())))?;
                self.runtime
                    .evm_approval_verifier
                    .verify_approval(&transfer, tx_hash, required_subunits)
                    .await
                    .map_err(CctpServiceError::Verifier)?;
            }
        }

        let updated = self
            .store
            .record_approval_submission(transfer_id, transfer.version, tx_hash, verified_at)
            .await
            .map_err(CctpServiceError::Store)?;

        if !transfer.sender.is_empty() {
            self.release_transfer_prepare_locks(&transfer).await;
        }

        Ok(updated)
    }

    pub async fn prepare_burn_wallet(
        &self,
        transfer_id: Uuid,
    ) -> Result<PreparedBurnBundle, CctpServiceError> {
        if !self.config.enabled || !self.config.is_configured() {
            return Err(CctpServiceError::NotEnabled);
        }
        if self.provider_killed().await {
            metrics::record_cctp_provider_killed_new_transfer();
            return Err(CctpServiceError::ProviderKilled);
        }

        let mut transfer = self
            .store
            .get(transfer_id)
            .await
            .map_err(CctpServiceError::Store)?
            .ok_or(CctpServiceError::NotFound)?;

        match transfer.status {
            CctpTransferStatus::Created => {
                Self::ensure_quote_not_expired(&transfer)?;
                Self::ensure_fee_not_expired(&transfer)?;
                self.ensure_burn_verifier_ready(transfer.direction)?;
                if transfer.direction == CctpDirection::StellarToEvm {
                    stellar_outbound_cctp_amount_strict(&transfer.amount)
                        .map_err(|_| CctpServiceError::StellarRemainder)?;
                }
                transfer = self
                    .store
                    .transition(
                        transfer_id,
                        transfer.version,
                        CctpTransferStatus::BurnPrepared,
                        TransferPatch::default(),
                    )
                    .await
                    .map_err(CctpServiceError::Store)?;
                metrics::record_cctp_transition("burn_prepared");
            }
            CctpTransferStatus::BurnPrepared => {
                Self::ensure_quote_not_expired(&transfer)?;
                Self::ensure_fee_not_expired(&transfer)?;
                self.ensure_burn_verifier_ready(transfer.direction)?;
            }
            _ => return Err(CctpServiceError::InvalidState),
        }

        let source = burn_prepare_source(&transfer)?;

        let bundle = match transfer.direction {
            CctpDirection::StellarToEvm => self
                .runtime
                .stellar_burn_builder
                .prepare_burn(&transfer, &self.config)
                .await
                .map_err(CctpServiceError::Builder)?,
            CctpDirection::EvmToStellar => self
                .runtime
                .evm_burn_builder
                .prepare_burn(&transfer, &self.config)
                .await
                .map_err(CctpServiceError::Builder)?,
        };

        let payload_hash = wallet_payload_hash(&bundle.primary, &self.config)
            .map_err(CctpServiceError::Verifier)?;
        let expires =
            chrono::DateTime::from_timestamp(bundle.expires_at, 0).unwrap_or_else(Utc::now);
        let kind = match bundle.step {
            BurnPrepareStep::Approval => CctpPrepareKind::Approval,
            BurnPrepareStep::Burn => CctpPrepareKind::Burn,
        };
        let prepared_payload =
            serialize_burn_bundle(&bundle).map_err(Self::map_payload_cache_error)?;
        match self
            .try_acquire_prepare_lock(Self::burn_reservation(
                source,
                transfer_id,
                kind,
                payload_hash.clone(),
                prepared_payload,
                expires,
            ))
            .await?
        {
            PrepareAcquireResult::Acquired => {}
            PrepareAcquireResult::Idempotent(active) => {
                if let Some(payload) = active.prepared_payload {
                    return deserialize_burn_bundle(&payload)
                        .map_err(Self::map_payload_cache_error);
                }
            }
            PrepareAcquireResult::ConflictOtherTransfer { .. } => {
                return Err(CctpServiceError::ActivePrepareExists);
            }
        }

        let step_str = match bundle.step {
            BurnPrepareStep::Approval => "approval",
            BurnPrepareStep::Burn => "burn",
        };
        let mut patch = TransferPatch {
            burn_prepare_step: Some(step_str.to_string()),
            burn_payload_hash: Some(payload_hash),
            ..Default::default()
        };
        if bundle.step == BurnPrepareStep::Approval {
            patch.approval_payload_hash = Some(patch.burn_payload_hash.clone().unwrap());
            if let Some(exp) = bundle.approval_expiration_ledger {
                patch.approval_expiration_ledger = Some(exp as u64);
            }
        }

        let _ = self
            .store
            .transition(
                transfer_id,
                transfer.version,
                CctpTransferStatus::BurnPrepared,
                patch,
            )
            .await
            .map_err(CctpServiceError::Store)?;

        Ok(bundle)
    }

    pub async fn prepare_mint(
        &self,
        transfer_id: Uuid,
    ) -> Result<PreparedMintBundle, CctpServiceError> {
        if !self.config.enabled || !self.config.is_configured() {
            return Err(CctpServiceError::NotEnabled);
        }
        let transfer = self
            .store
            .get(transfer_id)
            .await
            .map_err(CctpServiceError::Store)?
            .ok_or(CctpServiceError::NotFound)?;

        let allowed = matches!(
            transfer.status,
            CctpTransferStatus::AttestationReady
                | CctpTransferStatus::MintPrepared
                | CctpTransferStatus::MintFailedRetryable
        );
        if !allowed {
            return Err(CctpServiceError::InvalidState);
        }

        Self::ensure_quote_not_expired(&transfer)?;

        // Idempotent re-prepare while mint_prepared: return the locked unsigned payload.
        if transfer.direction == CctpDirection::EvmToStellar {
            let submitter = transfer
                .mint_submitter
                .as_deref()
                .ok_or(CctpServiceError::InvalidState)?;
            if let Some(cached) = self
                .active_mint_bundle_for_transfer(transfer_id, submitter)
                .await?
            {
                return Ok(cached);
            }
        }
        if transfer.status == CctpTransferStatus::MintPrepared {
            // Lock expired/missing — rebuild is only allowed from attestation_ready /
            // mint_failed_retryable so we do not silently rotate payload_hash mid-sign.
            return Err(CctpServiceError::InvalidState);
        }

        let bundle = match transfer.direction {
            CctpDirection::StellarToEvm => self
                .runtime
                .evm_mint_builder
                .prepare_mint(&transfer, &self.config)
                .await
                .map_err(CctpServiceError::Builder)?,
            CctpDirection::EvmToStellar => self
                .runtime
                .stellar_mint_builder
                .prepare_mint(&transfer, &self.config)
                .await
                .map_err(CctpServiceError::Builder)?,
        };

        // Trustline ChangeTrust is signed by the recipient G-account and submitted to Horizon
        // by the wallet; do not lock or mark mint_prepared until the real mint payload.
        if bundle.trustline_required
            || bundle.step == crate::cctp::builders::MintPrepareStep::Trustline
        {
            return Ok(bundle);
        }

        if transfer.direction == CctpDirection::EvmToStellar {
            let submitter = transfer
                .mint_submitter
                .as_deref()
                .ok_or(CctpServiceError::InvalidState)?;
            let expires =
                chrono::DateTime::from_timestamp(bundle.expires_at, 0).unwrap_or_else(Utc::now);
            let prepared_payload =
                serialize_mint_bundle(&bundle).map_err(Self::map_payload_cache_error)?;
            match self
                .try_acquire_prepare_lock(CctpActivePrepare {
                    source_account: submitter.to_string(),
                    transfer_id,
                    kind: CctpPrepareKind::Mint,
                    payload_hash: bundle.payload_hash.clone(),
                    prepared_payload: Some(prepared_payload),
                    expires_at: expires,
                    updated_at: Utc::now(),
                })
                .await?
            {
                PrepareAcquireResult::Acquired => {}
                PrepareAcquireResult::Idempotent(active) => {
                    if let Some(payload) = active.prepared_payload {
                        return deserialize_mint_bundle(&payload)
                            .map_err(Self::map_payload_cache_error);
                    }
                }
                PrepareAcquireResult::ConflictOtherTransfer { .. } => {
                    return Err(CctpServiceError::ActivePrepareExists);
                }
            }
        }

        let expires =
            chrono::DateTime::from_timestamp(bundle.expires_at, 0).unwrap_or_else(Utc::now);
        let updated = self
            .store
            .record_mint_prepared(
                transfer_id,
                transfer.version,
                &bundle.payload_hash,
                expires,
                transfer.mint_submitter.clone(),
            )
            .await
            .map_err(CctpServiceError::Store)?;
        metrics::record_cctp_transition("mint_prepared");
        let _ = updated;
        Ok(bundle)
    }

    pub async fn record_mint_submission(
        &self,
        transfer_id: Uuid,
        tx_hash: &str,
    ) -> Result<CctpTransfer, CctpServiceError> {
        let transfer = self
            .store
            .get(transfer_id)
            .await
            .map_err(CctpServiceError::Store)?
            .ok_or(CctpServiceError::NotFound)?;

        if transfer.status != CctpTransferStatus::MintPrepared {
            return Err(CctpServiceError::InvalidState);
        }

        Self::ensure_mint_payload_valid(&transfer)?;
        self.ensure_mint_verifier_ready(transfer.direction)?;

        let expected_payload_hash = transfer
            .mint_payload_hash
            .as_deref()
            .ok_or(CctpServiceError::InvalidState)?;

        let message = transfer
            .raw_message
            .as_ref()
            .ok_or(CctpServiceError::InvalidState)?;
        let attestation = transfer
            .attestation
            .as_ref()
            .ok_or(CctpServiceError::InvalidState)?;
        let nonce = transfer
            .message_nonce
            .as_ref()
            .ok_or(CctpServiceError::InvalidState)?;

        let facts = match transfer.direction {
            CctpDirection::StellarToEvm => self
                .runtime
                .evm_mint_verifier
                .verify_mint_submission(tx_hash, message, attestation, nonce, expected_payload_hash)
                .await
                .map_err(CctpServiceError::Verifier)?,
            CctpDirection::EvmToStellar => self
                .runtime
                .stellar_mint_verifier
                .verify_mint_submission(
                    tx_hash,
                    message,
                    attestation,
                    nonce,
                    expected_payload_hash,
                    transfer.mint_submitter.as_deref(),
                )
                .await
                .map_err(CctpServiceError::Verifier)?,
        };

        if facts.payload_hash != expected_payload_hash {
            return Err(CctpServiceError::MintPayloadHashMismatch);
        }

        if let MintVerifyOutcome::FailedRetryable { reason } = &facts.outcome {
            self.release_transfer_prepare_locks(&transfer).await;
            let retryable = self
                .store
                .transition(
                    transfer_id,
                    transfer.version,
                    CctpTransferStatus::MintFailedRetryable,
                    TransferPatch {
                        last_provider_error: Some(reason.clone()),
                        last_provider_code: Some("mint_retryable".into()),
                        ..Default::default()
                    },
                )
                .await
                .map_err(CctpServiceError::Store)?;
            metrics::record_cctp_transition("mint_failed_retryable");
            return Ok(retryable);
        }

        if !facts.submission_ok() {
            return Err(CctpServiceError::Verifier(VerifierError::Failed(
                "mint submission mismatch".into(),
            )));
        }

        if let Some(existing) = transfer.destination_tx_hash.as_deref() {
            if existing != tx_hash {
                return Err(CctpServiceError::Verifier(VerifierError::Failed(
                    "conflicting destination tx hash".into(),
                )));
            }
            return Ok(transfer);
        }

        let submitted = self
            .store
            .record_mint_submission(transfer_id, transfer.version, tx_hash)
            .await
            .map_err(CctpServiceError::Store)?;

        let completion = match transfer.direction {
            CctpDirection::StellarToEvm => self
                .runtime
                .evm_mint_verifier
                .verify_mint_completion(
                    tx_hash,
                    message,
                    nonce,
                    &transfer.recipient,
                    transfer.finality,
                )
                .await
                .map_err(CctpServiceError::Verifier)?,
            CctpDirection::EvmToStellar => self
                .runtime
                .stellar_mint_verifier
                .verify_mint_completion(
                    tx_hash,
                    message,
                    nonce,
                    &transfer.recipient,
                    transfer.finality,
                )
                .await
                .map_err(CctpServiceError::Verifier)?,
        };

        match completion {
            MintVerifyOutcome::Succeeded => {
                let completed = self
                    .store
                    .record_mint_completed(submitted.transfer_id, submitted.version)
                    .await
                    .map_err(CctpServiceError::Store)?;
                self.release_transfer_prepare_locks(&transfer).await;
                Ok(completed)
            }
            MintVerifyOutcome::Pending => Ok(submitted),
            MintVerifyOutcome::ReconciliationNonceConsumed => {
                self.record_mint_reconciliation_hint(submitted).await
            }
            MintVerifyOutcome::FailedRetryable { reason } => {
                let retryable = self
                    .store
                    .transition(
                        transfer_id,
                        submitted.version,
                        CctpTransferStatus::MintFailedRetryable,
                        TransferPatch {
                            last_provider_error: Some(reason),
                            last_provider_code: Some("mint_retryable".into()),
                            ..Default::default()
                        },
                    )
                    .await
                    .map_err(CctpServiceError::Store)?;
                self.release_transfer_prepare_locks(&transfer).await;
                metrics::record_cctp_transition("mint_failed_retryable");
                Ok(retryable)
            }
        }
    }

    async fn record_mint_reconciliation_hint(
        &self,
        transfer: CctpTransfer,
    ) -> Result<CctpTransfer, CctpServiceError> {
        if transfer.last_provider_code.as_deref() == Some("mint_reconciliation_nonce") {
            return Ok(transfer);
        }
        self.store
            .transition(
                transfer.transfer_id,
                transfer.version,
                transfer.status,
                TransferPatch {
                    last_provider_error: Some(
                        "nonce consumed on-chain without full mint delivery evidence; remains MintSubmitted"
                            .into(),
                    ),
                    last_provider_code: Some("mint_reconciliation_nonce".into()),
                    ..Default::default()
                },
            )
            .await
            .map_err(CctpServiceError::Store)
    }

    async fn poll_mint_completion(
        &self,
        transfer: CctpTransfer,
    ) -> Result<CctpTransfer, CctpServiceError> {
        let tx_hash = transfer
            .destination_tx_hash
            .as_ref()
            .ok_or(CctpServiceError::InvalidState)?;
        let message = transfer
            .raw_message
            .as_ref()
            .ok_or(CctpServiceError::InvalidState)?;
        let nonce = transfer
            .message_nonce
            .as_ref()
            .ok_or(CctpServiceError::InvalidState)?;

        let completion = match transfer.direction {
            CctpDirection::StellarToEvm => self
                .runtime
                .evm_mint_verifier
                .verify_mint_completion(
                    tx_hash,
                    message,
                    nonce,
                    &transfer.recipient,
                    transfer.finality,
                )
                .await
                .map_err(CctpServiceError::Verifier)?,
            CctpDirection::EvmToStellar => self
                .runtime
                .stellar_mint_verifier
                .verify_mint_completion(
                    tx_hash,
                    message,
                    nonce,
                    &transfer.recipient,
                    transfer.finality,
                )
                .await
                .map_err(CctpServiceError::Verifier)?,
        };

        match completion {
            MintVerifyOutcome::Succeeded => {
                let completed = self
                    .store
                    .record_mint_completed(transfer.transfer_id, transfer.version)
                    .await
                    .map_err(CctpServiceError::Store)?;
                self.release_transfer_prepare_locks(&transfer).await;
                Ok(completed)
            }
            MintVerifyOutcome::Pending => Ok(transfer),
            MintVerifyOutcome::ReconciliationNonceConsumed => {
                self.record_mint_reconciliation_hint(transfer).await
            }
            MintVerifyOutcome::FailedRetryable { reason } => {
                let retryable = self
                    .store
                    .transition(
                        transfer.transfer_id,
                        transfer.version,
                        CctpTransferStatus::MintFailedRetryable,
                        TransferPatch {
                            last_provider_error: Some(reason),
                            last_provider_code: Some("mint_retryable".into()),
                            ..Default::default()
                        },
                    )
                    .await
                    .map_err(CctpServiceError::Store)?;
                self.release_transfer_prepare_locks(&transfer).await;
                metrics::record_cctp_transition("mint_failed_retryable");
                Ok(retryable)
            }
        }
    }

    pub async fn get_transfer(&self, transfer_id: Uuid) -> Result<CctpTransfer, CctpServiceError> {
        self.store
            .get(transfer_id)
            .await
            .map_err(CctpServiceError::Store)?
            .ok_or(CctpServiceError::NotFound)
    }

    pub async fn cancel(&self, transfer_id: Uuid) -> Result<CctpTransfer, CctpServiceError> {
        let transfer = self
            .store
            .get(transfer_id)
            .await
            .map_err(CctpServiceError::Store)?
            .ok_or(CctpServiceError::NotFound)?;

        if !can_cancel(transfer.status) {
            return Err(CctpServiceError::InvalidState);
        }

        let updated = self
            .store
            .transition(
                transfer_id,
                transfer.version,
                CctpTransferStatus::Cancelled,
                TransferPatch::default(),
            )
            .await
            .map_err(CctpServiceError::Store)?;
        self.release_transfer_prepare_locks(&transfer).await;
        metrics::record_cctp_transition("cancelled");
        Ok(updated)
    }

    /// Deterministic tick for worker wiring — polls one transfer if eligible.
    pub async fn tick_transfer(&self, transfer_id: Uuid) -> Result<(), CctpServiceError> {
        self.poll_one_transfer(transfer_id).await?;
        Ok(())
    }
}

fn tx_hashes_equal(a: &str, b: &str) -> bool {
    normalize_tx_hash(a) == normalize_tx_hash(b)
}

fn normalize_tx_hash(hash: &str) -> String {
    let trimmed = hash.trim();
    let hex = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    hex.to_ascii_lowercase()
}

fn format_stellar_amount(subunits: u128) -> String {
    let whole = subunits / 10_000_000;
    let frac = subunits % 10_000_000;
    format!("{}.{:07}", whole, frac)
}

fn burn_prepare_source(transfer: &CctpTransfer) -> Result<&str, CctpServiceError> {
    if transfer.sender.is_empty() {
        return Err(CctpServiceError::InvalidState);
    }
    Ok(&transfer.sender)
}

fn wallet_payload_hash(
    payload: &PreparedWalletPayload,
    config: &CctpConfig,
) -> Result<String, VerifierError> {
    use sha2::Digest;
    match payload {
        PreparedWalletPayload::StellarXdr { xdr_envelope, .. } => {
            payload_hash_from_envelope_xdr(xdr_envelope, config)
        }
        PreparedWalletPayload::EvmTransaction { .. } => {
            let json =
                serde_json::to_string(payload).map_err(|e| VerifierError::Failed(e.to_string()))?;
            Ok(hex::encode(sha2::Sha256::digest(json.as_bytes())))
        }
    }
}
