//! CCTP component readiness aggregation — no trait defaults may report ready.

use std::sync::Arc;

use async_trait::async_trait;

use crate::cctp::approval::{EvmApprovalVerifier, FakeApprovalVerifier, StellarApprovalVerifier};
use crate::cctp::attestation::{AttestationVerifier, FakeAttestationVerifier};
use crate::cctp::builders::evm::{ProductionEvmCctpBuilder, SharedProductionEvmBuilder};
use crate::cctp::builders::stellar::{
    ProductionStellarCctpBuilder, SharedProductionStellarBuilder,
};
use crate::cctp::builders::{
    BuilderError, EvmCctpBurnBuilder, EvmCctpMintBuilder, PreparedMintBundle,
    StellarCctpBurnBuilder, StellarCctpMintBuilder,
};
use crate::cctp::config::CctpConfig;
use crate::cctp::store::CctpTransfer;
use crate::cctp::verifiers::{
    EvmBurnVerifier, EvmMintVerifier, FakeBurnVerifier, FakeMintVerifier, MintVerifyOutcome,
    StellarBurnVerifier, StellarMintVerifier, VerifiedBurnFacts, VerifiedMintFacts,
};
use crate::models::v2_cctp::{CctpDirection, SEPOLIA_CHAIN_ID, STELLAR_TESTNET_CHAIN_ID};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadinessComponent {
    StellarBurnBuilder,
    EvmBurnBuilder,
    StellarMintBuilder,
    EvmMintBuilder,
    StellarBurnVerifier,
    EvmBurnVerifier,
    StellarMintVerifier,
    EvmMintVerifier,
    EvmApprovalVerifier,
    StellarApprovalVerifier,
    AttestationVerifier,
}

#[derive(Debug, Clone, Default)]
pub struct CctpReadiness {
    pub missing: Vec<ReadinessComponent>,
}

impl CctpReadiness {
    pub fn is_ready(&self) -> bool {
        self.missing.is_empty()
    }
}

#[derive(Clone)]
pub struct CctpRuntime {
    pub stellar_burn_builder: Arc<dyn StellarCctpBurnBuilder>,
    pub evm_burn_builder: Arc<dyn EvmCctpBurnBuilder>,
    pub stellar_mint_builder: Arc<dyn StellarCctpMintBuilder>,
    pub evm_mint_builder: Arc<dyn EvmCctpMintBuilder>,
    pub stellar_burn_verifier: Arc<dyn StellarBurnVerifier>,
    pub evm_burn_verifier: Arc<dyn EvmBurnVerifier>,
    pub stellar_mint_verifier: Arc<dyn StellarMintVerifier>,
    pub evm_mint_verifier: Arc<dyn EvmMintVerifier>,
    pub evm_approval_verifier: Arc<dyn EvmApprovalVerifier>,
    pub stellar_approval_verifier: Arc<dyn StellarApprovalVerifier>,
    pub attestation_verifier: Arc<dyn AttestationVerifier>,
}

impl CctpRuntime {
    pub fn production_defaults() -> Self {
        use crate::cctp::approval::{NotReadyEvmApprovalVerifier, NotReadyStellarApprovalVerifier};
        use crate::cctp::attestation::NotReadyAttestationVerifier;
        use crate::cctp::builders::{
            NotReadyEvmBurnBuilder, NotReadyEvmMintBuilder, NotReadyStellarBurnBuilder,
            NotReadyStellarMintBuilder,
        };
        use crate::cctp::verifiers::{
            NotReadyEvmBurnVerifier, NotReadyEvmMintVerifier, NotReadyStellarBurnVerifier,
            NotReadyStellarMintVerifier,
        };

        Self {
            stellar_burn_builder: Arc::new(NotReadyStellarBurnBuilder),
            evm_burn_builder: Arc::new(NotReadyEvmBurnBuilder),
            stellar_mint_builder: Arc::new(NotReadyStellarMintBuilder),
            evm_mint_builder: Arc::new(NotReadyEvmMintBuilder),
            stellar_burn_verifier: Arc::new(NotReadyStellarBurnVerifier),
            evm_burn_verifier: Arc::new(NotReadyEvmBurnVerifier),
            stellar_mint_verifier: Arc::new(NotReadyStellarMintVerifier),
            evm_mint_verifier: Arc::new(NotReadyEvmMintVerifier),
            evm_approval_verifier: Arc::new(NotReadyEvmApprovalVerifier),
            stellar_approval_verifier: Arc::new(NotReadyStellarApprovalVerifier),
            attestation_verifier: Arc::new(NotReadyAttestationVerifier),
        }
    }

    /// Wire production defaults only — EVM/Stellar production components require
    /// `from_config_async` for semantic RPC probes.
    pub fn from_config(config: &CctpConfig) -> Self {
        let _ = config;
        Self::production_defaults()
    }

    /// Async bootstrap for attestation trust cache and production verifier wiring.
    pub async fn from_config_async(config: &CctpConfig) -> Self {
        let mut runtime = Self::from_config(config);
        wire_evm_components(config, &mut runtime).await;
        wire_stellar_verifiers(config, &mut runtime).await;
        wire_stellar_builders(config, &mut runtime).await;
        if let Some(verifier) = try_build_attestation_verifier_async(config).await {
            runtime.attestation_verifier = verifier;
        } else {
            crate::metrics::record_cctp_stellar_verifier_readiness("attestation", "not_ready");
        }
        log_stellar_verifier_readiness(&runtime, config);
        runtime
    }

    pub fn for_tests(
        stellar_burn: Arc<dyn StellarBurnVerifier>,
        evm_burn: Arc<dyn EvmBurnVerifier>,
        attestation: Arc<dyn AttestationVerifier>,
    ) -> Self {
        Self {
            stellar_burn_verifier: stellar_burn,
            evm_burn_verifier: evm_burn,
            attestation_verifier: attestation,
            ..Self::production_defaults()
        }
    }

    pub fn assess(&self, direction: CctpDirection) -> CctpReadiness {
        let mut missing = Vec::new();
        match direction {
            CctpDirection::StellarToEvm => {
                if !self.stellar_burn_builder.is_ready() {
                    missing.push(ReadinessComponent::StellarBurnBuilder);
                }
                if !self.stellar_burn_verifier.is_ready() {
                    missing.push(ReadinessComponent::StellarBurnVerifier);
                }
                if !self.stellar_approval_verifier.is_ready() {
                    missing.push(ReadinessComponent::StellarApprovalVerifier);
                }
                if !self.evm_mint_builder.is_ready() {
                    missing.push(ReadinessComponent::EvmMintBuilder);
                }
                if !self.evm_mint_verifier.is_ready() {
                    missing.push(ReadinessComponent::EvmMintVerifier);
                }
            }
            CctpDirection::EvmToStellar => {
                if !self.evm_burn_builder.is_ready() {
                    missing.push(ReadinessComponent::EvmBurnBuilder);
                }
                if !self.evm_burn_verifier.is_ready() {
                    missing.push(ReadinessComponent::EvmBurnVerifier);
                }
                if !self.evm_approval_verifier.is_ready() {
                    missing.push(ReadinessComponent::EvmApprovalVerifier);
                }
                if !self.stellar_mint_builder.is_ready() {
                    missing.push(ReadinessComponent::StellarMintBuilder);
                }
                if !self.stellar_mint_verifier.is_ready() {
                    missing.push(ReadinessComponent::StellarMintVerifier);
                }
            }
        }
        if !self.attestation_verifier.is_ready() {
            missing.push(ReadinessComponent::AttestationVerifier);
        }
        CctpReadiness { missing }
    }

    pub fn is_public_executable(&self, config: &CctpConfig) -> bool {
        config.enabled
            && config.is_configured()
            && self.stellar_burn_builder.is_ready()
            && self.evm_burn_builder.is_ready()
            && self.stellar_mint_builder.is_ready()
            && self.evm_mint_builder.is_ready()
            && self.stellar_burn_verifier.is_ready()
            && self.evm_burn_verifier.is_ready()
            && self.stellar_mint_verifier.is_ready()
            && self.evm_mint_verifier.is_ready()
            && self.evm_approval_verifier.is_ready()
            && self.stellar_approval_verifier.is_ready()
            && self.attestation_verifier.is_ready()
    }

    /// HTTP/PG harness: probe-ready fakes for Evm→Stellar Axum paths without weakening
    /// production `from_config` / `from_config_async` semantic probe gating.
    pub fn probe_ready_http_harness(config: &CctpConfig, payload_hash: &str) -> Self {
        let payload_hash = payload_hash.to_string();
        struct HarnessStellarMintBuilder(String);
        #[async_trait]
        impl StellarCctpMintBuilder for HarnessStellarMintBuilder {
            fn is_ready(&self) -> bool {
                true
            }
            async fn prepare_mint(
                &self,
                transfer: &CctpTransfer,
                config: &CctpConfig,
            ) -> Result<PreparedMintBundle, BuilderError> {
                Ok(PreparedMintBundle {
                    step: crate::cctp::builders::MintPrepareStep::Mint,
                    trustline_required: false,
                    primary: crate::models::v2_cctp::PreparedWalletPayload::StellarXdr {
                        network_passphrase: config.stellar_network_passphrase.clone(),
                        xdr_envelope: "AAAA".into(),
                        source: None,
                    },
                    expires_at: transfer.quote_expires_at.timestamp(),
                    payload_hash: self.0.clone(),
                })
            }
        }

        struct HarnessEvmBurnBuilder;
        #[async_trait]
        impl EvmCctpBurnBuilder for HarnessEvmBurnBuilder {
            fn is_ready(&self) -> bool {
                true
            }
            async fn prepare_burn(
                &self,
                _: &CctpTransfer,
                _: &CctpConfig,
            ) -> Result<crate::cctp::builders::PreparedBurnBundle, BuilderError> {
                Err(BuilderError::NotReady)
            }
        }

        let evm_sender = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0";
        let stellar_recipient = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";

        Self {
            stellar_mint_builder: Arc::new(HarnessStellarMintBuilder(payload_hash.clone())),
            evm_burn_builder: Arc::new(HarnessEvmBurnBuilder),
            evm_burn_verifier: Arc::new(FakeBurnVerifier {
                facts: VerifiedBurnFacts {
                    tx_hash: "burn".into(),
                    source_chain_id: SEPOLIA_CHAIN_ID.into(),
                    source_domain: 0,
                    destination_domain: 27,
                    sender: evm_sender.into(),
                    amount_cctp_subunits: 1,
                    burn_token_bytes32: [0; 32],
                    mint_recipient_bytes32: [0; 32],
                    destination_caller_bytes32: [0; 32],
                    min_finality_threshold: 2000,
                    hook_data: None,
                    token_messenger_bytes32: [0; 32],
                    block_or_ledger: None,
                },
                ready: true,
            }),
            evm_approval_verifier: Arc::new(FakeApprovalVerifier {
                facts: crate::cctp::approval::VerifiedApprovalFacts {
                    tx_hash: "approve".into(),
                    owner: evm_sender.into(),
                    token_contract: config.contracts.sepolia_usdc.clone(),
                    spender_contract: config.contracts.sepolia_token_messenger.clone(),
                    amount: 1,
                    chain_id: SEPOLIA_CHAIN_ID.into(),
                },
                ready: true,
            }),
            stellar_mint_verifier: Arc::new(FakeMintVerifier {
                facts: VerifiedMintFacts {
                    tx_hash: "mint-tx".into(),
                    destination_chain_id: STELLAR_TESTNET_CHAIN_ID.into(),
                    contract_address: "CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA"
                        .into(),
                    function_selector: "mint".into(),
                    message_hash: [0u8; 32],
                    attestation_hash: [0u8; 32],
                    nonce: "nonce-1".into(),
                    payload_hash,
                    outcome: MintVerifyOutcome::Pending,
                    recipient_evidence: Some(stellar_recipient.into()),
                },
                completion: MintVerifyOutcome::Pending,
                ready: true,
            }),
            attestation_verifier: Arc::new(FakeAttestationVerifier { ready: true }),
            ..Self::production_defaults()
        }
    }
}

async fn wire_evm_components(config: &CctpConfig, runtime: &mut CctpRuntime) {
    if config.sepolia_rpc_url.trim().is_empty() {
        crate::metrics::record_cctp_stellar_verifier_readiness("evm_burn_builder", "missing");
        return;
    }
    match ProductionEvmCctpBuilder::try_new(config).await {
        Ok(builder) if builder.is_production_ready() => {
            let shared = SharedProductionEvmBuilder(std::sync::Arc::new(builder));
            runtime.evm_burn_builder = std::sync::Arc::new(shared.clone());
            runtime.evm_mint_builder = std::sync::Arc::new(shared);
            crate::metrics::record_cctp_stellar_verifier_readiness("evm_burn_builder", "ready");
            crate::metrics::record_cctp_stellar_verifier_readiness("evm_mint_builder", "ready");
        }
        Ok(_) => {
            crate::metrics::record_cctp_stellar_verifier_readiness(
                "evm_burn_builder",
                "probe_failed",
            );
            crate::metrics::record_cctp_stellar_verifier_readiness(
                "evm_mint_builder",
                "probe_failed",
            );
        }
        Err(_) => {
            crate::metrics::record_cctp_stellar_verifier_readiness("evm_burn_builder", "not_ready");
            crate::metrics::record_cctp_stellar_verifier_readiness("evm_mint_builder", "not_ready");
        }
    }
    match crate::cctp::evm_approval_verifier::EvmRpcApprovalVerifier::try_new(config).await {
        Ok(v) if v.is_ready() => {
            runtime.evm_approval_verifier = std::sync::Arc::new(v);
            crate::metrics::record_cctp_stellar_verifier_readiness("evm_approval", "ready");
        }
        Ok(_) => {
            crate::metrics::record_cctp_stellar_verifier_readiness("evm_approval", "probe_failed");
        }
        Err(_) => {
            crate::metrics::record_cctp_stellar_verifier_readiness("evm_approval", "not_ready");
        }
    }
    match crate::cctp::evm_burn_verifier::EvmRpcBurnVerifier::try_new(config).await {
        Ok(v) if v.is_ready() => {
            runtime.evm_burn_verifier = std::sync::Arc::new(v);
            crate::metrics::record_cctp_stellar_verifier_readiness("evm_burn", "ready");
        }
        Ok(_) => {
            crate::metrics::record_cctp_stellar_verifier_readiness("evm_burn", "probe_failed");
        }
        Err(_) => {
            crate::metrics::record_cctp_stellar_verifier_readiness("evm_burn", "not_ready");
        }
    }
    match crate::cctp::evm_mint_verifier::EvmRpcMintVerifier::try_new(config).await {
        Ok(v) if v.is_ready() => {
            runtime.evm_mint_verifier = std::sync::Arc::new(v);
            crate::metrics::record_cctp_stellar_verifier_readiness("evm_mint", "ready");
        }
        Ok(_) => {
            crate::metrics::record_cctp_stellar_verifier_readiness("evm_mint", "probe_failed");
        }
        Err(_) => {
            crate::metrics::record_cctp_stellar_verifier_readiness("evm_mint", "not_ready");
        }
    }
}

async fn wire_stellar_builders(config: &CctpConfig, runtime: &mut CctpRuntime) {
    if config.stellar_rpc_url.trim().is_empty() {
        crate::metrics::record_cctp_stellar_verifier_readiness("stellar_burn_builder", "missing");
        crate::metrics::record_cctp_stellar_verifier_readiness("stellar_mint_builder", "missing");
        return;
    }
    match ProductionStellarCctpBuilder::try_new(config).await {
        Ok(builder) if builder.builder_ready() => {
            let shared = SharedProductionStellarBuilder(Arc::new(builder));
            runtime.stellar_burn_builder = Arc::new(shared.clone());
            runtime.stellar_mint_builder = Arc::new(shared);
            crate::metrics::record_cctp_stellar_verifier_readiness("stellar_burn_builder", "ready");
            crate::metrics::record_cctp_stellar_verifier_readiness("stellar_mint_builder", "ready");
        }
        Ok(_) => {
            crate::metrics::record_cctp_stellar_verifier_readiness(
                "stellar_burn_builder",
                "probe_failed",
            );
            crate::metrics::record_cctp_stellar_verifier_readiness(
                "stellar_mint_builder",
                "probe_failed",
            );
        }
        Err(_) => {
            crate::metrics::record_cctp_stellar_verifier_readiness(
                "stellar_burn_builder",
                "not_ready",
            );
            crate::metrics::record_cctp_stellar_verifier_readiness(
                "stellar_mint_builder",
                "not_ready",
            );
        }
    }
}

async fn wire_stellar_verifiers(config: &CctpConfig, runtime: &mut CctpRuntime) {
    if config.stellar_rpc_url.trim().is_empty() {
        crate::metrics::record_cctp_stellar_verifier_readiness("rpc", "missing");
        return;
    }
    match crate::cctp::stellar_approval_verifier::StellarRpcApprovalVerifier::new(config).await {
        Ok(v) if v.is_ready() => {
            runtime.stellar_approval_verifier = Arc::new(v);
            crate::metrics::record_cctp_stellar_verifier_readiness("approval", "ready");
        }
        Ok(_) => {
            crate::metrics::record_cctp_stellar_verifier_readiness("approval", "probe_failed");
        }
        Err(_) => {
            crate::metrics::record_cctp_stellar_verifier_readiness("approval", "not_ready");
        }
    }
    match crate::cctp::stellar_burn_verifier::StellarRpcBurnVerifier::new(config).await {
        Ok(v) if v.is_ready() => {
            runtime.stellar_burn_verifier = Arc::new(v);
            crate::metrics::record_cctp_stellar_verifier_readiness("burn", "ready");
        }
        Ok(_) => {
            crate::metrics::record_cctp_stellar_verifier_readiness("burn", "probe_failed");
        }
        Err(_) => {
            crate::metrics::record_cctp_stellar_verifier_readiness("burn", "not_ready");
        }
    }
    match crate::cctp::stellar_mint_verifier::StellarRpcMintVerifier::new(config).await {
        Ok(v) if v.is_ready() => {
            runtime.stellar_mint_verifier = Arc::new(v);
            crate::metrics::record_cctp_stellar_verifier_readiness("mint", "ready");
        }
        Ok(_) => {
            crate::metrics::record_cctp_stellar_verifier_readiness("mint", "probe_failed");
        }
        Err(_) => {
            crate::metrics::record_cctp_stellar_verifier_readiness("mint", "not_ready");
        }
    }
}

fn log_stellar_verifier_readiness(runtime: &CctpRuntime, config: &CctpConfig) {
    use tracing::warn;

    let stellar_ready = runtime.stellar_approval_verifier.is_ready()
        && runtime.stellar_burn_verifier.is_ready()
        && runtime.stellar_mint_verifier.is_ready();
    let attestation_ready = runtime.attestation_verifier.is_ready();

    if !stellar_ready {
        warn!(
            corridor = %config.corridor_id(),
            stellar_approval = runtime.stellar_approval_verifier.is_ready(),
            stellar_burn = runtime.stellar_burn_verifier.is_ready(),
            stellar_mint = runtime.stellar_mint_verifier.is_ready(),
            "CCTP Stellar transaction verifiers not fully ready"
        );
    }
    if !attestation_ready {
        warn!(
            corridor = %config.corridor_id(),
            "CCTP attestation verifier not ready after bootstrap"
        );
    }
    if !runtime.is_public_executable(config) {
        warn!(
            corridor = %config.corridor_id(),
            enabled = config.enabled,
            "CCTP public execution remains disabled"
        );
    }
}

async fn try_build_attestation_verifier_async(
    config: &CctpConfig,
) -> Option<Arc<dyn crate::cctp::attestation::AttestationVerifier>> {
    use crate::cctp::attestation::CircleAttestationVerifier;
    use crate::cctp::attestation_trust::{AttestationRefreshDeps, AttestationTrustCache};
    use crate::cctp::evm_attester_reader::evm_reader_arc;
    use crate::cctp::iris_public_keys::ReqwestIrisPublicKeySource;
    use crate::cctp::stellar_attester_reader::stellar_reader_arc;

    if config.sepolia_rpc_url.trim().is_empty() || config.stellar_rpc_url.trim().is_empty() {
        return None;
    }
    let iris_source = ReqwestIrisPublicKeySource::from_config(config).ok()?;
    let evm_reader = evm_reader_arc(config).ok()?;
    let stellar_reader = stellar_reader_arc(config).ok()?;

    let trust = Arc::new(AttestationTrustCache::from_config(config));
    let deps = AttestationRefreshDeps {
        iris_source: Arc::new(iris_source),
        readers: vec![evm_reader, stellar_reader],
    };
    let verifier = Arc::new(CircleAttestationVerifier::new(trust, deps));
    verifier.bootstrap().await.ok()?;
    if !verifier.is_ready() {
        return None;
    }
    Some(verifier)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_defaults_all_not_ready() {
        let rt = CctpRuntime::production_defaults();
        assert!(!rt.evm_burn_builder.is_ready());
        assert!(!rt.evm_mint_builder.is_ready());
        assert!(!rt.evm_burn_verifier.is_ready());
        assert!(!rt.evm_approval_verifier.is_ready());
        assert!(!rt.is_public_executable(&CctpConfig::default_testnet()));
    }

    #[test]
    fn from_config_does_not_pretend_evm_ready_without_probes() {
        let mut cfg = CctpConfig::default_testnet();
        cfg.sepolia_rpc_url = "https://sepolia.drpc.org".into();
        let rt = CctpRuntime::from_config(&cfg);
        assert!(!rt.evm_burn_builder.is_ready());
        assert!(!rt.evm_mint_builder.is_ready());
        assert!(!rt.evm_burn_verifier.is_ready());
        assert!(!rt.evm_mint_verifier.is_ready());
        assert!(!rt.evm_approval_verifier.is_ready());
        assert!(!rt.is_public_executable(&cfg));
    }

    #[tokio::test]
    async fn from_config_async_leaves_attestation_not_ready_without_live_deps() {
        let mut cfg = CctpConfig::default_testnet();
        cfg.sepolia_rpc_url.clear();
        cfg.stellar_rpc_url.clear();
        let rt = CctpRuntime::from_config_async(&cfg).await;
        assert!(!rt.attestation_verifier.is_ready());
        assert!(!rt.is_public_executable(&cfg));
    }

    #[test]
    fn sync_from_config_never_pretends_attestation_ready() {
        let mut cfg = CctpConfig::default_testnet();
        cfg.sepolia_rpc_url = "https://sepolia.drpc.org".into();
        cfg.stellar_rpc_url = "https://soroban-testnet.stellar.org".into();
        let rt = CctpRuntime::from_config(&cfg);
        assert!(!rt.attestation_verifier.is_ready());
    }

    #[test]
    fn evm_to_stellar_assess_requires_approval_verifier_when_probed() {
        let mut cfg = CctpConfig::default_testnet();
        cfg.sepolia_rpc_url = "https://sepolia.drpc.org".into();
        let rt = CctpRuntime::from_config(&cfg);
        let readiness = rt.assess(CctpDirection::EvmToStellar);
        assert!(readiness
            .missing
            .contains(&ReadinessComponent::EvmApprovalVerifier));
        assert!(readiness
            .missing
            .contains(&ReadinessComponent::StellarMintVerifier));
    }

    #[test]
    fn probe_ready_http_harness_satisfies_evm_to_stellar_without_sync_probes() {
        let mut cfg = CctpConfig::default_testnet();
        cfg.enabled = true;
        let rt = CctpRuntime::probe_ready_http_harness(&cfg, "harness-payload");
        assert!(rt.assess(CctpDirection::EvmToStellar).is_ready());
        assert!(!rt.assess(CctpDirection::StellarToEvm).is_ready());
        assert!(!rt.is_public_executable(&cfg));
    }
}
