//! Offline regression tests against pinned live Stellar Testnet CCTP transaction XDR.

use sha2::Digest;

use stellarroute_api::cctp::config::{CctpConfig, STELLAR_TESTNET_PASSPHRASE};
use stellarroute_api::cctp::encoding::stellar_contract_to_bytes32;
use stellarroute_api::cctp::fixtures::stellar_live_xdr::{
    burn_contract_events_xdr, burn_envelope_xdr, mint_contract_events_xdr, mint_envelope_xdr,
    BURN_CANONICAL_AMOUNT, BURN_DESTINATION_DOMAIN, BURN_MIN_FINALITY, BURN_TX_HASH,
    MINT_LOCAL_AMOUNT, MINT_SOURCE_DOMAIN, MINT_TX_HASH, STELLAR_CCTP_COMMIT,
};
use stellarroute_api::cctp::message::parse_cctp_v2_message;
use stellarroute_api::cctp::stellar_contract_events::{
    collect_contract_events, contract_hash, parse_deposit_for_burn, parse_message_received,
    parse_message_sent, parse_mint_and_forward,
};
use stellarroute_api::cctp::stellar_payload::transaction_hash_from_envelope_xdr;
use stellarroute_api::cctp::stellar_tx::{parse_invoke_envelope, scval_to_bytes};

#[test]
fn live_burn_fixture_provenance_and_hash() {
    assert_eq!(STELLAR_CCTP_COMMIT, "45746f2c8031");
    let hash = transaction_hash_from_envelope_xdr(&burn_envelope_xdr(), STELLAR_TESTNET_PASSPHRASE)
        .unwrap();
    assert_eq!(hash, BURN_TX_HASH);
}

#[test]
fn live_burn_decodes_invoke_and_events() {
    let cfg = CctpConfig::default_testnet();
    let invoke = parse_invoke_envelope(&burn_envelope_xdr()).unwrap();
    assert_eq!(
        invoke.contract_strkey,
        cfg.contracts.stellar_token_messenger
    );
    assert!(
        invoke.function == "deposit_for_burn" || invoke.function == "deposit_for_burn_with_hook"
    );

    let events = collect_contract_events(&[burn_contract_events_xdr()]).unwrap();
    let tm_hash = stellar_contract_to_bytes32(&cfg.contracts.stellar_token_messenger).unwrap();
    let mt_hash = stellar_contract_to_bytes32(&cfg.contracts.stellar_message_transmitter).unwrap();

    let mut burn_ev = None;
    let mut sent = None;
    for ev in &events {
        let h = contract_hash(ev).unwrap();
        if h == tm_hash {
            burn_ev = parse_deposit_for_burn(ev).ok();
        }
        if h == mt_hash {
            sent = parse_message_sent(ev).ok();
        }
    }
    let burn = burn_ev.expect("deposit_for_burn event");
    let message = sent.expect("message_sent").message;
    assert_eq!(burn.amount, BURN_CANONICAL_AMOUNT);
    assert_eq!(burn.destination_domain, BURN_DESTINATION_DOMAIN);
    assert_eq!(burn.min_finality_threshold, BURN_MIN_FINALITY);

    let parsed = parse_cctp_v2_message(&message).unwrap();
    assert_eq!(parsed.body.amount, BURN_CANONICAL_AMOUNT as u128);
    assert_eq!(parsed.destination_domain, BURN_DESTINATION_DOMAIN);
}

#[test]
fn live_mint_decodes_forwarder_invoke_and_dual_events() {
    let cfg = CctpConfig::default_testnet();
    let hash = transaction_hash_from_envelope_xdr(&mint_envelope_xdr(), STELLAR_TESTNET_PASSPHRASE)
        .unwrap();
    assert_eq!(hash, MINT_TX_HASH);

    let invoke = parse_invoke_envelope(&mint_envelope_xdr()).unwrap();
    assert_eq!(invoke.contract_strkey, cfg.contracts.stellar_cctp_forwarder);
    assert_eq!(invoke.function, "mint_and_forward");

    let events = collect_contract_events(&[mint_contract_events_xdr()]).unwrap();
    let fwd_hash = stellar_contract_to_bytes32(&cfg.contracts.stellar_cctp_forwarder).unwrap();
    let mt_hash = stellar_contract_to_bytes32(&cfg.contracts.stellar_message_transmitter).unwrap();

    let mut forward = None;
    let mut received = None;
    for ev in &events {
        let h = contract_hash(ev).unwrap();
        if h == fwd_hash {
            forward = parse_mint_and_forward(ev).ok();
        }
        if h == mt_hash {
            received = parse_message_received(ev).ok();
        }
    }
    let fwd = forward.expect("mint_and_forward event");
    let recv = received.expect("message_received event");
    assert_eq!(fwd.amount, MINT_LOCAL_AMOUNT);
    assert_eq!(recv.source_domain, MINT_SOURCE_DOMAIN);
    assert_eq!(
        stellarroute_api::cctp::stellar_contract_events::address_to_strkey(&recv.caller).unwrap(),
        cfg.contracts.stellar_cctp_forwarder
    );
}

#[test]
fn live_mint_message_body_matches_received_event() {
    let cfg = CctpConfig::default_testnet();
    let events = collect_contract_events(&[mint_contract_events_xdr()]).unwrap();
    let mt_hash = stellar_contract_to_bytes32(&cfg.contracts.stellar_message_transmitter).unwrap();
    let invoke = parse_invoke_envelope(&mint_envelope_xdr()).unwrap();
    let full_message = scval_to_bytes(&invoke.args[0]).unwrap();
    let recv = events
        .iter()
        .filter_map(|ev| {
            let h = contract_hash(ev).ok()?;
            if h == mt_hash {
                parse_message_received(ev).ok()
            } else {
                None
            }
        })
        .next()
        .expect("message_received");
    assert_eq!(
        recv.message_body,
        &full_message[stellarroute_api::cctp::message::MESSAGE_HEADER_LEN..]
    );
    assert_eq!(recv.source_domain, MINT_SOURCE_DOMAIN);
}

#[tokio::test]
#[ignore = "requires live Stellar testnet RPC — re-fetch canonical burn tx while retention permits"]
async fn live_refetch_burn_tx_matches_fixture() {
    use stellarroute_api::cctp::config::CctpConfig;
    use stellarroute_api::cctp::fixtures::stellar_live_xdr::{
        burn_envelope_sha256, burn_envelope_xdr, BURN_TX_HASH,
    };
    use stellarroute_api::cctp::stellar_rpc::StellarRpcClient;

    let cfg = CctpConfig::default_testnet();
    let client = StellarRpcClient::new(&cfg).expect("client");
    let tx = client
        .get_finalized_transaction(BURN_TX_HASH)
        .await
        .expect("burn tx");
    assert_eq!(tx.envelope_xdr, burn_envelope_xdr());
    let live_hash = stellarroute_api::cctp::stellar_payload::transaction_hash_from_envelope_xdr(
        &tx.envelope_xdr,
        stellarroute_api::cctp::config::STELLAR_TESTNET_PASSPHRASE,
    )
    .unwrap();
    assert_eq!(live_hash, BURN_TX_HASH);
    assert_eq!(
        hex::encode(sha2::Sha256::digest(tx.envelope_xdr.as_bytes())),
        burn_envelope_sha256()
    );
}

#[tokio::test]
#[ignore = "requires live Stellar testnet RPC — re-fetch canonical mint tx while retention permits"]
async fn live_refetch_mint_tx_matches_fixture() {
    use stellarroute_api::cctp::config::CctpConfig;
    use stellarroute_api::cctp::fixtures::stellar_live_xdr::{
        mint_envelope_sha256, mint_envelope_xdr, MINT_TX_HASH,
    };
    use stellarroute_api::cctp::stellar_rpc::StellarRpcClient;

    let cfg = CctpConfig::default_testnet();
    let client = StellarRpcClient::new(&cfg).expect("client");
    let tx = client
        .get_finalized_transaction(MINT_TX_HASH)
        .await
        .expect("mint tx");
    assert_eq!(tx.envelope_xdr, mint_envelope_xdr());
    let live_hash = stellarroute_api::cctp::stellar_payload::transaction_hash_from_envelope_xdr(
        &tx.envelope_xdr,
        stellarroute_api::cctp::config::STELLAR_TESTNET_PASSPHRASE,
    )
    .unwrap();
    assert_eq!(live_hash, MINT_TX_HASH);
    assert_eq!(
        hex::encode(sha2::Sha256::digest(tx.envelope_xdr.as_bytes())),
        mint_envelope_sha256()
    );
}
