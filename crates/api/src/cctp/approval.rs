//! On-chain token approval verification before burn prepare.

use async_trait::async_trait;

use crate::cctp::store::CctpTransfer;
use crate::cctp::verifiers::VerifierError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedApprovalFacts {
    pub tx_hash: String,
    pub owner: String,
    pub token_contract: String,
    pub spender_contract: String,
    pub amount: u128,
    pub chain_id: String,
}

#[async_trait]
pub trait EvmApprovalVerifier: Send + Sync {
    fn is_ready(&self) -> bool;
    async fn verify_approval(
        &self,
        transfer: &CctpTransfer,
        tx_hash: &str,
        required_amount: u128,
    ) -> Result<VerifiedApprovalFacts, VerifierError>;
}

#[async_trait]
pub trait StellarApprovalVerifier: Send + Sync {
    fn is_ready(&self) -> bool;
    async fn verify_approval(
        &self,
        transfer: &CctpTransfer,
        tx_hash: &str,
        required_amount: i128,
    ) -> Result<VerifiedApprovalFacts, VerifierError>;
}

pub struct NotReadyEvmApprovalVerifier;

#[async_trait]
impl EvmApprovalVerifier for NotReadyEvmApprovalVerifier {
    fn is_ready(&self) -> bool {
        false
    }
    async fn verify_approval(
        &self,
        _: &CctpTransfer,
        _: &str,
        _: u128,
    ) -> Result<VerifiedApprovalFacts, VerifierError> {
        Err(VerifierError::NotReady)
    }
}

/// Optional standalone SEP-41 approval — Soroban auth in burn tx may satisfy `transfer_from` without this.
pub struct NotReadyStellarApprovalVerifier;

#[async_trait]
impl StellarApprovalVerifier for NotReadyStellarApprovalVerifier {
    fn is_ready(&self) -> bool {
        false
    }
    async fn verify_approval(
        &self,
        _: &CctpTransfer,
        _: &str,
        _: i128,
    ) -> Result<VerifiedApprovalFacts, VerifierError> {
        Err(VerifierError::NotReady)
    }
}

/// Test double for approval service-path tests.
pub struct FakeApprovalVerifier {
    pub facts: VerifiedApprovalFacts,
    pub ready: bool,
}

#[async_trait]
impl EvmApprovalVerifier for FakeApprovalVerifier {
    fn is_ready(&self) -> bool {
        self.ready
    }
    async fn verify_approval(
        &self,
        _: &CctpTransfer,
        tx_hash: &str,
        required_amount: u128,
    ) -> Result<VerifiedApprovalFacts, VerifierError> {
        if !self.ready {
            return Err(VerifierError::NotReady);
        }
        if self.facts.tx_hash != tx_hash {
            return Err(VerifierError::Failed("tx_hash mismatch".into()));
        }
        if self.facts.amount < required_amount {
            return Err(VerifierError::Failed("insufficient approval amount".into()));
        }
        Ok(self.facts.clone())
    }
}

#[async_trait]
impl StellarApprovalVerifier for FakeApprovalVerifier {
    fn is_ready(&self) -> bool {
        self.ready
    }
    async fn verify_approval(
        &self,
        _: &CctpTransfer,
        tx_hash: &str,
        required_amount: i128,
    ) -> Result<VerifiedApprovalFacts, VerifierError> {
        if !self.ready {
            return Err(VerifierError::NotReady);
        }
        if self.facts.tx_hash != tx_hash {
            return Err(VerifierError::Failed("tx_hash mismatch".into()));
        }
        if self.facts.amount < required_amount as u128 {
            return Err(VerifierError::Failed("insufficient approval amount".into()));
        }
        Ok(self.facts.clone())
    }
}
