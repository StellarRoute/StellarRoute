//! CCTP transfer persistence and optimistic state transitions.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Mutex;
use thiserror::Error;
use uuid::Uuid;

use crate::cctp::bounds::{
    check_byte_len, check_str_len, MAX_ATTESTATION_BYTES, MAX_MESSAGE_NONCE_LEN,
    MAX_RAW_MESSAGE_BYTES, MAX_TX_HASH_LEN,
};
use crate::cctp::transitions::{is_allowed_transition, is_terminal};
use crate::models::v2_cctp::{CctpDirection, CctpFinality, CctpTransferStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CctpTransfer {
    pub transfer_id: Uuid,
    pub support_reference_id: String,
    pub corridor_id: String,
    pub provider: String,
    pub direction: CctpDirection,
    pub source_chain_id: String,
    pub destination_chain_id: String,
    pub source_asset: String,
    pub source_asset_canonical: String,
    pub destination_asset: String,
    pub destination_asset_canonical: String,
    pub sender: String,
    pub recipient: String,
    /// Stellar G-address fee-payer for `evm_to_stellar` mint (distinct from `recipient`).
    pub mint_submitter: Option<String>,
    pub amount: String,
    pub destination_amount: String,
    pub finality: CctpFinality,
    pub runtime_fee_quote: Option<String>,
    pub max_fee: Option<String>,
    pub fee_expires_at: Option<DateTime<Utc>>,
    pub quote_expires_at: DateTime<Utc>,
    pub status: CctpTransferStatus,
    pub source_tx_hash: Option<String>,
    pub source_approval_tx_hash: Option<String>,
    pub source_approval_verified_at: Option<DateTime<Utc>>,
    pub destination_tx_hash: Option<String>,
    pub iris_message_hash: Option<String>,
    pub message_nonce: Option<String>,
    pub raw_message: Option<Vec<u8>>,
    pub attestation: Option<Vec<u8>>,
    pub retry_count: u32,
    pub last_provider_error: Option<String>,
    pub last_provider_code: Option<String>,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub terminal_at: Option<DateTime<Utc>>,
    pub mint_payload_hash: Option<String>,
    pub mint_payload_expires_at: Option<DateTime<Utc>>,
    pub approval_payload_hash: Option<String>,
    pub approval_expiration_ledger: Option<u64>,
    pub burn_payload_hash: Option<String>,
    pub burn_prepare_step: Option<String>,
    /// SHA-256 hex digest of the one-time transfer access token.
    pub access_token_hash: Option<String>,
    pub last_polled_at: Option<DateTime<Utc>>,
    pub poll_lease_until: Option<DateTime<Utc>>,
    /// Lease owner hash while a reattest Iris call is in flight (`attestation_failed` unchanged).
    pub reattest_lease_owner_hash: Option<String>,
    pub reattest_lease_until: Option<DateTime<Utc>>,
    /// Iris reattest provider calls (success or failure); capped by `REATTEST_MAX_ATTEMPTS`.
    pub reattest_attempt_count: u32,
    /// Durable cooldown after failed reattest finalize.
    pub reattest_cooldown_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollLeaseOutcome {
    Acquired,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReattestClaimOutcome {
    Claimed,
    NotAllowed,
    InProgress,
}

#[derive(Debug, Error)]
pub enum CctpStoreError {
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
    #[error("not found")]
    NotFound,
    #[error("invalid transition")]
    InvalidTransition,
    #[error("duplicate source tx hash")]
    DuplicateSourceTxHash,
    #[error("version conflict")]
    VersionConflict,
    #[error("invalid persisted status: {0}")]
    InvalidStatus(String),
    #[error("invalid persisted direction: {0}")]
    InvalidDirection(String),
    #[error("payload too large")]
    PayloadTooLarge,
}

#[async_trait]
pub trait CctpTransferStore: Send + Sync {
    async fn insert(&self, transfer: &CctpTransfer) -> Result<(), CctpStoreError>;

    async fn get(&self, transfer_id: Uuid) -> Result<Option<CctpTransfer>, CctpStoreError>;

    /// Uniform lookup: returns `None` for missing transfer or wrong access token hash.
    async fn get_authorized(
        &self,
        transfer_id: Uuid,
        access_token_hash: &str,
    ) -> Result<Option<CctpTransfer>, CctpStoreError>;

    /// Atomically acquire poll lease or skip when another holder is active / interval not elapsed.
    async fn try_acquire_poll_lease(
        &self,
        transfer_id: Uuid,
        lease_secs: i64,
        min_interval_secs: i64,
    ) -> Result<Option<(CctpTransfer, PollLeaseOutcome)>, CctpStoreError>;

    /// Atomically claim reattest lease without changing saga status.
    async fn claim_reattest_lease(
        &self,
        transfer_id: Uuid,
        lease_owner_hash: &str,
        lease_secs: i64,
        max_attempts: u32,
    ) -> Result<Option<(CctpTransfer, ReattestClaimOutcome)>, CctpStoreError>;

    /// After successful Iris reattest: transition to awaiting_attestation, bump counters once.
    async fn finalize_reattest_success(
        &self,
        transfer_id: Uuid,
        lease_owner_hash: &str,
    ) -> Result<Option<CctpTransfer>, CctpStoreError>;

    /// After Iris failure: release lease, keep attestation_failed, count provider attempt, cooldown.
    async fn finalize_reattest_failure(
        &self,
        transfer_id: Uuid,
        lease_owner_hash: &str,
        provider_code: &str,
        provider_error: &str,
        cooldown_secs: i64,
    ) -> Result<Option<CctpTransfer>, CctpStoreError>;

    async fn transition(
        &self,
        transfer_id: Uuid,
        expected_version: i32,
        new_status: CctpTransferStatus,
        patch: TransferPatch,
    ) -> Result<CctpTransfer, CctpStoreError>;

    /// Record verified on-chain approval tx (hash + verified timestamp).
    async fn record_approval_submission(
        &self,
        transfer_id: Uuid,
        expected_version: i32,
        tx_hash: &str,
        verified_at: DateTime<Utc>,
    ) -> Result<CctpTransfer, CctpStoreError>;

    /// Atomically record verified burn: `burn_prepared` → `awaiting_attestation` with `source_tx_hash`.
    async fn record_verified_burn(
        &self,
        transfer_id: Uuid,
        expected_version: i32,
        tx_hash: &str,
    ) -> Result<CctpTransfer, CctpStoreError>;

    /// `attestation_ready` → `mint_prepared` with payload binding metadata (no signed payload).
    async fn record_mint_prepared(
        &self,
        transfer_id: Uuid,
        expected_version: i32,
        payload_hash: &str,
        expires_at: DateTime<Utc>,
        mint_submitter: Option<String>,
    ) -> Result<CctpTransfer, CctpStoreError>;

    /// Record destination mint tx hash after target verification.
    async fn record_mint_submission(
        &self,
        transfer_id: Uuid,
        expected_version: i32,
        tx_hash: &str,
    ) -> Result<CctpTransfer, CctpStoreError>;

    /// Mark mint completed after destination chain success evidence.
    async fn record_mint_completed(
        &self,
        transfer_id: Uuid,
        expected_version: i32,
    ) -> Result<CctpTransfer, CctpStoreError>;
}

#[derive(Debug, Clone, Default)]
pub struct TransferPatch {
    pub source_tx_hash: Option<String>,
    pub source_approval_tx_hash: Option<String>,
    pub source_approval_verified_at: Option<DateTime<Utc>>,
    pub destination_tx_hash: Option<String>,
    pub iris_message_hash: Option<String>,
    pub message_nonce: Option<String>,
    pub raw_message: Option<Vec<u8>>,
    pub attestation: Option<Vec<u8>>,
    pub runtime_fee_quote: Option<String>,
    pub max_fee: Option<String>,
    pub fee_expires_at: Option<DateTime<Utc>>,
    pub last_provider_error: Option<String>,
    pub last_provider_code: Option<String>,
    pub increment_retry: bool,
    pub clear_terminal_at: bool,
    pub mint_payload_hash: Option<String>,
    pub mint_payload_expires_at: Option<DateTime<Utc>>,
    pub clear_mint_payload: bool,
    pub mint_submitter: Option<String>,
    pub approval_payload_hash: Option<String>,
    pub approval_expiration_ledger: Option<u64>,
    pub burn_payload_hash: Option<String>,
    pub burn_prepare_step: Option<String>,
}

pub struct PgCctpTransferStore {
    pool: PgPool,
}

impl PgCctpTransferStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CctpTransferStore for PgCctpTransferStore {
    async fn insert(&self, transfer: &CctpTransfer) -> Result<(), CctpStoreError> {
        sqlx::query(
            r#"
            INSERT INTO cctp_transfers (
                transfer_id, support_reference_id, corridor_id, provider, direction,
                source_chain_id, destination_chain_id, source_asset, source_asset_canonical,
                destination_asset, destination_asset_canonical, sender, recipient, mint_submitter,
                amount, destination_amount, finality, runtime_fee_quote, max_fee,
                fee_expires_at, quote_expires_at, status, version, access_token_hash
            ) VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24
            )
            "#,
        )
        .bind(transfer.transfer_id)
        .bind(&transfer.support_reference_id)
        .bind(&transfer.corridor_id)
        .bind(&transfer.provider)
        .bind(direction_str(transfer.direction))
        .bind(&transfer.source_chain_id)
        .bind(&transfer.destination_chain_id)
        .bind(&transfer.source_asset)
        .bind(&transfer.source_asset_canonical)
        .bind(&transfer.destination_asset)
        .bind(&transfer.destination_asset_canonical)
        .bind(&transfer.sender)
        .bind(&transfer.recipient)
        .bind(&transfer.mint_submitter)
        .bind(&transfer.amount)
        .bind(&transfer.destination_amount)
        .bind(finality_str(transfer.finality))
        .bind(&transfer.runtime_fee_quote)
        .bind(&transfer.max_fee)
        .bind(transfer.fee_expires_at)
        .bind(transfer.quote_expires_at)
        .bind(status_str(transfer.status))
        .bind(transfer.version)
        .bind(&transfer.access_token_hash)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get(&self, transfer_id: Uuid) -> Result<Option<CctpTransfer>, CctpStoreError> {
        let row = sqlx::query_as::<_, TransferRow>(
            r#"
            SELECT * FROM cctp_transfers WHERE transfer_id = $1
            "#,
        )
        .bind(transfer_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.try_into_transfer()).transpose()?)
    }

    async fn get_authorized(
        &self,
        transfer_id: Uuid,
        access_token_hash: &str,
    ) -> Result<Option<CctpTransfer>, CctpStoreError> {
        let row = sqlx::query_as::<_, TransferRow>(
            r#"
            SELECT * FROM cctp_transfers
            WHERE transfer_id = $1 AND access_token_hash = $2
            "#,
        )
        .bind(transfer_id)
        .bind(access_token_hash)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.try_into_transfer()).transpose()?)
    }

    async fn try_acquire_poll_lease(
        &self,
        transfer_id: Uuid,
        lease_secs: i64,
        min_interval_secs: i64,
    ) -> Result<Option<(CctpTransfer, PollLeaseOutcome)>, CctpStoreError> {
        let row = sqlx::query_as::<_, TransferRow>(
            r#"
            UPDATE cctp_transfers
            SET poll_lease_until = NOW() + ($2 * INTERVAL '1 second'),
                last_polled_at = NOW(),
                updated_at = NOW()
            WHERE transfer_id = $1
              AND (poll_lease_until IS NULL OR poll_lease_until <= NOW())
              AND (
                last_polled_at IS NULL
                OR last_polled_at <= NOW() - ($3 * INTERVAL '1 second')
              )
            RETURNING *
            "#,
        )
        .bind(transfer_id)
        .bind(lease_secs)
        .bind(min_interval_secs)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(r) = row {
            return Ok(Some((r.try_into_transfer()?, PollLeaseOutcome::Acquired)));
        }

        let current = self.get(transfer_id).await?;
        Ok(current.map(|t| (t, PollLeaseOutcome::Skipped)))
    }

    async fn claim_reattest_lease(
        &self,
        transfer_id: Uuid,
        lease_owner_hash: &str,
        lease_secs: i64,
        max_attempts: u32,
    ) -> Result<Option<(CctpTransfer, ReattestClaimOutcome)>, CctpStoreError> {
        let lease_until = Utc::now() + chrono::Duration::seconds(lease_secs);
        let row = sqlx::query_as::<_, TransferRow>(
            r#"
            UPDATE cctp_transfers
            SET reattest_lease_owner_hash = $2,
                reattest_lease_until = $3,
                updated_at = NOW()
            WHERE transfer_id = $1
              AND status = 'attestation_failed'
              AND reattest_attempt_count < $4
              AND (reattest_cooldown_until IS NULL OR reattest_cooldown_until <= NOW())
              AND (
                reattest_lease_until IS NULL
                OR reattest_lease_until <= NOW()
                OR reattest_lease_owner_hash = $2
              )
            RETURNING *
            "#,
        )
        .bind(transfer_id)
        .bind(lease_owner_hash)
        .bind(lease_until)
        .bind(max_attempts as i32)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(r) = row {
            return Ok(Some((
                r.try_into_transfer()?,
                ReattestClaimOutcome::Claimed,
            )));
        }

        let current = self.get(transfer_id).await?;
        let Some(transfer) = current else {
            return Ok(None);
        };
        if transfer.status != CctpTransferStatus::AttestationFailed {
            return Ok(Some((transfer, ReattestClaimOutcome::NotAllowed)));
        }
        let lease_active = transfer
            .reattest_lease_until
            .is_some_and(|exp| exp > Utc::now());
        if lease_active && transfer.reattest_lease_owner_hash.as_deref() != Some(lease_owner_hash) {
            return Ok(Some((transfer, ReattestClaimOutcome::InProgress)));
        }
        Ok(Some((transfer, ReattestClaimOutcome::NotAllowed)))
    }

    async fn finalize_reattest_success(
        &self,
        transfer_id: Uuid,
        lease_owner_hash: &str,
    ) -> Result<Option<CctpTransfer>, CctpStoreError> {
        let row = sqlx::query_as::<_, TransferRow>(
            r#"
            UPDATE cctp_transfers
            SET status = 'awaiting_attestation',
                retry_count = retry_count + 1,
                reattest_attempt_count = reattest_attempt_count + 1,
                reattest_lease_owner_hash = NULL,
                reattest_lease_until = NULL,
                reattest_cooldown_until = NULL,
                last_provider_error = NULL,
                last_provider_code = NULL,
                terminal_at = NULL,
                version = version + 1,
                updated_at = NOW()
            WHERE transfer_id = $1
              AND status = 'attestation_failed'
              AND reattest_lease_owner_hash = $2
            RETURNING *
            "#,
        )
        .bind(transfer_id)
        .bind(lease_owner_hash)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.try_into_transfer()).transpose()?)
    }

    async fn finalize_reattest_failure(
        &self,
        transfer_id: Uuid,
        lease_owner_hash: &str,
        provider_code: &str,
        provider_error: &str,
        cooldown_secs: i64,
    ) -> Result<Option<CctpTransfer>, CctpStoreError> {
        let row = sqlx::query_as::<_, TransferRow>(
            r#"
            UPDATE cctp_transfers
            SET reattest_attempt_count = reattest_attempt_count + 1,
                reattest_lease_owner_hash = NULL,
                reattest_lease_until = NULL,
                reattest_cooldown_until = NOW() + ($5 * INTERVAL '1 second'),
                last_provider_code = $3,
                last_provider_error = $4,
                status = 'attestation_failed',
                version = version + 1,
                updated_at = NOW()
            WHERE transfer_id = $1
              AND status = 'attestation_failed'
              AND reattest_lease_owner_hash = $2
            RETURNING *
            "#,
        )
        .bind(transfer_id)
        .bind(lease_owner_hash)
        .bind(provider_code)
        .bind(provider_error)
        .bind(cooldown_secs)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.try_into_transfer()).transpose()?)
    }

    async fn transition(
        &self,
        transfer_id: Uuid,
        expected_version: i32,
        new_status: CctpTransferStatus,
        patch: TransferPatch,
    ) -> Result<CctpTransfer, CctpStoreError> {
        let current = self.get(transfer_id).await?;
        let Some(current) = current else {
            return Err(CctpStoreError::NotFound);
        };
        if current.version != expected_version {
            return Err(CctpStoreError::VersionConflict);
        }
        if !is_allowed_transition(current.status, new_status) {
            return Err(CctpStoreError::InvalidTransition);
        }

        let retry_count = if patch.increment_retry {
            current.retry_count + 1
        } else {
            current.retry_count
        };

        let terminal = is_terminal(new_status);
        let clear_terminal = patch.clear_terminal_at;

        validate_patch(&patch)?;

        let result = sqlx::query(
            r#"
            UPDATE cctp_transfers SET
                status = $2,
                source_tx_hash = COALESCE($3, source_tx_hash),
                source_approval_tx_hash = COALESCE($4, source_approval_tx_hash),
                destination_tx_hash = COALESCE($5, destination_tx_hash),
                iris_message_hash = COALESCE($6, iris_message_hash),
                message_nonce = COALESCE($7, message_nonce),
                raw_message = COALESCE($8, raw_message),
                attestation = COALESCE($9, attestation),
                runtime_fee_quote = COALESCE($10, runtime_fee_quote),
                max_fee = COALESCE($11, max_fee),
                fee_expires_at = COALESCE($12, fee_expires_at),
                last_provider_error = COALESCE($13, last_provider_error),
                last_provider_code = COALESCE($14, last_provider_code),
                mint_payload_hash = COALESCE($15, mint_payload_hash),
                mint_payload_expires_at = COALESCE($16, mint_payload_expires_at),
                mint_submitter = COALESCE($17, mint_submitter),
                approval_payload_hash = COALESCE($18, approval_payload_hash),
                approval_expiration_ledger = COALESCE($19, approval_expiration_ledger),
                burn_payload_hash = COALESCE($20, burn_payload_hash),
                burn_prepare_step = COALESCE($21, burn_prepare_step),
                retry_count = $22,
                version = version + 1,
                updated_at = NOW(),
                terminal_at = CASE
                    WHEN $23 THEN NOW()
                    WHEN $24 THEN NULL
                    ELSE terminal_at
                END
            WHERE transfer_id = $1 AND version = $25
            "#,
        )
        .bind(transfer_id)
        .bind(status_str(new_status))
        .bind(&patch.source_tx_hash)
        .bind(&patch.source_approval_tx_hash)
        .bind(&patch.destination_tx_hash)
        .bind(&patch.iris_message_hash)
        .bind(&patch.message_nonce)
        .bind(&patch.raw_message)
        .bind(&patch.attestation)
        .bind(&patch.runtime_fee_quote)
        .bind(&patch.max_fee)
        .bind(patch.fee_expires_at)
        .bind(&patch.last_provider_error)
        .bind(&patch.last_provider_code)
        .bind(&patch.mint_payload_hash)
        .bind(patch.mint_payload_expires_at)
        .bind(&patch.mint_submitter)
        .bind(&patch.approval_payload_hash)
        .bind(patch.approval_expiration_ledger.map(|v| v as i64))
        .bind(&patch.burn_payload_hash)
        .bind(&patch.burn_prepare_step)
        .bind(retry_count as i32)
        .bind(terminal)
        .bind(clear_terminal)
        .bind(expected_version)
        .execute(&self.pool)
        .await;

        match result {
            Ok(r) if r.rows_affected() == 0 => Err(CctpStoreError::VersionConflict),
            Ok(_) => self.get(transfer_id).await?.ok_or(CctpStoreError::NotFound),
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("idx_cctp_source_tx_hash_unique") {
                    Err(CctpStoreError::DuplicateSourceTxHash)
                } else {
                    Err(CctpStoreError::Database(e))
                }
            }
        }
    }

    async fn record_verified_burn(
        &self,
        transfer_id: Uuid,
        expected_version: i32,
        tx_hash: &str,
    ) -> Result<CctpTransfer, CctpStoreError> {
        check_str_len("tx_hash", tx_hash, MAX_TX_HASH_LEN)
            .map_err(|_| CctpStoreError::PayloadTooLarge)?;

        let mut tx = self.pool.begin().await?;
        let current = sqlx::query_as::<_, TransferRow>(
            r#"SELECT * FROM cctp_transfers WHERE transfer_id = $1 FOR UPDATE"#,
        )
        .bind(transfer_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(current) = current else {
            return Err(CctpStoreError::NotFound);
        };
        if current.version != expected_version {
            return Err(CctpStoreError::VersionConflict);
        }
        let status = parse_status(&current.status)?;

        if status != CctpTransferStatus::BurnPrepared {
            return Err(CctpStoreError::InvalidTransition);
        }

        sqlx::query(
            r#"
            UPDATE cctp_transfers SET
                status = 'awaiting_attestation',
                source_tx_hash = $2,
                version = version + 1,
                updated_at = NOW()
            WHERE transfer_id = $1 AND version = $3
            "#,
        )
        .bind(transfer_id)
        .bind(tx_hash)
        .bind(expected_version)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        self.get(transfer_id).await?.ok_or(CctpStoreError::NotFound)
    }

    async fn record_approval_submission(
        &self,
        transfer_id: Uuid,
        expected_version: i32,
        tx_hash: &str,
        verified_at: DateTime<Utc>,
    ) -> Result<CctpTransfer, CctpStoreError> {
        check_str_len("tx_hash", tx_hash, MAX_TX_HASH_LEN)
            .map_err(|_| CctpStoreError::PayloadTooLarge)?;

        let current = self.get(transfer_id).await?;
        let Some(current) = current else {
            return Err(CctpStoreError::NotFound);
        };
        if current.version != expected_version {
            return Err(CctpStoreError::VersionConflict);
        }
        if current.status != CctpTransferStatus::BurnPrepared {
            return Err(CctpStoreError::InvalidTransition);
        }

        let result = sqlx::query(
            r#"
            UPDATE cctp_transfers SET
                source_approval_tx_hash = $2,
                source_approval_verified_at = $3,
                version = version + 1,
                updated_at = NOW()
            WHERE transfer_id = $1 AND version = $4
            "#,
        )
        .bind(transfer_id)
        .bind(tx_hash)
        .bind(verified_at)
        .bind(expected_version)
        .execute(&self.pool)
        .await;

        match result {
            Ok(r) if r.rows_affected() == 0 => Err(CctpStoreError::VersionConflict),
            Ok(_) => self.get(transfer_id).await?.ok_or(CctpStoreError::NotFound),
            Err(e) => Err(CctpStoreError::Database(e)),
        }
    }

    async fn record_mint_prepared(
        &self,
        transfer_id: Uuid,
        expected_version: i32,
        payload_hash: &str,
        expires_at: DateTime<Utc>,
        mint_submitter: Option<String>,
    ) -> Result<CctpTransfer, CctpStoreError> {
        check_str_len("payload_hash", payload_hash, 128)
            .map_err(|_| CctpStoreError::PayloadTooLarge)?;
        self.transition(
            transfer_id,
            expected_version,
            CctpTransferStatus::MintPrepared,
            TransferPatch {
                mint_payload_hash: Some(payload_hash.to_string()),
                mint_payload_expires_at: Some(expires_at),
                mint_submitter,
                ..Default::default()
            },
        )
        .await
    }

    async fn record_mint_submission(
        &self,
        transfer_id: Uuid,
        expected_version: i32,
        tx_hash: &str,
    ) -> Result<CctpTransfer, CctpStoreError> {
        check_str_len("tx_hash", tx_hash, MAX_TX_HASH_LEN)
            .map_err(|_| CctpStoreError::PayloadTooLarge)?;
        self.transition(
            transfer_id,
            expected_version,
            CctpTransferStatus::MintSubmitted,
            TransferPatch {
                destination_tx_hash: Some(tx_hash.to_string()),
                ..Default::default()
            },
        )
        .await
    }

    async fn record_mint_completed(
        &self,
        transfer_id: Uuid,
        expected_version: i32,
    ) -> Result<CctpTransfer, CctpStoreError> {
        self.transition(
            transfer_id,
            expected_version,
            CctpTransferStatus::Completed,
            TransferPatch::default(),
        )
        .await
    }
}

#[derive(Default)]
pub struct InMemoryCctpTransferStore {
    transfers: Mutex<HashMap<Uuid, CctpTransfer>>,
    source_tx_hashes: Mutex<HashMap<String, Uuid>>,
    message_nonces: Mutex<HashMap<(String, String), Uuid>>,
}

#[async_trait]
impl CctpTransferStore for InMemoryCctpTransferStore {
    async fn insert(&self, transfer: &CctpTransfer) -> Result<(), CctpStoreError> {
        let mut guard = self.transfers.lock().unwrap();
        if guard.contains_key(&transfer.transfer_id) {
            return Err(CctpStoreError::Database(sqlx::Error::RowNotFound));
        }
        guard.insert(transfer.transfer_id, transfer.clone());
        Ok(())
    }

    async fn get(&self, transfer_id: Uuid) -> Result<Option<CctpTransfer>, CctpStoreError> {
        Ok(self.transfers.lock().unwrap().get(&transfer_id).cloned())
    }

    async fn get_authorized(
        &self,
        transfer_id: Uuid,
        access_token_hash: &str,
    ) -> Result<Option<CctpTransfer>, CctpStoreError> {
        let guard = self.transfers.lock().unwrap();
        let Some(transfer) = guard.get(&transfer_id) else {
            return Ok(None);
        };
        if transfer.access_token_hash.as_deref() != Some(access_token_hash) {
            return Ok(None);
        }
        Ok(Some(transfer.clone()))
    }

    async fn try_acquire_poll_lease(
        &self,
        transfer_id: Uuid,
        lease_secs: i64,
        min_interval_secs: i64,
    ) -> Result<Option<(CctpTransfer, PollLeaseOutcome)>, CctpStoreError> {
        use chrono::Duration;
        let mut guard = self.transfers.lock().unwrap();
        let Some(transfer) = guard.get_mut(&transfer_id) else {
            return Ok(None);
        };
        let now = Utc::now();
        let lease_active = transfer.poll_lease_until.is_some_and(|u| u > now);
        let interval_ok = transfer
            .last_polled_at
            .map(|t| t + Duration::seconds(min_interval_secs) <= now)
            .unwrap_or(true);
        if lease_active || !interval_ok {
            return Ok(Some((transfer.clone(), PollLeaseOutcome::Skipped)));
        }
        transfer.poll_lease_until = Some(now + Duration::seconds(lease_secs));
        transfer.last_polled_at = Some(now);
        transfer.updated_at = now;
        Ok(Some((transfer.clone(), PollLeaseOutcome::Acquired)))
    }

    async fn claim_reattest_lease(
        &self,
        transfer_id: Uuid,
        lease_owner_hash: &str,
        lease_secs: i64,
        max_attempts: u32,
    ) -> Result<Option<(CctpTransfer, ReattestClaimOutcome)>, CctpStoreError> {
        use chrono::Duration;
        let mut guard = self.transfers.lock().unwrap();
        let Some(transfer) = guard.get_mut(&transfer_id) else {
            return Ok(None);
        };
        let now = Utc::now();
        if transfer.status != CctpTransferStatus::AttestationFailed
            || transfer.reattest_attempt_count >= max_attempts
            || transfer
                .reattest_cooldown_until
                .is_some_and(|until| until > now)
        {
            return Ok(Some((transfer.clone(), ReattestClaimOutcome::NotAllowed)));
        }
        let lease_active = transfer.reattest_lease_until.is_some_and(|exp| exp > now);
        if lease_active && transfer.reattest_lease_owner_hash.as_deref() != Some(lease_owner_hash) {
            return Ok(Some((transfer.clone(), ReattestClaimOutcome::InProgress)));
        }
        if lease_active && transfer.reattest_lease_owner_hash.as_deref() == Some(lease_owner_hash) {
            transfer.reattest_lease_until = Some(now + Duration::seconds(lease_secs));
            transfer.updated_at = now;
            return Ok(Some((transfer.clone(), ReattestClaimOutcome::Claimed)));
        }
        transfer.reattest_lease_owner_hash = Some(lease_owner_hash.to_string());
        transfer.reattest_lease_until = Some(now + Duration::seconds(lease_secs));
        transfer.updated_at = now;
        Ok(Some((transfer.clone(), ReattestClaimOutcome::Claimed)))
    }

    async fn finalize_reattest_success(
        &self,
        transfer_id: Uuid,
        lease_owner_hash: &str,
    ) -> Result<Option<CctpTransfer>, CctpStoreError> {
        let mut guard = self.transfers.lock().unwrap();
        let Some(transfer) = guard.get_mut(&transfer_id) else {
            return Ok(None);
        };
        if transfer.status != CctpTransferStatus::AttestationFailed
            || transfer.reattest_lease_owner_hash.as_deref() != Some(lease_owner_hash)
        {
            return Ok(None);
        }
        transfer.status = CctpTransferStatus::AwaitingAttestation;
        transfer.retry_count += 1;
        transfer.reattest_attempt_count += 1;
        transfer.reattest_lease_owner_hash = None;
        transfer.reattest_lease_until = None;
        transfer.reattest_cooldown_until = None;
        transfer.last_provider_error = None;
        transfer.last_provider_code = None;
        transfer.terminal_at = None;
        transfer.version += 1;
        transfer.updated_at = Utc::now();
        Ok(Some(transfer.clone()))
    }

    async fn finalize_reattest_failure(
        &self,
        transfer_id: Uuid,
        lease_owner_hash: &str,
        provider_code: &str,
        provider_error: &str,
        cooldown_secs: i64,
    ) -> Result<Option<CctpTransfer>, CctpStoreError> {
        use chrono::Duration;
        let mut guard = self.transfers.lock().unwrap();
        let Some(transfer) = guard.get_mut(&transfer_id) else {
            return Ok(None);
        };
        if transfer.status != CctpTransferStatus::AttestationFailed
            || transfer.reattest_lease_owner_hash.as_deref() != Some(lease_owner_hash)
        {
            return Ok(None);
        }
        transfer.reattest_attempt_count += 1;
        transfer.reattest_lease_owner_hash = None;
        transfer.reattest_lease_until = None;
        transfer.reattest_cooldown_until = Some(Utc::now() + Duration::seconds(cooldown_secs));
        transfer.last_provider_code = Some(provider_code.to_string());
        transfer.last_provider_error = Some(provider_error.to_string());
        transfer.version += 1;
        transfer.updated_at = Utc::now();
        Ok(Some(transfer.clone()))
    }

    async fn transition(
        &self,
        transfer_id: Uuid,
        expected_version: i32,
        new_status: CctpTransferStatus,
        patch: TransferPatch,
    ) -> Result<CctpTransfer, CctpStoreError> {
        let mut guard = self.transfers.lock().unwrap();
        let transfer = guard
            .get_mut(&transfer_id)
            .ok_or(CctpStoreError::NotFound)?;
        if transfer.version != expected_version {
            return Err(CctpStoreError::VersionConflict);
        }
        if !is_allowed_transition(transfer.status, new_status) {
            return Err(CctpStoreError::InvalidTransition);
        }

        validate_patch(&patch)?;

        if let Some(hash) = &patch.source_tx_hash {
            let mut hashes = self.source_tx_hashes.lock().unwrap();
            if let Some(existing) = hashes.get(hash) {
                if *existing != transfer_id {
                    return Err(CctpStoreError::DuplicateSourceTxHash);
                }
            } else {
                hashes.insert(hash.clone(), transfer_id);
            }
            transfer.source_tx_hash = Some(hash.clone());
        }
        if let Some(v) = patch.source_approval_tx_hash {
            transfer.source_approval_tx_hash = Some(v);
        }
        if let Some(v) = patch.destination_tx_hash {
            transfer.destination_tx_hash = Some(v);
        }
        if let Some(v) = patch.iris_message_hash {
            transfer.iris_message_hash = Some(v);
        }
        if let Some(v) = patch.message_nonce {
            let mut nonces = self.message_nonces.lock().unwrap();
            let key = (transfer.source_chain_id.clone(), v.clone());
            if let Some(existing) = nonces.get(&key) {
                if *existing != transfer_id {
                    return Err(CctpStoreError::InvalidTransition);
                }
            } else {
                nonces.insert(key, transfer_id);
            }
            transfer.message_nonce = Some(v);
        }
        if let Some(v) = patch.raw_message {
            transfer.raw_message = Some(v);
        }
        if let Some(v) = patch.attestation {
            transfer.attestation = Some(v);
        }
        if let Some(v) = patch.runtime_fee_quote {
            transfer.runtime_fee_quote = Some(v);
        }
        if let Some(v) = patch.max_fee {
            transfer.max_fee = Some(v);
        }
        if let Some(v) = patch.fee_expires_at {
            transfer.fee_expires_at = Some(v);
        }
        if let Some(v) = patch.last_provider_error {
            transfer.last_provider_error = Some(v);
        }
        if let Some(v) = patch.last_provider_code {
            transfer.last_provider_code = Some(v);
        }
        if patch.increment_retry {
            transfer.retry_count += 1;
        }

        transfer.status = new_status;
        transfer.version += 1;
        transfer.updated_at = Utc::now();
        if patch.clear_terminal_at {
            transfer.terminal_at = None;
        } else if is_terminal(new_status) {
            transfer.terminal_at = Some(Utc::now());
        }
        if let Some(v) = patch.mint_payload_hash {
            transfer.mint_payload_hash = Some(v);
        }
        if let Some(v) = patch.mint_payload_expires_at {
            transfer.mint_payload_expires_at = Some(v);
        }
        if patch.clear_mint_payload {
            transfer.mint_payload_hash = None;
            transfer.mint_payload_expires_at = None;
        }
        if let Some(v) = patch.mint_submitter {
            transfer.mint_submitter = Some(v);
        }
        if let Some(v) = patch.approval_payload_hash {
            transfer.approval_payload_hash = Some(v);
        }
        if let Some(v) = patch.approval_expiration_ledger {
            transfer.approval_expiration_ledger = Some(v);
        }
        if let Some(v) = patch.burn_payload_hash {
            transfer.burn_payload_hash = Some(v);
        }
        if let Some(v) = patch.burn_prepare_step {
            transfer.burn_prepare_step = Some(v);
        }
        Ok(transfer.clone())
    }

    async fn record_approval_submission(
        &self,
        transfer_id: Uuid,
        expected_version: i32,
        tx_hash: &str,
        verified_at: DateTime<Utc>,
    ) -> Result<CctpTransfer, CctpStoreError> {
        check_str_len("tx_hash", tx_hash, MAX_TX_HASH_LEN)
            .map_err(|_| CctpStoreError::PayloadTooLarge)?;

        let mut guard = self.transfers.lock().unwrap();
        let transfer = guard
            .get_mut(&transfer_id)
            .ok_or(CctpStoreError::NotFound)?;
        if transfer.version != expected_version {
            return Err(CctpStoreError::VersionConflict);
        }
        if transfer.status != CctpTransferStatus::BurnPrepared {
            return Err(CctpStoreError::InvalidTransition);
        }
        if let Some(existing) = transfer.source_approval_tx_hash.as_deref() {
            if existing != tx_hash {
                return Err(CctpStoreError::InvalidTransition);
            }
            return Ok(transfer.clone());
        }
        transfer.source_approval_tx_hash = Some(tx_hash.to_string());
        transfer.source_approval_verified_at = Some(verified_at);
        transfer.version += 1;
        transfer.updated_at = Utc::now();
        Ok(transfer.clone())
    }

    async fn record_mint_prepared(
        &self,
        transfer_id: Uuid,
        expected_version: i32,
        payload_hash: &str,
        expires_at: DateTime<Utc>,
        mint_submitter: Option<String>,
    ) -> Result<CctpTransfer, CctpStoreError> {
        self.transition(
            transfer_id,
            expected_version,
            CctpTransferStatus::MintPrepared,
            TransferPatch {
                mint_payload_hash: Some(payload_hash.to_string()),
                mint_payload_expires_at: Some(expires_at),
                mint_submitter,
                ..Default::default()
            },
        )
        .await
    }

    async fn record_mint_submission(
        &self,
        transfer_id: Uuid,
        expected_version: i32,
        tx_hash: &str,
    ) -> Result<CctpTransfer, CctpStoreError> {
        self.transition(
            transfer_id,
            expected_version,
            CctpTransferStatus::MintSubmitted,
            TransferPatch {
                destination_tx_hash: Some(tx_hash.to_string()),
                ..Default::default()
            },
        )
        .await
    }

    async fn record_mint_completed(
        &self,
        transfer_id: Uuid,
        expected_version: i32,
    ) -> Result<CctpTransfer, CctpStoreError> {
        self.transition(
            transfer_id,
            expected_version,
            CctpTransferStatus::Completed,
            TransferPatch::default(),
        )
        .await
    }

    async fn record_verified_burn(
        &self,
        transfer_id: Uuid,
        expected_version: i32,
        tx_hash: &str,
    ) -> Result<CctpTransfer, CctpStoreError> {
        check_str_len("tx_hash", tx_hash, MAX_TX_HASH_LEN)
            .map_err(|_| CctpStoreError::PayloadTooLarge)?;

        let mut guard = self.transfers.lock().unwrap();
        let transfer = guard
            .get_mut(&transfer_id)
            .ok_or(CctpStoreError::NotFound)?;
        if transfer.version != expected_version {
            return Err(CctpStoreError::VersionConflict);
        }
        if transfer.status != CctpTransferStatus::BurnPrepared {
            return Err(CctpStoreError::InvalidTransition);
        }

        let mut hashes = self.source_tx_hashes.lock().unwrap();
        if let Some(existing) = hashes.get(tx_hash) {
            if *existing != transfer_id {
                return Err(CctpStoreError::DuplicateSourceTxHash);
            }
        } else {
            hashes.insert(tx_hash.to_string(), transfer_id);
        }

        transfer.source_tx_hash = Some(tx_hash.to_string());
        transfer.status = CctpTransferStatus::AwaitingAttestation;
        transfer.version += 1;
        transfer.updated_at = Utc::now();
        Ok(transfer.clone())
    }
}

#[derive(sqlx::FromRow)]
struct TransferRow {
    transfer_id: Uuid,
    support_reference_id: String,
    corridor_id: String,
    provider: String,
    direction: String,
    source_chain_id: String,
    destination_chain_id: String,
    source_asset: String,
    source_asset_canonical: String,
    destination_asset: String,
    destination_asset_canonical: String,
    sender: String,
    recipient: String,
    mint_submitter: Option<String>,
    amount: String,
    destination_amount: String,
    finality: String,
    runtime_fee_quote: Option<String>,
    max_fee: Option<String>,
    fee_expires_at: Option<DateTime<Utc>>,
    quote_expires_at: DateTime<Utc>,
    status: String,
    source_tx_hash: Option<String>,
    source_approval_tx_hash: Option<String>,
    source_approval_verified_at: Option<DateTime<Utc>>,
    destination_tx_hash: Option<String>,
    iris_message_hash: Option<String>,
    message_nonce: Option<String>,
    raw_message: Option<Vec<u8>>,
    attestation: Option<Vec<u8>>,
    retry_count: i32,
    last_provider_error: Option<String>,
    last_provider_code: Option<String>,
    version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    terminal_at: Option<DateTime<Utc>>,
    mint_payload_hash: Option<String>,
    mint_payload_expires_at: Option<DateTime<Utc>>,
    approval_payload_hash: Option<String>,
    approval_expiration_ledger: Option<i64>,
    burn_payload_hash: Option<String>,
    burn_prepare_step: Option<String>,
    access_token_hash: Option<String>,
    last_polled_at: Option<DateTime<Utc>>,
    poll_lease_until: Option<DateTime<Utc>>,
    reattest_lease_owner_hash: Option<String>,
    reattest_lease_until: Option<DateTime<Utc>>,
    reattest_attempt_count: i32,
    reattest_cooldown_until: Option<DateTime<Utc>>,
}

impl TransferRow {
    fn try_into_transfer(self) -> Result<CctpTransfer, CctpStoreError> {
        Ok(CctpTransfer {
            transfer_id: self.transfer_id,
            support_reference_id: self.support_reference_id,
            corridor_id: self.corridor_id,
            provider: self.provider,
            direction: parse_direction(&self.direction)?,
            source_chain_id: self.source_chain_id,
            destination_chain_id: self.destination_chain_id,
            source_asset: self.source_asset,
            source_asset_canonical: self.source_asset_canonical,
            destination_asset: self.destination_asset,
            destination_asset_canonical: self.destination_asset_canonical,
            sender: self.sender,
            recipient: self.recipient,
            mint_submitter: self.mint_submitter,
            amount: self.amount,
            destination_amount: self.destination_amount,
            finality: parse_finality(&self.finality),
            runtime_fee_quote: self.runtime_fee_quote,
            max_fee: self.max_fee,
            fee_expires_at: self.fee_expires_at,
            quote_expires_at: self.quote_expires_at,
            status: parse_status(&self.status)?,
            source_tx_hash: self.source_tx_hash,
            source_approval_tx_hash: self.source_approval_tx_hash,
            source_approval_verified_at: self.source_approval_verified_at,
            destination_tx_hash: self.destination_tx_hash,
            iris_message_hash: self.iris_message_hash,
            message_nonce: self.message_nonce,
            raw_message: self.raw_message,
            attestation: self.attestation,
            retry_count: self.retry_count as u32,
            last_provider_error: self.last_provider_error,
            last_provider_code: self.last_provider_code,
            version: self.version,
            created_at: self.created_at,
            updated_at: self.updated_at,
            terminal_at: self.terminal_at,
            mint_payload_hash: self.mint_payload_hash,
            mint_payload_expires_at: self.mint_payload_expires_at,
            approval_payload_hash: self.approval_payload_hash,
            approval_expiration_ledger: self.approval_expiration_ledger.map(|v| v as u64),
            burn_payload_hash: self.burn_payload_hash,
            burn_prepare_step: self.burn_prepare_step,
            access_token_hash: self.access_token_hash,
            last_polled_at: self.last_polled_at,
            poll_lease_until: self.poll_lease_until,
            reattest_lease_owner_hash: self.reattest_lease_owner_hash,
            reattest_lease_until: self.reattest_lease_until,
            reattest_attempt_count: self.reattest_attempt_count as u32,
            reattest_cooldown_until: self.reattest_cooldown_until,
        })
    }
}

pub(crate) fn direction_str(d: CctpDirection) -> &'static str {
    match d {
        CctpDirection::StellarToEvm => "stellar_to_evm",
        CctpDirection::EvmToStellar => "evm_to_stellar",
    }
}

pub(crate) fn finality_str(f: CctpFinality) -> &'static str {
    match f {
        CctpFinality::Standard => "standard",
        CctpFinality::Fast => "fast",
    }
}

pub(crate) fn status_str(s: CctpTransferStatus) -> &'static str {
    match s {
        CctpTransferStatus::Created => "created",
        CctpTransferStatus::BurnPrepared => "burn_prepared",
        CctpTransferStatus::BurnSubmitted => "burn_submitted",
        CctpTransferStatus::AwaitingAttestation => "awaiting_attestation",
        CctpTransferStatus::AttestationReady => "attestation_ready",
        CctpTransferStatus::MintPrepared => "mint_prepared",
        CctpTransferStatus::MintSubmitted => "mint_submitted",
        CctpTransferStatus::Completed => "completed",
        CctpTransferStatus::AttestationFailed => "attestation_failed",
        CctpTransferStatus::MintFailedRetryable => "mint_failed_retryable",
        CctpTransferStatus::Cancelled => "cancelled",
        CctpTransferStatus::ProviderKilled => "provider_killed",
    }
}

fn parse_direction(s: &str) -> Result<CctpDirection, CctpStoreError> {
    match s {
        "stellar_to_evm" => Ok(CctpDirection::StellarToEvm),
        "evm_to_stellar" => Ok(CctpDirection::EvmToStellar),
        other => Err(CctpStoreError::InvalidDirection(other.to_string())),
    }
}

fn parse_finality(s: &str) -> CctpFinality {
    match s {
        "fast" => CctpFinality::Fast,
        _ => CctpFinality::Standard,
    }
}

fn parse_status(s: &str) -> Result<CctpTransferStatus, CctpStoreError> {
    match s {
        "created" => Ok(CctpTransferStatus::Created),
        "burn_prepared" => Ok(CctpTransferStatus::BurnPrepared),
        "burn_submitted" => Ok(CctpTransferStatus::BurnSubmitted),
        "awaiting_attestation" => Ok(CctpTransferStatus::AwaitingAttestation),
        "attestation_ready" => Ok(CctpTransferStatus::AttestationReady),
        "mint_prepared" => Ok(CctpTransferStatus::MintPrepared),
        "mint_submitted" => Ok(CctpTransferStatus::MintSubmitted),
        "completed" => Ok(CctpTransferStatus::Completed),
        "attestation_failed" => Ok(CctpTransferStatus::AttestationFailed),
        "mint_failed_retryable" => Ok(CctpTransferStatus::MintFailedRetryable),
        "cancelled" => Ok(CctpTransferStatus::Cancelled),
        "provider_killed" => Ok(CctpTransferStatus::ProviderKilled),
        other => Err(CctpStoreError::InvalidStatus(other.to_string())),
    }
}

fn validate_patch(patch: &TransferPatch) -> Result<(), CctpStoreError> {
    if let Some(raw) = &patch.raw_message {
        check_byte_len("raw_message", raw, MAX_RAW_MESSAGE_BYTES)
            .map_err(|_| CctpStoreError::PayloadTooLarge)?;
    }
    if let Some(att) = &patch.attestation {
        check_byte_len("attestation", att, MAX_ATTESTATION_BYTES)
            .map_err(|_| CctpStoreError::PayloadTooLarge)?;
    }
    if let Some(hash) = &patch.source_tx_hash {
        check_str_len("source_tx_hash", hash, MAX_TX_HASH_LEN)
            .map_err(|_| CctpStoreError::PayloadTooLarge)?;
    }
    if let Some(nonce) = &patch.message_nonce {
        check_str_len("message_nonce", nonce, MAX_MESSAGE_NONCE_LEN)
            .map_err(|_| CctpStoreError::PayloadTooLarge)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use std::sync::Arc;

    fn sample_transfer() -> CctpTransfer {
        let now = Utc::now();
        CctpTransfer {
            transfer_id: Uuid::new_v4(),
            support_reference_id: "sup-1".into(),
            corridor_id: "c".into(),
            provider: "circle-cctp".into(),
            direction: CctpDirection::StellarToEvm,
            source_chain_id: "stellar:testnet".into(),
            destination_chain_id: "eip155:11155111".into(),
            source_asset: "a".into(),
            source_asset_canonical: "a".into(),
            destination_asset: "b".into(),
            destination_asset_canonical: "b".into(),
            sender: "".into(),
            recipient: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0".into(),
            mint_submitter: None,
            amount: "10".into(),
            destination_amount: "10".into(),
            finality: CctpFinality::Standard,
            runtime_fee_quote: None,
            max_fee: None,
            fee_expires_at: None,
            quote_expires_at: now + Duration::minutes(5),
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
            access_token_hash: None,
            last_polled_at: None,
            poll_lease_until: None,
            reattest_lease_owner_hash: None,
            reattest_lease_until: None,
            reattest_attempt_count: 0,
            reattest_cooldown_until: None,
        }
    }

    #[tokio::test]
    async fn in_memory_transition_happy_path() {
        let store = InMemoryCctpTransferStore::default();
        let t = sample_transfer();
        let id = t.transfer_id;
        store.insert(&t).await.unwrap();

        let updated = store
            .transition(
                id,
                1,
                CctpTransferStatus::BurnPrepared,
                TransferPatch::default(),
            )
            .await
            .unwrap();
        assert_eq!(updated.status, CctpTransferStatus::BurnPrepared);
        assert_eq!(updated.version, 2);
    }

    #[tokio::test]
    async fn in_memory_rejects_invalid_transition() {
        let store = InMemoryCctpTransferStore::default();
        let t = sample_transfer();
        let id = t.transfer_id;
        store.insert(&t).await.unwrap();
        let err = store
            .transition(
                id,
                1,
                CctpTransferStatus::Completed,
                TransferPatch::default(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CctpStoreError::InvalidTransition));
    }

    #[tokio::test]
    async fn duplicate_source_tx_hash_rejected() {
        let store = InMemoryCctpTransferStore::default();
        let t1 = sample_transfer();
        let t2 = sample_transfer();
        let id1 = t1.transfer_id;
        let id2 = t2.transfer_id;
        store.insert(&t1).await.unwrap();
        store.insert(&t2).await.unwrap();
        store
            .transition(
                id1,
                1,
                CctpTransferStatus::BurnPrepared,
                TransferPatch::default(),
            )
            .await
            .unwrap();
        store
            .transition(
                id1,
                2,
                CctpTransferStatus::BurnSubmitted,
                TransferPatch {
                    source_tx_hash: Some("hash1".into()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        store
            .transition(
                id2,
                1,
                CctpTransferStatus::BurnPrepared,
                TransferPatch::default(),
            )
            .await
            .unwrap();
        let err = store
            .transition(
                id2,
                2,
                CctpTransferStatus::BurnSubmitted,
                TransferPatch {
                    source_tx_hash: Some("hash1".into()),
                    ..Default::default()
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CctpStoreError::DuplicateSourceTxHash));
    }

    #[test]
    fn unknown_direction_errors() {
        let row = TransferRow {
            transfer_id: Uuid::new_v4(),
            support_reference_id: "s".into(),
            corridor_id: "c".into(),
            provider: "p".into(),
            direction: "invalid".into(),
            source_chain_id: "a".into(),
            destination_chain_id: "b".into(),
            source_asset: "a".into(),
            source_asset_canonical: "a".into(),
            destination_asset: "b".into(),
            destination_asset_canonical: "b".into(),
            sender: "".into(),
            recipient: "r".into(),
            mint_submitter: None,
            amount: "1".into(),
            destination_amount: "1".into(),
            finality: "standard".into(),
            runtime_fee_quote: None,
            max_fee: None,
            fee_expires_at: None,
            quote_expires_at: Utc::now(),
            status: "created".into(),
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
            created_at: Utc::now(),
            updated_at: Utc::now(),
            terminal_at: None,
            mint_payload_hash: None,
            mint_payload_expires_at: None,
            approval_payload_hash: None,
            approval_expiration_ledger: None,
            burn_payload_hash: None,
            burn_prepare_step: None,
            access_token_hash: None,
            last_polled_at: None,
            poll_lease_until: None,
            reattest_lease_owner_hash: None,
            reattest_lease_until: None,
            reattest_attempt_count: 0,
            reattest_cooldown_until: None,
        };
        let err = row.try_into_transfer().unwrap_err();
        assert!(matches!(err, CctpStoreError::InvalidDirection(_)));
    }

    #[tokio::test]
    async fn in_memory_reattest_claim_finalize_success_increments_counters_once() {
        let store = InMemoryCctpTransferStore::default();
        let mut t = sample_transfer();
        t.status = CctpTransferStatus::AttestationFailed;
        t.message_nonce = Some("nonce-1".into());
        let id = t.transfer_id;
        store.insert(&t).await.unwrap();

        let (_, claim) = store
            .claim_reattest_lease(id, "owner-a", 30, 5)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(claim, ReattestClaimOutcome::Claimed);
        let mid = store.get(id).await.unwrap().unwrap();
        assert_eq!(mid.status, CctpTransferStatus::AttestationFailed);
        assert_eq!(mid.reattest_lease_owner_hash.as_deref(), Some("owner-a"));

        let updated = store
            .finalize_reattest_success(id, "owner-a")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.status, CctpTransferStatus::AwaitingAttestation);
        assert_eq!(updated.retry_count, 1);
        assert_eq!(updated.reattest_attempt_count, 1);
        assert!(updated.reattest_lease_owner_hash.is_none());
        assert!(updated.last_provider_code.is_none());
    }

    #[tokio::test]
    async fn in_memory_reattest_failure_sets_cooldown_without_retry_count() {
        let store = InMemoryCctpTransferStore::default();
        let mut t = sample_transfer();
        t.status = CctpTransferStatus::AttestationFailed;
        t.message_nonce = Some("nonce-2".into());
        let id = t.transfer_id;
        store.insert(&t).await.unwrap();

        store
            .claim_reattest_lease(id, "owner-b", 30, 5)
            .await
            .unwrap();
        let failed = store
            .finalize_reattest_failure(id, "owner-b", "iris_reattest_failed", "provider down", 60)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(failed.status, CctpTransferStatus::AttestationFailed);
        assert_eq!(failed.retry_count, 0);
        assert_eq!(failed.reattest_attempt_count, 1);
        assert_eq!(
            failed.last_provider_code.as_deref(),
            Some("iris_reattest_failed")
        );
        assert!(failed.reattest_cooldown_until.is_some());
    }

    #[tokio::test]
    async fn in_memory_reattest_concurrent_claim_single_owner() {
        let store = Arc::new(InMemoryCctpTransferStore::default());
        let mut t = sample_transfer();
        t.status = CctpTransferStatus::AttestationFailed;
        t.message_nonce = Some("nonce-3".into());
        let id = t.transfer_id;
        store.insert(&t).await.unwrap();

        let s1 = store.clone();
        let s2 = store.clone();
        let (r1, r2) = tokio::join!(
            s1.claim_reattest_lease(id, "owner-1", 30, 5),
            s2.claim_reattest_lease(id, "owner-2", 30, 5),
        );
        let outcomes: Vec<_> = [r1.unwrap().unwrap(), r2.unwrap().unwrap()]
            .into_iter()
            .map(|(_, o)| o)
            .collect();
        assert_eq!(
            outcomes
                .iter()
                .filter(|o| **o == ReattestClaimOutcome::Claimed)
                .count(),
            1
        );
    }
}
