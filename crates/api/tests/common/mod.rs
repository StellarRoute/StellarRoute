//! Shared helpers for swap integration tests (not part of the production API surface).

use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use stellar_xdr::curr::{
    DecoratedSignature, Hash, Limits, ReadXdr, Signature, SignatureHint, TransactionEnvelope,
    TransactionSignaturePayload, TransactionSignaturePayloadTaggedTransaction, WriteXdr,
};

pub const TESTNET_PASSPHRASE: &str = "Test SDF Network ; September 2015";
pub const USDC_ISSUER: &str = "GCXKG6RN4ONIEPCMNFB732A436Z5PNDSRLGWK7GBLCMQLIFO4S7EYWVU";

pub fn test_keypair() -> ([u8; 32], String) {
    let seed = [7u8; 32];
    let sk = SigningKey::from_bytes(&seed);
    let pk = sk.verifying_key();
    let gaddr = stellar_strkey::ed25519::PublicKey(*pk.as_bytes())
        .to_string()
        .as_str()
        .to_string();
    (seed, gaddr)
}

pub fn sign_envelope_with_keypair(
    unsigned_xdr: &str,
    secret_seed: &[u8; 32],
    passphrase: &str,
) -> String {
    let envelope = TransactionEnvelope::from_xdr_base64(unsigned_xdr, Limits::none())
        .expect("unsigned envelope");
    let TransactionEnvelope::Tx(mut v1) = envelope else {
        panic!("expected Tx envelope");
    };
    let network_id = Hash(Sha256::digest(passphrase.as_bytes()).into());
    let payload = TransactionSignaturePayload {
        network_id,
        tagged_transaction: TransactionSignaturePayloadTaggedTransaction::Tx(v1.tx.clone()),
    };
    let bytes = payload.to_xdr(Limits::none()).expect("payload xdr");
    let hash: [u8; 32] = Sha256::digest(&bytes).into();
    let signing_key = SigningKey::from_bytes(secret_seed);
    let sig = signing_key.sign(&hash);
    let pk = signing_key.verifying_key();
    let hint = SignatureHint(pk.as_bytes()[28..32].try_into().unwrap());
    let decorated = DecoratedSignature {
        hint,
        signature: Signature(sig.to_bytes().to_vec().try_into().unwrap()),
    };
    v1.signatures = vec![decorated].try_into().unwrap();
    TransactionEnvelope::Tx(v1)
        .to_xdr_base64(Limits::none())
        .expect("signed xdr")
}
