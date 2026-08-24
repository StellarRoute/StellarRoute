//! Production Sepolia ERC-20 approval transaction verifier.

use alloy_primitives::Address;
use alloy_sol_types::{sol, SolCall};
use async_trait::async_trait;
use serde::Deserialize;

use crate::cctp::approval::{EvmApprovalVerifier, VerifiedApprovalFacts};
use crate::cctp::config::CctpConfig;
use crate::cctp::evm_rpc::EvmRpcClient;
use crate::cctp::store::CctpTransfer;
use crate::cctp::verifiers::VerifierError;
use crate::models::v2_cctp::SEPOLIA_CHAIN_ID;

sol! {
    interface IERC20Approve {
        function approve(address spender, uint256 amount) external returns (bool);
    }
}

const DEFAULT_MIN_CONFIRMATIONS: u64 = 1;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EthTransaction {
    from: Option<String>,
    to: Option<String>,
    input: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EthReceipt {
    status: Option<String>,
    block_number: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EthBlockNumber(String);

pub struct EvmRpcApprovalVerifier {
    rpc: EvmRpcClient,
    usdc: Address,
    token_messenger: Address,
    min_confirmations: u64,
    probe_ok: bool,
}

impl EvmRpcApprovalVerifier {
    pub fn new(config: &CctpConfig) -> Result<Self, VerifierError> {
        Self::with_confirmations(config, DEFAULT_MIN_CONFIRMATIONS)
    }

    pub fn with_confirmations(
        config: &CctpConfig,
        min_confirmations: u64,
    ) -> Result<Self, VerifierError> {
        let rpc = EvmRpcClient::new(&config.sepolia_rpc_url)?;
        let usdc = config
            .contracts
            .sepolia_usdc
            .trim()
            .parse()
            .map_err(|_| VerifierError::Failed("usdc address".into()))?;
        let token_messenger = config
            .contracts
            .sepolia_token_messenger
            .trim()
            .parse()
            .map_err(|_| VerifierError::Failed("token messenger address".into()))?;
        Ok(Self {
            rpc,
            usdc,
            token_messenger,
            min_confirmations,
            probe_ok: false,
        })
    }

    pub async fn try_new(config: &CctpConfig) -> Result<Self, VerifierError> {
        let probe = crate::cctp::evm_readiness_probes::probe_sepolia_with_failover(config).await;
        if !probe.all_ok() {
            return Err(VerifierError::NotReady);
        }
        let mut verifier = Self::with_confirmations(config, DEFAULT_MIN_CONFIRMATIONS)?;
        verifier.probe_ok = true;
        Ok(verifier)
    }

    async fn confirmations_ok(&self, receipt: &EthReceipt) -> Result<(), VerifierError> {
        let latest: EthBlockNumber = self
            .rpc
            .call("eth_blockNumber", serde_json::json!([]))
            .await?;
        let latest_num = u64::from_str_radix(latest.0.trim_start_matches("0x"), 16)
            .map_err(|_| VerifierError::Failed("block parse".into()))?;
        let tx_block = receipt
            .block_number
            .as_deref()
            .ok_or_else(|| VerifierError::Failed("pending".into()))?;
        let tx_num = u64::from_str_radix(tx_block.trim_start_matches("0x"), 16)
            .map_err(|_| VerifierError::Failed("tx block parse".into()))?;
        if latest_num.saturating_sub(tx_num) + 1 < self.min_confirmations {
            return Err(VerifierError::Failed("insufficient confirmations".into()));
        }
        Ok(())
    }
}

#[async_trait]
impl EvmApprovalVerifier for EvmRpcApprovalVerifier {
    fn is_ready(&self) -> bool {
        self.probe_ok
    }

    async fn verify_approval(
        &self,
        transfer: &CctpTransfer,
        tx_hash: &str,
        required_amount: u128,
    ) -> Result<VerifiedApprovalFacts, VerifierError> {
        if !self.is_ready() {
            return Err(VerifierError::NotReady);
        }
        self.rpc.ensure_chain().await?;
        let hash = EvmRpcClient::normalize_hash(tx_hash);
        let tx: EthTransaction = self
            .rpc
            .call("eth_getTransactionByHash", serde_json::json!([hash]))
            .await?;
        let receipt: EthReceipt = self
            .rpc
            .call("eth_getTransactionReceipt", serde_json::json!([hash]))
            .await?;

        if receipt.status.as_deref() != Some("0x1") {
            return Err(VerifierError::Failed("tx failed".into()));
        }
        self.confirmations_ok(&receipt).await?;

        let from = tx
            .from
            .as_deref()
            .ok_or_else(|| VerifierError::Failed("missing from".into()))?;
        if !from.eq_ignore_ascii_case(&transfer.sender) {
            return Err(VerifierError::Failed("wrong sender".into()));
        }
        let to = tx
            .to
            .as_deref()
            .ok_or_else(|| VerifierError::Failed("missing to".into()))?
            .to_ascii_lowercase();
        if to != format!("{:#x}", self.usdc).to_ascii_lowercase() {
            return Err(VerifierError::Failed("wrong token contract".into()));
        }

        let input = tx
            .input
            .as_deref()
            .ok_or_else(|| VerifierError::Failed("missing input".into()))?;
        let bytes = hex::decode(input.trim_start_matches("0x"))
            .map_err(|_| VerifierError::Failed("calldata hex".into()))?;
        let call = IERC20Approve::approveCall::abi_decode(&bytes, true)
            .map_err(|e| VerifierError::Failed(e.to_string()))?;
        if call.spender != self.token_messenger {
            return Err(VerifierError::Failed("wrong spender".into()));
        }
        let approved: u128 = call
            .amount
            .try_into()
            .map_err(|_| VerifierError::Failed("amount overflow".into()))?;
        if approved < required_amount {
            return Err(VerifierError::Failed("insufficient approval amount".into()));
        }

        Ok(VerifiedApprovalFacts {
            tx_hash: hash,
            owner: transfer.sender.clone(),
            token_contract: format!("{:#x}", self.usdc),
            spender_contract: format!("{:#x}", self.token_messenger),
            amount: approved,
            chain_id: SEPOLIA_CHAIN_ID.into(),
        })
    }
}
