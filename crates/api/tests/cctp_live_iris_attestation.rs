//! Live Iris + attestation verification against a known Stellar testnet burn.
//! Run: `cargo test -p stellarroute-api --test cctp_live_iris_attestation -- --ignored --nocapture`

use std::sync::Arc;
use std::time::Duration;

use chrono::{Duration as ChronoDuration, Utc};
use stellarroute_api::cctp::attestation::{AttestationVerifier, CircleAttestationVerifier};
use stellarroute_api::cctp::attestation_trust::{
    AttestationRefreshDeps, AttestationTrustCache, SystemClock,
};
use stellarroute_api::cctp::config::CctpConfig;
use stellarroute_api::cctp::evm_attester_reader::evm_reader_arc;
use stellarroute_api::cctp::expectations::build_corridor_expectations;
use stellarroute_api::cctp::iris::{IrisClient, IrisPollOutcome, ReqwestIrisClient};
use stellarroute_api::cctp::iris_public_keys::ReqwestIrisPublicKeySource;
use stellarroute_api::cctp::message::{decode_hex_message, validate_message_for_corridor};
use stellarroute_api::cctp::stellar_attester_reader::stellar_reader_arc;
use stellarroute_api::cctp::store::CctpTransfer;
use stellarroute_api::models::v2_cctp::{
    CctpDirection, CctpFinality, CctpTransferStatus, SEPOLIA_CHAIN_ID, STELLAR_TESTNET_CHAIN_ID,
};
use uuid::Uuid;

fn sample_transfer(recipient: &str, sender: &str) -> CctpTransfer {
    let now = Utc::now();
    CctpTransfer {
        transfer_id: Uuid::new_v4(),
        support_reference_id: "live-diag".into(),
        corridor_id: "circle-cctp:usdc:stellar-testnet:ethereum-sepolia".into(),
        provider: "circle-cctp".into(),
        direction: CctpDirection::StellarToEvm,
        source_chain_id: STELLAR_TESTNET_CHAIN_ID.into(),
        destination_chain_id: SEPOLIA_CHAIN_ID.into(),
        source_asset: "a".into(),
        source_asset_canonical: "a".into(),
        destination_asset: "b".into(),
        destination_asset_canonical: "b".into(),
        sender: sender.into(),
        recipient: recipient.into(),
        mint_submitter: None,
        amount: "1.0000000".into(),
        destination_amount: "1.0000000".into(),
        finality: CctpFinality::Standard,
        runtime_fee_quote: None,
        max_fee: None,
        fee_expires_at: None,
        quote_expires_at: now + ChronoDuration::minutes(5),
        status: CctpTransferStatus::AwaitingAttestation,
        source_tx_hash: None,
        source_approval_tx_hash: None,
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

#[tokio::test]
#[ignore = "live Iris + attestation verify against known Stellar burn"]
async fn verify_known_stellar_burn_iris_payload() {
    let burn_hash = std::env::var("CCTP_DIAG_BURN_HASH").unwrap_or_else(|_| {
        "26514bc123354d8c2ff72f73ad56da48824b03e851c33b2772f2df0f13a96c3d".into()
    });
    let recipient = std::env::var("CCTP_DIAG_EVM_RECIPIENT")
        .unwrap_or_else(|_| "0xB94E6a26CA0b75cF98c0441c80072957dc9CC533".into());
    let sender = std::env::var("CCTP_DIAG_STELLAR_SENDER")
        .unwrap_or_else(|_| "GBH24C7D2IFM4RF7SDWPLREQYQ3CL32PCJJEATMYIPWBKB6PPGTNAAIX".into());

    let mut cfg = CctpConfig::default_testnet();
    cfg.enabled = true;
    cfg.sepolia_rpc_url = "https://sepolia.drpc.org".into();

    let client = ReqwestIrisClient::from_config(&cfg).expect("iris client");
    let outcome = client
        .poll_messages_by_tx(27, &burn_hash)
        .await
        .expect("iris poll");
    let IrisPollOutcome::Complete(msg) = outcome else {
        panic!("expected complete iris message, got {outcome:?}");
    };
    eprintln!("iris event_nonce={}", msg.event_nonce);
    assert!(!msg.event_nonce.is_empty());

    let transfer = sample_transfer(&recipient, &sender);
    let expectations = build_corridor_expectations(&transfer, &cfg).expect("expectations");
    validate_message_for_corridor(&msg.message_hex, &expectations).expect("corridor validation");

    let raw = decode_hex_message(&msg.message_hex).expect("decode message");
    let attestation =
        decode_hex_message(msg.attestation_hex.as_ref().expect("attestation")).expect("decode att");

    let trust = Arc::new(AttestationTrustCache::new(
        Duration::from_secs(900),
        Duration::from_secs(86_400),
        Arc::new(SystemClock),
    ));
    let iris_source = ReqwestIrisPublicKeySource::from_config(&cfg).expect("iris keys");
    let verifier = CircleAttestationVerifier::new(
        trust,
        AttestationRefreshDeps {
            iris_source: Arc::new(iris_source),
            readers: vec![
                evm_reader_arc(&cfg).expect("evm reader"),
                stellar_reader_arc(&cfg).expect("stellar reader"),
            ],
        },
    );
    verifier.bootstrap().await.expect("bootstrap");
    verifier
        .verify_attestation(&raw, &attestation)
        .await
        .expect("live attestation verify");
}
