//! Adversarial security tests for CCTP backend core.

use std::sync::Arc;

use chrono::{Duration, Utc};
use stellarroute_api::cctp::attestation::{FakeAttestationVerifier, NotReadyAttestationVerifier};
use stellarroute_api::cctp::config::CctpConfig;
use stellarroute_api::cctp::expectations::build_corridor_expectations;
use stellarroute_api::cctp::iris::{
    IrisClient, IrisFeeQuote, IrisMessage, IrisMessageStatus, IrisPollOutcome,
};
use stellarroute_api::cctp::message::{build_synthetic_cctp_message, encode_message_hex};
use stellarroute_api::cctp::service::{CctpService, CctpServiceError};
use stellarroute_api::cctp::store::{
    CctpStoreError, CctpTransfer, CctpTransferStore, InMemoryCctpTransferStore, TransferPatch,
};
use stellarroute_api::cctp::verifiers::{
    FakeBurnVerifier, NotReadyEvmBurnVerifier, NotReadyStellarBurnVerifier, VerifiedBurnFacts,
};
use stellarroute_api::kill_switch::{KillSwitchManager, KillSwitchState};
use stellarroute_api::models::v2_cctp::{
    CctpChainAsset, CctpDirection, CctpFinality, CctpQuoteRequest, CctpTransferStatus,
    CCTP_PROVIDER_ID, CCTP_TESTNET_CORRIDOR_ID, SEPOLIA_CHAIN_ID, STELLAR_TESTNET_CHAIN_ID,
};
use stellarroute_routing::health::policy::OverrideDirective;
use uuid::Uuid;

const G_RECIPIENT: &str = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";
const EVM_RECIPIENT: &str = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0";
const TX_HASH: &str = "0xabc123";

struct MockIris {
    fees: IrisFeeQuote,
    poll_outcome: IrisPollOutcome,
}

#[async_trait::async_trait]
impl IrisClient for MockIris {
    async fn fetch_burn_fees(
        &self,
        _source: u32,
        _dest: u32,
    ) -> Result<IrisFeeQuote, stellarroute_api::cctp::iris::IrisError> {
        Ok(self.fees.clone())
    }

    async fn poll_messages_by_tx(
        &self,
        _source: u32,
        _tx_hash: &str,
    ) -> Result<IrisPollOutcome, stellarroute_api::cctp::iris::IrisError> {
        Ok(self.poll_outcome.clone())
    }

    async fn reattest(&self, _nonce: &str) -> Result<(), stellarroute_api::cctp::iris::IrisError> {
        Ok(())
    }
}

fn sample_quote(direction: CctpDirection) -> CctpQuoteRequest {
    let (recipient, sender) = match direction {
        CctpDirection::StellarToEvm => (EVM_RECIPIENT.into(), None),
        CctpDirection::EvmToStellar => (G_RECIPIENT.into(), Some(EVM_RECIPIENT.into())),
    };
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
        recipient,
        sender,
        mint_submitter: if direction == CctpDirection::EvmToStellar {
            Some(G_RECIPIENT.into())
        } else {
            None
        },
        finality: CctpFinality::Standard,
    }
}

fn base_service(
    store: Arc<dyn CctpTransferStore>,
    iris: Arc<dyn IrisClient>,
    stellar: Arc<dyn stellarroute_api::cctp::verifiers::StellarBurnVerifier>,
    evm: Arc<dyn stellarroute_api::cctp::verifiers::EvmBurnVerifier>,
    attestation: Arc<dyn stellarroute_api::cctp::attestation::AttestationVerifier>,
) -> CctpService {
    let mut cfg = CctpConfig::default_testnet();
    cfg.enabled = true;
    cfg.sepolia_rpc_url = "https://sepolia.drpc.org".into();
    CctpService {
        config: cfg,
        store,
        prepare_lock: Arc::new(
            stellarroute_api::cctp::prepare_lock::InMemoryCctpPrepareLockStore::default(),
        ),
        iris,
        kill_switch: Arc::new(KillSwitchManager::new(None)),
        runtime: stellarroute_api::cctp::readiness::CctpRuntime::for_tests(
            stellar,
            evm,
            attestation,
        ),
    }
}

async fn quoted_prepared(service: &CctpService, direction: CctpDirection) -> CctpTransfer {
    let transfer = service
        .quote_core(
            &sample_quote(direction),
            stellarroute_api::cctp::access::test_access_token_hash(),
        )
        .await
        .unwrap();
    service.prepare_burn(transfer.transfer_id).await.unwrap()
}

fn fake_facts(transfer: &CctpTransfer, config: &CctpConfig, tx_hash: &str) -> VerifiedBurnFacts {
    stellarroute_api::cctp::expectations::build_expected_burn_facts(transfer, config, tx_hash)
        .unwrap()
}

fn complete_iris_message(
    transfer: &CctpTransfer,
    config: &CctpConfig,
    tx_hash: &str,
    attestation_hex: &str,
) -> IrisMessage {
    let expectations = build_corridor_expectations(transfer, config).unwrap();
    let bytes = build_synthetic_cctp_message(&expectations);
    IrisMessage {
        message_hex: encode_message_hex(&bytes),
        attestation_hex: Some(attestation_hex.into()),
        cctp_version: 2,
        status: IrisMessageStatus::Complete,
        event_nonce: "42".into(),
        source_tx_hash: Some(tx_hash.into()),
    }
}

#[tokio::test]
async fn not_ready_burn_verifier_blocks_record_burn() {
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let iris = Arc::new(MockIris {
        fees: IrisFeeQuote {
            standard_fee: "1".into(),
            fast_fee: None,
        },
        poll_outcome: IrisPollOutcome::Pending,
    });
    let service = base_service(
        store.clone(),
        iris,
        Arc::new(NotReadyStellarBurnVerifier),
        Arc::new(NotReadyEvmBurnVerifier),
        Arc::new(NotReadyAttestationVerifier),
    );
    let transfer = service
        .quote_core(
            &sample_quote(CctpDirection::StellarToEvm),
            stellarroute_api::cctp::access::test_access_token_hash(),
        )
        .await
        .unwrap();
    store
        .transition(
            transfer.transfer_id,
            1,
            CctpTransferStatus::BurnPrepared,
            TransferPatch::default(),
        )
        .await
        .unwrap();
    let err = service
        .record_burn_submission(transfer.transfer_id, TX_HASH)
        .await
        .unwrap_err();
    assert!(matches!(err, CctpServiceError::VerifiersNotReady));
}

#[tokio::test]
async fn not_ready_attestation_blocks_attestation_ready() {
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let prepared = {
        let iris = Arc::new(MockIris {
            fees: IrisFeeQuote {
                standard_fee: "1".into(),
                fast_fee: None,
            },
            poll_outcome: IrisPollOutcome::Pending,
        });
        let stellar_facts = {
            let mut cfg = CctpConfig::default_testnet();
            cfg.enabled = true;
            let t = sample_transfer_stub(CctpDirection::StellarToEvm);
            fake_facts(&t, &cfg, TX_HASH)
        };
        let service = base_service(
            store.clone(),
            iris,
            Arc::new(FakeBurnVerifier {
                facts: stellar_facts,
                ready: true,
            }),
            Arc::new(NotReadyEvmBurnVerifier),
            Arc::new(NotReadyAttestationVerifier),
        );
        quoted_prepared(&service, CctpDirection::StellarToEvm).await
    };

    let msg = complete_iris_message(&prepared, &CctpConfig::default_testnet(), TX_HASH, "0xdead");
    let iris = Arc::new(MockIris {
        fees: IrisFeeQuote {
            standard_fee: "1".into(),
            fast_fee: None,
        },
        poll_outcome: IrisPollOutcome::Complete(msg),
    });
    let stellar_facts = fake_facts(&prepared, &CctpConfig::default_testnet(), TX_HASH);
    let service = base_service(
        store.clone(),
        iris,
        Arc::new(FakeBurnVerifier {
            facts: stellar_facts,
            ready: true,
        }),
        Arc::new(NotReadyEvmBurnVerifier),
        Arc::new(NotReadyAttestationVerifier),
    );
    let awaiting = service
        .record_burn_submission(prepared.transfer_id, TX_HASH)
        .await
        .unwrap();
    let err = service
        .poll_one_transfer(awaiting.transfer_id)
        .await
        .unwrap_err();
    assert!(matches!(err, CctpServiceError::VerifiersNotReady));
}

#[tokio::test]
async fn burn_fact_mismatch_rejected() {
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let iris = Arc::new(MockIris {
        fees: IrisFeeQuote {
            standard_fee: "1".into(),
            fast_fee: None,
        },
        poll_outcome: IrisPollOutcome::Pending,
    });
    let prepared = {
        let facts = fake_facts(
            &sample_transfer_stub(CctpDirection::StellarToEvm),
            &CctpConfig::default_testnet(),
            TX_HASH,
        );
        let service = base_service(
            store.clone(),
            iris.clone(),
            Arc::new(FakeBurnVerifier { facts, ready: true }),
            Arc::new(NotReadyEvmBurnVerifier),
            Arc::new(NotReadyAttestationVerifier),
        );
        quoted_prepared(&service, CctpDirection::StellarToEvm).await
    };
    let mut wrong_facts = fake_facts(&prepared, &CctpConfig::default_testnet(), TX_HASH);
    wrong_facts.amount_cctp_subunits += 1;
    let service = base_service(
        store,
        iris,
        Arc::new(FakeBurnVerifier {
            facts: wrong_facts,
            ready: true,
        }),
        Arc::new(NotReadyEvmBurnVerifier),
        Arc::new(NotReadyAttestationVerifier),
    );
    let err = service
        .record_burn_submission(prepared.transfer_id, TX_HASH)
        .await
        .unwrap_err();
    assert!(matches!(err, CctpServiceError::Verifier(_)));
}

#[tokio::test]
async fn attestation_ready_requires_crypto_verifier_and_binding() {
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let prepared = {
        let iris = Arc::new(MockIris {
            fees: IrisFeeQuote {
                standard_fee: "1".into(),
                fast_fee: None,
            },
            poll_outcome: IrisPollOutcome::Pending,
        });
        let facts = fake_facts(
            &sample_transfer_stub(CctpDirection::StellarToEvm),
            &CctpConfig::default_testnet(),
            TX_HASH,
        );
        let service = base_service(
            store.clone(),
            iris,
            Arc::new(FakeBurnVerifier { facts, ready: true }),
            Arc::new(NotReadyEvmBurnVerifier),
            Arc::new(FakeAttestationVerifier { ready: true }),
        );
        quoted_prepared(&service, CctpDirection::StellarToEvm).await
    };

    let msg = complete_iris_message(&prepared, &CctpConfig::default_testnet(), TX_HASH, "0xbeef");
    let iris = Arc::new(MockIris {
        fees: IrisFeeQuote {
            standard_fee: "1".into(),
            fast_fee: None,
        },
        poll_outcome: IrisPollOutcome::Complete(msg),
    });
    let facts = fake_facts(&prepared, &CctpConfig::default_testnet(), TX_HASH);
    let service = base_service(
        store,
        iris,
        Arc::new(FakeBurnVerifier { facts, ready: true }),
        Arc::new(NotReadyEvmBurnVerifier),
        Arc::new(FakeAttestationVerifier { ready: true }),
    );
    let awaiting = service
        .record_burn_submission(prepared.transfer_id, TX_HASH)
        .await
        .unwrap();
    let ready = service
        .poll_one_transfer(awaiting.transfer_id)
        .await
        .unwrap();
    assert_eq!(ready.status, CctpTransferStatus::AttestationReady);
}

#[tokio::test]
async fn iris_tx_hash_mismatch_rejected() {
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let prepared = quoted_prepared(
        &base_service(
            store.clone(),
            Arc::new(MockIris {
                fees: IrisFeeQuote {
                    standard_fee: "1".into(),
                    fast_fee: None,
                },
                poll_outcome: IrisPollOutcome::Pending,
            }),
            Arc::new(FakeBurnVerifier {
                facts: fake_facts(
                    &sample_transfer_stub(CctpDirection::StellarToEvm),
                    &CctpConfig::default_testnet(),
                    TX_HASH,
                ),
                ready: true,
            }),
            Arc::new(NotReadyEvmBurnVerifier),
            Arc::new(FakeAttestationVerifier { ready: true }),
        ),
        CctpDirection::StellarToEvm,
    )
    .await;

    let msg = complete_iris_message(
        &prepared,
        &CctpConfig::default_testnet(),
        "0xwrong",
        "0xbeef",
    );
    let iris = Arc::new(MockIris {
        fees: IrisFeeQuote {
            standard_fee: "1".into(),
            fast_fee: None,
        },
        poll_outcome: IrisPollOutcome::Complete(msg),
    });
    let facts = fake_facts(&prepared, &CctpConfig::default_testnet(), TX_HASH);
    let service = base_service(
        store,
        iris,
        Arc::new(FakeBurnVerifier { facts, ready: true }),
        Arc::new(NotReadyEvmBurnVerifier),
        Arc::new(FakeAttestationVerifier { ready: true }),
    );
    let awaiting = service
        .record_burn_submission(prepared.transfer_id, TX_HASH)
        .await
        .unwrap();
    let err = service
        .poll_one_transfer(awaiting.transfer_id)
        .await
        .unwrap_err();
    assert!(matches!(err, CctpServiceError::IrisTxHashMismatch));
}

#[tokio::test]
async fn empty_attestation_rejected() {
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let prepared = quoted_prepared(
        &base_service(
            store.clone(),
            Arc::new(MockIris {
                fees: IrisFeeQuote {
                    standard_fee: "1".into(),
                    fast_fee: None,
                },
                poll_outcome: IrisPollOutcome::Pending,
            }),
            Arc::new(FakeBurnVerifier {
                facts: fake_facts(
                    &sample_transfer_stub(CctpDirection::StellarToEvm),
                    &CctpConfig::default_testnet(),
                    TX_HASH,
                ),
                ready: true,
            }),
            Arc::new(NotReadyEvmBurnVerifier),
            Arc::new(FakeAttestationVerifier { ready: true }),
        ),
        CctpDirection::StellarToEvm,
    )
    .await;

    let mut msg =
        complete_iris_message(&prepared, &CctpConfig::default_testnet(), TX_HASH, "0xbeef");
    msg.attestation_hex = None;
    let iris = Arc::new(MockIris {
        fees: IrisFeeQuote {
            standard_fee: "1".into(),
            fast_fee: None,
        },
        poll_outcome: IrisPollOutcome::Complete(msg),
    });
    let facts = fake_facts(&prepared, &CctpConfig::default_testnet(), TX_HASH);
    let service = base_service(
        store,
        iris,
        Arc::new(FakeBurnVerifier { facts, ready: true }),
        Arc::new(NotReadyEvmBurnVerifier),
        Arc::new(FakeAttestationVerifier { ready: true }),
    );
    let awaiting = service
        .record_burn_submission(prepared.transfer_id, TX_HASH)
        .await
        .unwrap();
    let err = service
        .poll_one_transfer(awaiting.transfer_id)
        .await
        .unwrap_err();
    assert!(matches!(err, CctpServiceError::MissingAttestation));
}

#[tokio::test]
async fn poll_timeout_marks_attestation_failed() {
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let mut cfg = CctpConfig::default_testnet();
    cfg.enabled = true;
    cfg.sepolia_rpc_url = "https://sepolia.drpc.org".into();
    cfg.poll_timeout_secs = 1;

    let facts = fake_facts(
        &sample_transfer_stub(CctpDirection::StellarToEvm),
        &cfg,
        TX_HASH,
    );
    let service = CctpService {
        config: cfg.clone(),
        store: store.clone(),
        prepare_lock: Arc::new(
            stellarroute_api::cctp::prepare_lock::InMemoryCctpPrepareLockStore::default(),
        ),
        iris: Arc::new(MockIris {
            fees: IrisFeeQuote {
                standard_fee: "1".into(),
                fast_fee: None,
            },
            poll_outcome: IrisPollOutcome::Pending,
        }),
        kill_switch: Arc::new(KillSwitchManager::new(None)),
        runtime: stellarroute_api::cctp::readiness::CctpRuntime::for_tests(
            Arc::new(FakeBurnVerifier { facts, ready: true }),
            Arc::new(NotReadyEvmBurnVerifier),
            Arc::new(FakeAttestationVerifier { ready: true }),
        ),
    };

    let prepared = quoted_prepared(&service, CctpDirection::StellarToEvm).await;
    let awaiting = service
        .record_burn_submission(prepared.transfer_id, TX_HASH)
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let failed = service
        .poll_one_transfer(awaiting.transfer_id)
        .await
        .unwrap();
    assert_eq!(failed.status, CctpTransferStatus::AttestationFailed);
    assert!(failed.terminal_at.is_none());
}

#[tokio::test]
async fn reattest_recovery_clears_terminal_at() {
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let t = sample_transfer_stub(CctpDirection::StellarToEvm);
    let id = t.transfer_id;
    store.insert(&t).await.unwrap();
    store
        .transition(
            id,
            1,
            CctpTransferStatus::BurnPrepared,
            TransferPatch::default(),
        )
        .await
        .unwrap();
    store
        .transition(
            id,
            2,
            CctpTransferStatus::AwaitingAttestation,
            TransferPatch {
                source_tx_hash: Some(TX_HASH.into()),
                message_nonce: Some("42".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let failed = store
        .transition(
            id,
            3,
            CctpTransferStatus::AttestationFailed,
            TransferPatch {
                last_provider_error: Some("timeout".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(failed.terminal_at.is_none());

    let service = CctpService {
        config: CctpConfig::default_testnet(),
        store: store.clone(),
        prepare_lock: Arc::new(
            stellarroute_api::cctp::prepare_lock::InMemoryCctpPrepareLockStore::default(),
        ),
        iris: Arc::new(MockIris {
            fees: IrisFeeQuote {
                standard_fee: "1".into(),
                fast_fee: None,
            },
            poll_outcome: IrisPollOutcome::Pending,
        }),
        kill_switch: Arc::new(KillSwitchManager::new(None)),
        runtime: stellarroute_api::cctp::readiness::CctpRuntime::production_defaults(),
    };
    let recovered = service.reattest(id).await.unwrap();
    assert_eq!(recovered.status, CctpTransferStatus::AwaitingAttestation);
    assert!(recovered.terminal_at.is_none());
    assert_eq!(recovered.retry_count, 1);
}

#[tokio::test]
async fn provider_kill_blocks_quote_and_prepare_allows_in_flight_poll() {
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let facts = fake_facts(
        &sample_transfer_stub(CctpDirection::StellarToEvm),
        &CctpConfig::default_testnet(),
        TX_HASH,
    );

    let mut cfg = CctpConfig::default_testnet();
    cfg.enabled = true;
    cfg.sepolia_rpc_url = "https://sepolia.drpc.org".into();
    let service = CctpService {
        config: cfg,
        store: store.clone(),
        prepare_lock: Arc::new(
            stellarroute_api::cctp::prepare_lock::InMemoryCctpPrepareLockStore::default(),
        ),
        iris: Arc::new(MockIris {
            fees: IrisFeeQuote {
                standard_fee: "1".into(),
                fast_fee: None,
            },
            poll_outcome: IrisPollOutcome::Pending,
        }),
        kill_switch: Arc::new(KillSwitchManager::new(None)),
        runtime: stellarroute_api::cctp::readiness::CctpRuntime::for_tests(
            Arc::new(FakeBurnVerifier { facts, ready: true }),
            Arc::new(NotReadyEvmBurnVerifier),
            Arc::new(NotReadyAttestationVerifier),
        ),
    };

    let prepared = quoted_prepared(&service, CctpDirection::StellarToEvm).await;
    let created = service
        .quote_core(
            &sample_quote(CctpDirection::StellarToEvm),
            stellarroute_api::cctp::access::test_access_token_hash(),
        )
        .await
        .unwrap();
    let awaiting = service
        .record_burn_submission(prepared.transfer_id, TX_HASH)
        .await
        .unwrap();

    let kill = Arc::new(KillSwitchManager::new(None));
    kill.update_state(KillSwitchState {
        providers: std::collections::HashMap::from([(
            CCTP_PROVIDER_ID.into(),
            OverrideDirective::ForceExclude,
        )]),
        ..Default::default()
    })
    .await
    .unwrap();

    let killed_service = CctpService {
        config: service.config.clone(),
        store: store.clone(),
        prepare_lock: Arc::new(
            stellarroute_api::cctp::prepare_lock::InMemoryCctpPrepareLockStore::default(),
        ),
        iris: service.iris.clone(),
        kill_switch: kill,
        runtime: stellarroute_api::cctp::readiness::CctpRuntime::for_tests(
            service.runtime.stellar_burn_verifier.clone(),
            service.runtime.evm_burn_verifier.clone(),
            service.runtime.attestation_verifier.clone(),
        ),
    };

    assert!(matches!(
        killed_service
            .quote_core(
                &sample_quote(CctpDirection::StellarToEvm),
                stellarroute_api::cctp::access::test_access_token_hash()
            )
            .await,
        Err(CctpServiceError::ProviderKilled)
    ));
    assert!(matches!(
        killed_service.prepare_burn(created.transfer_id).await,
        Err(CctpServiceError::ProviderKilled)
    ));

    let polled = killed_service
        .poll_one_transfer(awaiting.transfer_id)
        .await
        .unwrap();
    assert_eq!(polled.status, CctpTransferStatus::AwaitingAttestation);
}

#[tokio::test]
async fn record_verified_burn_atomic_and_version_conflict() {
    let store = InMemoryCctpTransferStore::default();
    let t = sample_transfer_stub(CctpDirection::StellarToEvm);
    let id = t.transfer_id;
    store.insert(&t).await.unwrap();
    store
        .transition(
            id,
            1,
            CctpTransferStatus::BurnPrepared,
            TransferPatch::default(),
        )
        .await
        .unwrap();
    let err = store
        .record_verified_burn(id, 99, TX_HASH)
        .await
        .unwrap_err();
    assert!(matches!(err, CctpStoreError::VersionConflict));
    let awaiting = store.record_verified_burn(id, 2, TX_HASH).await.unwrap();
    assert_eq!(awaiting.status, CctpTransferStatus::AwaitingAttestation);
    assert_eq!(awaiting.source_tx_hash.as_deref(), Some(TX_HASH));
}

#[tokio::test]
async fn oversized_patch_rejected() {
    let store = InMemoryCctpTransferStore::default();
    let t = sample_transfer_stub(CctpDirection::StellarToEvm);
    let id = t.transfer_id;
    store.insert(&t).await.unwrap();
    store
        .transition(
            id,
            1,
            CctpTransferStatus::BurnPrepared,
            TransferPatch::default(),
        )
        .await
        .unwrap();
    let huge = vec![0u8; 9000];
    let err = store
        .transition(
            id,
            2,
            CctpTransferStatus::AwaitingAttestation,
            TransferPatch {
                raw_message: Some(huge),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, CctpStoreError::PayloadTooLarge));
}

#[tokio::test]
async fn reattest_without_nonce_repolls_by_tx_hash() {
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let t = sample_transfer_stub(CctpDirection::StellarToEvm);
    let id = t.transfer_id;
    store.insert(&t).await.unwrap();
    store
        .transition(
            id,
            1,
            CctpTransferStatus::BurnPrepared,
            TransferPatch::default(),
        )
        .await
        .unwrap();
    store
        .transition(
            id,
            2,
            CctpTransferStatus::AwaitingAttestation,
            TransferPatch {
                source_tx_hash: Some(TX_HASH.into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    store
        .transition(
            id,
            3,
            CctpTransferStatus::AttestationFailed,
            TransferPatch::default(),
        )
        .await
        .unwrap();

    let service = CctpService {
        config: CctpConfig::default_testnet(),
        store: store.clone(),
        prepare_lock: Arc::new(
            stellarroute_api::cctp::prepare_lock::InMemoryCctpPrepareLockStore::default(),
        ),
        iris: Arc::new(MockIris {
            fees: IrisFeeQuote {
                standard_fee: "1".into(),
                fast_fee: None,
            },
            poll_outcome: IrisPollOutcome::Pending,
        }),
        kill_switch: Arc::new(KillSwitchManager::new(None)),
        runtime: stellarroute_api::cctp::readiness::CctpRuntime::production_defaults(),
    };
    let recovered = service.reattest(id).await.unwrap();
    assert_eq!(recovered.status, CctpTransferStatus::AwaitingAttestation);
    assert!(recovered.message_nonce.is_none());
}

#[tokio::test]
async fn prepare_burn_rejects_expired_quote() {
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let mut t = sample_transfer_stub(CctpDirection::StellarToEvm);
    t.quote_expires_at = Utc::now() - Duration::seconds(1);
    let id = t.transfer_id;
    store.insert(&t).await.unwrap();

    let service = base_service(
        store,
        Arc::new(MockIris {
            fees: IrisFeeQuote {
                standard_fee: "1".into(),
                fast_fee: None,
            },
            poll_outcome: IrisPollOutcome::Pending,
        }),
        Arc::new(FakeBurnVerifier {
            facts: fake_facts(&t, &CctpConfig::default_testnet(), TX_HASH),
            ready: true,
        }),
        Arc::new(NotReadyEvmBurnVerifier),
        Arc::new(FakeAttestationVerifier { ready: true }),
    );
    let err = service.prepare_burn(id).await.unwrap_err();
    assert!(matches!(err, CctpServiceError::QuoteExpired));
}

#[tokio::test]
async fn fee_expired_blocks_prepare_burn() {
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let iris = Arc::new(MockIris {
        fees: IrisFeeQuote {
            standard_fee: "1".into(),
            fast_fee: None,
        },
        poll_outcome: IrisPollOutcome::Pending,
    });
    let service = base_service(
        store.clone(),
        iris,
        Arc::new(FakeBurnVerifier {
            facts: fake_facts(
                &sample_transfer_stub(CctpDirection::StellarToEvm),
                &CctpConfig::default_testnet(),
                TX_HASH,
            ),
            ready: true,
        }),
        Arc::new(NotReadyEvmBurnVerifier),
        Arc::new(NotReadyAttestationVerifier),
    );
    let mut transfer = sample_transfer_stub(CctpDirection::StellarToEvm);
    transfer.fee_expires_at = Some(Utc::now() - Duration::minutes(1));
    transfer.max_fee = Some("1".into());
    store.insert(&transfer).await.unwrap();
    let err = service
        .prepare_burn(transfer.transfer_id)
        .await
        .unwrap_err();
    assert!(matches!(err, CctpServiceError::FeeExpired));
    let unchanged = store.get(transfer.transfer_id).await.unwrap().unwrap();
    assert_eq!(unchanged.status, CctpTransferStatus::Created);
}

#[tokio::test]
async fn fee_expired_blocks_record_burn() {
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let iris = Arc::new(MockIris {
        fees: IrisFeeQuote {
            standard_fee: "1".into(),
            fast_fee: None,
        },
        poll_outcome: IrisPollOutcome::Pending,
    });
    let facts = fake_facts(
        &sample_transfer_stub(CctpDirection::StellarToEvm),
        &CctpConfig::default_testnet(),
        TX_HASH,
    );
    let service = base_service(
        store.clone(),
        iris,
        Arc::new(FakeBurnVerifier {
            facts: facts.clone(),
            ready: true,
        }),
        Arc::new(NotReadyEvmBurnVerifier),
        Arc::new(NotReadyAttestationVerifier),
    );
    let mut prepared = sample_transfer_stub(CctpDirection::StellarToEvm);
    prepared.status = CctpTransferStatus::BurnPrepared;
    prepared.fee_expires_at = Some(Utc::now() - Duration::minutes(1));
    prepared.max_fee = Some("1".into());
    store.insert(&prepared).await.unwrap();
    let err = service
        .record_burn_submission(prepared.transfer_id, TX_HASH)
        .await
        .unwrap_err();
    assert!(matches!(err, CctpServiceError::FeeExpired));
    let unchanged = store.get(prepared.transfer_id).await.unwrap().unwrap();
    assert_eq!(unchanged.status, CctpTransferStatus::BurnPrepared);
}

fn sample_transfer_stub(direction: CctpDirection) -> CctpTransfer {
    let now = Utc::now();
    CctpTransfer {
        transfer_id: Uuid::new_v4(),
        support_reference_id: "sup".into(),
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
        source_asset: "a".into(),
        source_asset_canonical: "a".into(),
        destination_asset: "b".into(),
        destination_asset_canonical: "b".into(),
        sender: if direction == CctpDirection::EvmToStellar {
            EVM_RECIPIENT.into()
        } else {
            "".into()
        },
        recipient: if direction == CctpDirection::StellarToEvm {
            EVM_RECIPIENT.into()
        } else {
            G_RECIPIENT.into()
        },
        mint_submitter: None,
        amount: "100.000000".into(),
        destination_amount: "100.000000".into(),
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
