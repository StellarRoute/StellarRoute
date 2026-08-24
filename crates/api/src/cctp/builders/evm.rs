//! EVM Sepolia unsigned CCTP v2 transaction builders.
//!
//! ABI sources: circlefin/evm-cctp-contracts `src/v2/TokenMessengerV2.sol`,
//! `src/v2/MessageTransmitterV2.sol`, OpenZeppelin ERC20 `approve`.

use alloy_primitives::{Address, Bytes, FixedBytes, U256};
use alloy_sol_types::{sol, SolCall};
use async_trait::async_trait;
use chrono::Utc;

use crate::cctp::builders::{
    BuilderError, EvmCctpBurnBuilder, EvmCctpMintBuilder, MintPrepareStep, PreparedBurnBundle,
    PreparedMintBundle,
};
use crate::cctp::config::{corridor_min_finality, CctpConfig};
use crate::cctp::encoding::{
    build_forwarder_hook_data_recipient, decimal_to_cctp_subunits, stellar_contract_to_bytes32,
};
use crate::cctp::store::CctpTransfer;
use crate::models::v2_cctp::{CctpDirection, PreparedWalletPayload, SEPOLIA_CHAIN_ID};

pub const SEPOLIA_CHAIN_ID_NUM: u64 = 11_155_111;

sol! {
    interface IERC20 {
        function approve(address spender, uint256 amount) external returns (bool);
    }

    interface ITokenMessengerV2 {
        function depositForBurn(
            uint256 amount,
            uint32 destinationDomain,
            bytes32 mintRecipient,
            address burnToken,
            bytes32 destinationCaller,
            uint256 maxFee,
            uint32 minFinalityThreshold
        ) external;

        function depositForBurnWithHook(
            uint256 amount,
            uint32 destinationDomain,
            bytes32 mintRecipient,
            address burnToken,
            bytes32 destinationCaller,
            uint256 maxFee,
            uint32 minFinalityThreshold,
            bytes hookData
        ) external;
    }

    interface IMessageTransmitterV2 {
        function receiveMessage(bytes message, bytes attestation) external returns (bool);
    }
}

/// ERC-20 allowance probe for EVM burn prepare gating.
#[async_trait::async_trait]
pub trait EvmAllowanceChecker: Send + Sync {
    async fn has_sufficient_allowance(
        &self,
        owner: &str,
        token: &str,
        spender: &str,
        amount: &str,
    ) -> Result<bool, BuilderError>;
}

pub struct FixedEvmAllowanceChecker {
    pub sufficient: bool,
}

#[async_trait::async_trait]
impl EvmAllowanceChecker for FixedEvmAllowanceChecker {
    async fn has_sufficient_allowance(
        &self,
        _owner: &str,
        _token: &str,
        _spender: &str,
        _amount: &str,
    ) -> Result<bool, BuilderError> {
        Ok(self.sufficient)
    }
}

pub struct ProductionEvmCctpBuilder {
    pub rpc_url: String,
    pub allowance: std::sync::Arc<dyn EvmAllowanceChecker>,
    pub probe_ok: bool,
}

impl ProductionEvmCctpBuilder {
    pub fn new(
        config: &CctpConfig,
        allowance: std::sync::Arc<dyn EvmAllowanceChecker>,
        probe_ok: bool,
    ) -> Self {
        Self {
            rpc_url: config.sepolia_rpc_url.clone(),
            allowance,
            probe_ok,
        }
    }

    pub async fn try_new(config: &CctpConfig) -> Result<Self, BuilderError> {
        if config.sepolia_rpc_url.trim().is_empty() {
            return Err(BuilderError::NotReady);
        }
        let probe = crate::cctp::evm_readiness_probes::probe_sepolia_with_failover(config).await;
        if !probe.all_ok() {
            return Err(BuilderError::NotReady);
        }
        let allowance: std::sync::Arc<dyn EvmAllowanceChecker> =
            match crate::cctp::evm_allowance::EvmRpcAllowanceChecker::new(config) {
                Ok(c) => std::sync::Arc::new(c),
                Err(_) => std::sync::Arc::new(FixedEvmAllowanceChecker { sufficient: false }),
            };
        Ok(Self::new(config, allowance, true))
    }

    pub fn from_config(config: &CctpConfig) -> Self {
        Self::new(
            config,
            std::sync::Arc::new(FixedEvmAllowanceChecker { sufficient: false }),
            false,
        )
    }

    pub(crate) fn is_production_ready(&self) -> bool {
        self.probe_ok
    }

    fn sepolia_ready(config: &CctpConfig) -> bool {
        !config.sepolia_rpc_url.trim().is_empty()
            && config.sepolia_domain == crate::cctp::config::SEPOLIA_DOMAIN
    }

    fn evm_burn_ready(&self) -> bool {
        self.is_production_ready()
    }

    async fn needs_approval(
        &self,
        transfer: &CctpTransfer,
        config: &CctpConfig,
    ) -> Result<bool, BuilderError> {
        Self::ensure_not_expired(transfer)?;
        let sufficient = self
            .allowance
            .has_sufficient_allowance(
                &transfer.sender,
                &config.contracts.sepolia_usdc,
                &config.contracts.sepolia_token_messenger,
                &transfer.amount,
            )
            .await?;
        Ok(!sufficient)
    }

    fn parse_address(addr: &str) -> Result<Address, BuilderError> {
        addr.trim()
            .parse()
            .map_err(|_| BuilderError::Validation(format!("invalid EVM address: {addr}")))
    }

    fn u256_from_decimal(amount: &str) -> Result<U256, BuilderError> {
        let subunits =
            decimal_to_cctp_subunits(amount).map_err(|e| BuilderError::Encoding(e.to_string()))?;
        Ok(U256::from(subunits))
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

    pub fn encode_approve(spender: &str, amount: &str) -> Result<Vec<u8>, BuilderError> {
        let call = IERC20::approveCall {
            spender: Self::parse_address(spender)?,
            amount: Self::u256_from_decimal(amount)?,
        };
        Ok(call.abi_encode())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn encode_deposit_for_burn(
        amount: u128,
        destination_domain: u32,
        mint_recipient: [u8; 32],
        burn_token: &str,
        destination_caller: [u8; 32],
        max_fee: &str,
        min_finality: u32,
    ) -> Result<Vec<u8>, BuilderError> {
        let call = ITokenMessengerV2::depositForBurnCall {
            amount: U256::from(amount),
            destinationDomain: destination_domain,
            mintRecipient: FixedBytes::from(mint_recipient),
            burnToken: Self::parse_address(burn_token)?,
            destinationCaller: FixedBytes::from(destination_caller),
            maxFee: Self::u256_from_decimal(max_fee)?,
            minFinalityThreshold: min_finality,
        };
        Ok(call.abi_encode())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn encode_deposit_for_burn_with_hook(
        amount: u128,
        destination_domain: u32,
        mint_recipient: [u8; 32],
        burn_token: &str,
        destination_caller: [u8; 32],
        max_fee: &str,
        min_finality: u32,
        hook_data: Vec<u8>,
    ) -> Result<Vec<u8>, BuilderError> {
        let call = ITokenMessengerV2::depositForBurnWithHookCall {
            amount: U256::from(amount),
            destinationDomain: destination_domain,
            mintRecipient: FixedBytes::from(mint_recipient),
            burnToken: Self::parse_address(burn_token)?,
            destinationCaller: FixedBytes::from(destination_caller),
            maxFee: Self::u256_from_decimal(max_fee)?,
            minFinalityThreshold: min_finality,
            hookData: Bytes::from(hook_data),
        };
        Ok(call.abi_encode())
    }

    pub fn encode_receive_message(message: &[u8], attestation: &[u8]) -> Vec<u8> {
        let call = IMessageTransmitterV2::receiveMessageCall {
            message: Bytes::copy_from_slice(message),
            attestation: Bytes::copy_from_slice(attestation),
        };
        call.abi_encode()
    }

    fn evm_tx_payload(to: &str, data: Vec<u8>) -> PreparedWalletPayload {
        PreparedWalletPayload::EvmTransaction {
            chain_id: SEPOLIA_CHAIN_ID.into(),
            to: to.to_string(),
            data: format!("0x{}", hex::encode(data)),
            value: "0".into(),
        }
    }
}

impl Default for ProductionEvmCctpBuilder {
    fn default() -> Self {
        Self::from_config(&CctpConfig::default_testnet())
    }
}

/// Share one `ProductionEvmCctpBuilder` across burn + mint trait objects.
#[derive(Clone)]
pub struct SharedProductionEvmBuilder(pub std::sync::Arc<ProductionEvmCctpBuilder>);

#[async_trait]
impl EvmCctpBurnBuilder for SharedProductionEvmBuilder {
    fn is_ready(&self) -> bool {
        self.0.is_production_ready()
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
impl EvmCctpMintBuilder for SharedProductionEvmBuilder {
    fn is_ready(&self) -> bool {
        self.0.is_production_ready()
    }
    async fn prepare_mint(
        &self,
        transfer: &CctpTransfer,
        config: &CctpConfig,
    ) -> Result<PreparedMintBundle, BuilderError> {
        self.0.prepare_mint(transfer, config).await
    }
}

#[async_trait]
impl EvmCctpBurnBuilder for ProductionEvmCctpBuilder {
    fn is_ready(&self) -> bool {
        self.evm_burn_ready()
    }

    async fn prepare_burn(
        &self,
        transfer: &CctpTransfer,
        config: &CctpConfig,
    ) -> Result<PreparedBurnBundle, BuilderError> {
        if !Self::sepolia_ready(config) || !self.evm_burn_ready() {
            return Err(BuilderError::NotReady);
        }
        if transfer.direction != CctpDirection::EvmToStellar {
            return Err(BuilderError::Validation(
                "EVM burn builder only supports evm_to_stellar".into(),
            ));
        }
        Self::ensure_not_expired(transfer)?;
        if transfer.sender.is_empty() {
            return Err(BuilderError::Validation(
                "sender required for EVM burn".into(),
            ));
        }

        let amount = decimal_to_cctp_subunits(&transfer.amount)
            .map_err(|e| BuilderError::Encoding(e.to_string()))?;
        let max_fee = transfer
            .max_fee
            .as_deref()
            .ok_or_else(|| BuilderError::Validation("max_fee missing".into()))?;

        let forwarder = stellar_contract_to_bytes32(&config.contracts.stellar_cctp_forwarder)
            .map_err(|e| BuilderError::Encoding(e.to_string()))?;
        let hook = build_forwarder_hook_data_recipient(&transfer.recipient)
            .map_err(|e| BuilderError::Encoding(e.to_string()))?;

        let expires_at = transfer.quote_expires_at.timestamp();

        if self.needs_approval(transfer, config).await? {
            let approval =
                Self::encode_approve(&config.contracts.sepolia_token_messenger, &transfer.amount)?;
            return Ok(PreparedBurnBundle {
                step: crate::cctp::builders::BurnPrepareStep::Approval,
                approval_required: true,
                primary: Self::evm_tx_payload(&config.contracts.sepolia_usdc, approval),
                required_approvals: vec![],
                required_prior_payloads: vec![],
                expires_at,
                approval_expiration_ledger: None,
            });
        }

        let burn_data = Self::encode_deposit_for_burn_with_hook(
            amount,
            config.stellar_domain,
            forwarder,
            &config.contracts.sepolia_usdc,
            forwarder,
            max_fee,
            corridor_min_finality(transfer.finality),
            hook,
        )?;

        Ok(PreparedBurnBundle {
            step: crate::cctp::builders::BurnPrepareStep::Burn,
            approval_required: false,
            primary: Self::evm_tx_payload(&config.contracts.sepolia_token_messenger, burn_data),
            required_approvals: vec![],
            required_prior_payloads: vec![],
            expires_at,
            approval_expiration_ledger: None,
        })
    }
}

#[async_trait]
impl EvmCctpMintBuilder for ProductionEvmCctpBuilder {
    fn is_ready(&self) -> bool {
        self.evm_burn_ready()
    }

    async fn prepare_mint(
        &self,
        transfer: &CctpTransfer,
        config: &CctpConfig,
    ) -> Result<PreparedMintBundle, BuilderError> {
        if transfer.direction != CctpDirection::StellarToEvm {
            return Err(BuilderError::Validation(
                "EVM mint builder only supports stellar_to_evm destination".into(),
            ));
        }
        let message = transfer
            .raw_message
            .as_ref()
            .ok_or_else(|| BuilderError::Validation("raw_message missing".into()))?;
        let attestation = transfer
            .attestation
            .as_ref()
            .ok_or_else(|| BuilderError::Validation("attestation missing".into()))?;

        let data = Self::encode_receive_message(message, attestation);
        let payload = Self::evm_tx_payload(&config.contracts.sepolia_message_transmitter, data);
        let payload_hash = hash_payload(&payload);
        let expires_at = (Utc::now() + chrono::Duration::minutes(10)).timestamp();

        Ok(PreparedMintBundle {
            step: MintPrepareStep::Mint,
            trustline_required: false,
            primary: payload,
            expires_at,
            payload_hash,
        })
    }
}

pub fn hash_payload(payload: &PreparedWalletPayload) -> String {
    use sha2::{Digest, Sha256};
    let json = serde_json::to_string(payload).unwrap_or_default();
    hex::encode(Sha256::digest(json.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cctp::config::{
        CctpConfig, FINALITY_STANDARD, SEPOLIA_DOMAIN, SEPOLIA_MESSAGE_TRANSMITTER,
        SEPOLIA_TOKEN_MESSENGER, SEPOLIA_USDC, STELLAR_CCTP_FORWARDER,
    };
    use crate::cctp::encoding::evm_address_to_bytes32;
    use crate::cctp::expectations::ANY_DESTINATION_CALLER;

    // Selector from Circle TokenMessengerV2.sol via alloy `sol!` codegen.
    #[test]
    fn golden_deposit_for_burn_selector_and_decode() {
        let mint = evm_address_to_bytes32("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0").unwrap();
        let data = ProductionEvmCctpBuilder::encode_deposit_for_burn(
            1_000_000,
            SEPOLIA_DOMAIN,
            mint,
            SEPOLIA_USDC,
            ANY_DESTINATION_CALLER,
            "1",
            FINALITY_STANDARD,
        )
        .unwrap();
        assert_eq!(&data[0..4], ITokenMessengerV2::depositForBurnCall::SELECTOR);
        let decoded = ITokenMessengerV2::depositForBurnCall::abi_decode(&data, true).unwrap();
        assert_eq!(decoded.amount, U256::from(1_000_000u64));
        assert_eq!(decoded.destinationDomain, SEPOLIA_DOMAIN);
        assert_eq!(decoded.minFinalityThreshold, FINALITY_STANDARD);
    }

    #[test]
    fn golden_approve_selector() {
        let data = ProductionEvmCctpBuilder::encode_approve(SEPOLIA_TOKEN_MESSENGER, "100.000000")
            .unwrap();
        assert_eq!(&data[0..4], IERC20::approveCall::SELECTOR);
    }

    #[test]
    fn golden_receive_message_selector() {
        let data = ProductionEvmCctpBuilder::encode_receive_message(&[1, 2, 3], &[4, 5]);
        assert_eq!(
            &data[0..4],
            IMessageTransmitterV2::receiveMessageCall::SELECTOR
        );
        let decoded = IMessageTransmitterV2::receiveMessageCall::abi_decode(&data, true).unwrap();
        assert_eq!(decoded.message.as_ref(), &[1, 2, 3]);
        assert_eq!(decoded.attestation.as_ref(), &[4, 5]);
    }

    #[test]
    fn deposit_for_burn_with_hook_encodes_forwarder_path() {
        let forwarder = stellar_contract_to_bytes32(STELLAR_CCTP_FORWARDER).unwrap();
        let hook = build_forwarder_hook_data_recipient(
            "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
        )
        .unwrap();
        let data = ProductionEvmCctpBuilder::encode_deposit_for_burn_with_hook(
            100_000_000,
            27,
            forwarder,
            SEPOLIA_USDC,
            forwarder,
            "1",
            FINALITY_STANDARD,
            hook,
        )
        .unwrap();
        assert!(data.len() > 4);
        let decoded =
            ITokenMessengerV2::depositForBurnWithHookCall::abi_decode(&data, true).unwrap();
        assert_eq!(decoded.destinationDomain, 27);
        assert!(!decoded.hookData.is_empty());
    }
}
