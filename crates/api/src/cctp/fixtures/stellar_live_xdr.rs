//! Pinned live Stellar Testnet CCTP transaction XDR (read-only RPC fetch 2026-07-30).
//!
//! Provenance:
//! - Burn: `getTransaction` `670c2b7061937108f2e475d68249d1ebf01f089b5309139fbc8806196341860c`
//! - Mint: `getTransaction` `c59b4c64a993fc317d7ed3ea415f061723b2c67f0e2db01cd3d65028a5c0fdc4`
//! - Circle stellar-cctp @ `45746f2c8031`

use serde_json::Value;
use sha2::{Digest, Sha256};

pub const STELLAR_CCTP_COMMIT: &str = "45746f2c8031";

pub const BURN_TX_HASH: &str = "670c2b7061937108f2e475d68249d1ebf01f089b5309139fbc8806196341860c";
pub const BURN_LEDGER: u32 = 3_867_580;
pub const BURN_CANONICAL_AMOUNT: i128 = 100_000;
pub const BURN_DESTINATION_DOMAIN: u32 = 0;
pub const BURN_MIN_FINALITY: u32 = 2000;

pub const MINT_TX_HASH: &str = "c59b4c64a993fc317d7ed3ea415f061723b2c67f0e2db01cd3d65028a5c0fdc4";
pub const MINT_LEDGER: u32 = 3_862_387;
pub const MINT_LOCAL_AMOUNT: i128 = 10_000_000;
pub const MINT_SOURCE_DOMAIN: u32 = 0;

const BURN_JSON: &str = include_str!("live_xdr/tx_670c2b70.json");
const MINT_JSON: &str = include_str!("live_xdr/tx_c59b4c64.json");

pub fn burn_fixture_json() -> Value {
    serde_json::from_str(BURN_JSON).expect("burn fixture json")
}

pub fn mint_fixture_json() -> Value {
    serde_json::from_str(MINT_JSON).expect("mint fixture json")
}

pub fn burn_envelope_xdr() -> String {
    burn_fixture_json()["result"]["envelopeXdr"]
        .as_str()
        .expect("burn envelope")
        .to_string()
}

pub fn mint_envelope_xdr() -> String {
    mint_fixture_json()["result"]["envelopeXdr"]
        .as_str()
        .expect("mint envelope")
        .to_string()
}

pub fn burn_contract_events_xdr() -> Vec<String> {
    burn_fixture_json()["result"]["events"]["contractEventsXdr"][0]
        .as_array()
        .expect("burn events")
        .iter()
        .map(|v| v.as_str().expect("event b64").to_string())
        .collect()
}

pub fn mint_contract_events_xdr() -> Vec<String> {
    mint_fixture_json()["result"]["events"]["contractEventsXdr"][0]
        .as_array()
        .expect("mint events")
        .iter()
        .map(|v| v.as_str().expect("event b64").to_string())
        .collect()
}

pub fn burn_envelope_sha256() -> String {
    hex::encode(Sha256::digest(burn_envelope_xdr().as_bytes()))
}

pub fn mint_envelope_sha256() -> String {
    hex::encode(Sha256::digest(mint_envelope_xdr().as_bytes()))
}

/// Fee-payer / operation source G from pinned live mint fixture (exists on Testnet).
pub fn mint_operation_source() -> String {
    crate::cctp::stellar_tx::parse_invoke_envelope(&mint_envelope_xdr())
        .expect("mint invoke")
        .operation_source
}
