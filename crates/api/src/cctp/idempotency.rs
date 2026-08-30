//! Durable atomic quote idempotency for `POST /api/v2/bridge/cctp/quote`.

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Mutex;
use thiserror::Error;
use uuid::Uuid;

use crate::cctp::access::hash_lease_owner;
use crate::cctp::bounds::check_str_len;
use crate::cctp::store::{CctpStoreError, CctpTransfer, CctpTransferStore};

pub const IDEMPOTENCY_HEADER: &str = "idempotency-key";
pub const MAX_IDEMPOTENCY_KEY_LEN: usize = 128;
pub const MAX_QUOTE_REQUEST_BYTES: usize = 8_192;
pub const IDEMPOTENCY_LEASE_SECS: i64 = 30;

fn idempotency_lease_secs() -> i64 {
    std::env::var("CCTP_IDEMPOTENCY_LEASE_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&v| v > 0)
        .unwrap_or(IDEMPOTENCY_LEASE_SECS)
}

#[derive(Debug, Error)]
pub enum CctpIdempotencyError {
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
    #[error("store: {0}")]
    Store(#[from] CctpStoreError),
    #[error("key too long")]
    KeyTooLong,
    #[error("request too large")]
    RequestTooLarge,
    #[error("conflict: idempotency key reused with different request")]
    Conflict,
    #[error("pending quote in progress")]
    PendingInProgress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IdempotencyState {
    #[default]
    Pending,
    Completed,
}

#[derive(Debug, Clone)]
pub struct IdempotencyClaim {
    pub transfer_id: Uuid,
    pub state: IdempotencyState,
    pub is_owner: bool,
    pub request_hash: String,
}

#[async_trait]
pub trait CctpQuoteIdempotencyStore: Send + Sync {
    /// Atomically claim or observe idempotency state for a quote request.
    async fn claim_quote(
        &self,
        key: &str,
        request_hash: &str,
        lease_owner_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<IdempotencyClaim, CctpIdempotencyError>;

    /// Finalize pending claim: insert transfer + mark idempotency completed (single transaction).
    async fn finalize_quote(
        &self,
        key: &str,
        lease_owner_hash: &str,
        transfer: &CctpTransfer,
    ) -> Result<(), CctpIdempotencyError>;

    /// Opportunistically delete expired idempotency rows (bounded).
    async fn cleanup_expired(&self, limit: u32) -> Result<u64, CctpIdempotencyError>;
}

pub fn hash_quote_request(body: &[u8]) -> Result<String, CctpIdempotencyError> {
    if body.len() > MAX_QUOTE_REQUEST_BYTES {
        return Err(CctpIdempotencyError::RequestTooLarge);
    }
    Ok(hex::encode(Sha256::digest(body)))
}

pub fn canonical_quote_request_hash(
    value: &serde_json::Value,
) -> Result<String, CctpIdempotencyError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|e| CctpIdempotencyError::Database(sqlx::Error::Decode(Box::new(e))))?;
    hash_quote_request(&bytes)
}

pub fn normalize_idempotency_key(key: &str) -> Result<String, CctpIdempotencyError> {
    let trimmed = key.trim();
    check_str_len("idempotency_key", trimmed, MAX_IDEMPOTENCY_KEY_LEN)
        .map_err(|_| CctpIdempotencyError::KeyTooLong)?;
    if trimmed.is_empty() {
        return Err(CctpIdempotencyError::KeyTooLong);
    }
    Ok(trimmed.to_string())
}

pub fn new_lease_owner_nonce() -> String {
    let mut raw = [0u8; 16];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut raw);
    hex::encode(raw)
}

pub fn lease_owner_hash_from_nonce(nonce: &str) -> String {
    hash_lease_owner(nonce)
}

pub struct PgCctpQuoteIdempotencyStore {
    pool: PgPool,
}

impl PgCctpQuoteIdempotencyStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct IdempotencyRow {
    request_hash: String,
    transfer_id: Uuid,
    state: String,
    lease_owner_hash: Option<String>,
    lease_expires_at: Option<DateTime<Utc>>,
}

fn parse_state(raw: &str) -> IdempotencyState {
    match raw {
        "completed" => IdempotencyState::Completed,
        _ => IdempotencyState::Pending,
    }
}

#[async_trait]
impl CctpQuoteIdempotencyStore for PgCctpQuoteIdempotencyStore {
    async fn claim_quote(
        &self,
        key: &str,
        request_hash: &str,
        lease_owner_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<IdempotencyClaim, CctpIdempotencyError> {
        let mut tx = self.pool.begin().await?;
        let transfer_id = Uuid::new_v4();
        let lease_until = Utc::now() + Duration::seconds(idempotency_lease_secs());

        let inserted = sqlx::query(
            r#"
            INSERT INTO cctp_quote_idempotency
                (idempotency_key, request_hash, transfer_id, state, lease_owner_hash, lease_expires_at, expires_at)
            VALUES ($1, $2, $3, 'pending', $4, $5, $6)
            ON CONFLICT (idempotency_key) DO NOTHING
            "#,
        )
        .bind(key)
        .bind(request_hash)
        .bind(transfer_id)
        .bind(lease_owner_hash)
        .bind(lease_until)
        .bind(expires_at)
        .execute(&mut *tx)
        .await?;

        if inserted.rows_affected() == 1 {
            tx.commit().await?;
            return Ok(IdempotencyClaim {
                transfer_id,
                state: IdempotencyState::Pending,
                is_owner: true,
                request_hash: request_hash.to_string(),
            });
        }

        let row = sqlx::query_as::<_, IdempotencyRow>(
            r#"
            SELECT request_hash, transfer_id, state, lease_owner_hash, lease_expires_at
            FROM cctp_quote_idempotency
            WHERE idempotency_key = $1 AND expires_at > NOW()
            FOR UPDATE
            "#,
        )
        .bind(key)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            tx.commit().await?;
            return self
                .claim_quote(key, request_hash, lease_owner_hash, expires_at)
                .await;
        };

        if row.request_hash != request_hash {
            tx.commit().await?;
            return Err(CctpIdempotencyError::Conflict);
        }

        let state = parse_state(&row.state);
        if state == IdempotencyState::Completed {
            tx.commit().await?;
            return Ok(IdempotencyClaim {
                transfer_id: row.transfer_id,
                state,
                is_owner: true,
                request_hash: row.request_hash,
            });
        }

        let now = Utc::now();
        let lease_active = row.lease_expires_at.is_some_and(|exp| exp > now);
        let is_owner = row.lease_owner_hash.as_deref() == Some(lease_owner_hash);

        if lease_active && !is_owner {
            tx.commit().await?;
            return Err(CctpIdempotencyError::PendingInProgress);
        }

        if !is_owner {
            sqlx::query(
                r#"
                UPDATE cctp_quote_idempotency
                SET lease_owner_hash = $2, lease_expires_at = $3
                WHERE idempotency_key = $1 AND state = 'pending'
                "#,
            )
            .bind(key)
            .bind(lease_owner_hash)
            .bind(lease_until)
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query(
                r#"
                UPDATE cctp_quote_idempotency
                SET lease_expires_at = $2
                WHERE idempotency_key = $1 AND state = 'pending'
                "#,
            )
            .bind(key)
            .bind(lease_until)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(IdempotencyClaim {
            transfer_id: row.transfer_id,
            state: IdempotencyState::Pending,
            is_owner: true,
            request_hash: row.request_hash,
        })
    }

    async fn finalize_quote(
        &self,
        key: &str,
        lease_owner_hash: &str,
        transfer: &CctpTransfer,
    ) -> Result<(), CctpIdempotencyError> {
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query_as::<_, (String, Uuid, String, Option<String>)>(
            r#"
            SELECT request_hash, transfer_id, state, lease_owner_hash
            FROM cctp_quote_idempotency
            WHERE idempotency_key = $1 AND expires_at > NOW()
            FOR UPDATE
            "#,
        )
        .bind(key)
        .fetch_optional(&mut *tx)
        .await?;

        let Some((_, reserved_id, state, owner)) = row else {
            return Err(CctpIdempotencyError::PendingInProgress);
        };
        if state == "completed" {
            return Ok(());
        }
        if reserved_id != transfer.transfer_id {
            return Err(CctpIdempotencyError::Conflict);
        }
        if owner.as_deref() != Some(lease_owner_hash) {
            return Err(CctpIdempotencyError::PendingInProgress);
        }

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
        .bind(crate::cctp::store::direction_str(transfer.direction))
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
        .bind(crate::cctp::store::finality_str(transfer.finality))
        .bind(&transfer.runtime_fee_quote)
        .bind(&transfer.max_fee)
        .bind(transfer.fee_expires_at)
        .bind(transfer.quote_expires_at)
        .bind(crate::cctp::store::status_str(transfer.status))
        .bind(transfer.version)
        .bind(&transfer.access_token_hash)
        .execute(&mut *tx)
        .await
        .map_err(CctpIdempotencyError::from)?;

        let updated = sqlx::query(
            r#"
            UPDATE cctp_quote_idempotency
            SET state = 'completed', lease_owner_hash = NULL, lease_expires_at = NULL
            WHERE idempotency_key = $1 AND state = 'pending' AND transfer_id = $2
            "#,
        )
        .bind(key)
        .bind(transfer.transfer_id)
        .execute(&mut *tx)
        .await?;

        if updated.rows_affected() != 1 {
            return Err(CctpIdempotencyError::PendingInProgress);
        }

        tx.commit().await?;
        Ok(())
    }

    async fn cleanup_expired(&self, limit: u32) -> Result<u64, CctpIdempotencyError> {
        let result = sqlx::query(
            r#"
            DELETE FROM cctp_quote_idempotency
            WHERE idempotency_key IN (
                SELECT idempotency_key FROM cctp_quote_idempotency
                WHERE expires_at <= NOW()
                LIMIT $1
            )
            "#,
        )
        .bind(limit as i64)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }
}

#[derive(Default)]
struct IdempotencyEntry {
    request_hash: String,
    transfer_id: Uuid,
    state: IdempotencyState,
    lease_owner_hash: Option<String>,
    lease_expires_at: Option<DateTime<Utc>>,
    expires_at: DateTime<Utc>,
}

#[derive(Default)]
pub struct InMemoryCctpQuoteIdempotencyStore {
    entries: Mutex<HashMap<String, IdempotencyEntry>>,
    transfer_store: Mutex<Option<std::sync::Arc<dyn CctpTransferStore>>>,
}

impl InMemoryCctpQuoteIdempotencyStore {
    pub fn bind_transfer_store(&self, store: std::sync::Arc<dyn CctpTransferStore>) {
        *self.transfer_store.lock().unwrap() = Some(store);
    }
}

#[async_trait]
impl CctpQuoteIdempotencyStore for InMemoryCctpQuoteIdempotencyStore {
    async fn claim_quote(
        &self,
        key: &str,
        request_hash: &str,
        lease_owner_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<IdempotencyClaim, CctpIdempotencyError> {
        let mut guard = self.entries.lock().unwrap();
        let now = Utc::now();
        if let Some(entry) = guard.get(key) {
            if entry.expires_at <= now {
                guard.remove(key);
            }
        }

        if let Some(entry) = guard.get(key) {
            if entry.request_hash != request_hash {
                return Err(CctpIdempotencyError::Conflict);
            }
            if entry.state == IdempotencyState::Completed {
                return Ok(IdempotencyClaim {
                    transfer_id: entry.transfer_id,
                    state: IdempotencyState::Completed,
                    is_owner: true,
                    request_hash: entry.request_hash.clone(),
                });
            }
            let lease_active = entry.lease_expires_at.is_some_and(|exp| exp > now);
            let is_owner = entry.lease_owner_hash.as_deref() == Some(lease_owner_hash);
            if lease_active && !is_owner {
                return Err(CctpIdempotencyError::PendingInProgress);
            }
            let lease_until = now + Duration::seconds(IDEMPOTENCY_LEASE_SECS);
            let entry = guard.get_mut(key).unwrap();
            entry.lease_owner_hash = Some(lease_owner_hash.to_string());
            entry.lease_expires_at = Some(lease_until);
            return Ok(IdempotencyClaim {
                transfer_id: entry.transfer_id,
                state: IdempotencyState::Pending,
                is_owner: true,
                request_hash: entry.request_hash.clone(),
            });
        }

        let transfer_id = Uuid::new_v4();
        let lease_until = now + Duration::seconds(IDEMPOTENCY_LEASE_SECS);
        guard.insert(
            key.to_string(),
            IdempotencyEntry {
                request_hash: request_hash.to_string(),
                transfer_id,
                state: IdempotencyState::Pending,
                lease_owner_hash: Some(lease_owner_hash.to_string()),
                lease_expires_at: Some(lease_until),
                expires_at,
            },
        );
        Ok(IdempotencyClaim {
            transfer_id,
            state: IdempotencyState::Pending,
            is_owner: true,
            request_hash: request_hash.to_string(),
        })
    }

    async fn finalize_quote(
        &self,
        key: &str,
        lease_owner_hash: &str,
        transfer: &CctpTransfer,
    ) -> Result<(), CctpIdempotencyError> {
        {
            let guard = self.entries.lock().unwrap();
            let Some(entry) = guard.get(key) else {
                return Err(CctpIdempotencyError::PendingInProgress);
            };
            if entry.state == IdempotencyState::Completed {
                return Ok(());
            }
            if entry.transfer_id != transfer.transfer_id {
                return Err(CctpIdempotencyError::Conflict);
            }
            if entry.lease_owner_hash.as_deref() != Some(lease_owner_hash) {
                return Err(CctpIdempotencyError::PendingInProgress);
            }
        }

        let store = self
            .transfer_store
            .lock()
            .unwrap()
            .clone()
            .ok_or(CctpIdempotencyError::Store(CctpStoreError::NotFound))?;
        store.insert(transfer).await?;

        let mut guard = self.entries.lock().unwrap();
        let entry = guard
            .get_mut(key)
            .ok_or(CctpIdempotencyError::PendingInProgress)?;
        entry.state = IdempotencyState::Completed;
        entry.lease_owner_hash = None;
        entry.lease_expires_at = None;
        Ok(())
    }

    async fn cleanup_expired(&self, limit: u32) -> Result<u64, CctpIdempotencyError> {
        let mut guard = self.entries.lock().unwrap();
        let now = Utc::now();
        let expired: Vec<String> = guard
            .iter()
            .filter(|(_, e)| e.expires_at <= now)
            .take(limit as usize)
            .map(|(k, _)| k.clone())
            .collect();
        let count = expired.len() as u64;
        for k in expired {
            guard.remove(&k);
        }
        Ok(count)
    }
}
