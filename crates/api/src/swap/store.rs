//! Swap prepare/submit persistence, sequence reservation, and idempotency.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Mutex;
use thiserror::Error;

/// A prepared swap quote awaiting client signature and submission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedSwapQuote {
    pub quote_id: String,
    /// Full G-address for signature verification (not emitted in audit logs).
    pub sender_account: String,
    pub sender_account_hash: String,
    pub unsigned_xdr_hash: String,
    pub expires_at: DateTime<Utc>,
    pub estimated_output: String,
    pub min_output: String,
    pub amount_in: String,
    pub execution_mode: String,
    pub network_passphrase: String,
    pub route_digest: String,
    pub price_digest: String,
    pub source_sequence: Option<i64>,
    pub timebounds_max: Option<i64>,
    pub base_fee: Option<i32>,
    pub valid_until_ledger: Option<i64>,
    pub submission_status: SubmissionStatus,
    pub tx_hash: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmissionStatus {
    Prepared,
    Submitting,
    Submitted,
    Failed,
}

impl SubmissionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Prepared => "prepared",
            Self::Submitting => "submitting",
            Self::Submitted => "submitted",
            Self::Failed => "failed",
        }
    }

    fn from_db(value: &str) -> Option<Self> {
        match value {
            "prepared" => Some(Self::Prepared),
            "submitting" => Some(Self::Submitting),
            "submitted" => Some(Self::Submitted),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Error)]
pub enum SwapStoreError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("quote not found")]
    NotFound,
    #[error("active prepare already exists for sender")]
    ActivePrepareExists,
    #[error("invalid state transition")]
    InvalidTransition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimSubmitOutcome {
    Claimed(Box<PreparedSwapQuote>),
    /// Another request already moved the quote to `submitting` (or a retry
    /// raced claim). Carry the bound quote so the caller can reconcile.
    InProgress(Box<PreparedSwapQuote>),
    AlreadySubmitted {
        tx_hash: String,
    },
    PermanentlyFailed,
}

#[async_trait]
pub trait SwapQuoteStore: Send + Sync {
    async fn insert_prepared(&self, quote: &PreparedSwapQuote) -> Result<(), SwapStoreError>;

    async fn get(&self, quote_id: &str) -> Result<Option<PreparedSwapQuote>, SwapStoreError>;

    /// Atomically transition `prepared` → `submitting` **with** the deterministic
    /// transaction hash. Never leaves `submitting` with a null `tx_hash`.
    async fn claim_for_submit(
        &self,
        quote_id: &str,
        tx_hash: &str,
    ) -> Result<ClaimSubmitOutcome, SwapStoreError>;

    async fn finalize_submit(&self, quote_id: &str, tx_hash: &str) -> Result<(), SwapStoreError>;

    async fn mark_failed(&self, quote_id: &str) -> Result<(), SwapStoreError>;

    /// Expire stale **prepared** rows for a sender. Never force-fails `submitting`
    /// quotes (they remain reconcilable past prepare TTL).
    async fn expire_stale_for_sender(&self, sender_account: &str) -> Result<u64, SwapStoreError>;
}

#[derive(Clone)]
pub struct PgSwapQuoteStore {
    pool: PgPool,
}

impl PgSwapQuoteStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SwapQuoteStore for PgSwapQuoteStore {
    async fn insert_prepared(&self, quote: &PreparedSwapQuote) -> Result<(), SwapStoreError> {
        self.expire_stale_for_sender(&quote.sender_account).await?;

        let active: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT quote_id FROM swap_prepared_quotes
            WHERE sender_account = $1
              AND submission_status IN ('prepared', 'submitting')
            LIMIT 1
            "#,
        )
        .bind(&quote.sender_account)
        .fetch_optional(&self.pool)
        .await?;
        if active.is_some() {
            return Err(SwapStoreError::ActivePrepareExists);
        }

        let result = sqlx::query(
            r#"
            INSERT INTO swap_prepared_quotes (
                quote_id, sender_account_hash, sender_account, unsigned_xdr_hash, expires_at,
                estimated_output, min_output, amount_in, execution_mode, network_passphrase,
                route_digest, price_digest, source_sequence, timebounds_max, base_fee,
                valid_until_ledger, submission_status, tx_hash
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)
            "#,
        )
        .bind(&quote.quote_id)
        .bind(&quote.sender_account_hash)
        .bind(&quote.sender_account)
        .bind(&quote.unsigned_xdr_hash)
        .bind(quote.expires_at)
        .bind(&quote.estimated_output)
        .bind(&quote.min_output)
        .bind(&quote.amount_in)
        .bind(&quote.execution_mode)
        .bind(&quote.network_passphrase)
        .bind(&quote.route_digest)
        .bind(&quote.price_digest)
        .bind(quote.source_sequence)
        .bind(quote.timebounds_max)
        .bind(quote.base_fee)
        .bind(quote.valid_until_ledger)
        .bind(quote.submission_status.as_str())
        .bind(&quote.tx_hash)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("idx_swap_prepared_active_sender") {
                    Err(SwapStoreError::ActivePrepareExists)
                } else {
                    Err(SwapStoreError::Database(e))
                }
            }
        }
    }

    async fn get(&self, quote_id: &str) -> Result<Option<PreparedSwapQuote>, SwapStoreError> {
        let row = sqlx::query_as::<_, PreparedQuoteRow>(
            r#"
            SELECT quote_id, sender_account_hash, COALESCE(sender_account, '') as sender_account,
                   unsigned_xdr_hash, expires_at, estimated_output, min_output,
                   COALESCE(amount_in, '') as amount_in,
                   COALESCE(execution_mode, 'classic_path_payment') as execution_mode,
                   COALESCE(network_passphrase, '') as network_passphrase,
                   COALESCE(route_digest, '') as route_digest,
                   COALESCE(price_digest, '') as price_digest,
                   source_sequence, timebounds_max, base_fee,
                   valid_until_ledger, submission_status, tx_hash
            FROM swap_prepared_quotes
            WHERE quote_id = $1
            "#,
        )
        .bind(quote_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(PreparedQuoteRow::into_quote))
    }

    async fn claim_for_submit(
        &self,
        quote_id: &str,
        tx_hash: &str,
    ) -> Result<ClaimSubmitOutcome, SwapStoreError> {
        if tx_hash.trim().is_empty() {
            return Err(SwapStoreError::InvalidTransition);
        }
        let mut tx = self.pool.begin().await?;
        let existing = sqlx::query_as::<_, PreparedQuoteRow>(
            r#"
            SELECT quote_id, sender_account_hash, COALESCE(sender_account, '') as sender_account,
                   unsigned_xdr_hash, expires_at, estimated_output, min_output,
                   COALESCE(amount_in, '') as amount_in,
                   COALESCE(execution_mode, 'classic_path_payment') as execution_mode,
                   COALESCE(network_passphrase, '') as network_passphrase,
                   COALESCE(route_digest, '') as route_digest,
                   COALESCE(price_digest, '') as price_digest,
                   source_sequence, timebounds_max, base_fee,
                   valid_until_ledger, submission_status, tx_hash
            FROM swap_prepared_quotes
            WHERE quote_id = $1
            FOR UPDATE
            "#,
        )
        .bind(quote_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = existing else {
            return Err(SwapStoreError::NotFound);
        };
        let status =
            SubmissionStatus::from_db(&row.submission_status).unwrap_or(SubmissionStatus::Failed);

        match status {
            SubmissionStatus::Submitted => {
                tx.commit().await?;
                return Ok(ClaimSubmitOutcome::AlreadySubmitted {
                    tx_hash: row.tx_hash.unwrap_or_default(),
                });
            }
            SubmissionStatus::Submitting => {
                tx.commit().await?;
                return Ok(ClaimSubmitOutcome::InProgress(Box::new(row.into_quote())));
            }
            SubmissionStatus::Failed => {
                tx.commit().await?;
                return Ok(ClaimSubmitOutcome::PermanentlyFailed);
            }
            SubmissionStatus::Prepared => {
                sqlx::query(
                    r#"
                    UPDATE swap_prepared_quotes
                    SET submission_status = 'submitting', tx_hash = $2
                    WHERE quote_id = $1 AND submission_status = 'prepared'
                    "#,
                )
                .bind(quote_id)
                .bind(tx_hash)
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
                let mut claimed = row.into_quote();
                claimed.submission_status = SubmissionStatus::Submitting;
                claimed.tx_hash = Some(tx_hash.to_string());
                Ok(ClaimSubmitOutcome::Claimed(Box::new(claimed)))
            }
        }
    }

    async fn finalize_submit(&self, quote_id: &str, tx_hash: &str) -> Result<(), SwapStoreError> {
        let result = sqlx::query(
            r#"
            UPDATE swap_prepared_quotes
            SET submission_status = 'submitted', tx_hash = $2, submitted_at = NOW()
            WHERE quote_id = $1 AND submission_status = 'submitting'
            "#,
        )
        .bind(quote_id)
        .bind(tx_hash)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(SwapStoreError::InvalidTransition);
        }
        Ok(())
    }

    async fn mark_failed(&self, quote_id: &str) -> Result<(), SwapStoreError> {
        sqlx::query(
            r#"
            UPDATE swap_prepared_quotes
            SET submission_status = 'failed'
            WHERE quote_id = $1 AND submission_status IN ('prepared', 'submitting')
            "#,
        )
        .bind(quote_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn expire_stale_for_sender(&self, sender_account: &str) -> Result<u64, SwapStoreError> {
        // Only `prepared` may TTL-expire. `submitting` stays reconcilable.
        let result = sqlx::query(
            r#"
            UPDATE swap_prepared_quotes
            SET submission_status = 'failed'
            WHERE sender_account = $1
              AND submission_status = 'prepared'
              AND expires_at <= NOW()
            "#,
        )
        .bind(sender_account)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }
}

#[derive(sqlx::FromRow)]
struct PreparedQuoteRow {
    quote_id: String,
    sender_account_hash: String,
    sender_account: String,
    unsigned_xdr_hash: String,
    expires_at: DateTime<Utc>,
    estimated_output: String,
    min_output: String,
    amount_in: String,
    execution_mode: String,
    network_passphrase: String,
    route_digest: String,
    price_digest: String,
    source_sequence: Option<i64>,
    timebounds_max: Option<i64>,
    base_fee: Option<i32>,
    valid_until_ledger: Option<i64>,
    submission_status: String,
    tx_hash: Option<String>,
}

impl PreparedQuoteRow {
    fn into_quote(self) -> PreparedSwapQuote {
        PreparedSwapQuote {
            quote_id: self.quote_id,
            sender_account: self.sender_account,
            sender_account_hash: self.sender_account_hash,
            unsigned_xdr_hash: self.unsigned_xdr_hash,
            expires_at: self.expires_at,
            estimated_output: self.estimated_output,
            min_output: self.min_output,
            amount_in: self.amount_in,
            execution_mode: self.execution_mode,
            network_passphrase: self.network_passphrase,
            route_digest: self.route_digest,
            price_digest: self.price_digest,
            source_sequence: self.source_sequence,
            timebounds_max: self.timebounds_max,
            base_fee: self.base_fee,
            valid_until_ledger: self.valid_until_ledger,
            submission_status: SubmissionStatus::from_db(&self.submission_status)
                .unwrap_or(SubmissionStatus::Failed),
            tx_hash: self.tx_hash,
        }
    }
}

#[derive(Default)]
pub struct InMemorySwapQuoteStore {
    quotes: Mutex<HashMap<String, PreparedSwapQuote>>,
}

impl InMemorySwapQuoteStore {
    /// Test helper: overwrite `expires_at` without changing submission status.
    pub fn set_expires_at_for_tests(&self, quote_id: &str, expires_at: DateTime<Utc>) {
        let mut guard = self.quotes.lock().unwrap();
        if let Some(q) = guard.get_mut(quote_id) {
            q.expires_at = expires_at;
        }
    }

    /// Test helper: overwrite `timebounds_max` without changing submission status.
    pub fn set_timebounds_max_for_tests(&self, quote_id: &str, timebounds_max: Option<i64>) {
        let mut guard = self.quotes.lock().unwrap();
        if let Some(q) = guard.get_mut(quote_id) {
            q.timebounds_max = timebounds_max;
        }
    }
}

#[async_trait]
impl SwapQuoteStore for InMemorySwapQuoteStore {
    async fn insert_prepared(&self, quote: &PreparedSwapQuote) -> Result<(), SwapStoreError> {
        self.expire_stale_for_sender(&quote.sender_account).await?;
        let mut guard = self.quotes.lock().unwrap();
        let now = Utc::now();
        for q in guard.values() {
            if q.sender_account != quote.sender_account {
                continue;
            }
            // Submitting always blocks (reconcilable past TTL). Prepared blocks while unexpired.
            let blocks = match q.submission_status {
                SubmissionStatus::Submitting => true,
                SubmissionStatus::Prepared => q.expires_at > now,
                _ => false,
            };
            if blocks {
                return Err(SwapStoreError::ActivePrepareExists);
            }
        }
        guard.insert(quote.quote_id.clone(), quote.clone());
        Ok(())
    }

    async fn get(&self, quote_id: &str) -> Result<Option<PreparedSwapQuote>, SwapStoreError> {
        Ok(self.quotes.lock().unwrap().get(quote_id).cloned())
    }

    async fn claim_for_submit(
        &self,
        quote_id: &str,
        tx_hash: &str,
    ) -> Result<ClaimSubmitOutcome, SwapStoreError> {
        if tx_hash.trim().is_empty() {
            return Err(SwapStoreError::InvalidTransition);
        }
        let mut guard = self.quotes.lock().unwrap();
        let Some(quote) = guard.get_mut(quote_id) else {
            return Err(SwapStoreError::NotFound);
        };
        match quote.submission_status {
            SubmissionStatus::Submitted => Ok(ClaimSubmitOutcome::AlreadySubmitted {
                tx_hash: quote.tx_hash.clone().unwrap_or_default(),
            }),
            SubmissionStatus::Submitting => {
                Ok(ClaimSubmitOutcome::InProgress(Box::new(quote.clone())))
            }
            SubmissionStatus::Failed => Ok(ClaimSubmitOutcome::PermanentlyFailed),
            SubmissionStatus::Prepared => {
                quote.submission_status = SubmissionStatus::Submitting;
                quote.tx_hash = Some(tx_hash.to_string());
                Ok(ClaimSubmitOutcome::Claimed(Box::new(quote.clone())))
            }
        }
    }

    async fn finalize_submit(&self, quote_id: &str, tx_hash: &str) -> Result<(), SwapStoreError> {
        let mut guard = self.quotes.lock().unwrap();
        let Some(quote) = guard.get_mut(quote_id) else {
            return Err(SwapStoreError::NotFound);
        };
        if quote.submission_status != SubmissionStatus::Submitting {
            return Err(SwapStoreError::InvalidTransition);
        }
        quote.submission_status = SubmissionStatus::Submitted;
        quote.tx_hash = Some(tx_hash.to_string());
        Ok(())
    }

    async fn mark_failed(&self, quote_id: &str) -> Result<(), SwapStoreError> {
        let mut guard = self.quotes.lock().unwrap();
        if let Some(quote) = guard.get_mut(quote_id) {
            if matches!(
                quote.submission_status,
                SubmissionStatus::Prepared | SubmissionStatus::Submitting
            ) {
                quote.submission_status = SubmissionStatus::Failed;
            }
        }
        Ok(())
    }

    async fn expire_stale_for_sender(&self, sender_account: &str) -> Result<u64, SwapStoreError> {
        let mut guard = self.quotes.lock().unwrap();
        let now = Utc::now();
        let mut n = 0u64;
        for q in guard.values_mut() {
            if q.sender_account == sender_account
                && q.submission_status == SubmissionStatus::Prepared
                && q.expires_at <= now
            {
                q.submission_status = SubmissionStatus::Failed;
                n += 1;
            }
        }
        Ok(n)
    }
}

pub fn hash_xdr(xdr: &str) -> String {
    let digest = Sha256::digest(xdr.as_bytes());
    hex::encode(digest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn sample_quote(id: &str, sender: &str) -> PreparedSwapQuote {
        PreparedSwapQuote {
            quote_id: id.to_string(),
            sender_account: sender.to_string(),
            sender_account_hash: "hash".into(),
            unsigned_xdr_hash: "uh".into(),
            expires_at: Utc::now() + Duration::minutes(5),
            estimated_output: "98".into(),
            min_output: "97".into(),
            amount_in: "100".into(),
            execution_mode: "classic_path_payment".into(),
            network_passphrase: "test".into(),
            route_digest: "rd".into(),
            price_digest: "pd".into(),
            source_sequence: Some(1),
            timebounds_max: Some(1),
            base_fee: Some(100),
            valid_until_ledger: None,
            submission_status: SubmissionStatus::Prepared,
            tx_hash: None,
        }
    }

    #[tokio::test]
    async fn concurrent_prepare_same_sender_rejected() {
        let store = InMemorySwapQuoteStore::default();
        store
            .insert_prepared(&sample_quote("q1", "GABC"))
            .await
            .unwrap();
        let err = store
            .insert_prepared(&sample_quote("q2", "GABC"))
            .await
            .unwrap_err();
        assert!(matches!(err, SwapStoreError::ActivePrepareExists));
    }

    #[tokio::test]
    async fn claim_persists_tx_hash_atomically() {
        let store = InMemorySwapQuoteStore::default();
        store
            .insert_prepared(&sample_quote("q1", "G1"))
            .await
            .unwrap();
        let outcome = store.claim_for_submit("q1", "deadbeef").await.unwrap();
        match outcome {
            ClaimSubmitOutcome::Claimed(q) => {
                assert_eq!(q.submission_status, SubmissionStatus::Submitting);
                assert_eq!(q.tx_hash.as_deref(), Some("deadbeef"));
            }
            other => panic!("unexpected {other:?}"),
        }
        let stored = store.get("q1").await.unwrap().unwrap();
        assert_eq!(stored.submission_status, SubmissionStatus::Submitting);
        assert_eq!(stored.tx_hash.as_deref(), Some("deadbeef"));
    }

    #[tokio::test]
    async fn expire_stale_does_not_fail_submitting() {
        let store = InMemorySwapQuoteStore::default();
        let mut q = sample_quote("q1", "G1");
        q.expires_at = Utc::now() - Duration::minutes(1);
        store.insert_prepared(&q).await.unwrap();
        // Force into submitting with hash (simulates in-flight past TTL).
        store.claim_for_submit("q1", "abc").await.unwrap();
        store.set_expires_at_for_tests("q1", Utc::now() - Duration::minutes(1));
        let n = store.expire_stale_for_sender("G1").await.unwrap();
        assert_eq!(n, 0);
        let after = store.get("q1").await.unwrap().unwrap();
        assert_eq!(after.submission_status, SubmissionStatus::Submitting);
        assert_eq!(after.tx_hash.as_deref(), Some("abc"));
    }

    #[tokio::test]
    async fn expire_stale_fails_prepared_only() {
        let store = InMemorySwapQuoteStore::default();
        let mut q = sample_quote("q1", "G1");
        q.expires_at = Utc::now() - Duration::minutes(1);
        store.insert_prepared(&q).await.unwrap();
        let n = store.expire_stale_for_sender("G1").await.unwrap();
        assert_eq!(n, 1);
        assert_eq!(
            store.get("q1").await.unwrap().unwrap().submission_status,
            SubmissionStatus::Failed
        );
    }

    #[tokio::test]
    async fn failed_quote_cannot_be_reclaimed() {
        let store = InMemorySwapQuoteStore::default();
        store
            .insert_prepared(&sample_quote("q1", "G1"))
            .await
            .unwrap();
        store.mark_failed("q1").await.unwrap();
        assert!(matches!(
            store.claim_for_submit("q1", "abc").await.unwrap(),
            ClaimSubmitOutcome::PermanentlyFailed
        ));
    }
}
