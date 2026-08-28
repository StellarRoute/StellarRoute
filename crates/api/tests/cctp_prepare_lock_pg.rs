//! Postgres integration tests for CCTP prepare-lock atomicity.
//! Hermetic: each test uses unique source/transfer rows and explicit cleanup.
//! Run via `scripts/cctp-pg-test.sh` or `TEST_DATABASE_URL=... cargo test --test cctp_prepare_lock_pg -- --ignored`

use std::sync::Arc;

use chrono::{Duration, Utc};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use stellarroute_api::cctp::prepare_lock::{
    CctpActivePrepare, CctpPrepareKind, CctpPrepareLockError, CctpPrepareLockStore,
    InMemoryCctpPrepareLockStore, PgCctpPrepareLockStore, PrepareAcquireResult,
    MAX_PREPARED_PAYLOAD_LEN,
};
use stellarroute_api::cctp::store::{CctpTransfer, CctpTransferStore, PgCctpTransferStore};
use stellarroute_api::models::v2_cctp::{CctpDirection, CctpFinality, CctpTransferStatus};
use tokio::sync::Barrier;
use uuid::Uuid;

fn unique_source(tag: &str) -> String {
    format!("G_CCTP_LOCK_{tag}_{}", Uuid::new_v4().simple())
}

fn sample_transfer(id: Uuid, sender: &str) -> CctpTransfer {
    let now = Utc::now();
    CctpTransfer {
        transfer_id: id,
        support_reference_id: format!("sup-lock-{}", Uuid::new_v4().simple()),
        corridor_id: "c".into(),
        provider: "circle-cctp".into(),
        direction: CctpDirection::StellarToEvm,
        source_chain_id: "stellar:testnet".into(),
        destination_chain_id: "eip155:11155111".into(),
        source_asset: "a".into(),
        source_asset_canonical: "a".into(),
        destination_asset: "b".into(),
        destination_asset_canonical: "b".into(),
        sender: sender.into(),
        recipient: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0".into(),
        mint_submitter: None,
        amount: "10".into(),
        destination_amount: "10".into(),
        finality: CctpFinality::Standard,
        runtime_fee_quote: None,
        max_fee: Some("1".into()),
        fee_expires_at: Some(now + Duration::minutes(10)),
        quote_expires_at: now + Duration::minutes(10),
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

fn reservation(
    source: &str,
    transfer_id: Uuid,
    hash: &str,
    payload: Option<&str>,
) -> CctpActivePrepare {
    CctpActivePrepare {
        source_account: source.into(),
        transfer_id,
        kind: CctpPrepareKind::Burn,
        payload_hash: hash.into(),
        prepared_payload: payload.map(str::to_string),
        expires_at: Utc::now() + Duration::minutes(5),
        updated_at: Utc::now(),
    }
}

async fn pg_pool_from_env() -> PgPool {
    let url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL");
    PgPoolOptions::new()
        .max_connections(8)
        .connect(&url)
        .await
        .expect("connect")
}

async fn insert_transfer(pool: &PgPool, transfer_id: Uuid, source: &str) {
    PgCctpTransferStore::new(pool.clone())
        .insert(&sample_transfer(transfer_id, source))
        .await
        .unwrap();
}

async fn cleanup_rows(pool: &PgPool, source: &str, transfer_ids: &[Uuid]) {
    for tid in transfer_ids {
        sqlx::query("DELETE FROM cctp_active_prepares WHERE transfer_id = $1")
            .bind(tid)
            .execute(pool)
            .await
            .ok();
        sqlx::query("DELETE FROM cctp_transfers WHERE transfer_id = $1")
            .bind(tid)
            .execute(pool)
            .await
            .ok();
    }
    sqlx::query("DELETE FROM cctp_active_prepares WHERE source_account = $1")
        .bind(source)
        .execute(pool)
        .await
        .ok();
}

async fn assert_same_transfer_idempotent<S: CctpPrepareLockStore + ?Sized>(
    locks: &S,
    source: &str,
    transfer_id: Uuid,
    hash: &str,
    payload: &str,
) {
    let first = reservation(source, transfer_id, hash, Some(payload));
    assert!(matches!(
        locks.try_acquire(&first).await.unwrap(),
        PrepareAcquireResult::Acquired
    ));
    let retry = reservation(source, transfer_id, hash, Some(payload));
    match locks.try_acquire(&retry).await.unwrap() {
        PrepareAcquireResult::Idempotent(active) => {
            assert_eq!(active.prepared_payload.as_deref(), Some(payload));
            assert_eq!(active.payload_hash, hash);
        }
        other => panic!("expected idempotent retry, got {other:?}"),
    }
}

async fn assert_payload_hash_mismatch<S: CctpPrepareLockStore + ?Sized>(
    locks: &S,
    source: &str,
    transfer_id: Uuid,
) {
    locks
        .try_acquire(&reservation(
            source,
            transfer_id,
            "hash-a",
            Some("payload-a"),
        ))
        .await
        .unwrap();
    let err = locks
        .try_acquire(&reservation(
            source,
            transfer_id,
            "hash-b",
            Some("payload-b"),
        ))
        .await
        .unwrap_err();
    assert_eq!(err, CctpPrepareLockError::PayloadHashMismatch);
}

async fn assert_payload_too_large<S: CctpPrepareLockStore + ?Sized>(
    locks: &S,
    source: &str,
    transfer_id: Uuid,
) {
    let oversized = "x".repeat(MAX_PREPARED_PAYLOAD_LEN + 1);
    let err = locks
        .try_acquire(&reservation(
            source,
            transfer_id,
            "big-hash",
            Some(&oversized),
        ))
        .await
        .unwrap_err();
    assert_eq!(err, CctpPrepareLockError::PayloadTooLarge);
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with CCTP migrations applied"]
async fn pg_same_transfer_idempotent() {
    let pool = pg_pool_from_env().await;
    let locks = PgCctpPrepareLockStore::new(pool.clone());
    let source = unique_source("idem");
    let tid = Uuid::new_v4();
    insert_transfer(&pool, tid, &source).await;
    assert_same_transfer_idempotent(&locks, &source, tid, "hash-a", "payload-a").await;
    cleanup_rows(&pool, &source, &[tid]).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with CCTP migrations applied"]
async fn pg_conflict_other_transfer() {
    let pool = pg_pool_from_env().await;
    let locks = PgCctpPrepareLockStore::new(pool.clone());
    let source = unique_source("conflict");
    let t1 = Uuid::new_v4();
    let t2 = Uuid::new_v4();
    insert_transfer(&pool, t1, &source).await;
    insert_transfer(&pool, t2, &source).await;
    locks
        .try_acquire(&reservation(&source, t1, "a", Some("p1")))
        .await
        .unwrap();
    assert!(matches!(
        locks
            .try_acquire(&reservation(&source, t2, "b", Some("p2")))
            .await
            .unwrap(),
        PrepareAcquireResult::ConflictOtherTransfer { .. }
    ));
    cleanup_rows(&pool, &source, &[t1, t2]).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with CCTP migrations applied"]
async fn pg_wrong_transfer_release_noop() {
    let pool = pg_pool_from_env().await;
    let locks = PgCctpPrepareLockStore::new(pool.clone());
    let source = unique_source("release");
    let tid = Uuid::new_v4();
    insert_transfer(&pool, tid, &source).await;
    locks
        .try_acquire(&reservation(&source, tid, "a", Some("p")))
        .await
        .unwrap();
    assert!(!locks.release(&source, Uuid::new_v4()).await.unwrap());
    assert!(locks.get_active(&source).await.unwrap().is_some());
    cleanup_rows(&pool, &source, &[tid]).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with CCTP migrations applied"]
async fn pg_same_transfer_payload_hash_mismatch() {
    let pool = pg_pool_from_env().await;
    let locks = PgCctpPrepareLockStore::new(pool.clone());
    let source = unique_source("mismatch");
    let tid = Uuid::new_v4();
    insert_transfer(&pool, tid, &source).await;
    assert_payload_hash_mismatch(&locks, &source, tid).await;
    cleanup_rows(&pool, &source, &[tid]).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with CCTP migrations applied"]
async fn pg_cached_payload_byte_equality_and_bounds() {
    let pool = pg_pool_from_env().await;
    let locks = PgCctpPrepareLockStore::new(pool.clone());
    let source = unique_source("bounds");
    let tid = Uuid::new_v4();
    insert_transfer(&pool, tid, &source).await;
    let payload = "{\"step\":\"burn\",\"primary\":{\"type\":\"stellar_xdr\"}}";
    assert!(payload.len() <= MAX_PREPARED_PAYLOAD_LEN);
    assert_same_transfer_idempotent(&locks, &source, tid, "hash-x", payload).await;
    let too_large_tid = Uuid::new_v4();
    insert_transfer(&pool, too_large_tid, &source).await;
    assert_payload_too_large(&locks, &source, too_large_tid).await;
    cleanup_rows(&pool, &source, &[tid, too_large_tid]).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with CCTP migrations applied"]
async fn pg_concurrent_same_source_race() {
    let pool = pg_pool_from_env().await;
    let locks = Arc::new(PgCctpPrepareLockStore::new(pool.clone()));
    let source = unique_source("race");
    let t1 = Uuid::new_v4();
    let t2 = Uuid::new_v4();
    insert_transfer(&pool, t1, &source).await;
    insert_transfer(&pool, t2, &source).await;
    let barrier = Arc::new(Barrier::new(2));
    let b1 = barrier.clone();
    let b2 = barrier.clone();
    let l1 = locks.clone();
    let l2 = locks.clone();
    let s1 = source.clone();
    let s2 = source.clone();
    let h1 = async move {
        b1.wait().await;
        l1.try_acquire(&reservation(&s1, t1, "r1", Some("p1")))
            .await
    };
    let h2 = async move {
        b2.wait().await;
        l2.try_acquire(&reservation(&s2, t2, "r2", Some("p2")))
            .await
    };
    let (r1, r2) = tokio::join!(h1, h2);
    let outcomes = [r1.unwrap(), r2.unwrap()];
    let acquired = outcomes
        .iter()
        .filter(|o| matches!(o, PrepareAcquireResult::Acquired))
        .count();
    let conflicts = outcomes
        .iter()
        .filter(|o| matches!(o, PrepareAcquireResult::ConflictOtherTransfer { .. }))
        .count();
    assert_eq!(acquired, 1);
    assert_eq!(conflicts, 1);
    cleanup_rows(&pool, &source, &[t1, t2]).await;
}

#[tokio::test]
async fn memory_pg_parity_core_semantics() {
    let mem = InMemoryCctpPrepareLockStore::default();
    let source_idem = unique_source("parity-idem");
    let tid = Uuid::new_v4();
    assert_same_transfer_idempotent(&mem, &source_idem, tid, "hash-p", "payload-p").await;

    let source_mismatch = unique_source("parity-mismatch");
    let mismatch_tid = Uuid::new_v4();
    assert_payload_hash_mismatch(&mem, &source_mismatch, mismatch_tid).await;

    let source_large = unique_source("parity-large");
    let large_tid = Uuid::new_v4();
    assert_payload_too_large(&mem, &source_large, large_tid).await;

    if let Ok(url) = std::env::var("TEST_DATABASE_URL") {
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect(&url)
            .await
            .expect("connect");
        let locks = PgCctpPrepareLockStore::new(pool.clone());
        let pg_source_idem = unique_source("parity-pg-idem");
        let pg_tid = Uuid::new_v4();
        insert_transfer(&pool, pg_tid, &pg_source_idem).await;
        assert_same_transfer_idempotent(&locks, &pg_source_idem, pg_tid, "hash-p", "payload-p")
            .await;

        let pg_source_mismatch = unique_source("parity-pg-mismatch");
        let pg_mismatch_tid = Uuid::new_v4();
        insert_transfer(&pool, pg_mismatch_tid, &pg_source_mismatch).await;
        assert_payload_hash_mismatch(&locks, &pg_source_mismatch, pg_mismatch_tid).await;

        let pg_source_large = unique_source("parity-pg-large");
        let pg_large_tid = Uuid::new_v4();
        insert_transfer(&pool, pg_large_tid, &pg_source_large).await;
        assert_payload_too_large(&locks, &pg_source_large, pg_large_tid).await;

        cleanup_rows(&pool, &pg_source_idem, &[pg_tid]).await;
        cleanup_rows(&pool, &pg_source_mismatch, &[pg_mismatch_tid]).await;
        cleanup_rows(&pool, &pg_source_large, &[pg_large_tid]).await;
    }
}
