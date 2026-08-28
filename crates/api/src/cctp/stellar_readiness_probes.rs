//! Non-mutating Stellar CCTP contract readiness probes beyond RPC reachability.

use std::sync::Arc;

use stellar_xdr::curr::ScVal;

use crate::cctp::config::{CctpConfig, STELLAR_USDC_DECIMALS};
use crate::cctp::stellar_contract_events::{address_to_strkey, scval_to_u32};
use crate::cctp::stellar_rpc::{scval_to_bool, StellarRpcClient};
use crate::cctp::verifiers::VerifierError;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StellarContractProbeResult {
    pub rpc_ok: bool,
    pub message_transmitter_ok: bool,
    pub forwarder_ok: bool,
    pub token_messenger_ok: bool,
    pub usdc_ok: bool,
}

impl StellarContractProbeResult {
    pub fn all_ok(&self) -> bool {
        self.rpc_ok
            && self.message_transmitter_ok
            && self.forwarder_ok
            && self.token_messenger_ok
            && self.usdc_ok
    }
}

pub async fn probe_stellar_contracts(config: &CctpConfig) -> StellarContractProbeResult {
    let mut out = StellarContractProbeResult::default();
    let Ok(rpc) = StellarRpcClient::new(config) else {
        return out;
    };
    let rpc = Arc::new(rpc);
    out.rpc_ok = rpc.latest_ledger().await.is_ok();
    if !out.rpc_ok {
        return out;
    }

    out.message_transmitter_ok = probe_message_transmitter(&rpc, config).await.is_ok();
    out.forwarder_ok = probe_forwarder(&rpc, config).await.is_ok();
    out.token_messenger_ok = probe_token_messenger(&rpc, config).await.is_ok();
    out.usdc_ok = probe_usdc_decimals(&rpc, config).await.is_ok();

    out
}

async fn probe_message_transmitter(
    rpc: &StellarRpcClient,
    config: &CctpConfig,
) -> Result<(), VerifierError> {
    let zero_nonce = [0u8; 32];
    let _used = rpc
        .simulate_is_nonce_used(&config.contracts.stellar_message_transmitter, zero_nonce)
        .await?;
    let paused = rpc
        .simulate_scval(
            &config.contracts.stellar_message_transmitter,
            "paused",
            vec![],
        )
        .await?;
    ensure_not_paused(&paused)?;
    Ok(())
}

async fn probe_forwarder(rpc: &StellarRpcClient, config: &CctpConfig) -> Result<(), VerifierError> {
    let val = rpc
        .simulate_scval(
            &config.contracts.stellar_cctp_forwarder,
            "get_message_transmitter",
            vec![],
        )
        .await?;
    let ScVal::Address(addr) = val else {
        return Err(VerifierError::Failed("forwarder probe return type".into()));
    };
    let decoded = address_to_strkey(&addr)?;
    if decoded != config.contracts.stellar_message_transmitter {
        return Err(VerifierError::Failed(
            "forwarder message transmitter mismatch".into(),
        ));
    }
    Ok(())
}

async fn probe_token_messenger(
    rpc: &StellarRpcClient,
    config: &CctpConfig,
) -> Result<(), VerifierError> {
    let paused = rpc
        .simulate_scval(&config.contracts.stellar_token_messenger, "paused", vec![])
        .await?;
    ensure_not_paused(&paused)
}

async fn probe_usdc_decimals(
    rpc: &StellarRpcClient,
    config: &CctpConfig,
) -> Result<(), VerifierError> {
    let val = rpc
        .simulate_scval(&config.contracts.stellar_usdc, "decimals", vec![])
        .await?;
    let decimals = scval_to_u32(&val)?;
    if decimals != STELLAR_USDC_DECIMALS {
        return Err(VerifierError::Failed("usdc decimals mismatch".into()));
    }
    Ok(())
}

fn ensure_not_paused(val: &ScVal) -> Result<(), VerifierError> {
    if scval_to_bool(val)? {
        return Err(VerifierError::Failed("contract paused".into()));
    }
    Ok(())
}

#[cfg(test)]
mod probe_tests {
    use super::*;
    use crate::cctp::stellar_rpc::{scval_to_bool, u32_scval};

    #[test]
    fn rejects_paused_contract() {
        assert!(ensure_not_paused(&ScVal::Bool(true)).is_err());
        ensure_not_paused(&ScVal::Bool(false)).unwrap();
    }

    #[test]
    fn rejects_malformed_paused_response() {
        assert!(ensure_not_paused(&ScVal::U32(0)).is_err());
    }

    #[test]
    fn rejects_wrong_usdc_decimals() {
        let err = match scval_to_u32(&u32_scval(6)) {
            Ok(v) if v != STELLAR_USDC_DECIMALS => {
                VerifierError::Failed("usdc decimals mismatch".into())
            }
            _ => VerifierError::Failed("unexpected".into()),
        };
        assert!(matches!(err, VerifierError::Failed(ref m) if m.contains("decimals")));
    }

    #[test]
    fn strict_bool_decode() {
        assert!(scval_to_bool(&ScVal::U32(1)).is_err());
        assert_eq!(scval_to_bool(&ScVal::Bool(true)).unwrap(), true);
    }

    #[tokio::test]
    #[ignore = "diagnostic — live Stellar RPC contract probe fields"]
    async fn live_probe_fields() {
        let cfg = CctpConfig::default_testnet();
        let out = probe_stellar_contracts(&cfg).await;
        eprintln!("{out:?}");
        assert!(out.all_ok());
    }
}

pub async fn simulate_allowance(
    rpc: &StellarRpcClient,
    token: &str,
    owner: &str,
    spender: &str,
) -> Result<i128, VerifierError> {
    use crate::cctp::builders::stellar::encoder::{account_address, contract_address};
    let owner_addr = account_address(owner).map_err(|e| VerifierError::Failed(e.to_string()))?;
    let spender_addr =
        contract_address(spender).map_err(|e| VerifierError::Failed(e.to_string()))?;
    let val = rpc
        .simulate_scval(
            token,
            "allowance",
            vec![ScVal::Address(owner_addr), ScVal::Address(spender_addr)],
        )
        .await?;
    crate::cctp::stellar_contract_events::scval_to_i128(&val)
}
