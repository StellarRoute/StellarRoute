//! Per-Stellar-source active unsigned prepare reservation (multi-instance safe).
//!
//! Mirrors swap `ActivePrepareExists` semantics: at most one live unsigned prepare
//! per source account across API replicas. Same-transfer re-prepare with a new
//! payload hash replaces the cached payload (e.g. fresh Stellar sequence).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Mutex;
use thiserror::Error;
use uuid::Uuid;

pub const MAX_PAYLOAD_HASH_LEN: usize = 128;
pub const MAX_PREPARED_PAYLOAD_LEN: usize = 131_072;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CctpPrepareKind {
    Approval,
    Burn,
    Mint,
}

impl CctpPrepareKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Approval => "approval",
            Self::Burn => "burn",
            Self::Mint => "mint",
        }
    }

    pub fn parse_kind(s: &str) -> Option<Self> {
        match s {
            "approval" => Some(Self::Approval),
            "burn" => Some(Self::Burn),
            "mint" => Some(Self::Mint),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CctpActivePrepare {
    pub source_account: String,
    pub transfer_id: Uuid,
    pub kind: CctpPrepareKind,
    pub payload_hash: String,
    pub prepared_payload: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrepareAcquireResult {
    Acquired,
    Idempotent(CctpActivePrepare),
    ConflictOtherTransfer { holder_transfer_id: Uuid },
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CctpPrepareLockError {
    #[error("active prepare already exists for source")]
    ActivePrepareExists,
    #[error("payload hash mismatch for same transfer")]
    PayloadHashMismatch,
    #[error("prepared payload too large")]
    PayloadTooLarge,
    #[error("database: {0}")]
    Database(String),
}

fn validate_reservation(reservation: &CctpActivePrepare) -> Result<(), CctpPrepareLockError> {
    if reservation.payload_hash.is_empty() || reservation.payload_hash.len() > MAX_PAYLOAD_HASH_LEN
    {
        return Err(CctpPrepareLockError::PayloadTooLarge);
    }
    if let Some(payload) = &reservation.prepared_payload {
        if payload.is_empty() || payload.len() > MAX_PREPARED_PAYLOAD_LEN {
            return Err(CctpPrepareLockError::PayloadTooLarge);
        }
    }
    Ok(())
}

fn is_pg_unique_violation(err: &sqlx::Error) -> bool {
    matches!(
        err,
        sqlx::Error::Database(db_err) if db_err.code().as_deref() == Some("23505")
    )
}

fn map_active_row_conflict(
    active: CctpActivePrepare,
    reservation: &CctpActivePrepare,
) -> Result<PrepareAcquireResult, CctpPrepareLockError> {
    if active.transfer_id == reservation.transfer_id {
        if active.payload_hash == reservation.payload_hash {
            return Ok(PrepareAcquireResult::Idempotent(active));
        }
        // Same transfer, new payload (e.g. refreshed sequence) — caller updates the row.
        return Ok(PrepareAcquireResult::Acquired);
    }
    Ok(PrepareAcquireResult::ConflictOtherTransfer {
        holder_transfer_id: active.transfer_id,
    })
}

#[async_trait]
pub trait CctpPrepareLockStore: Send + Sync {
    async fn expire_stale_for_source(
        &self,
        source_account: &str,
    ) -> Result<u64, CctpPrepareLockError>;

    async fn try_acquire(
        &self,
        reservation: &CctpActivePrepare,
    ) -> Result<PrepareAcquireResult, CctpPrepareLockError>;

    async fn release(
        &self,
        source_account: &str,
        transfer_id: Uuid,
    ) -> Result<bool, CctpPrepareLockError>;

    async fn get_active(
        &self,
        source_account: &str,
    ) -> Result<Option<CctpActivePrepare>, CctpPrepareLockError>;

    async fn get_for_transfer(
        &self,
        transfer_id: Uuid,
    ) -> Result<Option<CctpActivePrepare>, CctpPrepareLockError>;
}

#[derive(Default)]
pub struct InMemoryCctpPrepareLockStore {
    locks: Mutex<HashMap<String, CctpActivePrepare>>,
}

impl InMemoryCctpPrepareLockStore {
    fn purge_expired(guard: &mut HashMap<String, CctpActivePrepare>, source: &str) -> u64 {
        let now = Utc::now();
        let mut removed = 0u64;
        guard.retain(|k, v| {
            if k == source && v.expires_at <= now {
                removed += 1;
                false
            } else {
                true
            }
        });
        removed
    }
}

#[async_trait]
impl CctpPrepareLockStore for InMemoryCctpPrepareLockStore {
    async fn expire_stale_for_source(
        &self,
        source_account: &str,
    ) -> Result<u64, CctpPrepareLockError> {
        let mut guard = self.locks.lock().unwrap();
        Ok(Self::purge_expired(&mut guard, source_account))
    }

    async fn try_acquire(
        &self,
        reservation: &CctpActivePrepare,
    ) -> Result<PrepareAcquireResult, CctpPrepareLockError> {
        validate_reservation(reservation)?;
        let mut guard = self.locks.lock().unwrap();
        Self::purge_expired(&mut guard, &reservation.source_account);
        if let Some(existing) = guard.get(&reservation.source_account) {
            if existing.expires_at > Utc::now() {
                if existing.transfer_id == reservation.transfer_id {
                    if existing.payload_hash == reservation.payload_hash {
                        return Ok(PrepareAcquireResult::Idempotent(existing.clone()));
                    }
                    guard.insert(
                        reservation.source_account.clone(),
                        CctpActivePrepare {
                            updated_at: Utc::now(),
                            ..reservation.clone()
                        },
                    );
                    return Ok(PrepareAcquireResult::Acquired);
                }
                return Ok(PrepareAcquireResult::ConflictOtherTransfer {
                    holder_transfer_id: existing.transfer_id,
                });
            }
            guard.remove(&reservation.source_account);
        }
        guard.insert(
            reservation.source_account.clone(),
            CctpActivePrepare {
                updated_at: Utc::now(),
                ..reservation.clone()
            },
        );
        Ok(PrepareAcquireResult::Acquired)
    }

    async fn release(
        &self,
        source_account: &str,
        transfer_id: Uuid,
    ) -> Result<bool, CctpPrepareLockError> {
        let mut guard = self.locks.lock().unwrap();
        if let Some(existing) = guard.get(source_account) {
            if existing.transfer_id == transfer_id {
                guard.remove(source_account);
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn get_active(
        &self,
        source_account: &str,
    ) -> Result<Option<CctpActivePrepare>, CctpPrepareLockError> {
        let mut guard = self.locks.lock().unwrap();
        Self::purge_expired(&mut guard, source_account);
        Ok(guard.get(source_account).cloned())
    }

    async fn get_for_transfer(
        &self,
        transfer_id: Uuid,
    ) -> Result<Option<CctpActivePrepare>, CctpPrepareLockError> {
        let mut guard = self.locks.lock().unwrap();
        let now = Utc::now();
        guard.retain(|_, v| v.expires_at > now);
        Ok(guard
            .values()
            .find(|v| v.transfer_id == transfer_id)
            .cloned())
    }
}

pub struct PgCctpPrepareLockStore {
    pool: PgPool,
}

impl PgCctpPrepareLockStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn map_row(
        source_account: String,
        transfer_id: Uuid,
        kind: String,
        payload_hash: String,
        prepared_payload: Option<String>,
        expires_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> CctpActivePrepare {
        CctpActivePrepare {
            source_account,
            transfer_id,
            kind: CctpPrepareKind::parse_kind(&kind).unwrap_or(CctpPrepareKind::Burn),
            payload_hash,
            prepared_payload,
            expires_at,
            updated_at,
        }
    }
}

#[async_trait]
impl CctpPrepareLockStore for PgCctpPrepareLockStore {
    async fn expire_stale_for_source(
        &self,
        source_account: &str,
    ) -> Result<u64, CctpPrepareLockError> {
        let result = sqlx::query(
            r#"DELETE FROM cctp_active_prepares WHERE source_account = $1 AND expires_at <= NOW()"#,
        )
        .bind(source_account)
        .execute(&self.pool)
        .await
        .map_err(|e| CctpPrepareLockError::Database(e.to_string()))?;
        Ok(result.rows_affected())
    }

    async fn try_acquire(
        &self,
        reservation: &CctpActivePrepare,
    ) -> Result<PrepareAcquireResult, CctpPrepareLockError> {
        validate_reservation(reservation)?;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| CctpPrepareLockError::Database(e.to_string()))?;

        sqlx::query(
            r#"DELETE FROM cctp_active_prepares WHERE source_account = $1 AND expires_at <= NOW()"#,
        )
        .bind(&reservation.source_account)
        .execute(&mut *tx)
        .await
        .map_err(|e| CctpPrepareLockError::Database(e.to_string()))?;

        let existing = sqlx::query_as::<
            _,
            (
                String,
                Uuid,
                String,
                String,
                Option<String>,
                DateTime<Utc>,
                DateTime<Utc>,
            ),
        >(
            r#"
            SELECT source_account, transfer_id, prepare_kind, payload_hash,
                   prepared_payload, expires_at, updated_at
            FROM cctp_active_prepares
            WHERE source_account = $1 AND expires_at > NOW()
            FOR UPDATE
            "#,
        )
        .bind(&reservation.source_account)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| CctpPrepareLockError::Database(e.to_string()))?;

        if let Some(row) = existing {
            let active = Self::map_row(row.0, row.1, row.2, row.3, row.4, row.5, row.6);
            if active.transfer_id == reservation.transfer_id {
                if active.payload_hash == reservation.payload_hash {
                    tx.rollback()
                        .await
                        .map_err(|e| CctpPrepareLockError::Database(e.to_string()))?;
                    return Ok(PrepareAcquireResult::Idempotent(active));
                }
                let now = Utc::now();
                sqlx::query(
                    r#"
                    UPDATE cctp_active_prepares
                    SET prepare_kind = $2, payload_hash = $3, prepared_payload = $4,
                        expires_at = $5, updated_at = $6
                    WHERE source_account = $1 AND transfer_id = $7
                    "#,
                )
                .bind(&reservation.source_account)
                .bind(reservation.kind.as_str())
                .bind(&reservation.payload_hash)
                .bind(&reservation.prepared_payload)
                .bind(reservation.expires_at)
                .bind(now)
                .bind(reservation.transfer_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| CctpPrepareLockError::Database(e.to_string()))?;
                tx.commit()
                    .await
                    .map_err(|e| CctpPrepareLockError::Database(e.to_string()))?;
                return Ok(PrepareAcquireResult::Acquired);
            }
            tx.rollback()
                .await
                .map_err(|e| CctpPrepareLockError::Database(e.to_string()))?;
            return map_active_row_conflict(active, reservation);
        }

        let now = Utc::now();
        let insert_result = sqlx::query(
            r#"
            INSERT INTO cctp_active_prepares (
                source_account, transfer_id, prepare_kind, payload_hash,
                prepared_payload, expires_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(&reservation.source_account)
        .bind(reservation.transfer_id)
        .bind(reservation.kind.as_str())
        .bind(&reservation.payload_hash)
        .bind(&reservation.prepared_payload)
        .bind(reservation.expires_at)
        .bind(now)
        .execute(&mut *tx)
        .await;

        match insert_result {
            Ok(_) => {
                tx.commit()
                    .await
                    .map_err(|e| CctpPrepareLockError::Database(e.to_string()))?;
                Ok(PrepareAcquireResult::Acquired)
            }
            Err(e) if is_pg_unique_violation(&e) => {
                let row = sqlx::query_as::<
                    _,
                    (
                        String,
                        Uuid,
                        String,
                        String,
                        Option<String>,
                        DateTime<Utc>,
                        DateTime<Utc>,
                    ),
                >(
                    r#"
                    SELECT source_account, transfer_id, prepare_kind, payload_hash,
                           prepared_payload, expires_at, updated_at
                    FROM cctp_active_prepares
                    WHERE source_account = $1 AND expires_at > NOW()
                    FOR UPDATE
                    "#,
                )
                .bind(&reservation.source_account)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| CctpPrepareLockError::Database(e.to_string()))?;

                if let Some(row) = row {
                    let active = Self::map_row(row.0, row.1, row.2, row.3, row.4, row.5, row.6);
                    if active.transfer_id == reservation.transfer_id {
                        if active.payload_hash == reservation.payload_hash {
                            tx.rollback()
                                .await
                                .map_err(|e| CctpPrepareLockError::Database(e.to_string()))?;
                            return Ok(PrepareAcquireResult::Idempotent(active));
                        }
                        let now = Utc::now();
                        sqlx::query(
                            r#"
                            UPDATE cctp_active_prepares
                            SET prepare_kind = $2, payload_hash = $3, prepared_payload = $4,
                                expires_at = $5, updated_at = $6
                            WHERE source_account = $1 AND transfer_id = $7
                            "#,
                        )
                        .bind(&reservation.source_account)
                        .bind(reservation.kind.as_str())
                        .bind(&reservation.payload_hash)
                        .bind(&reservation.prepared_payload)
                        .bind(reservation.expires_at)
                        .bind(now)
                        .bind(reservation.transfer_id)
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| CctpPrepareLockError::Database(e.to_string()))?;
                        tx.commit()
                            .await
                            .map_err(|e| CctpPrepareLockError::Database(e.to_string()))?;
                        return Ok(PrepareAcquireResult::Acquired);
                    }
                    tx.rollback()
                        .await
                        .map_err(|e| CctpPrepareLockError::Database(e.to_string()))?;
                    map_active_row_conflict(active, reservation)
                } else {
                    tx.rollback()
                        .await
                        .map_err(|e| CctpPrepareLockError::Database(e.to_string()))?;
                    Err(CctpPrepareLockError::Database(
                        "unique violation without active row".into(),
                    ))
                }
            }
            Err(e) => {
                tx.rollback()
                    .await
                    .map_err(|e| CctpPrepareLockError::Database(e.to_string()))?;
                Err(CctpPrepareLockError::Database(e.to_string()))
            }
        }
    }

    async fn release(
        &self,
        source_account: &str,
        transfer_id: Uuid,
    ) -> Result<bool, CctpPrepareLockError> {
        let result = sqlx::query(
            r#"DELETE FROM cctp_active_prepares WHERE source_account = $1 AND transfer_id = $2"#,
        )
        .bind(source_account)
        .bind(transfer_id)
        .execute(&self.pool)
        .await
        .map_err(|e| CctpPrepareLockError::Database(e.to_string()))?;
        Ok(result.rows_affected() > 0)
    }

    async fn get_active(
        &self,
        source_account: &str,
    ) -> Result<Option<CctpActivePrepare>, CctpPrepareLockError> {
        self.expire_stale_for_source(source_account).await?;
        let row = sqlx::query_as::<
            _,
            (
                String,
                Uuid,
                String,
                String,
                Option<String>,
                DateTime<Utc>,
                DateTime<Utc>,
            ),
        >(
            r#"
            SELECT source_account, transfer_id, prepare_kind, payload_hash,
                   prepared_payload, expires_at, updated_at
            FROM cctp_active_prepares
            WHERE source_account = $1 AND expires_at > NOW()
            "#,
        )
        .bind(source_account)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| CctpPrepareLockError::Database(e.to_string()))?;
        Ok(row.map(|r| Self::map_row(r.0, r.1, r.2, r.3, r.4, r.5, r.6)))
    }

    async fn get_for_transfer(
        &self,
        transfer_id: Uuid,
    ) -> Result<Option<CctpActivePrepare>, CctpPrepareLockError> {
        let row = sqlx::query_as::<
            _,
            (
                String,
                Uuid,
                String,
                String,
                Option<String>,
                DateTime<Utc>,
                DateTime<Utc>,
            ),
        >(
            r#"
            SELECT source_account, transfer_id, prepare_kind, payload_hash,
                   prepared_payload, expires_at, updated_at
            FROM cctp_active_prepares
            WHERE transfer_id = $1 AND expires_at > NOW()
            "#,
        )
        .bind(transfer_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| CctpPrepareLockError::Database(e.to_string()))?;
        Ok(row.map(|r| Self::map_row(r.0, r.1, r.2, r.3, r.4, r.5, r.6)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn reservation(
        source: &str,
        transfer_id: Uuid,
        kind: CctpPrepareKind,
        hash: &str,
        payload: Option<&str>,
    ) -> CctpActivePrepare {
        CctpActivePrepare {
            source_account: source.into(),
            transfer_id,
            kind,
            payload_hash: hash.into(),
            prepared_payload: payload.map(str::to_string),
            expires_at: Utc::now() + Duration::minutes(5),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn concurrent_prepare_same_source_rejected() {
        let store = InMemoryCctpPrepareLockStore::default();
        let source = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";
        let r1 = reservation(source, Uuid::new_v4(), CctpPrepareKind::Approval, "a", None);
        store.try_acquire(&r1).await.unwrap();
        let r2 = reservation(source, Uuid::new_v4(), CctpPrepareKind::Burn, "b", None);
        assert!(matches!(
            store.try_acquire(&r2).await,
            Ok(PrepareAcquireResult::ConflictOtherTransfer { .. })
        ));
    }

    #[tokio::test]
    async fn same_transfer_idempotent_returns_cached() {
        let store = InMemoryCctpPrepareLockStore::default();
        let source = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";
        let tid = Uuid::new_v4();
        let r1 = reservation(source, tid, CctpPrepareKind::Approval, "a", Some("payload"));
        assert!(matches!(
            store.try_acquire(&r1).await.unwrap(),
            PrepareAcquireResult::Acquired
        ));
        let r2 = reservation(source, tid, CctpPrepareKind::Approval, "a", Some("payload"));
        assert!(matches!(
            store.try_acquire(&r2).await.unwrap(),
            PrepareAcquireResult::Idempotent(_)
        ));
    }

    #[tokio::test]
    async fn same_transfer_new_hash_replaces_payload() {
        let store = InMemoryCctpPrepareLockStore::default();
        let source = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";
        let tid = Uuid::new_v4();
        let r1 = reservation(source, tid, CctpPrepareKind::Burn, "hash-a", Some("payload-a"));
        assert!(matches!(
            store.try_acquire(&r1).await.unwrap(),
            PrepareAcquireResult::Acquired
        ));
        let r2 = reservation(source, tid, CctpPrepareKind::Burn, "hash-b", Some("payload-b"));
        assert!(matches!(
            store.try_acquire(&r2).await.unwrap(),
            PrepareAcquireResult::Acquired
        ));
        let active = store.get_active(source).await.unwrap().unwrap();
        assert_eq!(active.payload_hash, "hash-b");
        assert_eq!(active.prepared_payload.as_deref(), Some("payload-b"));
    }

    #[tokio::test]
    async fn wrong_transfer_release_is_noop() {
        let store = InMemoryCctpPrepareLockStore::default();
        let source = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";
        let r1 = reservation(source, Uuid::new_v4(), CctpPrepareKind::Approval, "a", None);
        store.try_acquire(&r1).await.unwrap();
        assert!(!store.release(source, Uuid::new_v4()).await.unwrap());
        assert!(store.get_active(source).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn distinct_sources_proceed() {
        let store = InMemoryCctpPrepareLockStore::default();
        store
            .try_acquire(&reservation(
                "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                Uuid::new_v4(),
                CctpPrepareKind::Burn,
                "b",
                None,
            ))
            .await
            .unwrap();
        store
            .try_acquire(&reservation(
                "GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB",
                Uuid::new_v4(),
                CctpPrepareKind::Burn,
                "c",
                None,
            ))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn expired_reservation_recovers() {
        let store = InMemoryCctpPrepareLockStore::default();
        let source = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";
        let stale = CctpActivePrepare {
            expires_at: Utc::now() - Duration::seconds(1),
            ..reservation(
                source,
                Uuid::new_v4(),
                CctpPrepareKind::Approval,
                "old",
                None,
            )
        };
        store.try_acquire(&stale).await.unwrap();
        let fresh = reservation(source, Uuid::new_v4(), CctpPrepareKind::Burn, "new", None);
        assert!(matches!(
            store.try_acquire(&fresh).await.unwrap(),
            PrepareAcquireResult::Acquired
        ));
    }
}
