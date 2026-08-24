//! Per-field adversarial matrix for `record_burn_submission`.

use std::sync::Arc;

use stellarroute_api::cctp::attestation::NotReadyAttestationVerifier;
use stellarroute_api::cctp::config::CctpConfig;
use stellarroute_api::cctp::expectations::build_expected_burn_facts;
use stellarroute_api::cctp::iris::{IrisClient, IrisFeeQuote, IrisPollOutcome};
use stellarroute_api::cctp::service::{CctpService, CctpServiceError};
use stellarroute_api::cctp::store::{CctpTransferStore, InMemoryCctpTransferStore};
use stellarroute_api::cctp::verifiers::{
    FakeBurnVerifier, NotReadyEvmBurnVerifier, NotReadyStellarBurnVerifier, VerifiedBurnFacts,
};
use stellarroute_api::kill_switch::KillSwitchManager;
use stellarroute_api::models::v2_cctp::{
    CctpChainAsset, CctpDirection, CctpFinality, CctpQuoteRequest, CctpTransferStatus,
    CCTP_PROVIDER_ID, CCTP_TESTNET_CORRIDOR_ID, SEPOLIA_CHAIN_ID, STELLAR_TESTNET_CHAIN_ID,
};

const G_SENDER: &str = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";
const EVM_SENDER: &str = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0";
const EVM_RECIPIENT: &str = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0";
const TX_HASH: &str = "0xabc123";

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

fn sample_quote(direction: CctpDirection) -> CctpQuoteRequest {
    CctpQuoteRequest {
        corridor_id: CCTP_TESTNET_CORRIDOR_ID.into(),
        provider: CCTP_PROVIDER_ID.into(),
        direction,
        source_chain_id: if direction == CctpDirection::StellarToEvm {
            STELLAR_TESTNET_CHAIN_ID.into()
        } else {
            SEPOLIA_CHAIN_ID.into()
        },
        destination_chain_id: if direction == CctpDirection::StellarToEvm {
            SEPOLIA_CHAIN_ID.into()
        } else {
            STELLAR_TESTNET_CHAIN_ID.into()
        },
        source_asset: if direction == CctpDirection::StellarToEvm {
            CctpChainAsset::stellar_testnet_usdc()
        } else {
            CctpChainAsset::sepolia_usdc()
        },
        destination_asset: if direction == CctpDirection::StellarToEvm {
            CctpChainAsset::sepolia_usdc()
        } else {
            CctpChainAsset::stellar_testnet_usdc()
        },
        amount: "100.000000".into(),
        recipient: if direction == CctpDirection::StellarToEvm {
            EVM_RECIPIENT.into()
        } else {
            G_SENDER.into()
        },
        sender: if direction == CctpDirection::EvmToStellar {
            Some(EVM_SENDER.into())
        } else {
            Some(G_SENDER.into())
        },
        mint_submitter: if direction == CctpDirection::EvmToStellar {
            Some(G_SENDER.into())
        } else {
            None
        },
        finality: CctpFinality::Standard,
    }
}

fn base_service(
    store: Arc<dyn CctpTransferStore>,
    stellar: Arc<dyn stellarroute_api::cctp::verifiers::StellarBurnVerifier>,
    evm: Arc<dyn stellarroute_api::cctp::verifiers::EvmBurnVerifier>,
) -> CctpService {
    let mut cfg = CctpConfig::default_testnet();
    cfg.enabled = true;
    CctpService {
        config: cfg,
        store,
        prepare_lock: Arc::new(
            stellarroute_api::cctp::prepare_lock::InMemoryCctpPrepareLockStore::default(),
        ),
        iris: Arc::new(MockIris),
        kill_switch: Arc::new(KillSwitchManager::new(None)),
        runtime: stellarroute_api::cctp::readiness::CctpRuntime::for_tests(
            stellar,
            evm,
            Arc::new(NotReadyAttestationVerifier),
        ),
    }
}

async fn prepared_transfer(
    store: Arc<dyn CctpTransferStore>,
    direction: CctpDirection,
) -> stellarroute_api::cctp::store::CctpTransfer {
    let cfg = CctpConfig::default_testnet();
    let stub = build_expected_burn_facts(
        &stellarroute_api::cctp::store::CctpTransfer {
            transfer_id: uuid::Uuid::new_v4(),
            support_reference_id: "s".into(),
            corridor_id: CCTP_TESTNET_CORRIDOR_ID.into(),
            provider: CCTP_PROVIDER_ID.into(),
            direction,
            source_chain_id: sample_quote(direction).source_chain_id,
            destination_chain_id: sample_quote(direction).destination_chain_id,
            source_asset: "a".into(),
            source_asset_canonical: "a".into(),
            destination_asset: "b".into(),
            destination_asset_canonical: "b".into(),
            sender: sample_quote(direction).sender.clone().unwrap_or_default(),
            recipient: sample_quote(direction).recipient.clone(),
            mint_submitter: None,
            amount: "100.000000".into(),
            destination_amount: "100.000000".into(),
            finality: CctpFinality::Standard,
            runtime_fee_quote: None,
            max_fee: None,
            fee_expires_at: None,
            quote_expires_at: chrono::Utc::now() + chrono::Duration::minutes(10),
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
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
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
        },
        &cfg,
        TX_HASH,
    )
    .unwrap();
    let (stellar, evm) = match direction {
        CctpDirection::StellarToEvm => (
            Arc::new(FakeBurnVerifier {
                facts: stub,
                ready: true,
            }) as Arc<dyn stellarroute_api::cctp::verifiers::StellarBurnVerifier>,
            Arc::new(NotReadyEvmBurnVerifier)
                as Arc<dyn stellarroute_api::cctp::verifiers::EvmBurnVerifier>,
        ),
        CctpDirection::EvmToStellar => (
            Arc::new(NotReadyStellarBurnVerifier)
                as Arc<dyn stellarroute_api::cctp::verifiers::StellarBurnVerifier>,
            Arc::new(FakeBurnVerifier {
                facts: stub,
                ready: true,
            }) as Arc<dyn stellarroute_api::cctp::verifiers::EvmBurnVerifier>,
        ),
    };
    let service = base_service(store.clone(), stellar, evm);
    let transfer = service
        .quote_core(
            &sample_quote(direction),
            stellarroute_api::cctp::access::test_access_token_hash(),
        )
        .await
        .unwrap();
    service.prepare_burn(transfer.transfer_id).await.unwrap()
}

async fn assert_field_mismatch_rejected(
    direction: CctpDirection,
    mutate: impl FnOnce(&mut VerifiedBurnFacts),
) {
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let prepared = prepared_transfer(store.clone(), direction).await;
    let cfg = CctpConfig::default_testnet();
    let mut wrong = build_expected_burn_facts(&prepared, &cfg, TX_HASH).unwrap();
    mutate(&mut wrong);

    let (stellar, evm) = match direction {
        CctpDirection::StellarToEvm => (
            Arc::new(FakeBurnVerifier {
                facts: wrong,
                ready: true,
            }) as Arc<dyn stellarroute_api::cctp::verifiers::StellarBurnVerifier>,
            Arc::new(NotReadyEvmBurnVerifier)
                as Arc<dyn stellarroute_api::cctp::verifiers::EvmBurnVerifier>,
        ),
        CctpDirection::EvmToStellar => (
            Arc::new(NotReadyStellarBurnVerifier)
                as Arc<dyn stellarroute_api::cctp::verifiers::StellarBurnVerifier>,
            Arc::new(FakeBurnVerifier {
                facts: wrong,
                ready: true,
            }) as Arc<dyn stellarroute_api::cctp::verifiers::EvmBurnVerifier>,
        ),
    };
    let service = base_service(store.clone(), stellar, evm);
    let before = store.get(prepared.transfer_id).await.unwrap().unwrap();
    let err = service
        .record_burn_submission(prepared.transfer_id, TX_HASH)
        .await
        .unwrap_err();
    assert!(matches!(err, CctpServiceError::Verifier(_)));
    let after = store.get(prepared.transfer_id).await.unwrap().unwrap();
    assert_eq!(after.version, before.version);
    assert_eq!(after.status, CctpTransferStatus::BurnPrepared);
    assert!(after.source_tx_hash.is_none());
}

#[tokio::test]
async fn stellar_amount_mismatch() {
    assert_field_mismatch_rejected(CctpDirection::StellarToEvm, |f| {
        f.amount_cctp_subunits += 1;
    })
    .await;
}

#[tokio::test]
async fn stellar_sender_mismatch() {
    assert_field_mismatch_rejected(CctpDirection::StellarToEvm, |f| {
        f.sender = "GCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC".into();
    })
    .await;
}

#[tokio::test]
async fn stellar_destination_domain_mismatch() {
    assert_field_mismatch_rejected(CctpDirection::StellarToEvm, |f| f.destination_domain = 99)
        .await;
}

#[tokio::test]
async fn stellar_source_domain_mismatch() {
    assert_field_mismatch_rejected(CctpDirection::StellarToEvm, |f| f.source_domain = 99).await;
}

#[tokio::test]
async fn stellar_burn_token_mismatch() {
    assert_field_mismatch_rejected(CctpDirection::StellarToEvm, |f| {
        f.burn_token_bytes32[0] ^= 0xff;
    })
    .await;
}

#[tokio::test]
async fn stellar_mint_recipient_mismatch() {
    assert_field_mismatch_rejected(CctpDirection::StellarToEvm, |f| {
        f.mint_recipient_bytes32[0] ^= 0xff;
    })
    .await;
}

#[tokio::test]
async fn stellar_destination_caller_mismatch() {
    assert_field_mismatch_rejected(CctpDirection::StellarToEvm, |f| {
        f.destination_caller_bytes32[0] ^= 0xff;
    })
    .await;
}

#[tokio::test]
async fn stellar_finality_mismatch() {
    assert_field_mismatch_rejected(CctpDirection::StellarToEvm, |f| {
        f.min_finality_threshold = 1000;
    })
    .await;
}

#[tokio::test]
async fn stellar_token_messenger_mismatch() {
    assert_field_mismatch_rejected(CctpDirection::StellarToEvm, |f| {
        f.token_messenger_bytes32[0] ^= 0xff;
    })
    .await;
}

#[tokio::test]
async fn stellar_hook_extra_mismatch() {
    assert_field_mismatch_rejected(CctpDirection::StellarToEvm, |f| {
        f.hook_data = Some(vec![0x01]);
    })
    .await;
}

#[tokio::test]
async fn stellar_tx_hash_mismatch() {
    assert_field_mismatch_rejected(CctpDirection::StellarToEvm, |f| {
        f.tx_hash = "0xdeadbeef".into();
    })
    .await;
}

#[tokio::test]
async fn evm_amount_mismatch() {
    assert_field_mismatch_rejected(CctpDirection::EvmToStellar, |f| {
        f.amount_cctp_subunits += 1;
    })
    .await;
}

#[tokio::test]
async fn evm_sender_mismatch() {
    assert_field_mismatch_rejected(CctpDirection::EvmToStellar, |f| {
        f.sender = "0x0000000000000000000000000000000000000001".into();
    })
    .await;
}

#[tokio::test]
async fn evm_hook_missing_mismatch() {
    assert_field_mismatch_rejected(CctpDirection::EvmToStellar, |f| f.hook_data = None).await;
}

#[tokio::test]
async fn evm_hook_wrong_mismatch() {
    assert_field_mismatch_rejected(CctpDirection::EvmToStellar, |f| {
        f.hook_data = Some(vec![0xff]);
    })
    .await;
}

#[tokio::test]
async fn evm_source_chain_mismatch() {
    assert_field_mismatch_rejected(CctpDirection::EvmToStellar, |f| {
        f.source_chain_id = "eip155:1".into();
    })
    .await;
}

#[tokio::test]
async fn evm_tx_hash_mismatch() {
    assert_field_mismatch_rejected(CctpDirection::EvmToStellar, |f| {
        f.tx_hash = "0xdeadbeef".into();
    })
    .await;
}

#[tokio::test]
async fn evm_source_domain_mismatch() {
    assert_field_mismatch_rejected(CctpDirection::EvmToStellar, |f| f.source_domain = 99).await;
}

#[tokio::test]
async fn evm_destination_domain_mismatch() {
    assert_field_mismatch_rejected(CctpDirection::EvmToStellar, |f| f.destination_domain = 99)
        .await;
}

#[tokio::test]
async fn evm_burn_token_mismatch() {
    assert_field_mismatch_rejected(CctpDirection::EvmToStellar, |f| {
        f.burn_token_bytes32[0] ^= 0xff;
    })
    .await;
}

#[tokio::test]
async fn evm_mint_recipient_mismatch() {
    assert_field_mismatch_rejected(CctpDirection::EvmToStellar, |f| {
        f.mint_recipient_bytes32[0] ^= 0xff;
    })
    .await;
}

#[tokio::test]
async fn evm_destination_caller_mismatch() {
    assert_field_mismatch_rejected(CctpDirection::EvmToStellar, |f| {
        f.destination_caller_bytes32[0] ^= 0xff;
    })
    .await;
}

#[tokio::test]
async fn evm_finality_mismatch() {
    assert_field_mismatch_rejected(CctpDirection::EvmToStellar, |f| {
        f.min_finality_threshold = 1000;
    })
    .await;
}

#[tokio::test]
async fn evm_token_messenger_mismatch() {
    assert_field_mismatch_rejected(CctpDirection::EvmToStellar, |f| {
        f.token_messenger_bytes32[0] ^= 0xff;
    })
    .await;
}
