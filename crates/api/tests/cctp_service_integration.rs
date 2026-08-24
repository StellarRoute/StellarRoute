//! CCTP service integration tests (in-memory store + mock Iris).

use std::sync::Arc;

use chrono::{Duration, Utc};
use stellarroute_api::cctp::config::CctpConfig;
use stellarroute_api::cctp::iris::{
    IrisClient, IrisFeeQuote, IrisMessage, IrisMessageStatus, IrisPollOutcome,
};
use stellarroute_api::cctp::service::CctpService;
use stellarroute_api::cctp::store::{CctpTransferStore, InMemoryCctpTransferStore};
use stellarroute_api::kill_switch::{KillSwitchManager, KillSwitchState};
use stellarroute_api::models::v2_cctp::{
    CctpChainAsset, CctpDirection, CctpFinality, CctpQuoteRequest, CctpTransferStatus,
    CCTP_PROVIDER_ID, CCTP_TESTNET_CORRIDOR_ID, SEPOLIA_CHAIN_ID, STELLAR_TESTNET_CHAIN_ID,
};
use stellarroute_routing::health::policy::OverrideDirective;

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

fn sample_quote_request() -> CctpQuoteRequest {
    CctpQuoteRequest {
        corridor_id: CCTP_TESTNET_CORRIDOR_ID.into(),
        provider: CCTP_PROVIDER_ID.into(),
        direction: CctpDirection::StellarToEvm,
        source_chain_id: STELLAR_TESTNET_CHAIN_ID.into(),
        destination_chain_id: SEPOLIA_CHAIN_ID.into(),
        source_asset: CctpChainAsset::stellar_testnet_usdc(),
        destination_asset: CctpChainAsset::sepolia_usdc(),
        amount: "100.000000".into(),
        recipient: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0".into(),
        sender: None,
        mint_submitter: None,
        finality: CctpFinality::Standard,
    }
}

#[tokio::test]
async fn quote_core_creates_transfer_when_enabled() {
    let mut cfg = CctpConfig::default_testnet();
    cfg.enabled = true;
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let iris = Arc::new(MockIris {
        fees: IrisFeeQuote {
            standard_fee: "1".into(),
            fast_fee: None,
        },
        poll_outcome: IrisPollOutcome::Pending,
    });
    let kill = Arc::new(KillSwitchManager::new(None));
    let service = CctpService {
        config: cfg,
        store,
        prepare_lock: Arc::new(
            stellarroute_api::cctp::prepare_lock::InMemoryCctpPrepareLockStore::default(),
        ),
        iris,
        kill_switch: kill,
        runtime: stellarroute_api::cctp::readiness::CctpRuntime::production_defaults(),
    };

    let transfer = service
        .quote_core(
            &sample_quote_request(),
            stellarroute_api::cctp::access::test_access_token_hash(),
        )
        .await
        .unwrap();
    assert_eq!(transfer.status, CctpTransferStatus::Created);
    assert!(!service.config.is_executable());
}

#[tokio::test]
async fn provider_kill_switch_blocks_new_quote() {
    let mut cfg = CctpConfig::default_testnet();
    cfg.enabled = true;
    let store: Arc<dyn CctpTransferStore> = Arc::new(InMemoryCctpTransferStore::default());
    let iris = Arc::new(MockIris {
        fees: IrisFeeQuote {
            standard_fee: "1".into(),
            fast_fee: None,
        },
        poll_outcome: IrisPollOutcome::Pending,
    });
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

    let service = CctpService {
        config: cfg,
        store,
        prepare_lock: Arc::new(
            stellarroute_api::cctp::prepare_lock::InMemoryCctpPrepareLockStore::default(),
        ),
        iris,
        kill_switch: kill,
        runtime: stellarroute_api::cctp::readiness::CctpRuntime::production_defaults(),
    };

    let err = service
        .quote_core(
            &sample_quote_request(),
            stellarroute_api::cctp::access::test_access_token_hash(),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        stellarroute_api::cctp::service::CctpServiceError::ProviderKilled
    ));
}
