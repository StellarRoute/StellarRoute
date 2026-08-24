//! Production Stellar Testnet `deposit_for_burn` verifier via Soroban RPC `getTransaction`.

use async_trait::async_trait;
use std::sync::Arc;

use crate::cctp::config::CctpConfig;
use crate::cctp::encoding::{
    stellar_contract_to_bytes32, stellar_local_to_canonical_amount,
    stellar_local_to_canonical_amount_allow_zero,
};
use crate::cctp::message::parse_cctp_v2_message;
use crate::cctp::stellar_contract_events::{
    address_to_strkey, contract_hash, parse_deposit_for_burn, parse_message_sent,
    DepositForBurnEvent,
};
use crate::cctp::stellar_rpc::StellarRpcClient;
use crate::cctp::stellar_tx::{
    chain_id_string, ensure_testnet_binding, parse_invoke_envelope, scval_to_address,
    scval_to_bytes, scval_to_bytes32, scval_to_i128, scval_to_u32, TxStatus,
};
use crate::cctp::verifiers::{StellarBurnVerifier, VerifiedBurnFacts, VerifierError};

pub struct StellarRpcBurnVerifier {
    rpc: Arc<StellarRpcClient>,
    token_messenger: String,
    message_transmitter: String,
    probe_ok: bool,
}

struct DecodedBurnInvoke {
    caller: String,
    local_amount: i128,
    destination_domain: u32,
    mint_recipient: [u8; 32],
    burn_token: String,
    destination_caller: [u8; 32],
    local_max_fee: i128,
    min_finality: u32,
    hook_data: Vec<u8>,
}

impl StellarRpcBurnVerifier {
    pub async fn new(config: &CctpConfig) -> Result<Self, VerifierError> {
        ensure_testnet_binding(config)?;
        if config.stellar_rpc_url.trim().is_empty() {
            return Err(VerifierError::NotReady);
        }
        let rpc = Arc::new(StellarRpcClient::new(config)?);
        let probe_ok = if cfg!(test) {
            rpc.latest_ledger().await.is_ok()
        } else {
            crate::cctp::stellar_readiness_probes::probe_stellar_contracts(config)
                .await
                .all_ok()
        };
        Ok(Self {
            rpc,
            token_messenger: config.contracts.stellar_token_messenger.clone(),
            message_transmitter: config.contracts.stellar_message_transmitter.clone(),
            probe_ok,
        })
    }

    fn decode_burn_invoke(
        invoke: &crate::cctp::stellar_tx::ParsedInvoke,
    ) -> Result<DecodedBurnInvoke, VerifierError> {
        let with_hook = invoke.function == "deposit_for_burn_with_hook";
        if invoke.function != "deposit_for_burn" && !with_hook {
            return Err(VerifierError::Failed("wrong function".into()));
        }
        let expected_len = if with_hook { 9 } else { 8 };
        if invoke.args.len() != expected_len {
            return Err(VerifierError::Failed("burn arg count".into()));
        }
        let hook_data = if with_hook {
            scval_to_bytes(&invoke.args[8])?
        } else {
            Vec::new()
        };
        Ok(DecodedBurnInvoke {
            caller: address_to_strkey(&scval_to_address(&invoke.args[0])?)?,
            local_amount: scval_to_i128(&invoke.args[1])?,
            destination_domain: scval_to_u32(&invoke.args[2])?,
            mint_recipient: scval_to_bytes32(&invoke.args[3])?,
            burn_token: address_to_strkey(&scval_to_address(&invoke.args[4])?)?,
            destination_caller: scval_to_bytes32(&invoke.args[5])?,
            local_max_fee: scval_to_i128(&invoke.args[6])?,
            min_finality: scval_to_u32(&invoke.args[7])?,
            hook_data,
        })
    }

    fn invoke_matches_event(call: &DecodedBurnInvoke, event: &DepositForBurnEvent) -> bool {
        let canonical_amount =
            stellar_local_to_canonical_amount(call.local_amount).unwrap_or(i128::MIN);
        let canonical_max_fee =
            stellar_local_to_canonical_amount_allow_zero(call.local_max_fee).unwrap_or(i128::MIN);
        address_to_strkey(&event.depositor).ok().as_deref() == Some(call.caller.as_str())
            && event.destination_domain == call.destination_domain
            && event.mint_recipient == call.mint_recipient
            && address_to_strkey(&event.burn_token).ok().as_deref()
                == Some(call.burn_token.as_str())
            && event.destination_caller == call.destination_caller
            && event.min_finality_threshold == call.min_finality
            && event.hook_data == call.hook_data
            && event.amount == canonical_amount
            && event.max_fee == canonical_max_fee
    }

    fn find_burn_events(
        events: &[stellar_xdr::curr::ContractEvent],
        token_messenger_hash: [u8; 32],
        message_transmitter_hash: [u8; 32],
    ) -> Result<(DepositForBurnEvent, Vec<u8>), VerifierError> {
        let mut burn_matches = 0usize;
        let mut burn_event: Option<DepositForBurnEvent> = None;
        let mut message_matches = 0usize;
        let mut message_bytes: Option<Vec<u8>> = None;

        for event in events {
            let hash = contract_hash(event)?;
            if hash == token_messenger_hash {
                if let Ok(parsed) = parse_deposit_for_burn(event) {
                    burn_matches += 1;
                    burn_event = Some(parsed);
                }
            }
            if hash == message_transmitter_hash {
                if let Ok(parsed) = parse_message_sent(event) {
                    message_matches += 1;
                    message_bytes = Some(parsed.message);
                }
            }
        }

        if burn_matches != 1 {
            return Err(VerifierError::Failed(
                "ambiguous deposit_for_burn events".into(),
            ));
        }
        if message_matches != 1 {
            return Err(VerifierError::Failed(
                "ambiguous message_sent events".into(),
            ));
        }
        Ok((
            burn_event.ok_or_else(|| VerifierError::Failed("no burn event".into()))?,
            message_bytes.ok_or_else(|| VerifierError::Failed("no message event".into()))?,
        ))
    }
}

#[async_trait]
impl StellarBurnVerifier for StellarRpcBurnVerifier {
    fn is_ready(&self) -> bool {
        self.probe_ok && self.rpc.is_ready()
    }

    async fn verify_burn(&self, tx_hash: &str) -> Result<VerifiedBurnFacts, VerifierError> {
        if !self.is_ready() {
            return Err(VerifierError::NotReady);
        }
        let tx = self.rpc.get_finalized_transaction(tx_hash).await?;
        if tx.status != TxStatus::Success {
            return Err(VerifierError::Failed("tx failed".into()));
        }

        let invoke = parse_invoke_envelope(&tx.envelope_xdr)?;
        if invoke.contract_strkey != self.token_messenger {
            return Err(VerifierError::Failed("wrong contract".into()));
        }
        let call = Self::decode_burn_invoke(&invoke)?;
        if call.caller != invoke.operation_source {
            return Err(VerifierError::Failed("caller/op-source mismatch".into()));
        }

        let tm_hash = stellar_contract_to_bytes32(&self.token_messenger)
            .map_err(|e| VerifierError::Failed(e.to_string()))?;
        let mt_hash = stellar_contract_to_bytes32(&self.message_transmitter)
            .map_err(|e| VerifierError::Failed(e.to_string()))?;

        let (event, raw_message) = Self::find_burn_events(&tx.contract_events, tm_hash, mt_hash)?;

        if !Self::invoke_matches_event(&call, &event) {
            return Err(VerifierError::Failed("invoke/event mismatch".into()));
        }

        let parsed_msg = parse_cctp_v2_message(&raw_message)
            .map_err(|e| VerifierError::Failed(e.to_string()))?;

        if parsed_msg.body.amount != event.amount as u128 {
            return Err(VerifierError::Failed(
                "message/event amount mismatch".into(),
            ));
        }
        if parsed_msg.body.mint_recipient != event.mint_recipient {
            return Err(VerifierError::Failed(
                "message/event recipient mismatch".into(),
            ));
        }
        if parsed_msg.destination_domain != event.destination_domain {
            return Err(VerifierError::Failed(
                "message/event domain mismatch".into(),
            ));
        }
        if event.destination_token_messenger != parsed_msg.recipient {
            return Err(VerifierError::Failed(
                "destination token messenger mismatch".into(),
            ));
        }

        let canonical_from_invoke = stellar_local_to_canonical_amount(call.local_amount)
            .map_err(|e| VerifierError::Failed(e.to_string()))?;
        if canonical_from_invoke != event.amount {
            return Err(VerifierError::Failed("invoke/event amount mismatch".into()));
        }

        let hook_data = if event.hook_data.is_empty() {
            None
        } else {
            Some(event.hook_data.clone())
        };

        Ok(VerifiedBurnFacts {
            tx_hash: tx.tx_hash,
            source_chain_id: chain_id_string(),
            source_domain: parsed_msg.source_domain,
            destination_domain: event.destination_domain,
            sender: call.caller,
            amount_cctp_subunits: event.amount as u128,
            burn_token_bytes32: stellar_contract_to_bytes32(&call.burn_token)
                .map_err(|e| VerifierError::Failed(e.to_string()))?,
            mint_recipient_bytes32: event.mint_recipient,
            destination_caller_bytes32: event.destination_caller,
            min_finality_threshold: event.min_finality_threshold,
            hook_data,
            token_messenger_bytes32: tm_hash,
            block_or_ledger: Some(tx.ledger.to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cctp::builders::stellar::encoder::{
        contract_address, deposit_for_burn_args, encode_invoke_at_sequence,
    };
    use crate::cctp::config::{CctpConfig, FINALITY_STANDARD, STELLAR_TESTNET_DOMAIN};
    use crate::cctp::encoding::evm_address_to_bytes32;
    use crate::cctp::expectations::ANY_DESTINATION_CALLER;
    use crate::cctp::stellar_contract_events::test_helpers::{
        deposit_for_burn_event, event_to_b64, message_sent_event,
    };
    use stellar_xdr::curr::{ScBytes, ScVal};

    #[tokio::test]
    async fn not_ready_without_rpc() {
        let mut cfg = CctpConfig::default_testnet();
        cfg.stellar_rpc_url = String::new();
        assert!(matches!(
            StellarRpcBurnVerifier::new(&cfg).await,
            Err(VerifierError::NotReady)
        ));
    }

    #[test]
    fn invoke_matches_event_allows_zero_max_fee() {
        let call = DecodedBurnInvoke {
            caller: "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF".into(),
            local_amount: 10_000_000,
            destination_domain: 0,
            mint_recipient: [9u8; 32],
            burn_token: "CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA".into(),
            destination_caller: [0u8; 32],
            local_max_fee: 0,
            min_finality: FINALITY_STANDARD,
            hook_data: vec![],
        };
        let event = DepositForBurnEvent {
            burn_token: contract_address(&call.burn_token).unwrap(),
            depositor: {
                use stellar_xdr::curr::{AccountId, PublicKey, ScAddress, Uint256};
                let pk = stellar_strkey::ed25519::PublicKey::from_string(&call.caller).unwrap();
                ScAddress::Account(AccountId(PublicKey::PublicKeyTypeEd25519(Uint256(pk.0))))
            },
            min_finality_threshold: FINALITY_STANDARD,
            amount: 1_000_000,
            mint_recipient: [9u8; 32],
            destination_domain: 0,
            destination_token_messenger: [1u8; 32],
            destination_caller: [0u8; 32],
            max_fee: 0,
            hook_data: vec![],
        };
        assert!(StellarRpcBurnVerifier::invoke_matches_event(&call, &event));
    }

    #[test]
    fn decode_burn_invoke_from_encoder_args() {
        let cfg = CctpConfig::default_testnet();
        let source = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";
        let mint = evm_address_to_bytes32("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0").unwrap();
        let args = deposit_for_burn_args(
            source,
            10_000_000,
            cfg.sepolia_domain,
            mint,
            &cfg.contracts.stellar_usdc,
            1,
            FINALITY_STANDARD,
        )
        .unwrap();
        let xdr = encode_invoke_at_sequence(
            source,
            &cfg.contracts.stellar_token_messenger,
            "deposit_for_burn",
            args,
            50,
        )
        .unwrap();
        let invoke = parse_invoke_envelope(&xdr).unwrap();
        let decoded = StellarRpcBurnVerifier::decode_burn_invoke(&invoke).unwrap();
        assert_eq!(decoded.destination_domain, cfg.sepolia_domain);
        assert_eq!(decoded.min_finality, FINALITY_STANDARD);
        assert_eq!(decoded.destination_caller, ANY_DESTINATION_CALLER);
    }
}

#[cfg(test)]
mod live_diag {
    use super::*;
    use crate::cctp::config::CctpConfig;
    use crate::cctp::stellar_readiness_probes::probe_stellar_contracts;

    #[tokio::test]
    #[ignore = "live diag"]
    async fn verify_known_burn_tx() {
        let mut cfg = CctpConfig::default_testnet();
        cfg.enabled = true;
        cfg.stellar_rpc_url = "https://soroban-testnet.stellar.org".into();
        cfg.sepolia_rpc_url = "https://sepolia.drpc.org".into();
        let probe = probe_stellar_contracts(&cfg).await;
        eprintln!("probe={probe:?} all_ok={}", probe.all_ok());
        let rpc = StellarRpcClient::new(&cfg).expect("rpc");
        eprintln!(
            "rpc_ready={} latest={:?}",
            rpc.is_ready(),
            rpc.latest_ledger().await
        );
        let mut v = StellarRpcBurnVerifier::new(&cfg).await.expect("verifier");
        eprintln!(
            "constructed probe_ok={} is_ready={}",
            v.probe_ok,
            v.is_ready()
        );
        // Bypass probe for diagnosis
        v.probe_ok = true;
        let hash = std::env::var("CCTP_DIAG_BURN_HASH").unwrap_or_else(|_| {
            "951714911af3ea05180704e365dbb6e98b93dbcd56a72dffdd9920fa0af5abde".into()
        });
        match v.verify_burn(&hash).await {
            Ok(facts) => {
                eprintln!(
                    "OK amount={} sender={} domains={}->{} recipient={:?}",
                    facts.amount_cctp_subunits,
                    facts.sender,
                    facts.source_domain,
                    facts.destination_domain,
                    hex::encode(facts.mint_recipient_bytes32)
                );
            }
            Err(e) => panic!("verify failed: {e:?}"),
        }
    }
}
