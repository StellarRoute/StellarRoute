//! Unsigned CCTP wallet transaction builders (no signing/broadcast).

pub mod evm;
pub mod stellar;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::cctp::config::CctpConfig;
use crate::cctp::store::CctpTransfer;
use crate::models::v2_cctp::PreparedWalletPayload;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BuilderError {
    #[error("not ready")]
    NotReady,
    #[error("quote expired")]
    QuoteExpired,
    #[error("fee quote expired")]
    FeeExpired,
    #[error("validation: {0}")]
    Validation(String),
    #[error("simulation failed: {0}")]
    SimulationFailed(String),
    #[error("encoding: {0}")]
    Encoding(String),
    #[error("account lookup: {0}")]
    AccountLookup(String),
}

/// Ordered burn prepare step — approval and burn are never co-returned with duplicate sequences.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BurnPrepareStep {
    Approval,
    Burn,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedBurnBundle {
    pub step: BurnPrepareStep,
    /// True when `step == Approval` and wallet must submit approval before requesting burn payload.
    pub approval_required: bool,
    pub primary: PreparedWalletPayload,
    /// Deprecated: use `step` + `primary` instead of bundling multiple signed envelopes.
    pub required_approvals: Vec<PreparedWalletPayload>,
    pub required_prior_payloads: Vec<PreparedWalletPayload>,
    pub expires_at: i64,
    /// Set when `step == Approval` — persisted for freshness checks.
    pub approval_expiration_ledger: Option<u32>,
}

/// Ordered mint prepare step — trustline (EVM→Stellar) before `mint_and_forward`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MintPrepareStep {
    Trustline,
    Mint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedMintBundle {
    #[serde(default = "default_mint_prepare_step")]
    pub step: MintPrepareStep,
    /// True when `step == Trustline` and wallet must submit ChangeTrust before mint.
    #[serde(default)]
    pub trustline_required: bool,
    pub primary: PreparedWalletPayload,
    pub expires_at: i64,
    pub payload_hash: String,
}

fn default_mint_prepare_step() -> MintPrepareStep {
    MintPrepareStep::Mint
}

#[async_trait]
pub trait StellarCctpBurnBuilder: Send + Sync {
    fn is_ready(&self) -> bool;
    async fn prepare_burn(
        &self,
        transfer: &CctpTransfer,
        config: &CctpConfig,
    ) -> Result<PreparedBurnBundle, BuilderError>;
}

#[async_trait]
pub trait EvmCctpBurnBuilder: Send + Sync {
    fn is_ready(&self) -> bool;
    async fn prepare_burn(
        &self,
        transfer: &CctpTransfer,
        config: &CctpConfig,
    ) -> Result<PreparedBurnBundle, BuilderError>;
}

#[async_trait]
pub trait StellarCctpMintBuilder: Send + Sync {
    fn is_ready(&self) -> bool;
    async fn prepare_mint(
        &self,
        transfer: &CctpTransfer,
        config: &CctpConfig,
    ) -> Result<PreparedMintBundle, BuilderError>;
}

#[async_trait]
pub trait EvmCctpMintBuilder: Send + Sync {
    fn is_ready(&self) -> bool;
    async fn prepare_mint(
        &self,
        transfer: &CctpTransfer,
        config: &CctpConfig,
    ) -> Result<PreparedMintBundle, BuilderError>;
}

pub struct NotReadyStellarBurnBuilder;
#[async_trait]
impl StellarCctpBurnBuilder for NotReadyStellarBurnBuilder {
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

pub struct NotReadyEvmBurnBuilder;
#[async_trait]
impl EvmCctpBurnBuilder for NotReadyEvmBurnBuilder {
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

pub struct NotReadyStellarMintBuilder;
#[async_trait]
impl StellarCctpMintBuilder for NotReadyStellarMintBuilder {
    fn is_ready(&self) -> bool {
        false
    }
    async fn prepare_mint(
        &self,
        _: &CctpTransfer,
        _: &CctpConfig,
    ) -> Result<PreparedMintBundle, BuilderError> {
        Err(BuilderError::NotReady)
    }
}

pub struct NotReadyEvmMintBuilder;
#[async_trait]
impl EvmCctpMintBuilder for NotReadyEvmMintBuilder {
    fn is_ready(&self) -> bool {
        false
    }
    async fn prepare_mint(
        &self,
        _: &CctpTransfer,
        _: &CctpConfig,
    ) -> Result<PreparedMintBundle, BuilderError> {
        Err(BuilderError::NotReady)
    }
}
