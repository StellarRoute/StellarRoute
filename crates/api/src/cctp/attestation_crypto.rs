//! Pure CCTP v2 attestation cryptography — contract-parity with Circle Attestable.
//!
//! Pinned sources:
//! - `circlefin/evm-cctp-contracts` `src/roles/Attestable.sol` @ `a92a2b4e7e6e`
//! - `circlefin/stellar-cctp` `packages/cctp-roles/src/attestable/storage.rs` @ `45746f2c8031`
//!
//! Low-s policy: reject high-s signatures on both EVM and Stellar destination paths
//! (intersection with Soroban SDK malleability rules).

use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
use tiny_keccak::{Hasher, Keccak};

use crate::cctp::bounds::{check_byte_len, MAX_ATTESTATION_BYTES, MAX_RAW_MESSAGE_BYTES};

pub const SIGNATURE_LENGTH: usize = 65;

/// secp256k1 half-order (reject high-s malleability per Stellar contract / Soroban SDK).
const SECP256K1_HALF_N: [u8; 32] = [
    0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0x5d, 0x57, 0x6e, 0x73, 0x57, 0xa4, 0x50, 0x1d, 0xdf, 0xe9, 0x2f, 0x46, 0x68, 0x1b, 0x20, 0xa0,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttestationCryptoError {
    EmptyMessage,
    EmptyAttestation,
    MessageTooLarge,
    AttestationTooLarge,
    InvalidAttestationLength,
    InvalidSignatureComponent,
    HighS,
    InvalidRecoveryId,
    RecoveryFailed,
    InvalidSignatureOrder,
    UnknownSigner,
    ThresholdZero,
    ThresholdOverflow,
    ExtraSignatures,
}

impl AttestationCryptoError {
    pub fn reason_label(&self) -> &'static str {
        match self {
            Self::EmptyMessage => "empty_message",
            Self::EmptyAttestation => "empty_attestation",
            Self::MessageTooLarge => "message_too_large",
            Self::AttestationTooLarge => "attestation_too_large",
            Self::InvalidAttestationLength => "invalid_length",
            Self::InvalidSignatureComponent => "invalid_rs",
            Self::HighS => "high_s",
            Self::InvalidRecoveryId => "invalid_v",
            Self::RecoveryFailed => "recovery_failed",
            Self::InvalidSignatureOrder => "invalid_order",
            Self::UnknownSigner => "unknown_signer",
            Self::ThresholdZero => "threshold_zero",
            Self::ThresholdOverflow => "threshold_overflow",
            Self::ExtraSignatures => "extra_signatures",
        }
    }
}

pub fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(data);
    hasher.finalize(&mut out);
    out
}

pub fn eth_address_from_pubkey_xy(xy: &[u8; 64]) -> [u8; 20] {
    let hash = keccak256(xy);
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&hash[12..32]);
    addr
}

fn normalize_recovery_id(v: u8) -> Result<u8, AttestationCryptoError> {
    match v {
        27 | 28 => Ok(v - 27),
        0 | 1 => Ok(v),
        _ => Err(AttestationCryptoError::InvalidRecoveryId),
    }
}

fn is_high_s(s: &[u8; 32]) -> bool {
    s > &SECP256K1_HALF_N
}

fn parse_signature_component(
    bytes: &[u8],
) -> Result<([u8; 32], [u8; 32], u8), AttestationCryptoError> {
    if bytes.len() != SIGNATURE_LENGTH {
        return Err(AttestationCryptoError::InvalidSignatureComponent);
    }
    let mut r = [0u8; 32];
    let mut s = [0u8; 32];
    r.copy_from_slice(&bytes[0..32]);
    s.copy_from_slice(&bytes[32..64]);
    let v = bytes[64];
    if r == [0u8; 32] || s == [0u8; 32] {
        return Err(AttestationCryptoError::InvalidSignatureComponent);
    }
    if is_high_s(&s) {
        return Err(AttestationCryptoError::HighS);
    }
    Ok((r, s, v))
}

/// Recover Ethereum address from message digest + 65-byte signature.
pub fn recover_signer_address(
    digest: &[u8; 32],
    signature: &[u8],
) -> Result<[u8; 20], AttestationCryptoError> {
    let (r, s, v) = parse_signature_component(signature)?;
    let rid_byte = normalize_recovery_id(v)?;
    let recovery_id =
        RecoveryId::from_byte(rid_byte).ok_or(AttestationCryptoError::InvalidRecoveryId)?;
    let mut sig64 = [0u8; 64];
    sig64[0..32].copy_from_slice(&r);
    sig64[32..64].copy_from_slice(&s);
    let sig = Signature::try_from(&sig64[..])
        .map_err(|_| AttestationCryptoError::InvalidSignatureComponent)?;
    let vk = VerifyingKey::recover_from_prehash(digest, &sig, recovery_id)
        .map_err(|_| AttestationCryptoError::RecoveryFailed)?;
    let encoded = vk.to_encoded_point(false);
    let xy = encoded.as_bytes();
    // 0x04 || x || y
    let mut pubkey_xy = [0u8; 64];
    pubkey_xy.copy_from_slice(&xy[1..65]);
    Ok(eth_address_from_pubkey_xy(&pubkey_xy))
}

/// Verify attestation against enabled attester set and on-chain threshold.
///
/// `enabled_sorted` must be sorted ascending by address bytes.
pub fn verify_attestation_signatures(
    message: &[u8],
    attestation: &[u8],
    signature_threshold: u32,
    enabled_sorted: &[[u8; 20]],
) -> Result<Vec<[u8; 20]>, AttestationCryptoError> {
    if message.is_empty() {
        return Err(AttestationCryptoError::EmptyMessage);
    }
    if attestation.is_empty() {
        return Err(AttestationCryptoError::EmptyAttestation);
    }
    check_byte_len("raw_message", message, MAX_RAW_MESSAGE_BYTES)
        .map_err(|_| AttestationCryptoError::MessageTooLarge)?;
    check_byte_len("attestation", attestation, MAX_ATTESTATION_BYTES)
        .map_err(|_| AttestationCryptoError::AttestationTooLarge)?;

    if signature_threshold == 0 {
        return Err(AttestationCryptoError::ThresholdZero);
    }

    let expected_len = (signature_threshold as usize)
        .checked_mul(SIGNATURE_LENGTH)
        .ok_or(AttestationCryptoError::ThresholdOverflow)?;
    if attestation.len() != expected_len {
        return Err(AttestationCryptoError::InvalidAttestationLength);
    }

    let digest = keccak256(message);
    let mut recovered = Vec::with_capacity(signature_threshold as usize);
    let mut latest = [0u8; 20];

    for i in 0..signature_threshold as usize {
        let start = i * SIGNATURE_LENGTH;
        let sig = &attestation[start..start + SIGNATURE_LENGTH];
        let addr = recover_signer_address(&digest, sig)?;

        if addr <= latest {
            return Err(AttestationCryptoError::InvalidSignatureOrder);
        }
        if enabled_sorted.binary_search(&addr).is_err() {
            return Err(AttestationCryptoError::UnknownSigner);
        }
        latest = addr;
        recovered.push(addr);
    }

    Ok(recovered)
}

pub fn checked_attestation_len(threshold: u32) -> Result<usize, AttestationCryptoError> {
    if threshold == 0 {
        return Err(AttestationCryptoError::ThresholdZero);
    }
    (threshold as usize)
        .checked_mul(SIGNATURE_LENGTH)
        .ok_or(AttestationCryptoError::ThresholdOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cctp::fixtures::circle_attestation_v2::{
        ATTESTER_ADDRESS_1, ATTESTER_ADDRESS_2, ATTESTER_ADDRESS_3, FIXTURE_DUPE_ATTESTATION,
        FIXTURE_INVALID_ORDER_ATTESTATION, FIXTURE_INVALID_ORDER_MESSAGE,
        FIXTURE_VALID_ATTESTATION, FIXTURE_VALID_MESSAGE,
    };

    fn sorted_enabled() -> Vec<[u8; 20]> {
        let mut v = vec![ATTESTER_ADDRESS_1, ATTESTER_ADDRESS_2, ATTESTER_ADDRESS_3];
        v.sort();
        v
    }

    #[test]
    fn official_fixture_valid_two_signatures() {
        let mut enabled = vec![ATTESTER_ADDRESS_1, ATTESTER_ADDRESS_2, ATTESTER_ADDRESS_3];
        enabled.sort();
        let recovered = verify_attestation_signatures(
            FIXTURE_VALID_MESSAGE,
            FIXTURE_VALID_ATTESTATION,
            2,
            &enabled,
        )
        .expect("valid fixture");
        assert_eq!(recovered.len(), 2);
        // Addresses must be strictly increasing; attester2 < attester1 lexicographically.
        assert_eq!(recovered[0], ATTESTER_ADDRESS_2);
        assert_eq!(recovered[1], ATTESTER_ADDRESS_1);
    }

    #[test]
    fn official_fixture_invalid_order() {
        let mut enabled = vec![ATTESTER_ADDRESS_1, ATTESTER_ADDRESS_2];
        enabled.sort();
        let err = verify_attestation_signatures(
            FIXTURE_INVALID_ORDER_MESSAGE,
            FIXTURE_INVALID_ORDER_ATTESTATION,
            2,
            &enabled,
        )
        .unwrap_err();
        assert_eq!(err, AttestationCryptoError::InvalidSignatureOrder);
    }

    #[test]
    fn official_fixture_duplicate_signatures() {
        let mut enabled = vec![ATTESTER_ADDRESS_1, ATTESTER_ADDRESS_2];
        enabled.sort();
        let err = verify_attestation_signatures(
            FIXTURE_INVALID_ORDER_MESSAGE,
            FIXTURE_DUPE_ATTESTATION,
            2,
            &enabled,
        )
        .unwrap_err();
        assert_eq!(err, AttestationCryptoError::InvalidSignatureOrder);
    }

    #[test]
    fn rejects_unknown_signer() {
        let enabled = vec![ATTESTER_ADDRESS_3];
        let err = verify_attestation_signatures(
            FIXTURE_VALID_MESSAGE,
            FIXTURE_VALID_ATTESTATION,
            2,
            &enabled,
        )
        .unwrap_err();
        assert_eq!(err, AttestationCryptoError::UnknownSigner);
    }

    #[test]
    fn rejects_wrong_length() {
        let enabled = sorted_enabled();
        let short = &FIXTURE_VALID_ATTESTATION[..FIXTURE_VALID_ATTESTATION.len() - 1];
        let err =
            verify_attestation_signatures(FIXTURE_VALID_MESSAGE, short, 2, &enabled).unwrap_err();
        assert_eq!(err, AttestationCryptoError::InvalidAttestationLength);
    }

    #[test]
    fn rejects_threshold_zero() {
        let enabled = sorted_enabled();
        let err = verify_attestation_signatures(
            FIXTURE_VALID_MESSAGE,
            FIXTURE_VALID_ATTESTATION,
            0,
            &enabled,
        )
        .unwrap_err();
        assert_eq!(err, AttestationCryptoError::ThresholdZero);
    }

    #[test]
    fn rejects_bad_v() {
        let digest = keccak256(FIXTURE_VALID_MESSAGE);
        let mut sig = FIXTURE_VALID_ATTESTATION[0..SIGNATURE_LENGTH].to_vec();
        sig[64] = 30;
        let err = recover_signer_address(&digest, &sig).unwrap_err();
        assert_eq!(err, AttestationCryptoError::InvalidRecoveryId);
    }

    #[test]
    fn rejects_zero_r() {
        let digest = keccak256(FIXTURE_VALID_MESSAGE);
        let mut sig = FIXTURE_VALID_ATTESTATION[0..SIGNATURE_LENGTH].to_vec();
        sig[0..32].fill(0);
        let err = recover_signer_address(&digest, &sig).unwrap_err();
        assert_eq!(err, AttestationCryptoError::InvalidSignatureComponent);
    }

    #[test]
    fn rejects_high_s_malleability() {
        use crate::cctp::fixtures::circle_attestation_v2::SECP256K1_N;

        fn sub_be(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
            let mut out = [0u8; 32];
            let mut borrow = 0i16;
            for i in (0..32).rev() {
                let mut v = a[i] as i16 - b[i] as i16 - borrow;
                if v < 0 {
                    v += 256;
                    borrow = 1;
                } else {
                    borrow = 0;
                }
                out[i] = v as u8;
            }
            out
        }

        let digest = keccak256(FIXTURE_VALID_MESSAGE);
        let mut sig = FIXTURE_VALID_ATTESTATION[0..SIGNATURE_LENGTH].to_vec();
        let mut s = [0u8; 32];
        s.copy_from_slice(&sig[32..64]);
        let s_prime = sub_be(&SECP256K1_N, &s);
        sig[32..64].copy_from_slice(&s_prime);
        sig[64] = if sig[64] == 27 { 28 } else { 27 };
        let err = recover_signer_address(&digest, &sig).unwrap_err();
        assert_eq!(err, AttestationCryptoError::HighS);
    }
}
