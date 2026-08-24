//! Adversarial tests for approval verification before burn prepare.

use std::sync::Arc;

use chrono::Utc;
use stellarroute_api::cctp::approval::{
    FakeApprovalVerifier, NotReadyEvmApprovalVerifier, NotReadyStellarApprovalVerifier,
    VerifiedApprovalFacts,
};
use stellarroute_api::cctp::attestation::NotReadyAttestationVerifier;
use stellarroute_api::cctp::config::CctpConfig;
use stellarroute_api::cctp::iris::{IrisClient, IrisFeeQuote, IrisPollOutcome};
use stellarroute_api::cctp::service::{CctpService, CctpServiceError};
use stellarroute_api::cctp::store::{CctpTransferStore, InMemoryCctpTransferStore};
use stellarroute_api::cctp::verifiers::{NotReadyEvmBurnVerifier, NotReadyStellarBurnVerifier};
use stellarroute_api::kill_switch::KillSwitchManager;
use stellarroute_api::models::v2_cctp::{
    CctpChainAsset, CctpDirection, CctpFinality, CctpQuoteRequest, CctpTransferStatus,
    CCTP_PROVIDER_ID, CCTP_TESTNET_CORRIDOR_ID, SEPOLIA_CHAIN_ID, STELLAR_TESTNET_CHAIN_ID,
};

struct MockIris;
#[async_trait::async_trait]
impl IrisClient for MockIris {
    async fn fetch_burn_fees(
        &self,
        _: u32,
        _: u32,
    ) -> Result<stellarroute_api::cctp::iris::IrisFeeQuote, stellarroute_api::cctp::iris::IrisError>
    {
        Ok(IrisFeeQuote {
            standard_fee: "1".into(),
            fast_fee: None,
        })
    }
    async fn poll_messages_by_tx(
        &self,
        _: u32,
        _: &str,
    ) -> Result<IrisPollOutcome, stellarroute_api::cctp::iris::IrisError> {
        Ok(IrisPollOutcome::Pending)
    }
    async fn reattest(&self, _: &str) -> Result<(), stellarroute_api::cctp::iris::IrisError> {
        Ok(())
    }
}

fn service_with_approval(
    store: Arc<dyn CctpTransferStore>,
    evm_approval: Arc<dyn stellarroute_api::cctp::approval::EvmApprovalVerifier>,
    stellar_approval: Arc<dyn stellarroute_api::cctp::approval::StellarApprovalVerifier>,
) -> CctpService {
    let mut cfg = CctpConfig::default_testnet();
    cfg.enabled = true;
    let mut runtime = stellarroute_api::cctp::readiness::CctpRuntime::for_tests(
        Arc::new(NotReadyStellarBurnVerifier),
        Arc::new(NotReadyEvmBurnVerifier),
        Arc::new(NotReadyAttestationVerifier),
    );
    runtime.evm_approval_verifier = evm_approval;
    runtime.stellar_approval_verifier = stellar_approval;
    CctpService {
        config: cfg,
        store,
        prepare_lock: Arc::new(
            stellarroute_api::cctp::prepare_lock::InMemoryCctpPrepareLockStore::default(),
        ),
        iris: Arc::new(MockIris),
        kill_switch: Arc::new(KillSwitchManager::new(None)),
        runtime,
    }
}

async fn burn_prepared_evm() -> (Arc<dyn CctpTransferStore>, uuid::Uuid) {
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let now = Utc::now();
    let transfer = stellarroute_api::cctp::store::CctpTransfer {
        transfer_id: uuid::Uuid::new_v4(),
        support_reference_id: "sup".into(),
        corridor_id: CCTP_TESTNET_CORRIDOR_ID.into(),
        provider: CCTP_PROVIDER_ID.into(),
        direction: CctpDirection::EvmToStellar,
        source_chain_id: SEPOLIA_CHAIN_ID.into(),
        destination_chain_id: STELLAR_TESTNET_CHAIN_ID.into(),
        source_asset: "a".into(),
        source_asset_canonical: "a".into(),
        destination_asset: "b".into(),
        destination_asset_canonical: "b".into(),
        sender: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0".into(),
        recipient: "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF".into(),
        mint_submitter: None,
        amount: "10.000000".into(),
        destination_amount: "10.000000".into(),
        finality: CctpFinality::Standard,
        runtime_fee_quote: Some("1".into()),
        max_fee: Some("1".into()),
        fee_expires_at: Some(now + chrono::Duration::minutes(10)),
        quote_expires_at: now + chrono::Duration::minutes(10),
        status: CctpTransferStatus::BurnPrepared,
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
    };
    let id = transfer.transfer_id;
    store.insert(&transfer).await.unwrap();
    (store, id)
}

#[tokio::test]
async fn not_ready_approval_verifier_blocks_submission() {
    let (store, id) = burn_prepared_evm().await;
    let service = service_with_approval(
        store.clone(),
        Arc::new(NotReadyEvmApprovalVerifier),
        Arc::new(NotReadyStellarApprovalVerifier),
    );
    let err = service
        .record_approval_submission(id, "0xapprove")
        .await
        .unwrap_err();
    assert!(matches!(err, CctpServiceError::VerifiersNotReady));
    let unchanged = store.get(id).await.unwrap().unwrap();
    assert!(unchanged.source_approval_verified_at.is_none());
}

#[tokio::test]
async fn bogus_approval_hash_rejected() {
    let (store, id) = burn_prepared_evm().await;
    let facts = VerifiedApprovalFacts {
        tx_hash: "0xgood".into(),
        owner: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0".into(),
        token_contract: "0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238".into(),
        spender_contract: "0x8FE6f2dE824bBc5223A5E6E1A3F1B2d8C9e3A1B2".into(),
        amount: 10_000_000,
        chain_id: SEPOLIA_CHAIN_ID.into(),
    };
    let service = service_with_approval(
        store.clone(),
        Arc::new(FakeApprovalVerifier { facts, ready: true }),
        Arc::new(NotReadyStellarApprovalVerifier),
    );
    let err = service
        .record_approval_submission(id, "0xbad")
        .await
        .unwrap_err();
    assert!(matches!(err, CctpServiceError::Verifier(_)));
}

#[tokio::test]
async fn verified_approval_persists_timestamp() {
    let (store, id) = burn_prepared_evm().await;
    let facts = VerifiedApprovalFacts {
        tx_hash: "0xapprove".into(),
        owner: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0".into(),
        token_contract: "0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238".into(),
        spender_contract: "0x8FE6f2dE824bBc5223A5E6E1A3F1B2d8C9e3A1B2".into(),
        amount: 10_000_000,
        chain_id: SEPOLIA_CHAIN_ID.into(),
    };
    let service = service_with_approval(
        store.clone(),
        Arc::new(FakeApprovalVerifier { facts, ready: true }),
        Arc::new(NotReadyStellarApprovalVerifier),
    );
    let updated = service
        .record_approval_submission(id, "0xapprove")
        .await
        .unwrap();
    assert_eq!(
        updated.source_approval_tx_hash.as_deref(),
        Some("0xapprove")
    );
    assert!(updated.source_approval_verified_at.is_some());
}

#[tokio::test]
async fn conflicting_approval_hash_rejected() {
    let (store, id) = burn_prepared_evm().await;
    let facts = VerifiedApprovalFacts {
        tx_hash: "0xapprove".into(),
        owner: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0".into(),
        token_contract: "0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238".into(),
        spender_contract: "0x8FE6f2dE824bBc5223A5E6E1A3F1B2d8C9e3A1B2".into(),
        amount: 10_000_000,
        chain_id: SEPOLIA_CHAIN_ID.into(),
    };
    let service = service_with_approval(
        store.clone(),
        Arc::new(FakeApprovalVerifier { facts, ready: true }),
        Arc::new(NotReadyStellarApprovalVerifier),
    );
    service
        .record_approval_submission(id, "0xapprove")
        .await
        .unwrap();
    let err = service
        .record_approval_submission(id, "0xother")
        .await
        .unwrap_err();
    assert!(matches!(err, CctpServiceError::Verifier(_)));
}

#[tokio::test]
async fn unverified_hash_does_not_set_verified_at_via_store_direct() {
    let store = InMemoryCctpTransferStore::default();
    let mut t = stellarroute_api::cctp::store::CctpTransfer {
        transfer_id: uuid::Uuid::new_v4(),
        support_reference_id: "s".into(),
        corridor_id: "c".into(),
        provider: CCTP_PROVIDER_ID.into(),
        direction: CctpDirection::EvmToStellar,
        source_chain_id: SEPOLIA_CHAIN_ID.into(),
        destination_chain_id: STELLAR_TESTNET_CHAIN_ID.into(),
        source_asset: "a".into(),
        source_asset_canonical: "a".into(),
        destination_asset: "b".into(),
        destination_asset_canonical: "b".into(),
        sender: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0".into(),
        recipient: "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF".into(),
        mint_submitter: None,
        amount: "10".into(),
        destination_amount: "10".into(),
        finality: CctpFinality::Standard,
        runtime_fee_quote: None,
        max_fee: None,
        fee_expires_at: None,
        quote_expires_at: Utc::now() + chrono::Duration::minutes(10),
        status: CctpTransferStatus::BurnPrepared,
        source_tx_hash: None,
        source_approval_tx_hash: Some("0xhash-only".into()),
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
    let id = t.transfer_id;
    store.insert(&t).await.unwrap();
    assert!(t.source_approval_verified_at.is_none());
    assert_eq!(t.status, CctpTransferStatus::BurnPrepared);
}
