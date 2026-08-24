//! Postgres integration tests for CCTP store (migrations 0015–0019).
//! Run with isolated local Postgres: `TEST_DATABASE_URL=postgres://... cargo test -p stellarroute-api --test cctp_store_integration -- --ignored`

use chrono::{Duration, Utc};
use sqlx::postgres::PgPoolOptions;
use stellarroute_api::cctp::store::{
    CctpTransfer, CctpTransferStore, InMemoryCctpTransferStore, PgCctpTransferStore, TransferPatch,
};
use stellarroute_api::models::v2_cctp::{CctpDirection, CctpFinality, CctpTransferStatus};
use uuid::Uuid;

fn sample_transfer() -> CctpTransfer {
    let now = Utc::now();
    CctpTransfer {
        transfer_id: Uuid::new_v4(),
        support_reference_id: "sup-db".into(),
        corridor_id: "c".into(),
        provider: "circle-cctp".into(),
        direction: CctpDirection::StellarToEvm,
        source_chain_id: "stellar:testnet".into(),
        destination_chain_id: "eip155:11155111".into(),
        source_asset: "a".into(),
        source_asset_canonical: "a".into(),
        destination_asset: "b".into(),
        destination_asset_canonical: "b".into(),
        sender: "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF".into(),
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

fn sample_transfer_unique() -> CctpTransfer {
    let mut t = sample_transfer();
    t.transfer_id = Uuid::new_v4();
    t.support_reference_id = format!("sup-db-{}", t.transfer_id);
    t
}

#[tokio::test]
async fn in_memory_approval_and_mint_paths() {
    let store = InMemoryCctpTransferStore::default();
    let t = sample_transfer();
    let id = t.transfer_id;
    store.insert(&t).await.unwrap();
    let prepared = store
        .transition(
            id,
            1,
            CctpTransferStatus::BurnPrepared,
            TransferPatch::default(),
        )
        .await
        .unwrap();
    let with_approval = store
        .record_approval_submission(id, prepared.version, "approval-tx-1", Utc::now())
        .await
        .unwrap();
    assert_eq!(
        with_approval.source_approval_tx_hash.as_deref(),
        Some("approval-tx-1")
    );
    let awaiting = store
        .record_verified_burn(id, with_approval.version, "burn-tx-1")
        .await
        .unwrap();
    assert_eq!(awaiting.status, CctpTransferStatus::AwaitingAttestation);
    let attestation_ready = store
        .transition(
            id,
            awaiting.version,
            CctpTransferStatus::AttestationReady,
            TransferPatch {
                raw_message: Some(vec![1]),
                attestation: Some(vec![2]),
                message_nonce: Some("n1".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let mint_prepared = store
        .record_mint_prepared(
            id,
            attestation_ready.version,
            "hash-1",
            Utc::now() + Duration::minutes(5),
            None,
        )
        .await
        .unwrap();
    assert_eq!(mint_prepared.mint_payload_hash.as_deref(), Some("hash-1"));
    let submitted = store
        .record_mint_submission(id, mint_prepared.version, "0xdest")
        .await
        .unwrap();
    assert_eq!(submitted.status, CctpTransferStatus::MintSubmitted);
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL with migrations 0015-0019 applied"]
async fn pg_store_prepare_submit_retry_paths() {
    let url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL");
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect");
    for migration in [
        include_str!("../migrations/0015_cctp_transfers.sql"),
        include_str!("../migrations/0016_cctp_transfers_hardening.sql"),
        include_str!("../migrations/0017_cctp_mint_metadata.sql"),
        include_str!("../migrations/0018_cctp_approval_tx_hash.sql"),
        include_str!("../migrations/0019_cctp_approval_verified_at.sql"),
        include_str!("../migrations/20260730_cctp_review_fixes.sql"),
        include_str!("../migrations/20260731_cctp_prepare_lock_hardening.sql"),
    ] {
        for stmt in migration
            .split(';')
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            sqlx::query(stmt).execute(&pool).await.ok();
        }
    }
    let store = PgCctpTransferStore::new(pool.clone());
    let t = sample_transfer_unique();
    let id = t.transfer_id;
    let approval_hash = format!("stellar-approve-{id}");
    let burn_hash = format!("stellar-burn-{id}");
    store.insert(&t).await.unwrap();
    let prepared = store
        .transition(
            id,
            1,
            CctpTransferStatus::BurnPrepared,
            TransferPatch::default(),
        )
        .await
        .unwrap();
    let approved = store
        .record_approval_submission(id, prepared.version, &approval_hash, Utc::now())
        .await
        .unwrap();
    assert!(approved.source_approval_tx_hash.is_some());
    let burned = store
        .record_verified_burn(id, approved.version, &burn_hash)
        .await
        .unwrap();
    assert_eq!(burned.status, CctpTransferStatus::AwaitingAttestation);
    sqlx::query("DELETE FROM cctp_transfers WHERE transfer_id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
}
