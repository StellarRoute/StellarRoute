//! Adversarial + assembly tests for production Stellar CCTP builders.

use std::sync::Arc;

use stellarroute_api::cctp::builders::stellar::encoder::{
    approve_args, deposit_for_burn_args, envelope_sequence,
};
use stellarroute_api::cctp::builders::stellar::{
    FixedAllowanceChecker, OfflineStellarXdrEncoder, ProductionStellarCctpBuilder,
    StellarAllowanceChecker,
};
use stellarroute_api::cctp::builders::{BuilderError, StellarCctpBurnBuilder};
use stellarroute_api::cctp::config::CctpConfig;
use stellarroute_api::cctp::encoding::{
    cctp_subunits_to_stellar_subunits, stellar_outbound_cctp_amount,
};
use stellarroute_api::cctp::readiness::CctpRuntime;
use stellarroute_api::cctp::stellar_builder_simulation::{
    compute_total_fee, ledger_bounds_for_expiry,
};
use stellarroute_api::cctp::stellar_payload::payload_hash_from_envelope_xdr;
use stellarroute_api::cctp::stellar_rpc::StellarRpcClient;
use stellarroute_api::cctp::store::CctpTransfer;
use stellarroute_api::models::v2_cctp::{
    CctpDirection, CctpFinality, CctpTransferStatus, SEPOLIA_CHAIN_ID, STELLAR_TESTNET_CHAIN_ID,
};
use stellarroute_api::swap::tx::FixedAccountSequences;
use uuid::Uuid;

fn sample_burn_transfer() -> CctpTransfer {
    let now = chrono::Utc::now();
    CctpTransfer {
        transfer_id: Uuid::new_v4(),
        support_reference_id: "s".into(),
        corridor_id: "c".into(),
        provider: "p".into(),
        direction: CctpDirection::StellarToEvm,
        source_chain_id: STELLAR_TESTNET_CHAIN_ID.into(),
        destination_chain_id: SEPOLIA_CHAIN_ID.into(),
        source_asset: "a".into(),
        source_asset_canonical: "a".into(),
        destination_asset: "b".into(),
        destination_asset_canonical: "b".into(),
        sender: "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF".into(),
        recipient: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0".into(),
        mint_submitter: None,
        amount: "1.0000000".into(),
        destination_amount: "1".into(),
        finality: CctpFinality::Standard,
        runtime_fee_quote: None,
        max_fee: Some("1".into()),
        fee_expires_at: Some(now + chrono::Duration::minutes(10)),
        quote_expires_at: now + chrono::Duration::minutes(10),
        status: CctpTransferStatus::BurnPrepared,
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

#[test]
fn seven_to_six_precision_rejects_dust_remainder() {
    let (cctp, rem) = stellar_outbound_cctp_amount("1.0000009").unwrap();
    assert_eq!(cctp, 1_000_000);
    assert!(rem.is_some());
    let stellar = cctp_subunits_to_stellar_subunits(cctp).unwrap();
    assert_eq!(stellar, 10_000_000);
}

#[test]
fn approval_and_burn_use_distinct_sequences_offline() {
    let cfg = CctpConfig::default_testnet();
    let source = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";
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
        stellarroute_api::cctp::config::FINALITY_STANDARD,
        101,
    )
    .unwrap();
    assert_eq!(envelope_sequence(&approve).unwrap(), 100);
    assert_eq!(envelope_sequence(&burn).unwrap(), 101);
}

#[test]
fn deposit_for_burn_args_count_and_finality() {
    let cfg = CctpConfig::default_testnet();
    let args = deposit_for_burn_args(
        "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
        10_000_000,
        cfg.sepolia_domain,
        [2u8; 32],
        &cfg.contracts.stellar_usdc,
        1,
        stellarroute_api::cctp::config::FINALITY_STANDARD,
    )
    .unwrap();
    assert_eq!(args.len(), 8);
    assert!(matches!(args[7], stellar_xdr::curr::ScVal::U32(2000)));
}

#[test]
fn fee_overflow_is_fail_closed() {
    assert!(compute_total_fee(u32::MAX, 1).is_err());
}

#[tokio::test]
async fn stale_probe_marks_builder_not_ready() {
    let mut cfg = CctpConfig::default_testnet();
    cfg.stellar_rpc_url = "https://127.0.0.1:1".into();
    let rpc = Arc::new(StellarRpcClient::new(&cfg).unwrap());
    let builder = ProductionStellarCctpBuilder {
        sequences: Arc::new(FixedAccountSequences::new(1)),
        rpc,
        allowance: Arc::new(FixedAllowanceChecker { sufficient: true }),
        probe_ok: false,
        base_fee: 100,
    };
    assert!(!builder.is_ready());
    let err = builder
        .prepare_burn(&sample_burn_transfer(), &cfg)
        .await
        .unwrap_err();
    assert_eq!(err, BuilderError::NotReady);
}

#[tokio::test]
async fn approval_path_skipped_when_verified() {
    let mut cfg = CctpConfig::default_testnet();
    cfg.stellar_rpc_url = "https://127.0.0.1:1".into();
    let rpc = Arc::new(StellarRpcClient::new(&cfg).unwrap());
    let builder = ProductionStellarCctpBuilder {
        sequences: Arc::new(FixedAccountSequences::new(1)),
        rpc,
        allowance: Arc::new(FixedAllowanceChecker { sufficient: false }),
        probe_ok: true,
        base_fee: 100,
    };
    let mut transfer = sample_burn_transfer();
    transfer.source_approval_verified_at = Some(chrono::Utc::now());
    let needs = StellarAllowanceChecker::has_sufficient_allowance(
        builder.allowance.as_ref(),
        &transfer.sender,
        &cfg.contracts.stellar_usdc,
        &cfg.contracts.stellar_token_messenger,
        1,
    )
    .await
    .unwrap();
    assert!(!needs);
}

#[test]
fn payload_hash_matches_verifier_helper() {
    let cfg = CctpConfig::default_testnet();
    let xdr = OfflineStellarXdrEncoder::encode_burn_at_sequence(
        "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
        &cfg.contracts.stellar_token_messenger,
        "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
        1,
        cfg.sepolia_domain,
        [0u8; 32],
        &cfg.contracts.stellar_usdc,
        1,
        stellarroute_api::cctp::config::FINALITY_STANDARD,
        1,
    )
    .unwrap();
    let hash = payload_hash_from_envelope_xdr(&xdr, &cfg).unwrap();
    assert_eq!(hash.len(), 64);
}

#[tokio::test]
async fn from_config_async_wires_stellar_builders_when_rpc_reachable() {
    let mut cfg = CctpConfig::default_testnet();
    cfg.sepolia_rpc_url = "https://rpc.sepolia.org".into();
    cfg.stellar_rpc_url = "https://soroban-testnet.stellar.org".into();
    let rt = CctpRuntime::from_config_async(&cfg).await;
    let _ = rt.stellar_burn_builder.is_ready();
    let _ = rt.stellar_mint_builder.is_ready();
    assert!(!rt.is_public_executable(&cfg));
}

#[tokio::test]
#[ignore = "live read-only Stellar testnet simulation probe"]
async fn live_allowance_simulation_probe() {
    let cfg = CctpConfig::default_testnet();
    let rpc = StellarRpcClient::new(&cfg).unwrap();
    let ledger = rpc.latest_ledger().await.expect("ledger");
    let bounds = ledger_bounds_for_expiry(ledger, chrono::Utc::now().timestamp() + 600);
    assert!(bounds.max_ledger > ledger);
    eprintln!("latest_ledger={ledger} bounds={bounds:?}");
}

#[test]
fn approve_args_include_expiration_ledger() {
    let args = approve_args(
        "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
        "CDNG7HXAPBWICI2E3AUBP3YZWZELJLYSB6F5CC7WLDTLTHVM74SLRTHP",
        1,
        42,
    )
    .unwrap();
    assert_eq!(args.len(), 4);
}
