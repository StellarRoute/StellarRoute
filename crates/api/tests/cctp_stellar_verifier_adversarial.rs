//! Adversarial + live-readiness tests for Stellar CCTP transaction verifiers.

use std::sync::Arc;

use stellarroute_api::cctp::approval::NotReadyStellarApprovalVerifier;
use stellarroute_api::cctp::attestation::NotReadyAttestationVerifier;
use stellarroute_api::cctp::config::CctpConfig;
use stellarroute_api::cctp::iris::{IrisClient, IrisFeeQuote, IrisPollOutcome};
use stellarroute_api::cctp::readiness::CctpRuntime;
use stellarroute_api::cctp::service::{CctpService, CctpServiceError};
use stellarroute_api::cctp::stellar_burn_verifier::StellarRpcBurnVerifier;
use stellarroute_api::cctp::stellar_rpc::StellarRpcClient;
use stellarroute_api::cctp::store::{CctpTransferStore, InMemoryCctpTransferStore};
use stellarroute_api::cctp::verifiers::{
    NotReadyEvmBurnVerifier, NotReadyStellarBurnVerifier, VerifierError,
};
use stellarroute_api::kill_switch::KillSwitchManager;
use stellarroute_api::models::v2_cctp::{
    CctpDirection, CctpFinality, CctpTransferStatus, CCTP_PROVIDER_ID, CCTP_TESTNET_CORRIDOR_ID,
    SEPOLIA_CHAIN_ID, STELLAR_TESTNET_CHAIN_ID,
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

#[tokio::test]
async fn not_ready_stellar_burn_verifier_does_not_mutate_store() {
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let now = chrono::Utc::now();
    let transfer = stellarroute_api::cctp::store::CctpTransfer {
        transfer_id: uuid::Uuid::new_v4(),
        support_reference_id: "s".into(),
        corridor_id: CCTP_TESTNET_CORRIDOR_ID.into(),
        provider: CCTP_PROVIDER_ID.into(),
        direction: CctpDirection::StellarToEvm,
        source_chain_id: STELLAR_TESTNET_CHAIN_ID.into(),
        destination_chain_id: SEPOLIA_CHAIN_ID.into(),
        source_asset: "a".into(),
        source_asset_canonical: "a".into(),
        destination_asset: "b".into(),
        destination_asset_canonical: "b".into(),
        sender: "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF".into(),
        recipient: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0".into(),
        mint_submitter: None,
        amount: "1.0000000".into(),
        destination_amount: "1".into(),
        finality: CctpFinality::Standard,
        runtime_fee_quote: None,
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

    let mut cfg = CctpConfig::default_testnet();
    cfg.enabled = true;
    let service = CctpService {
        config: cfg,
        store: store.clone(),
        prepare_lock: Arc::new(
            stellarroute_api::cctp::prepare_lock::InMemoryCctpPrepareLockStore::default(),
        ),
        iris: Arc::new(MockIris),
        kill_switch: Arc::new(KillSwitchManager::new(None)),
        runtime: CctpRuntime::for_tests(
            Arc::new(NotReadyStellarBurnVerifier),
            Arc::new(NotReadyEvmBurnVerifier),
            Arc::new(NotReadyAttestationVerifier),
        ),
    };

    let err = service
        .record_burn_submission(id, "a".repeat(64).as_str())
        .await
        .unwrap_err();
    assert!(matches!(err, CctpServiceError::VerifiersNotReady));
    assert!(store
        .get(id)
        .await
        .unwrap()
        .unwrap()
        .source_tx_hash
        .is_none());
}

#[tokio::test]
async fn from_config_async_wires_stellar_verifiers_with_live_rpc() {
    let mut cfg = CctpConfig::default_testnet();
    cfg.sepolia_rpc_url = "https://rpc.sepolia.org".into();
    cfg.stellar_rpc_url = "https://soroban-testnet.stellar.org".into();
    let rt = CctpRuntime::from_config_async(&cfg).await;
    assert!(
        rt.stellar_burn_verifier.is_ready(),
        "burn verifier should wire with live RPC + contract probes"
    );
    assert!(
        rt.stellar_mint_verifier.is_ready(),
        "mint verifier should wire with live RPC + contract probes"
    );
    assert!(
        rt.stellar_approval_verifier.is_ready(),
        "optional approval verifier should wire with live RPC"
    );
    assert!(!rt.is_public_executable(&cfg));
}

#[tokio::test]
#[ignore = "requires live Stellar testnet RPC — read-only contract probe"]
async fn live_stellar_rpc_get_transaction_not_found_is_transient_safe() {
    let cfg = CctpConfig::default_testnet();
    let client = StellarRpcClient::new(&cfg).expect("client");
    let bogus = "0".repeat(64);
    let err = client.get_finalized_transaction(&bogus).await.unwrap_err();
    assert_eq!(err, VerifierError::TxNotFound);
}

#[tokio::test]
#[ignore = "requires live Stellar testnet RPC — read-only is_nonce_used simulation"]
async fn live_stellar_is_nonce_used_simulation() {
    use rand::RngCore;

    let cfg = CctpConfig::default_testnet();
    let client = StellarRpcClient::new(&cfg).expect("client");
    // High-entropy nonce — probe only that simulation succeeds, not a fixed on-chain value.
    let mut nonce = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut nonce);
    let _used = client
        .simulate_is_nonce_used(&cfg.contracts.stellar_message_transmitter, nonce)
        .await
        .expect("simulate is_nonce_used against live MessageTransmitter");
}

#[test]
fn stellar_burn_verifier_not_ready_without_rpc_url() {
    let mut cfg = CctpConfig::default_testnet();
    cfg.stellar_rpc_url.clear();
    let fut = StellarRpcBurnVerifier::new(&cfg);
    assert!(matches!(
        tokio::runtime::Runtime::new().unwrap().block_on(fut),
        Err(VerifierError::NotReady)
    ));
}
