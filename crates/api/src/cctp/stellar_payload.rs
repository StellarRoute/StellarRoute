//! Shared Stellar envelope payload hashing — builder and verifier must agree.

use sha2::{Digest, Sha256};
use stellar_xdr::curr::{
    Hash, Limited, Limits, ReadXdr, TransactionEnvelope, TransactionSignaturePayload,
    TransactionSignaturePayloadTaggedTransaction, VecM, WriteXdr,
};

use crate::cctp::config::{CctpConfig, STELLAR_TESTNET_PASSPHRASE};
use crate::cctp::verifiers::VerifierError;
use crate::models::v2_cctp::PreparedWalletPayload;

/// Canonical network passphrase for payload hash and tx-id binding.
pub fn passphrase_for_config(config: &CctpConfig) -> String {
    if config.stellar_network_passphrase.is_empty() {
        STELLAR_TESTNET_PASSPHRASE.to_string()
    } else {
        config.stellar_network_passphrase.clone()
    }
}

/// Re-encode a v1 envelope with signatures cleared.
///
/// Prepare returns unsigned XDR; Freighter submits a signed envelope. Hashing must
/// bind the transaction body only so mint submit verification matches prepare.
pub fn unsigned_envelope_xdr(envelope_xdr: &str) -> Result<String, VerifierError> {
    let env = TransactionEnvelope::from_xdr_base64(envelope_xdr, Limits::none())
        .map_err(|e| VerifierError::Failed(e.to_string()))?;
    let unsigned = match env {
        TransactionEnvelope::Tx(mut v1) => {
            v1.signatures = VecM::default();
            TransactionEnvelope::Tx(v1)
        }
        TransactionEnvelope::TxFeeBump(_) => {
            return Err(VerifierError::Failed(
                "fee-bump envelopes unsupported".into(),
            ));
        }
        TransactionEnvelope::TxV0(_) => {
            return Err(VerifierError::Failed("v0 envelopes unsupported".into()));
        }
    };
    unsigned
        .to_xdr_base64(Limits::none())
        .map_err(|e| VerifierError::Failed(e.to_string()))
}

/// SHA256(JSON `PreparedWalletPayload::StellarXdr`) — matches builder mint/burn payloads.
///
/// Signatures are stripped before hashing so prepared (unsigned) and on-chain (signed)
/// envelopes produce the same digest when the tx body is unchanged.
pub fn payload_hash_from_envelope_xdr(
    envelope_xdr: &str,
    config: &CctpConfig,
) -> Result<String, VerifierError> {
    let xdr_envelope = unsigned_envelope_xdr(envelope_xdr)?;
    let payload = PreparedWalletPayload::StellarXdr {
        network_passphrase: passphrase_for_config(config),
        xdr_envelope,
        source: None,
    };
    let json = serde_json::to_string(&payload).map_err(|e| VerifierError::Failed(e.to_string()))?;
    Ok(hex::encode(Sha256::digest(json.as_bytes())))
}

/// Stellar transaction hash from signed envelope XDR + network passphrase (protocol v1).
pub fn transaction_hash_from_envelope_xdr(
    envelope_xdr: &str,
    network_passphrase: &str,
) -> Result<String, VerifierError> {
    let env = TransactionEnvelope::from_xdr_base64(envelope_xdr, Limits::none())
        .map_err(|e| VerifierError::Failed(e.to_string()))?;
    let payload = match env {
        TransactionEnvelope::Tx(v1) => TransactionSignaturePayload {
            network_id: Hash(Sha256::digest(network_passphrase.as_bytes()).into()),
            tagged_transaction: TransactionSignaturePayloadTaggedTransaction::Tx(v1.tx),
        },
        TransactionEnvelope::TxFeeBump(_) => {
            return Err(VerifierError::Failed(
                "fee-bump envelopes unsupported".into(),
            ));
        }
        TransactionEnvelope::TxV0(_) => {
            return Err(VerifierError::Failed("v0 envelopes unsupported".into()));
        }
    };
    let mut bytes = Vec::new();
    let mut writer = Limited::new(&mut bytes, Limits::none());
    payload
        .write_xdr(&mut writer)
        .map_err(|e| VerifierError::Failed(e.to_string()))?;
    Ok(hex::encode(Sha256::digest(&bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cctp::config::CctpConfig;

    #[test]
    fn signed_and_unsigned_mint_envelopes_share_payload_hash() {
        let cfg = CctpConfig::default_testnet();
        // Live Sepolia→Stellar mint: prepare XDR vs Freighter-signed Horizon envelope.
        let prepared = include_str!("testdata/mint_prepare_unsigned.xdr.b64").trim();
        let signed = include_str!("testdata/mint_submit_signed.xdr.b64").trim();
        let expected = "cef33bffa3fb06d73c5b66cb90912b3531f961d1606348ab96cb40c1ed6e2725";

        let unsigned_norm = unsigned_envelope_xdr(prepared).unwrap();
        // Re-encode of already-unsigned prepare must be stable with the stored hash path.
        let h_prep_raw = {
            let payload = PreparedWalletPayload::StellarXdr {
                network_passphrase: passphrase_for_config(&cfg),
                xdr_envelope: prepared.to_string(),
                source: None,
            };
            hex::encode(Sha256::digest(
                serde_json::to_string(&payload).unwrap().as_bytes(),
            ))
        };
        assert_eq!(h_prep_raw, expected);

        assert_eq!(
            unsigned_norm, prepared,
            "unsigned prepare XDR must round-trip so legacy mint_payload_hash still verifies"
        );
        let h_prep = payload_hash_from_envelope_xdr(prepared, &cfg).unwrap();
        let h_signed = payload_hash_from_envelope_xdr(signed, &cfg).unwrap();
        assert_eq!(h_prep, expected);
        assert_eq!(h_signed, expected, "signed Freighter mint must match prepare hash");
    }
}
