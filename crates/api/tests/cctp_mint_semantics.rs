//! Mint verifier error semantics and payload binding service-path tests.

use std::sync::Arc;

use chrono::{Duration, Utc};
use stellarroute_api::cctp::attestation::NotReadyAttestationVerifier;
use stellarroute_api::cctp::config::CctpConfig;
use stellarroute_api::cctp::iris::{IrisClient, IrisFeeQuote, IrisPollOutcome};
use stellarroute_api::cctp::service::{CctpService, CctpServiceError};
use stellarroute_api::cctp::store::{CctpTransfer, CctpTransferStore, InMemoryCctpTransferStore};
use stellarroute_api::cctp::verifiers::{
    FakeMintVerifier, MintVerifyOutcome, NotReadyEvmMintVerifier, VerifiedMintFacts, VerifierError,
};
use stellarroute_api::kill_switch::KillSwitchManager;
use stellarroute_api::models::v2_cctp::{
    CctpDirection, CctpFinality, CctpTransferStatus, CCTP_PROVIDER_ID, CCTP_TESTNET_CORRIDOR_ID,
    SEPOLIA_CHAIN_ID, STELLAR_TESTNET_CHAIN_ID,
};
use uuid::Uuid;

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

fn mint_prepared_transfer() -> CctpTransfer {
    let now = Utc::now();
    CctpTransfer {
        transfer_id: Uuid::new_v4(),
        support_reference_id: "sup".into(),
        corridor_id: CCTP_TESTNET_CORRIDOR_ID.into(),
        provider: CCTP_PROVIDER_ID.into(),
        direction: CctpDirection::StellarToEvm,
        source_chain_id: STELLAR_TESTNET_CHAIN_ID.into(),
        destination_chain_id: SEPOLIA_CHAIN_ID.into(),
        source_asset: "a".into(),
        source_asset_canonical: "a".into(),
        destination_asset: "b".into(),
        destination_asset_canonical: "b".into(),
        sender: "".into(),
        recipient: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0".into(),
        mint_submitter: None,
        amount: "1".into(),
        destination_amount: "1".into(),
        finality: CctpFinality::Standard,
        runtime_fee_quote: None,
        max_fee: None,
        fee_expires_at: None,
        quote_expires_at: now + Duration::minutes(10),
        status: CctpTransferStatus::MintPrepared,
        source_tx_hash: Some("0xabc".into()),
        source_approval_tx_hash: None,
        source_approval_verified_at: None,
        destination_tx_hash: None,
        iris_message_hash: None,
        message_nonce: Some("42".into()),
        raw_message: Some(vec![1, 2, 3]),
        attestation: Some(vec![4, 5, 6]),
        retry_count: 0,
        last_provider_error: None,
        last_provider_code: None,
        version: 1,
        created_at: now,
        updated_at: now,
        terminal_at: None,
        mint_payload_hash: Some("payload-hash-abc".into()),
        mint_payload_expires_at: Some(now + Duration::minutes(10)),
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

fn service_with_mint_verifier(
    store: Arc<dyn CctpTransferStore>,
    mint: Arc<dyn stellarroute_api::cctp::verifiers::EvmMintVerifier>,
) -> CctpService {
    let mut runtime = stellarroute_api::cctp::readiness::CctpRuntime::production_defaults();
    runtime.evm_mint_verifier = mint;
    CctpService {
        config: {
            let mut c = CctpConfig::default_testnet();
            c.enabled = true;
            c
        },
        store,
        prepare_lock: Arc::new(
            stellarroute_api::cctp::prepare_lock::InMemoryCctpPrepareLockStore::default(),
        ),
        iris: Arc::new(MockIris),
        kill_switch: Arc::new(KillSwitchManager::new(None)),
        runtime,
    }
}

fn base_facts() -> VerifiedMintFacts {
    VerifiedMintFacts {
        tx_hash: "0xmint".into(),
        destination_chain_id: SEPOLIA_CHAIN_ID.into(),
        contract_address: "0xE737e5cEBEEBa77EFE34D4aa090756590b1CE275".into(),
        function_selector: "receiveMessage".into(),
        message_hash: [0u8; 32],
        attestation_hash: [0u8; 32],
        nonce: "42".into(),
        payload_hash: "payload-hash-abc".into(),
        outcome: MintVerifyOutcome::Pending,
        recipient_evidence: None,
    }
}

#[tokio::test]
async fn not_ready_mint_verifier_preserves_state() {
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let transfer = mint_prepared_transfer();
    let id = transfer.transfer_id;
    store.insert(&transfer).await.unwrap();
    let service = service_with_mint_verifier(store.clone(), Arc::new(NotReadyEvmMintVerifier));
    let err = service
        .record_mint_submission(id, "0xmint")
        .await
        .unwrap_err();
    assert!(matches!(err, CctpServiceError::VerifiersNotReady));
    let unchanged = store.get(id).await.unwrap().unwrap();
    assert_eq!(unchanged.status, CctpTransferStatus::MintPrepared);
    assert!(unchanged.destination_tx_hash.is_none());
}

#[tokio::test]
async fn transient_mint_verifier_error_preserves_state() {
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let transfer = mint_prepared_transfer();
    let id = transfer.transfer_id;
    store.insert(&transfer).await.unwrap();

    struct TransientMintVerifier;
    #[async_trait::async_trait]
    impl stellarroute_api::cctp::verifiers::EvmMintVerifier for TransientMintVerifier {
        fn is_ready(&self) -> bool {
            true
        }
        async fn verify_mint_submission(
            &self,
            _: &str,
            _: &[u8],
            _: &[u8],
            _: &str,
            _: &str,
        ) -> Result<VerifiedMintFacts, VerifierError> {
            Err(VerifierError::Transient("rpc down".into()))
        }
        async fn verify_mint_completion(
            &self,
            _: &str,
            _: &[u8],
            _: &str,
            _: &str,
            _: CctpFinality,
        ) -> Result<MintVerifyOutcome, VerifierError> {
            Err(VerifierError::NotReady)
        }
    }

    let service = service_with_mint_verifier(store.clone(), Arc::new(TransientMintVerifier));
    let err = service
        .record_mint_submission(id, "0xmint")
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        CctpServiceError::Verifier(VerifierError::Transient(_))
    ));
    let unchanged = store.get(id).await.unwrap().unwrap();
    assert_eq!(unchanged.status, CctpTransferStatus::MintPrepared);
}

#[tokio::test]
async fn pending_mint_stays_mint_submitted_not_retryable() {
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let transfer = mint_prepared_transfer();
    let id = transfer.transfer_id;
    store.insert(&transfer).await.unwrap();
    let service = service_with_mint_verifier(
        store.clone(),
        Arc::new(FakeMintVerifier {
            facts: base_facts(),
            completion: MintVerifyOutcome::Pending,
            ready: true,
        }),
    );
    let submitted = service.record_mint_submission(id, "0xmint").await.unwrap();
    assert_eq!(submitted.status, CctpTransferStatus::MintSubmitted);
}

#[tokio::test]
async fn verified_failure_transitions_retryable() {
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let transfer = mint_prepared_transfer();
    let id = transfer.transfer_id;
    store.insert(&transfer).await.unwrap();
    let service = service_with_mint_verifier(
        store.clone(),
        Arc::new(FakeMintVerifier {
            facts: base_facts(),
            completion: MintVerifyOutcome::FailedRetryable {
                reason: "on-chain revert".into(),
            },
            ready: true,
        }),
    );
    let retryable = service.record_mint_submission(id, "0xmint").await.unwrap();
    assert_eq!(retryable.status, CctpTransferStatus::MintFailedRetryable);
}

#[tokio::test]
async fn expired_mint_payload_rejected() {
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let mut transfer = mint_prepared_transfer();
    transfer.mint_payload_expires_at = Some(Utc::now() - Duration::minutes(1));
    let id = transfer.transfer_id;
    store.insert(&transfer).await.unwrap();
    let service = service_with_mint_verifier(
        store.clone(),
        Arc::new(FakeMintVerifier {
            facts: base_facts(),
            completion: MintVerifyOutcome::Succeeded,
            ready: true,
        }),
    );
    let err = service
        .record_mint_submission(id, "0xmint")
        .await
        .unwrap_err();
    assert!(matches!(err, CctpServiceError::MintPayloadExpired));
}

#[tokio::test]
async fn payload_hash_mismatch_rejected() {
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let transfer = mint_prepared_transfer();
    let id = transfer.transfer_id;
    store.insert(&transfer).await.unwrap();
    let mut facts = base_facts();
    facts.payload_hash = "wrong-hash".into();
    let service = service_with_mint_verifier(
        store.clone(),
        Arc::new(FakeMintVerifier {
            facts,
            completion: MintVerifyOutcome::Pending,
            ready: true,
        }),
    );
    let err = service
        .record_mint_submission(id, "0xmint")
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        CctpServiceError::Verifier(VerifierError::Failed(_))
    ));
}

#[tokio::test]
async fn verified_success_transitions_completed() {
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let transfer = mint_prepared_transfer();
    let id = transfer.transfer_id;
    store.insert(&transfer).await.unwrap();
    let service = service_with_mint_verifier(
        store.clone(),
        Arc::new(FakeMintVerifier {
            facts: base_facts(),
            completion: MintVerifyOutcome::Succeeded,
            ready: true,
        }),
    );
    let completed = service.record_mint_submission(id, "0xmint").await.unwrap();
    assert_eq!(completed.status, CctpTransferStatus::Completed);
}

#[tokio::test]
async fn reconciliation_nonce_consumed_does_not_complete() {
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let transfer = mint_prepared_transfer();
    let id = transfer.transfer_id;
    store.insert(&transfer).await.unwrap();
    let service = service_with_mint_verifier(
        store.clone(),
        Arc::new(FakeMintVerifier {
            facts: base_facts(),
            completion: MintVerifyOutcome::ReconciliationNonceConsumed,
            ready: true,
        }),
    );
    let submitted = service.record_mint_submission(id, "0xmint").await.unwrap();
    assert_eq!(submitted.status, CctpTransferStatus::MintSubmitted);
    assert_eq!(
        submitted.last_provider_code.as_deref(),
        Some("mint_reconciliation_nonce")
    );
    assert!(submitted
        .last_provider_error
        .as_deref()
        .unwrap_or("")
        .contains("without full mint delivery evidence"));
}

fn service_with_stellar_mint_verifier(
    store: Arc<dyn CctpTransferStore>,
    mint: Arc<dyn stellarroute_api::cctp::verifiers::StellarMintVerifier>,
) -> CctpService {
    let mut runtime = stellarroute_api::cctp::readiness::CctpRuntime::production_defaults();
    runtime.stellar_mint_verifier = mint;
    CctpService {
        config: {
            let mut c = CctpConfig::default_testnet();
            c.enabled = true;
            c
        },
        store,
        prepare_lock: Arc::new(
            stellarroute_api::cctp::prepare_lock::InMemoryCctpPrepareLockStore::default(),
        ),
        iris: Arc::new(MockIris),
        kill_switch: Arc::new(KillSwitchManager::new(None)),
        runtime,
    }
}

fn evm_to_stellar_mint_submitted() -> CctpTransfer {
    let mut t = mint_prepared_transfer();
    t.direction = CctpDirection::EvmToStellar;
    t.source_chain_id = SEPOLIA_CHAIN_ID.into();
    t.destination_chain_id = STELLAR_TESTNET_CHAIN_ID.into();
    t.status = CctpTransferStatus::MintSubmitted;
    t.destination_tx_hash =
        Some("c59b4c64a993fc317d7ed3ea415f061723b2c67f0e2db01cd3d65028a5c0fdc4".into());
    t
}

#[tokio::test]
async fn poll_mint_pending_then_completes() {
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let transfer = evm_to_stellar_mint_submitted();
    let id = transfer.transfer_id;
    store.insert(&transfer).await.unwrap();

    struct PollMintVerifier {
        calls: std::sync::atomic::AtomicUsize,
    }
    #[async_trait::async_trait]
    impl stellarroute_api::cctp::verifiers::StellarMintVerifier for PollMintVerifier {
        fn is_ready(&self) -> bool {
            true
        }
        async fn verify_mint_submission(
            &self,
            _: &str,
            _: &[u8],
            _: &[u8],
            _: &str,
            _: &str,
            _: Option<&str>,
        ) -> Result<VerifiedMintFacts, VerifierError> {
            Err(VerifierError::NotReady)
        }
        async fn verify_mint_completion(
            &self,
            _: &str,
            _: &[u8],
            _: &str,
            _: &str,
            _: CctpFinality,
        ) -> Result<MintVerifyOutcome, VerifierError> {
            let n = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if n == 0 {
                Ok(MintVerifyOutcome::Pending)
            } else {
                Ok(MintVerifyOutcome::Succeeded)
            }
        }
    }

    let service = service_with_stellar_mint_verifier(
        store.clone(),
        Arc::new(PollMintVerifier {
            calls: std::sync::atomic::AtomicUsize::new(0),
        }),
    );
    let still_pending = service.poll_one_transfer(id).await.unwrap();
    assert_eq!(still_pending.status, CctpTransferStatus::MintSubmitted);
    let completed = service.poll_one_transfer(id).await.unwrap();
    assert_eq!(completed.status, CctpTransferStatus::Completed);
    let again = service.poll_one_transfer(id).await.unwrap();
    assert_eq!(again.status, CctpTransferStatus::Completed);
}

#[tokio::test]
async fn poll_mint_stays_pending_without_full_evidence() {
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let transfer = evm_to_stellar_mint_submitted();
    let id = transfer.transfer_id;
    store.insert(&transfer).await.unwrap();
    let service = service_with_stellar_mint_verifier(
        store.clone(),
        Arc::new(FakeMintVerifier {
            facts: VerifiedMintFacts {
                tx_hash: transfer.destination_tx_hash.clone().unwrap(),
                destination_chain_id: STELLAR_TESTNET_CHAIN_ID.into(),
                contract_address: "fwd".into(),
                function_selector: "mint_and_forward".into(),
                message_hash: [0u8; 32],
                attestation_hash: [0u8; 32],
                nonce: transfer.message_nonce.clone().unwrap(),
                payload_hash: transfer.mint_payload_hash.clone().unwrap(),
                outcome: MintVerifyOutcome::Pending,
                recipient_evidence: None,
            },
            completion: MintVerifyOutcome::Pending,
            ready: true,
        }),
    );
    let polled = service.poll_one_transfer(id).await.unwrap();
    assert_eq!(polled.status, CctpTransferStatus::MintSubmitted);
    let polled_again = service.poll_one_transfer(id).await.unwrap();
    assert_eq!(polled_again.status, CctpTransferStatus::MintSubmitted);
}
