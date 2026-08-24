//! Stellar Testnet Soroban unsigned CCTP transaction builders.
//!
//! Production builder requires Soroban RPC simulation; offline encoder is test-only.

pub mod encoder;

use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;

use crate::cctp::builders::{
    BuilderError, BurnPrepareStep, MintPrepareStep, PreparedBurnBundle, PreparedMintBundle,
    StellarCctpBurnBuilder, StellarCctpMintBuilder,
};
use crate::cctp::config::{corridor_min_finality, CctpConfig, STELLAR_TESTNET_PASSPHRASE};
use crate::cctp::encoding::{
    cctp_subunits_to_stellar_subunits, decimal_to_cctp_subunits, evm_address_to_bytes32,
    stellar_outbound_cctp_amount_strict,
};
use crate::cctp::message::parse_cctp_v2_message;
use crate::cctp::stellar_allowance::StellarRpcAllowanceChecker;
use crate::cctp::stellar_builder_simulation::{
    approval_expiration_ledger, ledger_bounds_for_expiry, probe_strict_simulation_assembly,
    simulate_and_assemble_invoke, time_bounds_for_expiry, APPROVAL_LEDGER_SAFETY_MARGIN,
};
use crate::cctp::stellar_payload::{passphrase_for_config, payload_hash_from_envelope_xdr};
use crate::cctp::stellar_readiness_probes::probe_stellar_contracts;
use crate::cctp::stellar_rpc::StellarRpcClient;
use crate::cctp::stellar_sequence::RpcAccountSequenceSource;
use crate::cctp::stellar_trustline::{
    build_unsigned_change_trust_xdr, default_change_trust_timeout_secs,
    recipient_trustline_account, HorizonUsdcTrustlineProbe, UsdcTrustlineProbe,
};
use crate::cctp::store::CctpTransfer;
use crate::models::v2_cctp::{CctpDirection, PreparedWalletPayload};
use crate::swap::tx::{AccountSequenceSource, DEFAULT_BASE_FEE};

use encoder::{
    approve_args, deposit_for_burn_args, encode_invoke_at_sequence, mint_and_forward_args,
    InvokeTxParams,
};

/// Soroban token allowance probe — production implementations query on-chain state.
#[async_trait]
pub trait StellarAllowanceChecker: Send + Sync {
    async fn has_sufficient_allowance(
        &self,
        owner: &str,
        token: &str,
        spender: &str,
        amount: i128,
    ) -> Result<bool, BuilderError>;
}

/// Test double for allowance gating.
pub struct FixedAllowanceChecker {
    pub sufficient: bool,
}

#[async_trait]
impl StellarAllowanceChecker for FixedAllowanceChecker {
    async fn has_sufficient_allowance(
        &self,
        _owner: &str,
        _token: &str,
        _spender: &str,
        _amount: i128,
    ) -> Result<bool, BuilderError> {
        Ok(self.sufficient)
    }
}

/// Offline XDR encoder — not production-ready; never enters runtime aggregate.
pub struct OfflineStellarXdrEncoder;

impl OfflineStellarXdrEncoder {
    pub fn encode_approval_at_sequence(
        source: &str,
        token: &str,
        spender: &str,
        amount: i128,
        expiration_ledger: u32,
        account_sequence: i64,
    ) -> Result<String, BuilderError> {
        encode_invoke_at_sequence(
            source,
            token,
            "approve",
            approve_args(source, spender, amount, expiration_ledger)?,
            account_sequence,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn encode_burn_at_sequence(
        source: &str,
        token_messenger: &str,
        caller: &str,
        amount: i128,
        destination_domain: u32,
        mint_recipient: [u8; 32],
        burn_token: &str,
        max_fee: i128,
        min_finality: u32,
        account_sequence: i64,
    ) -> Result<String, BuilderError> {
        encode_invoke_at_sequence(
            source,
            token_messenger,
            "deposit_for_burn",
            deposit_for_burn_args(
                caller,
                amount,
                destination_domain,
                mint_recipient,
                burn_token,
                max_fee,
                min_finality,
            )?,
            account_sequence,
        )
    }
}

#[async_trait]
impl StellarCctpBurnBuilder for OfflineStellarXdrEncoder {
    fn is_ready(&self) -> bool {
        false
    }

    async fn prepare_burn(
        &self,
        _: &CctpTransfer,
        _: &CctpConfig,
    ) -> Result<PreparedBurnBundle, BuilderError> {
        Err(BuilderError::NotReady)
    }
}

pub struct ProductionStellarCctpBuilder {
    pub sequences: Arc<dyn AccountSequenceSource>,
    pub rpc: Arc<StellarRpcClient>,
    pub allowance: Arc<dyn StellarAllowanceChecker>,
    pub trustline: Arc<dyn UsdcTrustlineProbe>,
    pub probe_ok: bool,
    pub base_fee: u32,
}

#[derive(Clone)]
pub struct SharedProductionStellarBuilder(pub Arc<ProductionStellarCctpBuilder>);

impl ProductionStellarCctpBuilder {
    pub async fn try_new(config: &CctpConfig) -> Result<Self, BuilderError> {
        if config.stellar_rpc_url.trim().is_empty() {
            return Err(BuilderError::NotReady);
        }
        Self::ensure_testnet_config(config)?;
        let rpc = Arc::new(
            StellarRpcClient::new(config).map_err(|e| BuilderError::Validation(e.to_string()))?,
        );
        let probe = probe_stellar_contracts(config).await.all_ok()
            && probe_strict_simulation_assembly(&rpc, config).await;
        let allowance: Arc<dyn StellarAllowanceChecker> =
            match StellarRpcAllowanceChecker::new(config).await {
                Ok(c) if c.is_ready() => Arc::new(c),
                _ => Arc::new(FixedAllowanceChecker { sufficient: false }),
            };
        let sequences = Arc::new(RpcAccountSequenceSource::new(config, rpc.clone()));
        let trustline: Arc<dyn UsdcTrustlineProbe> =
            Arc::new(HorizonUsdcTrustlineProbe::new(&config.stellar_horizon_url));
        Ok(Self {
            sequences,
            rpc,
            allowance,
            trustline,
            probe_ok: probe,
            base_fee: DEFAULT_BASE_FEE,
        })
    }

    fn is_production_ready(&self) -> bool {
        self.probe_ok && self.rpc.is_ready()
    }

    pub fn builder_ready(&self) -> bool {
        self.is_production_ready()
    }

    fn ensure_testnet_config(config: &CctpConfig) -> Result<(), BuilderError> {
        let passphrase = if config.stellar_network_passphrase.is_empty() {
            STELLAR_TESTNET_PASSPHRASE
        } else {
            &config.stellar_network_passphrase
        };
        if passphrase != STELLAR_TESTNET_PASSPHRASE {
            return Err(BuilderError::Validation("wrong network passphrase".into()));
        }
        if config.stellar_domain != crate::cctp::config::STELLAR_TESTNET_DOMAIN {
            return Err(BuilderError::Validation("wrong stellar domain".into()));
        }
        Ok(())
    }

    fn ensure_not_expired(transfer: &CctpTransfer) -> Result<(), BuilderError> {
        if Utc::now() > transfer.quote_expires_at {
            return Err(BuilderError::QuoteExpired);
        }
        if let Some(fee_exp) = transfer.fee_expires_at {
            if Utc::now() > fee_exp {
                return Err(BuilderError::FeeExpired);
            }
        }
        Ok(())
    }

    fn validate_g_sender(sender: &str) -> Result<(), BuilderError> {
        if stellar_strkey::ed25519::PublicKey::from_string(sender.trim()).is_err() {
            return Err(BuilderError::Validation(
                "sender must be valid G-address".into(),
            ));
        }
        Ok(())
    }

    fn validate_mint_recipient(recipient: &str) -> Result<(), BuilderError> {
        match crate::cctp::stellar_muxed::parse_recipient_strkey(recipient) {
            Ok(crate::cctp::stellar_muxed::StellarRecipientKey::Account(_))
            | Ok(crate::cctp::stellar_muxed::StellarRecipientKey::Muxed { .. }) => Ok(()),
            Ok(crate::cctp::stellar_muxed::StellarRecipientKey::Contract(_)) => Err(
                BuilderError::Validation("contract recipient not allowed for corridor".into()),
            ),
            Err(_) => Err(BuilderError::Validation(
                "mint recipient must be G or M address".into(),
            )),
        }
    }

    async fn latest_ledger(&self) -> Result<u32, BuilderError> {
        self.rpc
            .latest_ledger()
            .await
            .map_err(|e| BuilderError::AccountLookup(e.to_string()))
    }

    async fn build_simulated_invoke(
        &self,
        source: &str,
        contract: &str,
        function: &str,
        args: Vec<stellar_xdr::curr::ScVal>,
        config: &CctpConfig,
        transfer: &CctpTransfer,
    ) -> Result<String, BuilderError> {
        Self::ensure_testnet_config(config)?;
        let current = self
            .sequences
            .current_sequence(source)
            .await
            .map_err(|e| BuilderError::AccountLookup(e.to_string()))?;
        let sequence = current.saturating_add(1);
        let latest = self.latest_ledger().await?;
        let quote_exp = transfer.quote_expires_at.timestamp();
        let params = InvokeTxParams {
            source: source.to_string(),
            contract: contract.to_string(),
            function: function.to_string(),
            args,
            sequence,
            base_fee: self.base_fee,
            time_bounds: time_bounds_for_expiry(quote_exp),
            ledger_bounds: ledger_bounds_for_expiry(latest, quote_exp),
        };
        simulate_and_assemble_invoke(&self.rpc, params).await
    }

    fn stellar_payload(
        xdr: String,
        passphrase: &str,
        source: Option<String>,
    ) -> PreparedWalletPayload {
        PreparedWalletPayload::StellarXdr {
            network_passphrase: passphrase.to_string(),
            xdr_envelope: xdr,
            source,
        }
    }

    async fn needs_approval(
        &self,
        transfer: &CctpTransfer,
        config: &CctpConfig,
        stellar_amount: i128,
    ) -> Result<bool, BuilderError> {
        if let Some(exp) = transfer.approval_expiration_ledger {
            let latest = self.latest_ledger().await?;
            if (latest as u64) >= exp.saturating_sub(APPROVAL_LEDGER_SAFETY_MARGIN as u64) {
                return Ok(true);
            }
        }
        let sufficient = self
            .allowance
            .has_sufficient_allowance(
                &transfer.sender,
                &config.contracts.stellar_usdc,
                &config.contracts.stellar_token_messenger,
                stellar_amount,
            )
            .await?;
        Ok(!sufficient)
    }

    fn validate_mint_message(
        transfer: &CctpTransfer,
        config: &CctpConfig,
        message: &[u8],
    ) -> Result<(), BuilderError> {
        let parsed = parse_cctp_v2_message(message)
            .map_err(|e| BuilderError::Validation(format!("message parse: {e}")))?;
        let expectations = crate::cctp::expectations::build_corridor_expectations(transfer, config)
            .map_err(|e| BuilderError::Validation(e.to_string()))?;
        if parsed.source_domain != expectations.source_domain {
            return Err(BuilderError::Validation("wrong source domain".into()));
        }
        if parsed.destination_domain != expectations.destination_domain {
            return Err(BuilderError::Validation("wrong destination domain".into()));
        }
        if let Some(nonce) = transfer.message_nonce.as_deref() {
            let expected = nonce.trim().strip_prefix("0x").unwrap_or(nonce.trim());
            let actual = hex::encode(parsed.nonce);
            if !actual.eq_ignore_ascii_case(expected) {
                return Err(BuilderError::Validation("nonce mismatch".into()));
            }
        }
        if parsed.body.burn_token != expectations.burn_token {
            return Err(BuilderError::Validation("burn token mismatch".into()));
        }
        if parsed.body.mint_recipient != expectations.mint_recipient {
            return Err(BuilderError::Validation("mint recipient mismatch".into()));
        }
        if parsed.body.amount != expectations.amount_cctp_subunits {
            return Err(BuilderError::Validation("amount mismatch".into()));
        }
        if parsed.min_finality_threshold != expectations.min_finality {
            return Err(BuilderError::Validation("finality mismatch".into()));
        }
        if expectations.hook_data_required_empty && !parsed.body.hook_data.is_empty() {
            return Err(BuilderError::Validation("unexpected hook data".into()));
        }
        if let Some(expected_hook) = expectations.hook_data.as_ref() {
            if parsed.body.hook_data != *expected_hook {
                return Err(BuilderError::Validation("hook data mismatch".into()));
            }
        }
        Ok(())
    }
}

#[async_trait]
impl StellarCctpBurnBuilder for ProductionStellarCctpBuilder {
    fn is_ready(&self) -> bool {
        self.is_production_ready()
    }

    async fn prepare_burn(
        &self,
        transfer: &CctpTransfer,
        config: &CctpConfig,
    ) -> Result<PreparedBurnBundle, BuilderError> {
        if !self.is_production_ready() {
            return Err(BuilderError::NotReady);
        }
        if transfer.direction != CctpDirection::StellarToEvm {
            return Err(BuilderError::Validation(
                "Stellar burn builder only supports stellar_to_evm".into(),
            ));
        }
        Self::ensure_not_expired(transfer)?;
        if transfer.sender.is_empty() {
            return Err(BuilderError::Validation(
                "sender required for Stellar burn".into(),
            ));
        }
        Self::validate_g_sender(&transfer.sender)?;

        let cctp_amount = stellar_outbound_cctp_amount_strict(&transfer.amount)
            .map_err(|e| BuilderError::Encoding(e.to_string()))?;
        let stellar_amount = cctp_subunits_to_stellar_subunits(cctp_amount)
            .map_err(|e| BuilderError::Encoding(e.to_string()))?
            as i128;
        let max_fee = transfer
            .max_fee
            .as_deref()
            .ok_or_else(|| BuilderError::Validation("max_fee missing".into()))?;
        let max_fee_stellar = cctp_subunits_to_stellar_subunits(
            decimal_to_cctp_subunits(max_fee).map_err(|e| BuilderError::Encoding(e.to_string()))?,
        )
        .map_err(|e| BuilderError::Encoding(e.to_string()))? as i128;

        let passphrase = passphrase_for_config(config);
        let expires_at = transfer.quote_expires_at.timestamp();
        let latest = self.latest_ledger().await?;
        let approval_exp = approval_expiration_ledger(latest, expires_at);

        if self
            .needs_approval(transfer, config, stellar_amount)
            .await?
        {
            let approve_xdr = self
                .build_simulated_invoke(
                    &transfer.sender,
                    &config.contracts.stellar_usdc,
                    "approve",
                    approve_args(
                        &transfer.sender,
                        &config.contracts.stellar_token_messenger,
                        stellar_amount,
                        approval_exp,
                    )?,
                    config,
                    transfer,
                )
                .await?;
            return Ok(PreparedBurnBundle {
                step: BurnPrepareStep::Approval,
                approval_required: true,
                primary: Self::stellar_payload(approve_xdr, &passphrase, None),
                required_approvals: vec![],
                required_prior_payloads: vec![],
                expires_at,
                approval_expiration_ledger: Some(approval_exp),
            });
        }

        let mint_recipient = evm_address_to_bytes32(&transfer.recipient)
            .map_err(|e| BuilderError::Encoding(e.to_string()))?;
        let min_finality = corridor_min_finality(transfer.finality);
        let burn_xdr = self
            .build_simulated_invoke(
                &transfer.sender,
                &config.contracts.stellar_token_messenger,
                "deposit_for_burn",
                deposit_for_burn_args(
                    &transfer.sender,
                    stellar_amount,
                    config.sepolia_domain,
                    mint_recipient,
                    &config.contracts.stellar_usdc,
                    max_fee_stellar,
                    min_finality,
                )?,
                config,
                transfer,
            )
            .await?;

        Ok(PreparedBurnBundle {
            step: BurnPrepareStep::Burn,
            approval_required: false,
            primary: Self::stellar_payload(burn_xdr, &passphrase, None),
            required_approvals: vec![],
            required_prior_payloads: vec![],
            expires_at,
            approval_expiration_ledger: None,
        })
    }
}

#[async_trait]
impl StellarCctpMintBuilder for ProductionStellarCctpBuilder {
    fn is_ready(&self) -> bool {
        self.is_production_ready()
    }

    async fn prepare_mint(
        &self,
        transfer: &CctpTransfer,
        config: &CctpConfig,
    ) -> Result<PreparedMintBundle, BuilderError> {
        if !self.is_production_ready() {
            return Err(BuilderError::NotReady);
        }
        if transfer.direction != CctpDirection::EvmToStellar {
            return Err(BuilderError::Validation(
                "Stellar mint builder only supports evm_to_stellar destination".into(),
            ));
        }
        Self::ensure_not_expired(transfer)?;
        let message = transfer
            .raw_message
            .as_ref()
            .ok_or_else(|| BuilderError::Validation("raw_message missing".into()))?;
        let attestation = transfer
            .attestation
            .as_ref()
            .ok_or_else(|| BuilderError::Validation("attestation missing".into()))?;

        if transfer.recipient.is_empty() {
            return Err(BuilderError::Validation("recipient required".into()));
        }
        Self::validate_mint_recipient(&transfer.recipient)?;
        let submitter = transfer
            .mint_submitter
            .as_deref()
            .ok_or_else(|| BuilderError::Validation("mint_submitter required".into()))?;
        Self::validate_g_sender(submitter)?;
        Self::validate_mint_message(transfer, config, message)?;

        let passphrase = passphrase_for_config(config);
        let quote_exp = transfer.quote_expires_at.timestamp();
        let max_ttl = config.mint_payload_ttl_secs as i64;
        let expires_at = quote_exp.min(Utc::now().timestamp() + max_ttl);

        let trustline_account = recipient_trustline_account(&transfer.recipient)?;
        let has_trustline = self
            .trustline
            .has_usdc_trustline(&trustline_account)
            .await?;
        if !has_trustline {
            let current = self
                .sequences
                .current_sequence(&trustline_account)
                .await
                .map_err(|e| BuilderError::AccountLookup(e.to_string()))?;
            let (xdr, _) = build_unsigned_change_trust_xdr(
                &trustline_account,
                current,
                &passphrase,
                self.base_fee,
                default_change_trust_timeout_secs(),
            )?;
            let payload = Self::stellar_payload(
                xdr.clone(),
                &passphrase,
                Some(trustline_account.clone()),
            );
            let payload_hash = {
                use sha2::{Digest, Sha256};
                let json = serde_json::to_string(&payload).unwrap_or_default();
                hex::encode(Sha256::digest(json.as_bytes()))
            };
            return Ok(PreparedMintBundle {
                step: MintPrepareStep::Trustline,
                trustline_required: true,
                primary: payload,
                expires_at,
                payload_hash,
            });
        }

        let xdr = self
            .build_simulated_invoke(
                submitter,
                &config.contracts.stellar_cctp_forwarder,
                "mint_and_forward",
                mint_and_forward_args(message, attestation)?,
                config,
                transfer,
            )
            .await?;

        let payload = Self::stellar_payload(xdr.clone(), &passphrase, None);
        let payload_hash = payload_hash_from_envelope_xdr(&xdr, config)
            .map_err(|e| BuilderError::Encoding(e.to_string()))?;

        Ok(PreparedMintBundle {
            step: MintPrepareStep::Mint,
            trustline_required: false,
            primary: payload,
            expires_at,
            payload_hash,
        })
    }
}

#[async_trait]
impl StellarCctpBurnBuilder for SharedProductionStellarBuilder {
    fn is_ready(&self) -> bool {
        self.0.builder_ready()
    }

    async fn prepare_burn(
        &self,
        transfer: &CctpTransfer,
        config: &CctpConfig,
    ) -> Result<PreparedBurnBundle, BuilderError> {
        self.0.prepare_burn(transfer, config).await
    }
}

#[async_trait]
impl StellarCctpMintBuilder for SharedProductionStellarBuilder {
    fn is_ready(&self) -> bool {
        self.0.builder_ready()
    }

    async fn prepare_mint(
        &self,
        transfer: &CctpTransfer,
        config: &CctpConfig,
    ) -> Result<PreparedMintBundle, BuilderError> {
        self.0.prepare_mint(transfer, config).await
    }
}

#[cfg(test)]
mod tests {
    use super::encoder::envelope_sequence;
    use super::*;
    use crate::cctp::config::CctpConfig;
    use crate::cctp::stellar_trustline::FixedUsdcTrustlineProbe;
    use crate::swap::tx::FixedAccountSequences;
    use chrono::Duration;
    use uuid::Uuid;

    fn sample_stellar_burn_transfer(approval_hash: Option<String>) -> CctpTransfer {
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
            finality: crate::models::v2_cctp::CctpFinality::Standard,
            runtime_fee_quote: Some("1".into()),
            max_fee: Some("1".into()),
            fee_expires_at: Some(now + Duration::minutes(10)),
            quote_expires_at: now + Duration::minutes(10),
            status: crate::models::v2_cctp::CctpTransferStatus::BurnPrepared,
            source_tx_hash: None,
            source_approval_tx_hash: approval_hash,
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

    #[test]
    fn offline_encoder_uses_distinct_sequences_for_approval_then_burn() {
        let source = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";
        let cfg = CctpConfig::default_testnet();
        let approve = OfflineStellarXdrEncoder::encode_approval_at_sequence(
            source,
            &cfg.contracts.stellar_usdc,
            &cfg.contracts.stellar_token_messenger,
            1_000_000,
            9_999,
            100,
        )
        .unwrap();
        let burn = OfflineStellarXdrEncoder::encode_burn_at_sequence(
            source,
            &cfg.contracts.stellar_token_messenger,
            source,
            1_000_000,
            cfg.sepolia_domain,
            [1u8; 32],
            &cfg.contracts.stellar_usdc,
            1,
            crate::cctp::config::FINALITY_STANDARD,
            101,
        )
        .unwrap();
        assert_eq!(envelope_sequence(&approve).unwrap(), 100);
        assert_eq!(envelope_sequence(&burn).unwrap(), 101);
        assert_ne!(approve, burn);
    }

    #[test]
    fn offline_encoder_is_not_production_ready() {
        assert!(!OfflineStellarXdrEncoder.is_ready());
    }

    #[tokio::test]
    async fn prepared_burn_envelope_uses_next_tx_sequence() {
        use crate::models::v2_cctp::PreparedWalletPayload;
        use serde_json::json;
        use stellar_xdr::curr::{
            LedgerFootprint, Limits, SorobanResources, SorobanTransactionData,
            SorobanTransactionDataExt, WriteXdr,
        };
        use wiremock::matchers::{body_string_contains, method};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let transaction_data = SorobanTransactionData {
            ext: SorobanTransactionDataExt::V0,
            resources: SorobanResources {
                footprint: LedgerFootprint {
                    read_only: Default::default(),
                    read_write: Default::default(),
                },
                instructions: 0,
                disk_read_bytes: 0,
                write_bytes: 0,
            },
            resource_fee: 0,
        }
        .to_xdr_base64(Limits::none())
        .expect("soroban tx data xdr");

        let server = MockServer::start().await;
        let mut cfg = CctpConfig::default_testnet();
        cfg.stellar_rpc_url = server.uri();

        Mock::given(method("POST"))
            .and(body_string_contains("getLatestLedger"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": { "sequence": 50_000 }
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(body_string_contains("simulateTransaction"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": {
                    "transactionData": transaction_data,
                    "minResourceFee": "100",
                    "results": [{ "xdr": "AAAAAA==" }]
                }
            })))
            .mount(&server)
            .await;

        let rpc = Arc::new(StellarRpcClient::new(&cfg).unwrap());
        let builder = ProductionStellarCctpBuilder {
            sequences: Arc::new(FixedAccountSequences::new(100)),
            rpc,
            allowance: Arc::new(FixedAllowanceChecker { sufficient: true }),
            trustline: Arc::new(FixedUsdcTrustlineProbe { present: true }),
            probe_ok: true,
            base_fee: DEFAULT_BASE_FEE,
        };
        let bundle = builder
            .prepare_burn(&sample_stellar_burn_transfer(None), &cfg)
            .await
            .expect("prepare_burn");
        let PreparedWalletPayload::StellarXdr { xdr_envelope, .. } = bundle.primary else {
            panic!("expected stellar xdr payload");
        };
        assert_eq!(envelope_sequence(&xdr_envelope).unwrap(), 101);
    }

    #[tokio::test]
    async fn approval_gate_returns_only_approval_payload_when_allowance_insufficient() {
        let mut cfg = CctpConfig::default_testnet();
        cfg.stellar_rpc_url = "http://127.0.0.1:1".into();
        let rpc = Arc::new(StellarRpcClient::new(&cfg).unwrap());
        let builder = ProductionStellarCctpBuilder {
            sequences: Arc::new(FixedAccountSequences::new(100)),
            rpc,
            allowance: Arc::new(FixedAllowanceChecker { sufficient: false }),
            trustline: Arc::new(FixedUsdcTrustlineProbe { present: true }),
            probe_ok: true,
            base_fee: DEFAULT_BASE_FEE,
        };
        let transfer = sample_stellar_burn_transfer(None);
        let needs = builder
            .needs_approval(&transfer, &CctpConfig::default_testnet(), 1)
            .await
            .unwrap();
        assert!(needs);
        let sufficient_builder = ProductionStellarCctpBuilder {
            sequences: Arc::new(FixedAccountSequences::new(100)),
            rpc: Arc::new(StellarRpcClient::new(&cfg).unwrap()),
            allowance: Arc::new(FixedAllowanceChecker { sufficient: true }),
            trustline: Arc::new(FixedUsdcTrustlineProbe { present: true }),
            probe_ok: true,
            base_fee: DEFAULT_BASE_FEE,
        };
        let needs_when_sufficient = sufficient_builder
            .needs_approval(&transfer, &CctpConfig::default_testnet(), 1)
            .await
            .unwrap();
        assert!(!needs_when_sufficient);
    }

    #[tokio::test]
    async fn production_not_ready_without_probe() {
        let mut cfg = CctpConfig::default_testnet();
        cfg.stellar_rpc_url = "http://127.0.0.1:1".into();
        let rpc = Arc::new(StellarRpcClient::new(&cfg).unwrap());
        let builder = ProductionStellarCctpBuilder {
            sequences: Arc::new(FixedAccountSequences::new(1)),
            rpc,
            allowance: Arc::new(FixedAllowanceChecker { sufficient: true }),
            trustline: Arc::new(FixedUsdcTrustlineProbe { present: true }),
            probe_ok: false,
            base_fee: DEFAULT_BASE_FEE,
        };
        assert!(!StellarCctpBurnBuilder::is_ready(&builder));
        let err = builder
            .prepare_burn(
                &sample_stellar_burn_transfer(None),
                &CctpConfig::default_testnet(),
            )
            .await
            .unwrap_err();
        assert_eq!(err, BuilderError::NotReady);
    }

    #[test]
    fn rejects_contract_recipient_for_mint() {
        let err = ProductionStellarCctpBuilder::validate_mint_recipient(
            "CDNG7HXAPBWICI2E3AUBP3YZWZELJLYSB6F5CC7WLDTLTHVM74SLRTHP",
        )
        .unwrap_err();
        assert!(matches!(err, BuilderError::Validation(_)));
    }

    #[tokio::test]
    #[ignore = "live Stellar Testnet RPC; run with --ignored for configured-ready diagnostics"]
    async fn live_prepare_burn_uses_pinned_burn_fixture_sender() {
        use crate::cctp::fixtures::stellar_live_xdr::burn_envelope_xdr;
        use crate::cctp::stellar_tx::parse_invoke_envelope;

        let mut cfg = CctpConfig::default_testnet();
        cfg.enabled = true;
        cfg.stellar_rpc_url = "https://soroban-testnet.stellar.org".into();
        cfg.sepolia_rpc_url = std::env::var("CCTP_SEPOLIA_RPC_URL")
            .unwrap_or_else(|_| "https://sepolia.drpc.org".into());

        let sender = parse_invoke_envelope(&burn_envelope_xdr())
            .expect("burn fixture")
            .operation_source;
        let builder = ProductionStellarCctpBuilder::try_new(&cfg)
            .await
            .expect("builder try_new");
        assert!(builder.is_production_ready(), "builder must be probe-ready");

        let mut transfer = sample_stellar_burn_transfer(None);
        transfer.sender = sender;
        transfer.amount = "10.0000000".into();
        transfer.destination_amount = "10.0000000".into();

        let bundle = builder
            .prepare_burn(&transfer, &cfg)
            .await
            .unwrap_or_else(|e| panic!("live prepare_burn failed: {e:?}"));
        assert!(
            bundle.approval_required || matches!(bundle.step, BurnPrepareStep::Burn),
            "expected approval or burn payload"
        );
    }
}
