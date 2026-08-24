//! CCTP service burn prepare two-step flow + prepare-lock integration.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use stellarroute_api::cctp::attestation::NotReadyAttestationVerifier;
use stellarroute_api::cctp::builders::{
    BuilderError, BurnPrepareStep, PreparedBurnBundle, StellarCctpBurnBuilder,
};
use stellarroute_api::cctp::config::CctpConfig;
use stellarroute_api::cctp::iris::{IrisClient, IrisError, IrisFeeQuote, IrisPollOutcome};
use stellarroute_api::cctp::prepare_lock::InMemoryCctpPrepareLockStore;
use stellarroute_api::cctp::readiness::CctpRuntime;
use stellarroute_api::cctp::service::{CctpService, CctpServiceError};
use stellarroute_api::cctp::store::{CctpTransfer, CctpTransferStore, InMemoryCctpTransferStore};
use stellarroute_api::cctp::verifiers::{NotReadyEvmBurnVerifier, NotReadyStellarBurnVerifier};
use stellarroute_api::kill_switch::KillSwitchManager;
use stellarroute_api::models::v2_cctp::{
    CctpDirection, CctpFinality, CctpTransferStatus, PreparedWalletPayload,
};
use uuid::Uuid;

struct MockIris;

#[async_trait]
impl IrisClient for MockIris {
    async fn fetch_burn_fees(&self, _: u32, _: u32) -> Result<IrisFeeQuote, IrisError> {
        Ok(IrisFeeQuote {
            standard_fee: "1".into(),
            fast_fee: None,
        })
    }

    async fn poll_messages_by_tx(&self, _: u32, _: &str) -> Result<IrisPollOutcome, IrisError> {
        Ok(IrisPollOutcome::Pending)
    }

    async fn reattest(&self, _: &str) -> Result<(), IrisError> {
        Ok(())
    }
}

struct SequenceRefreshBurnBuilder {
    sequence: AtomicUsize,
}

#[async_trait]
impl StellarCctpBurnBuilder for SequenceRefreshBurnBuilder {
    fn is_ready(&self) -> bool {
        true
    }

    async fn prepare_burn(
        &self,
        transfer: &CctpTransfer,
        _: &CctpConfig,
    ) -> Result<PreparedBurnBundle, BuilderError> {
        let seq = self.sequence.fetch_add(1, Ordering::SeqCst) + 100;
        let expires = transfer.quote_expires_at.timestamp();
        Ok(PreparedBurnBundle {
            step: BurnPrepareStep::Burn,
            approval_required: false,
            primary: PreparedWalletPayload::StellarXdr {
                network_passphrase: "Test SDF Network ; September 2015".into(),
                xdr_envelope: format!("AAAA-burn-seq-{seq}"),
            },
            required_approvals: vec![],
            required_prior_payloads: vec![],
            expires_at: expires,
            approval_expiration_ledger: None,
        })
    }
}

struct SteppedBurnBuilder {
    calls: AtomicUsize,
}

#[async_trait]
impl StellarCctpBurnBuilder for SteppedBurnBuilder {
    fn is_ready(&self) -> bool {
        true
    }

    async fn prepare_burn(
        &self,
        transfer: &CctpTransfer,
        _: &CctpConfig,
    ) -> Result<PreparedBurnBundle, BuilderError> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        let expires = transfer.quote_expires_at.timestamp();
        if n == 0 {
            Ok(PreparedBurnBundle {
                step: BurnPrepareStep::Approval,
                approval_required: true,
                primary: PreparedWalletPayload::StellarXdr {
                    network_passphrase: "Test SDF Network ; September 2015".into(),
                    xdr_envelope: "AAAA-approval".into(),
                },
                required_approvals: vec![],
                required_prior_payloads: vec![],
                expires_at: expires,
                approval_expiration_ledger: Some(99_999),
            })
        } else {
            Ok(PreparedBurnBundle {
                step: BurnPrepareStep::Burn,
                approval_required: false,
                primary: PreparedWalletPayload::StellarXdr {
                    network_passphrase: "Test SDF Network ; September 2015".into(),
                    xdr_envelope: format!("AAAA-burn-{n}"),
                },
                required_approvals: vec![],
                required_prior_payloads: vec![],
                expires_at: expires,
                approval_expiration_ledger: None,
            })
        }
    }
}

fn sample_transfer() -> CctpTransfer {
    let now = Utc::now();
    CctpTransfer {
        transfer_id: Uuid::new_v4(),
        support_reference_id: "sup".into(),
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
        amount: "1.0000000".into(),
        destination_amount: "1.0000000".into(),
        finality: CctpFinality::Standard,
        runtime_fee_quote: Some("1".into()),
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

struct ReadyBurnVerifier;

#[async_trait]
impl stellarroute_api::cctp::verifiers::StellarBurnVerifier for ReadyBurnVerifier {
    fn is_ready(&self) -> bool {
        true
    }

    async fn verify_burn(
        &self,
        _: &str,
    ) -> Result<
        stellarroute_api::cctp::verifiers::VerifiedBurnFacts,
        stellarroute_api::cctp::verifiers::VerifierError,
    > {
        Err(stellarroute_api::cctp::verifiers::VerifierError::NotReady)
    }
}

fn test_service(builder: Arc<dyn StellarCctpBurnBuilder>) -> CctpService {
    let mut cfg = CctpConfig::default_testnet();
    cfg.enabled = true;
    cfg.sepolia_rpc_url = "https://ethereum-sepolia-rpc.publicnode.com".into();
    let mut runtime = CctpRuntime::for_tests(
        Arc::new(NotReadyStellarBurnVerifier),
        Arc::new(NotReadyEvmBurnVerifier),
        Arc::new(NotReadyAttestationVerifier),
    );
    runtime.stellar_burn_builder = builder;
    runtime.stellar_burn_verifier = Arc::new(ReadyBurnVerifier);
    CctpService {
        config: cfg,
        store: Arc::new(InMemoryCctpTransferStore::default()),
        prepare_lock: Arc::new(InMemoryCctpPrepareLockStore::default()),
        iris: Arc::new(MockIris),
        kill_switch: Arc::new(KillSwitchManager::new(None)),
        runtime,
    }
}

#[tokio::test]
async fn re_prepare_same_transfer_refreshes_sequence() {
    let builder: Arc<dyn StellarCctpBurnBuilder> = Arc::new(SequenceRefreshBurnBuilder {
        sequence: AtomicUsize::new(0),
    });
    let svc = test_service(builder);
    let t = sample_transfer();
    let id = t.transfer_id;
    svc.store.insert(&t).await.unwrap();

    let first = svc.prepare_burn_wallet(id).await.unwrap();
    assert_eq!(first.step, BurnPrepareStep::Burn);
    let first_xdr = match &first.primary {
        PreparedWalletPayload::StellarXdr { xdr_envelope, .. } => xdr_envelope.clone(),
        _ => panic!("expected stellar xdr"),
    };
    assert!(first_xdr.contains("seq-100"));

    let second = svc.prepare_burn_wallet(id).await.unwrap();
    let second_xdr = match &second.primary {
        PreparedWalletPayload::StellarXdr { xdr_envelope, .. } => xdr_envelope.clone(),
        _ => panic!("expected stellar xdr"),
    };
    assert!(second_xdr.contains("seq-101"));
    assert_ne!(first_xdr, second_xdr);
}

#[tokio::test]
async fn two_step_burn_prepare_after_lock_release() {
    let builder: Arc<dyn StellarCctpBurnBuilder> = Arc::new(SteppedBurnBuilder {
        calls: AtomicUsize::new(0),
    });
    let svc = test_service(builder.clone());
    let t = sample_transfer();
    let id = t.transfer_id;
    let sender = t.sender.clone();
    svc.store.insert(&t).await.unwrap();

    let approval = svc.prepare_burn_wallet(id).await.unwrap();
    assert_eq!(approval.step, BurnPrepareStep::Approval);

    svc.prepare_lock.release(&sender, id).await.unwrap();

    let burn = svc.prepare_burn_wallet(id).await.unwrap();
    assert_eq!(burn.step, BurnPrepareStep::Burn);
    assert_ne!(approval.primary, burn.primary);
}

#[tokio::test]
async fn concurrent_prepare_same_source_rejected() {
    let builder: Arc<dyn StellarCctpBurnBuilder> = Arc::new(SteppedBurnBuilder {
        calls: AtomicUsize::new(0),
    });
    let svc = test_service(builder);
    let t1 = sample_transfer();
    let t2 = sample_transfer();
    let id1 = t1.transfer_id;
    let id2 = t2.transfer_id;
    svc.store.insert(&t1).await.unwrap();
    svc.store.insert(&t2).await.unwrap();
    svc.prepare_burn_wallet(id1).await.unwrap();
    let err = svc.prepare_burn_wallet(id2).await.unwrap_err();
    assert!(matches!(err, CctpServiceError::ActivePrepareExists));
}
