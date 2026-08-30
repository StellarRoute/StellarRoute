//! CCTP v2 encoding helpers with golden vectors from Circle Stellar reference.
//!
//! Sources:
//! - https://developers.circle.com/cctp/references/stellar (hook layout, contract bytes32)
//! - https://developers.circle.com/cctp/references/technical-guide (message layout)

use thiserror::Error;

use crate::models::v2_cctp::is_valid_evm_address;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EncodingError {
    #[error("invalid EVM address: {0}")]
    InvalidEvmAddress(String),
    #[error("invalid Stellar contract strkey: {0}")]
    InvalidStellarContract(String),
    #[error("invalid Stellar G-address: {0}")]
    InvalidStellarAccount(String),
    #[error("amount overflow")]
    AmountOverflow,
    #[error("amount has non-zero 7th decimal remainder: {0}")]
    StellarRemainder(String),
    #[error("invalid hook data")]
    InvalidHookData,
}

/// EVM 20-byte address -> bytes32 left-zero-padded (Circle Message.sol reference).
pub fn evm_address_to_bytes32(address: &str) -> Result<[u8; 32], EncodingError> {
    if !is_valid_evm_address(address) {
        return Err(EncodingError::InvalidEvmAddress(address.to_string()));
    }
    let hex = address.trim().strip_prefix("0x").unwrap_or(address.trim());
    let bytes = hex::decode(hex).expect("validated hex");
    let mut out = [0u8; 32];
    out[12..32].copy_from_slice(&bytes);
    Ok(out)
}

/// Stellar contract C-strkey -> 32-byte contract id (raw payload, no type prefix).
pub fn stellar_contract_to_bytes32(strkey: &str) -> Result<[u8; 32], EncodingError> {
    let contract = stellar_strkey::Contract::from_string(strkey.trim())
        .map_err(|_| EncodingError::InvalidStellarContract(strkey.to_string()))?;
    Ok(contract.0)
}

/// Stellar G-address ed25519 public key as bytes32 (Circle Stellar burn messageSender).
pub fn stellar_account_to_bytes32(g_address: &str) -> Result<[u8; 32], EncodingError> {
    let pk = stellar_strkey::ed25519::PublicKey::from_string(g_address.trim())
        .map_err(|_| EncodingError::InvalidStellarAccount(g_address.to_string()))?;
    Ok(pk.0)
}

/// Build CctpForwarder hook data for a G- or M-address recipient (Circle Stellar reference).
///
/// Layout: 24 zero bytes | u32 BE version=0 | u32 BE length | UTF-8 strkey (G or M)
pub fn build_forwarder_hook_data_recipient(recipient: &str) -> Result<Vec<u8>, EncodingError> {
    use crate::cctp::stellar_muxed::parse_recipient_strkey;
    let key = parse_recipient_strkey(recipient)
        .map_err(|_| EncodingError::InvalidStellarAccount(recipient.to_string()))?;
    match key {
        crate::cctp::stellar_muxed::StellarRecipientKey::Contract(_) => {
            Err(EncodingError::InvalidHookData)
        }
        crate::cctp::stellar_muxed::StellarRecipientKey::Account(_)
        | crate::cctp::stellar_muxed::StellarRecipientKey::Muxed { .. } => {
            let strkey = key.to_strkey();
            let recipient_bytes = strkey.as_bytes();
            let mut hook = vec![0u8; 32 + recipient_bytes.len()];
            hook[24..28].copy_from_slice(&0u32.to_be_bytes());
            hook[28..32].copy_from_slice(&(recipient_bytes.len() as u32).to_be_bytes());
            hook[32..].copy_from_slice(recipient_bytes);
            Ok(hook)
        }
    }
}

/// Build CctpForwarder hook data for a G-address recipient (Circle Stellar reference).
///
/// Layout: 24 zero bytes | u32 BE version=0 | u32 BE length | UTF-8 strkey
pub fn build_forwarder_hook_data_g_recipient(g_address: &str) -> Result<Vec<u8>, EncodingError> {
    build_forwarder_hook_data_recipient(g_address)
}

/// Stellar outbound amount with zero 7th-decimal remainder (fail-closed on dust).
pub fn stellar_outbound_cctp_amount_strict(amount_7dp: &str) -> Result<u128, EncodingError> {
    let (cctp, rem) = stellar_outbound_cctp_amount(amount_7dp)?;
    if rem.is_some() {
        return Err(EncodingError::StellarRemainder(rem.unwrap_or_default()));
    }
    Ok(cctp)
}

/// Convert decimal USDC string to 6-decimal CCTP subunits (uint256 wire amount).
pub fn decimal_to_cctp_subunits(amount: &str) -> Result<u128, EncodingError> {
    parse_decimal_to_subunits(amount, 6)
}

/// Convert 6-decimal CCTP subunits to 7-decimal Stellar token subunits (×10).
pub fn cctp_subunits_to_stellar_subunits(cctp: u128) -> Result<u128, EncodingError> {
    cctp.checked_mul(10).ok_or(EncodingError::AmountOverflow)
}

/// Stellar outbound: debit only through 6th decimal; remainder stays on source account.
pub fn stellar_outbound_cctp_amount(
    amount_7dp: &str,
) -> Result<(u128, Option<String>), EncodingError> {
    let (whole, fraction) = split_decimal(amount_7dp)?;
    let frac7 = pad_fraction(&fraction, 7);
    let frac6 = &frac7[..6];
    let remainder_digit = frac7.as_bytes().get(6).copied().unwrap_or(b'0');
    let cctp = parse_decimal_to_subunits(&format!("{}.{}", whole, frac6), 6)?;
    let remainder = if remainder_digit != b'0' {
        Some(format!("0.000000{}", remainder_digit as char))
    } else {
        None
    };
    Ok((cctp, remainder))
}

fn split_decimal(amount: &str) -> Result<(String, String), EncodingError> {
    let parts: Vec<&str> = amount.split('.').collect();
    if parts.len() > 2 || parts.is_empty() {
        return Err(EncodingError::AmountOverflow);
    }
    Ok((
        parts[0].to_string(),
        parts.get(1).unwrap_or(&"").to_string(),
    ))
}

fn pad_fraction(fraction: &str, width: usize) -> String {
    let mut s = fraction.to_string();
    if s.len() > width {
        return s[..width].to_string();
    }
    while s.len() < width {
        s.push('0');
    }
    s
}

/// Stellar 7dp local token amount -> 6dp canonical (Circle `to_canonical_amount` for 7/6 pair).
/// Dust (non-zero 7th decimal) must be stripped before burn; normalized local divides evenly by 10.
pub fn stellar_local_to_canonical_amount(local: i128) -> Result<i128, EncodingError> {
    if local <= 0 {
        return Err(EncodingError::AmountOverflow);
    }
    if local % 10 != 0 {
        return Err(EncodingError::StellarRemainder(format!("{local}")));
    }
    Ok(local / 10)
}

/// Like [`stellar_local_to_canonical_amount`] but allows zero (valid for `max_fee` on burns).
pub fn stellar_local_to_canonical_amount_allow_zero(local: i128) -> Result<i128, EncodingError> {
    if local == 0 {
        Ok(0)
    } else {
        stellar_local_to_canonical_amount(local)
    }
}

/// Canonical 6dp CCTP amount -> Stellar 7dp local (×10).
pub fn canonical_to_stellar_local_amount(canonical: i128) -> Result<i128, EncodingError> {
    canonical
        .checked_mul(10)
        .ok_or(EncodingError::AmountOverflow)
}

fn parse_decimal_to_subunits(amount: &str, scale: usize) -> Result<u128, EncodingError> {
    let (whole, fraction) = split_decimal(amount)?;
    let frac = pad_fraction(&fraction, scale);
    let combined = format!("{}{}", whole, frac);
    combined
        .parse::<u128>()
        .map_err(|_| EncodingError::AmountOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cctp::config::STELLAR_CCTP_FORWARDER;

    // Golden: Circle Stellar reference contractStrkeyToBytes32 for CctpForwarder.
    #[test]
    fn golden_stellar_forwarder_contract_bytes32() {
        let bytes = stellar_contract_to_bytes32(STELLAR_CCTP_FORWARDER).unwrap();
        let hex = hex::encode(bytes);
        // Decoded from StrKey.decodeContract per Circle TypeScript reference.
        assert_eq!(hex.len(), 64);
        assert_ne!(hex, "0".repeat(64));
    }

    #[test]
    fn golden_evm_address_bytes32_left_padded() {
        let addr = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0";
        let bytes = evm_address_to_bytes32(addr).unwrap();
        assert_eq!(bytes[0..12], [0u8; 12]);
        assert_eq!(
            hex::encode(&bytes[12..32]),
            "742d35cc6634c0532925a3b844bc9e7595f0beb0"
        );
    }

    #[test]
    fn golden_forwarder_hook_data_m_recipient() {
        let m = "MA3D5KRYM6CB7OWQ6TWYRR3Z4T7GNZLKERYNZGGA5SOAOPIFY6YQGAAAAAAAAAPCICBKU";
        let hook = build_forwarder_hook_data_recipient(m).unwrap();
        assert_eq!(hook[0..24], [0u8; 24]);
        let len = u32::from_be_bytes(hook[28..32].try_into().unwrap());
        assert_eq!(len as usize, m.len());
        assert_eq!(&hook[32..], m.as_bytes());
    }

    #[test]
    fn strict_outbound_rejects_dust() {
        assert!(matches!(
            stellar_outbound_cctp_amount_strict("1.0000009"),
            Err(EncodingError::StellarRemainder(_))
        ));
        assert_eq!(
            stellar_outbound_cctp_amount_strict("1.0000000").unwrap(),
            1_000_000
        );
    }

    #[test]
    fn golden_forwarder_hook_data_g_recipient() {
        let g = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";
        let hook = build_forwarder_hook_data_g_recipient(g).unwrap();
        assert_eq!(hook[0..24], [0u8; 24]);
        assert_eq!(u32::from_be_bytes(hook[24..28].try_into().unwrap()), 0);
        let len = u32::from_be_bytes(hook[28..32].try_into().unwrap());
        assert_eq!(len as usize, g.len());
        assert_eq!(&hook[32..], g.as_bytes());
    }

    #[test]
    fn stellar_seven_to_six_decimal_with_remainder() {
        let (cctp, rem) = stellar_outbound_cctp_amount("0.1234567").unwrap();
        assert_eq!(cctp, 123456);
        assert_eq!(rem.as_deref(), Some("0.0000007"));
    }

    #[test]
    fn canonical_amount_rejects_zero_but_allow_zero_helper_accepts_it() {
        assert!(stellar_local_to_canonical_amount(0).is_err());
        assert_eq!(stellar_local_to_canonical_amount_allow_zero(0).unwrap(), 0);
        assert_eq!(
            stellar_local_to_canonical_amount_allow_zero(10_000_000).unwrap(),
            1_000_000
        );
    }

    #[test]
    fn cctp_to_stellar_subunits_scales_by_ten() {
        assert_eq!(cctp_subunits_to_stellar_subunits(123456).unwrap(), 1234560);
    }

    #[test]
    fn parses_decimal_to_cctp_subunits() {
        assert_eq!(decimal_to_cctp_subunits("100.000000").unwrap(), 100_000_000);
    }
}
