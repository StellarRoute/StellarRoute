//! Production Stellar Testnet `mint_and_forward` verifier via Soroban RPC `getTransaction`.

use async_trait::async_trait;
use sha2::Digest;
use std::sync::Arc;

use crate::cctp::config::{corridor_min_finality, CctpConfig};
use crate::cctp::encoding::{canonical_to_stellar_local_amount, stellar_contract_to_bytes32};
use crate::cctp::evm_mint_verifier::EvmRpcMintVerifier;
use crate::cctp::message::{parse_cctp_v2_message, MESSAGE_HEADER_LEN};
use crate::cctp::stellar_contract_events::{
    contract_hash, parse_message_received, parse_mint_and_forward, MessageReceivedEvent,
    MintAndForwardEvent,
};
use crate::cctp::stellar_payload::payload_hash_from_envelope_xdr;
use crate::cctp::stellar_readiness_probes::{probe_stellar_contracts, StellarContractProbeResult};
use crate::cctp::stellar_rpc::StellarRpcClient;
use crate::cctp::stellar_tx::{
    ensure_testnet_binding, parse_invoke_envelope, scval_to_bytes, FinalizedTx, TxStatus,
};
use crate::cctp::verifiers::{
    MintVerifyOutcome, StellarMintVerifier, VerifiedMintFacts, VerifierError,
};
use crate::models::v2_cctp::{CctpFinality, STELLAR_TESTNET_CHAIN_ID};

pub struct StellarRpcMintVerifier {
    rpc: Arc<StellarRpcClient>,
    forwarder: String,
    message_transmitter: String,
    usdc: String,
    config: CctpConfig,
    probe_ok: bool,
}

struct BoundMintEvents {
    forward: MintAndForwardEvent,
    received: MessageReceivedEvent,
}

impl StellarRpcMintVerifier {
    /// Shared readiness decision for mint verifier construction (test + production).
    pub fn contract_probe_ready(probe: &StellarContractProbeResult) -> bool {
        probe.all_ok()
    }

    pub async fn evaluate_contract_probe(config: &CctpConfig) -> bool {
        Self::contract_probe_ready(&probe_stellar_contracts(config).await)
    }

    pub async fn new(config: &CctpConfig) -> Result<Self, VerifierError> {
        ensure_testnet_binding(config)?;
        if config.stellar_rpc_url.trim().is_empty() {
            return Err(VerifierError::NotReady);
        }
        let rpc = Arc::new(StellarRpcClient::new(config)?);
        let probe_ok = Self::evaluate_contract_probe(config).await;
        Ok(Self {
            rpc,
            forwarder: config.contracts.stellar_cctp_forwarder.clone(),
            message_transmitter: config.contracts.stellar_message_transmitter.clone(),
            usdc: config.contracts.stellar_usdc.clone(),
            config: config.clone(),
            probe_ok,
        })
    }

    fn hash32(data: &[u8]) -> [u8; 32] {
        sha2::Sha256::digest(data).into()
    }

    fn decode_mint_invoke(
        invoke: &crate::cctp::stellar_tx::ParsedInvoke,
    ) -> Result<(Vec<u8>, Vec<u8>), VerifierError> {
        if invoke.function != "mint_and_forward" {
            return Err(VerifierError::Failed("wrong function".into()));
        }
        if invoke.args.len() != 2 {
            return Err(VerifierError::Failed("mint arg count".into()));
        }
        Ok((
            scval_to_bytes(&invoke.args[0])?,
            scval_to_bytes(&invoke.args[1])?,
        ))
    }

    fn find_bound_mint_events(
        tx: &FinalizedTx,
        expected_nonce: [u8; 32],
        forwarder_hash: [u8; 32],
        mt_hash: [u8; 32],
        forwarder_strkey: &str,
    ) -> Result<Option<BoundMintEvents>, VerifierError> {
        let mut forward_matches = 0usize;
        let mut forward_event: Option<MintAndForwardEvent> = None;
        let mut received_matches = 0usize;
        let mut received_event: Option<MessageReceivedEvent> = None;

        for event in &tx.contract_events {
            let hash = contract_hash(event)?;
            if hash == forwarder_hash {
                if let Ok(ev) = parse_mint_and_forward(event) {
                    forward_matches += 1;
                    forward_event = Some(ev);
                }
            }
            if hash == mt_hash {
                if let Ok(ev) = parse_message_received(event) {
                    if ev.nonce == expected_nonce {
                        received_matches += 1;
                        received_event = Some(ev);
                    }
                }
            }
        }

        if forward_matches == 0 && received_matches == 0 {
            return Ok(None);
        }
        if forward_matches != 1 || received_matches != 1 {
            return Err(VerifierError::Failed("ambiguous mint events".into()));
        }

        let forward =
            forward_event.ok_or_else(|| VerifierError::Failed("no forward event".into()))?;
        let received =
            received_event.ok_or_else(|| VerifierError::Failed("no message_received".into()))?;

        let caller = crate::cctp::stellar_contract_events::address_to_strkey(&received.caller)?;
        if caller != forwarder_strkey {
            return Err(VerifierError::Failed("caller not forwarder".into()));
        }

        Ok(Some(BoundMintEvents { forward, received }))
    }

    fn bind_completion_evidence(
        &self,
        message: &[u8],
        nonce: &str,
        recipient: &str,
        expected_amount_cctp: u128,
        quoted_finality: CctpFinality,
        events: &BoundMintEvents,
    ) -> Result<(), VerifierError> {
        let expected_nonce = EvmRpcMintVerifier::parse_stored_nonce(nonce)?;
        if events.received.nonce != expected_nonce {
            return Err(VerifierError::Failed("nonce mismatch".into()));
        }
        if events.received.message_body.len() + MESSAGE_HEADER_LEN != message.len() {
            return Err(VerifierError::Failed("message length mismatch".into()));
        }
        if message[MESSAGE_HEADER_LEN..] != events.received.message_body {
            return Err(VerifierError::Failed("message_body mismatch".into()));
        }

        let parsed =
            parse_cctp_v2_message(message).map_err(|e| VerifierError::Failed(e.to_string()))?;
        if parsed.nonce != expected_nonce {
            return Err(VerifierError::Failed("parsed nonce mismatch".into()));
        }
        if events.received.source_domain != parsed.source_domain {
            return Err(VerifierError::Failed("source domain mismatch".into()));
        }
        if events.received.sender != parsed.sender {
            return Err(VerifierError::Failed("sender mismatch".into()));
        }
        if events.received.finality_threshold_executed != parsed.finality_threshold_executed {
            return Err(VerifierError::Failed("executed finality mismatch".into()));
        }
        let expected_min = corridor_min_finality(quoted_finality);
        if parsed.min_finality_threshold != expected_min {
            return Err(VerifierError::Failed("min finality policy mismatch".into()));
        }
        if parsed.finality_threshold_executed < parsed.min_finality_threshold {
            return Err(VerifierError::Failed("finality below minimum".into()));
        }

        // Forwarder delivers burn amount minus fee_executed (Iris Fast fees are non-zero).
        let net_cctp = expected_amount_cctp.saturating_sub(parsed.body.fee_executed);
        let expected_forward = canonical_to_stellar_local_amount(net_cctp as i128)
            .map_err(|e| VerifierError::Failed(e.to_string()))?;
        if events.forward.amount != expected_forward {
            return Err(VerifierError::Failed("forward amount mismatch".into()));
        }

        let usdc_addr = stellar_contract_to_bytes32(&self.usdc)
            .map_err(|e| VerifierError::Failed(e.to_string()))?;
        let token_addr =
            crate::cctp::stellar_contract_events::address_to_strkey(&events.forward.token)?;
        if token_addr != self.usdc {
            return Err(VerifierError::Failed("forward token mismatch".into()));
        }
        let _ = usdc_addr;

        crate::cctp::stellar_muxed::stellar_recipients_match(
            recipient,
            &events.forward.forward_recipient,
        )?;

        Ok(())
    }

    async fn completion_outcome(
        &self,
        tx_hash: &str,
        message: &[u8],
        nonce: &str,
        recipient: &str,
        expected_amount_cctp: u128,
        quoted_finality: CctpFinality,
    ) -> Result<MintVerifyOutcome, VerifierError> {
        let expected_nonce = EvmRpcMintVerifier::parse_stored_nonce(nonce)?;
        let tx = self.rpc.get_finalized_transaction(tx_hash).await?;
        if tx.status == TxStatus::Failed {
            return Ok(MintVerifyOutcome::FailedRetryable {
                reason: "tx failed".into(),
            });
        }
        if tx.status != TxStatus::Success {
            return Ok(MintVerifyOutcome::Pending);
        }

        let mt_hash = stellar_contract_to_bytes32(&self.message_transmitter)
            .map_err(|e| VerifierError::Failed(e.to_string()))?;
        let fwd_hash = stellar_contract_to_bytes32(&self.forwarder)
            .map_err(|e| VerifierError::Failed(e.to_string()))?;

        match Self::find_bound_mint_events(&tx, expected_nonce, fwd_hash, mt_hash, &self.forwarder)?
        {
            Some(events) => {
                self.bind_completion_evidence(
                    message,
                    nonce,
                    recipient,
                    expected_amount_cctp,
                    quoted_finality,
                    &events,
                )?;
                Ok(MintVerifyOutcome::Succeeded)
            }
            None => {
                if self
                    .rpc
                    .simulate_is_nonce_used(&self.message_transmitter, expected_nonce)
                    .await?
                {
                    Ok(MintVerifyOutcome::ReconciliationNonceConsumed)
                } else {
                    Ok(MintVerifyOutcome::Pending)
                }
            }
        }
    }
}

#[async_trait]
impl StellarMintVerifier for StellarRpcMintVerifier {
    fn is_ready(&self) -> bool {
        self.probe_ok && self.rpc.is_ready()
    }

    async fn verify_mint_submission(
        &self,
        tx_hash: &str,
        message: &[u8],
        attestation: &[u8],
        nonce: &str,
        expected_payload_hash: &str,
        expected_mint_submitter: Option<&str>,
    ) -> Result<VerifiedMintFacts, VerifierError> {
        if !self.is_ready() {
            return Err(VerifierError::NotReady);
        }

        let tx = self.rpc.get_finalized_transaction(tx_hash).await?;
        if tx.status == TxStatus::Failed {
            return Ok(VerifiedMintFacts {
                tx_hash: tx.tx_hash,
                destination_chain_id: STELLAR_TESTNET_CHAIN_ID.into(),
                contract_address: self.forwarder.clone(),
                function_selector: "mint_and_forward".into(),
                message_hash: Self::hash32(message),
                attestation_hash: Self::hash32(attestation),
                nonce: nonce.to_string(),
                payload_hash: expected_payload_hash.to_string(),
                outcome: MintVerifyOutcome::FailedRetryable {
                    reason: "tx failed".into(),
                },
                recipient_evidence: None,
            });
        }

        let invoke = parse_invoke_envelope(&tx.envelope_xdr)?;
        if let Some(expected) = expected_mint_submitter {
            if invoke.operation_source != expected {
                return Err(VerifierError::Failed("mint submitter mismatch".into()));
            }
        }
        if invoke.contract_strkey != self.forwarder {
            return Err(VerifierError::Failed("wrong contract".into()));
        }
        let (tx_message, tx_attestation) = Self::decode_mint_invoke(&invoke)?;
        if tx_message != message || tx_attestation != attestation {
            return Err(VerifierError::Failed("message/attestation mismatch".into()));
        }

        let computed_hash = payload_hash_from_envelope_xdr(&tx.envelope_xdr, &self.config)?;
        if computed_hash != expected_payload_hash {
            return Err(VerifierError::Failed("payload hash mismatch".into()));
        }

        let outcome = MintVerifyOutcome::Pending;

        Ok(VerifiedMintFacts {
            tx_hash: tx.tx_hash,
            destination_chain_id: STELLAR_TESTNET_CHAIN_ID.into(),
            contract_address: self.forwarder.clone(),
            function_selector: "mint_and_forward".into(),
            message_hash: Self::hash32(message),
            attestation_hash: Self::hash32(attestation),
            nonce: nonce.to_string(),
            payload_hash: expected_payload_hash.to_string(),
            outcome,
            recipient_evidence: Some(invoke.operation_source.clone()),
        })
    }

    async fn verify_mint_completion(
        &self,
        tx_hash: &str,
        message: &[u8],
        nonce: &str,
        recipient: &str,
        quoted_finality: CctpFinality,
    ) -> Result<MintVerifyOutcome, VerifierError> {
        if !self.is_ready() {
            return Err(VerifierError::NotReady);
        }
        let amount_cctp = parse_cctp_v2_message(message)
            .map_err(|e| VerifierError::Failed(e.to_string()))?
            .body
            .amount;
        self.completion_outcome(
            tx_hash,
            message,
            nonce,
            recipient,
            amount_cctp,
            quoted_finality,
        )
        .await
    }
}

#[cfg(test)]
impl StellarRpcMintVerifier {
    fn for_binding_tests(cfg: &CctpConfig) -> Self {
        Self {
            rpc: Arc::new(StellarRpcClient::new(cfg).expect("test rpc client")),
            forwarder: cfg.contracts.stellar_cctp_forwarder.clone(),
            message_transmitter: cfg.contracts.stellar_message_transmitter.clone(),
            usdc: cfg.contracts.stellar_usdc.clone(),
            config: cfg.clone(),
            probe_ok: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cctp::config::CctpConfig;
    use crate::cctp::encoding::stellar_contract_to_bytes32;
    use crate::cctp::fixtures::stellar_live_xdr::{
        mint_contract_events_xdr, mint_envelope_xdr, mint_fixture_json, MINT_LEDGER,
        MINT_LOCAL_AMOUNT, MINT_TX_HASH,
    };
    use crate::cctp::stellar_contract_events::{collect_contract_events, contract_hash};

    fn mint_finalized_tx() -> FinalizedTx {
        FinalizedTx {
            tx_hash: MINT_TX_HASH.into(),
            status: TxStatus::Success,
            ledger: MINT_LEDGER,
            latest_ledger: Some(MINT_LEDGER),
            created_at: None,
            envelope_xdr: mint_envelope_xdr(),
            contract_events: collect_contract_events(&[mint_contract_events_xdr()]).unwrap(),
        }
    }

    fn mint_binding_context() -> (CctpConfig, Vec<u8>, String, String, u128, BoundMintEvents) {
        let cfg = CctpConfig::default_testnet();
        let invoke = parse_invoke_envelope(&mint_envelope_xdr()).unwrap();
        let message = scval_to_bytes(&invoke.args[0]).unwrap();
        let parsed = parse_cctp_v2_message(&message).unwrap();
        let nonce = format!("0x{}", hex::encode(parsed.nonce));
        let tx = mint_finalized_tx();
        let fwd_hash = stellar_contract_to_bytes32(&cfg.contracts.stellar_cctp_forwarder).unwrap();
        let mt_hash =
            stellar_contract_to_bytes32(&cfg.contracts.stellar_message_transmitter).unwrap();
        let events = StellarRpcMintVerifier::find_bound_mint_events(
            &tx,
            parsed.nonce,
            fwd_hash,
            mt_hash,
            &cfg.contracts.stellar_cctp_forwarder,
        )
        .unwrap()
        .expect("dual events");
        let recipient = events.forward.forward_recipient.clone();
        let amount = parsed.body.amount;
        (cfg, message, nonce, recipient, amount, events)
    }

    #[tokio::test]
    async fn not_ready_without_rpc() {
        let mut cfg = CctpConfig::default_testnet();
        cfg.stellar_rpc_url = String::new();
        assert!(matches!(
            StellarRpcMintVerifier::new(&cfg).await,
            Err(VerifierError::NotReady)
        ));
    }

    #[test]
    fn contract_probe_ready_matches_semantic_flags() {
        let partial = StellarContractProbeResult {
            rpc_ok: true,
            message_transmitter_ok: true,
            forwarder_ok: false,
            token_messenger_ok: true,
            usdc_ok: true,
        };
        assert!(!StellarRpcMintVerifier::contract_probe_ready(&partial));
        let ready = StellarContractProbeResult {
            rpc_ok: true,
            message_transmitter_ok: true,
            forwarder_ok: true,
            token_messenger_ok: true,
            usdc_ok: true,
        };
        assert!(StellarRpcMintVerifier::contract_probe_ready(&ready));
    }

    #[test]
    fn payload_hash_uses_shared_helper() {
        let cfg = CctpConfig::default_testnet();
        let xdr = mint_envelope_xdr();
        let h1 = payload_hash_from_envelope_xdr(&xdr, &cfg).unwrap();
        let h2 = payload_hash_from_envelope_xdr(&xdr, &cfg).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn live_fixture_dual_events_bind() {
        let (cfg, message, nonce, recipient, amount, events) = mint_binding_context();
        let verifier = StellarRpcMintVerifier::for_binding_tests(&cfg);
        verifier
            .bind_completion_evidence(
                &message,
                &nonce,
                &recipient,
                amount,
                CctpFinality::Standard,
                &events,
            )
            .unwrap();
    }

    #[test]
    fn rejects_message_received_only() {
        let cfg = CctpConfig::default_testnet();
        let tx = mint_finalized_tx();
        let mt_hash =
            stellar_contract_to_bytes32(&cfg.contracts.stellar_message_transmitter).unwrap();
        let received_only: Vec<_> = tx
            .contract_events
            .iter()
            .filter(|ev| contract_hash(ev).ok() == Some(mt_hash))
            .cloned()
            .collect();
        let mut partial = tx.clone();
        partial.contract_events = received_only;
        let parsed = parse_cctp_v2_message(
            &scval_to_bytes(&parse_invoke_envelope(&mint_envelope_xdr()).unwrap().args[0]).unwrap(),
        )
        .unwrap();
        let fwd_hash = stellar_contract_to_bytes32(&cfg.contracts.stellar_cctp_forwarder).unwrap();
        assert!(matches!(
            StellarRpcMintVerifier::find_bound_mint_events(
                &partial,
                parsed.nonce,
                fwd_hash,
                mt_hash,
                &cfg.contracts.stellar_cctp_forwarder,
            ),
            Err(VerifierError::Failed(ref m)) if m.contains("ambiguous")
        ));
    }

    #[test]
    fn rejects_forwarder_only() {
        let cfg = CctpConfig::default_testnet();
        let tx = mint_finalized_tx();
        let fwd_hash = stellar_contract_to_bytes32(&cfg.contracts.stellar_cctp_forwarder).unwrap();
        let forward_only: Vec<_> = tx
            .contract_events
            .iter()
            .filter(|ev| contract_hash(ev).ok() == Some(fwd_hash))
            .cloned()
            .collect();
        let mut partial = tx.clone();
        partial.contract_events = forward_only;
        let parsed = parse_cctp_v2_message(
            &scval_to_bytes(&parse_invoke_envelope(&mint_envelope_xdr()).unwrap().args[0]).unwrap(),
        )
        .unwrap();
        let mt_hash =
            stellar_contract_to_bytes32(&cfg.contracts.stellar_message_transmitter).unwrap();
        assert!(matches!(
            StellarRpcMintVerifier::find_bound_mint_events(
                &partial,
                parsed.nonce,
                fwd_hash,
                mt_hash,
                &cfg.contracts.stellar_cctp_forwarder,
            ),
            Err(VerifierError::Failed(ref m)) if m.contains("ambiguous")
        ));
    }

    #[test]
    fn rejects_duplicate_forward_events() {
        let cfg = CctpConfig::default_testnet();
        let tx = mint_finalized_tx();
        let fwd_hash = stellar_contract_to_bytes32(&cfg.contracts.stellar_cctp_forwarder).unwrap();
        let forward_ev = tx
            .contract_events
            .iter()
            .find(|ev| contract_hash(ev).ok() == Some(fwd_hash))
            .cloned()
            .unwrap();
        let mut dup = tx.clone();
        dup.contract_events.push(forward_ev);
        let parsed = parse_cctp_v2_message(
            &scval_to_bytes(&parse_invoke_envelope(&mint_envelope_xdr()).unwrap().args[0]).unwrap(),
        )
        .unwrap();
        let mt_hash =
            stellar_contract_to_bytes32(&cfg.contracts.stellar_message_transmitter).unwrap();
        assert!(matches!(
            StellarRpcMintVerifier::find_bound_mint_events(
                &dup,
                parsed.nonce,
                fwd_hash,
                mt_hash,
                &cfg.contracts.stellar_cctp_forwarder,
            ),
            Err(VerifierError::Failed(ref m)) if m.contains("ambiguous")
        ));
    }

    #[test]
    fn rejects_wrong_recipient_amount_body_nonce() {
        let (cfg, message, nonce, recipient, amount, mut events) = mint_binding_context();
        let verifier = StellarRpcMintVerifier::for_binding_tests(&cfg);

        events.forward.forward_recipient = "GINVALID".into();
        assert!(matches!(
            verifier.bind_completion_evidence(
                &message,
                &nonce,
                &recipient,
                amount,
                CctpFinality::Standard,
                &events,
            ),
            Err(VerifierError::Failed(ref m))
                if m.contains("recipient") || m.contains("strkey")
        ));
        events = mint_binding_context().5;
        events.forward.amount = MINT_LOCAL_AMOUNT + 1;
        assert!(matches!(
            verifier.bind_completion_evidence(
                &message,
                &nonce,
                &recipient,
                amount,
                CctpFinality::Standard,
                &events,
            ),
            Err(VerifierError::Failed(ref m)) if m.contains("amount")
        ));
        events = mint_binding_context().5;
        events.received.message_body[0] ^= 0xff;
        assert!(matches!(
            verifier.bind_completion_evidence(
                &message,
                &nonce,
                &recipient,
                amount,
                CctpFinality::Standard,
                &events,
            ),
            Err(VerifierError::Failed(ref m)) if m.contains("message_body")
        ));
        events = mint_binding_context().5;
        let bad_nonce = "0x0000000000000000000000000000000000000000000000000000000000000001";
        assert!(matches!(
            verifier.bind_completion_evidence(
                &message,
                bad_nonce,
                &recipient,
                amount,
                CctpFinality::Standard,
                &events,
            ),
            Err(VerifierError::Failed(ref m)) if m.contains("nonce")
        ));
    }

    #[test]
    fn m_recipient_completes_with_dual_event_evidence() {
        const TEST_M: &str =
            "MA3D5KRYM6CB7OWQ6TWYRR3Z4T7GNZLKERYNZGGA5SOAOPIFY6YQGAAAAAAAAAPCICBKU";
        let (cfg, message, nonce, _g_recipient, amount, mut events) = mint_binding_context();
        events.forward.forward_recipient = TEST_M.to_string();
        let verifier = StellarRpcMintVerifier::for_binding_tests(&cfg);
        verifier
            .bind_completion_evidence(
                &message,
                &nonce,
                TEST_M,
                amount,
                CctpFinality::Standard,
                &events,
            )
            .unwrap();
    }

    #[test]
    fn m_wrong_mux_id_does_not_complete_binding() {
        const TEST_M: &str =
            "MA3D5KRYM6CB7OWQ6TWYRR3Z4T7GNZLKERYNZGGA5SOAOPIFY6YQGAAAAAAAAAPCICBKU";
        const TEST_G: &str = "GA3D5KRYM6CB7OWQ6TWYRR3Z4T7GNZLKERYNZGGA5SOAOPIFY6YQHES5";
        let (cfg, message, nonce, _g_recipient, amount, mut events) = mint_binding_context();
        let pk = stellar_strkey::ed25519::PublicKey::from_string(TEST_G).unwrap();
        let wrong_m = format!(
            "{}",
            stellar_strkey::ed25519::MuxedAccount {
                ed25519: pk.0,
                id: 42,
            }
        );
        events.forward.forward_recipient = wrong_m;
        let verifier = StellarRpcMintVerifier::for_binding_tests(&cfg);
        assert!(matches!(
            verifier.bind_completion_evidence(
                &message,
                &nonce,
                TEST_M,
                amount,
                CctpFinality::Standard,
                &events,
            ),
            Err(VerifierError::Failed(ref m)) if m.contains("recipient")
        ));
    }

    #[tokio::test]
    async fn verify_mint_submission_rejects_wrong_submitter() {
        use crate::cctp::fixtures::stellar_live_xdr::{mint_envelope_xdr, MINT_TX_HASH};
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let mut cfg = CctpConfig::default_testnet();
        cfg.stellar_rpc_url = server.uri();
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mint_fixture_json()))
            .mount(&server)
            .await;

        let verifier = StellarRpcMintVerifier::for_binding_tests(&cfg);
        let invoke = parse_invoke_envelope(&mint_envelope_xdr()).unwrap();
        let message = scval_to_bytes(&invoke.args[0]).unwrap();
        let attestation = scval_to_bytes(&invoke.args[1]).unwrap();
        let err = verifier
            .verify_mint_submission(
                MINT_TX_HASH,
                &message,
                &attestation,
                "0x00",
                "deadbeef",
                Some("GWRONGSUBMITTERAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"),
            )
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            VerifierError::Failed(ref m) if m.contains("submitter")
        ));
    }

    #[test]
    fn rejects_finality_executed_mismatch() {
        let (cfg, message, nonce, recipient, amount, mut events) = mint_binding_context();
        events.received.finality_threshold_executed ^= 1;
        let verifier = StellarRpcMintVerifier::for_binding_tests(&cfg);
        assert!(matches!(
            verifier.bind_completion_evidence(
                &message,
                &nonce,
                &recipient,
                amount,
                CctpFinality::Standard,
                &events,
            ),
            Err(VerifierError::Failed(ref m)) if m.contains("finality")
        ));
    }

    #[test]
    fn forward_amount_nets_out_fee_executed() {
        let (cfg, mut message, nonce, recipient, amount, mut events) = mint_binding_context();
        // fee_executed u256 at body offset 164 → absolute 148+164 = 312
        let fee: u128 = 500;
        message[312..344].fill(0);
        message[344 - 16..344].copy_from_slice(&fee.to_be_bytes());
        // Keep received.message_body in sync with patched message.
        events.received.message_body = message[MESSAGE_HEADER_LEN..].to_vec();
        let net_local = canonical_to_stellar_local_amount((amount - fee) as i128).unwrap();
        events.forward.amount = net_local;
        let verifier = StellarRpcMintVerifier::for_binding_tests(&cfg);
        verifier
            .bind_completion_evidence(
                &message,
                &nonce,
                &recipient,
                amount,
                CctpFinality::Standard,
                &events,
            )
            .unwrap();
    }
}
