//! Circle Soroban `MuxedAddress` → canonical Stellar strkey decoding.
//!
//! Wire (pinned `circlefin/stellar-cctp@45746f2c8031`, Soroban SDK 23):
//! - `ScVal::Address(ScAddress::Account)` → G strkey
//! - `ScVal::Address(ScAddress::MuxedAccount(MuxedEd25519Account))` → M strkey (nonzero id supported)
//! - `ScVal::Address(ScAddress::Contract)` → C strkey (rejected for Evm→Stellar forward recipients;
//!   frozen public quote API accepts G/M account strkeys only, not contract destinations)

use stellar_xdr::curr::{AccountId, PublicKey, ScAddress, ScVal, Uint256};

use crate::cctp::verifiers::VerifierError;

/// Decoded forward-recipient identity for equality checks (G, M, or C).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StellarRecipientKey {
    Account([u8; 32]),
    Muxed { ed25519: [u8; 32], id: u64 },
    Contract([u8; 32]),
}

impl StellarRecipientKey {
    pub fn to_strkey(&self) -> String {
        match self {
            Self::Account(bytes) => {
                format!("{}", stellar_strkey::ed25519::PublicKey(*bytes))
            }
            Self::Muxed { ed25519, id } => {
                format!(
                    "{}",
                    stellar_strkey::ed25519::MuxedAccount {
                        ed25519: *ed25519,
                        id: *id,
                    }
                )
            }
            Self::Contract(bytes) => format!("{}", stellar_strkey::Contract(*bytes)),
        }
    }
}

/// Parse a stored/quoted recipient strkey (G or M). Contract (C) strkeys parse but are not valid
/// Evm→Stellar corridor recipients per frozen public API.
pub fn parse_recipient_strkey(s: &str) -> Result<StellarRecipientKey, VerifierError> {
    let trimmed = s.trim();
    if let Ok(pk) = stellar_strkey::ed25519::PublicKey::from_string(trimmed) {
        return Ok(StellarRecipientKey::Account(pk.0));
    }
    if let Ok(muxed) = stellar_strkey::ed25519::MuxedAccount::from_string(trimmed) {
        return Ok(StellarRecipientKey::Muxed {
            ed25519: muxed.ed25519,
            id: muxed.id,
        });
    }
    if let Ok(contract) = stellar_strkey::Contract::from_string(trimmed) {
        return Ok(StellarRecipientKey::Contract(contract.0));
    }
    Err(VerifierError::Failed("invalid recipient strkey".into()))
}

/// Compare expected transfer recipient to on-chain forward recipient (case-insensitive strkey).
pub fn stellar_recipients_match(expected: &str, actual: &str) -> Result<(), VerifierError> {
    let expected_key = parse_recipient_strkey(expected)?;
    if matches!(expected_key, StellarRecipientKey::Contract(_)) {
        return Err(VerifierError::Failed(
            "contract recipient not allowed for corridor".into(),
        ));
    }
    let actual_key = parse_recipient_strkey(actual)?;
    if matches!(actual_key, StellarRecipientKey::Contract(_)) {
        return Err(VerifierError::Failed(
            "contract forward recipient unsupported".into(),
        ));
    }
    if expected_key != actual_key {
        return Err(VerifierError::Failed("recipient mismatch".into()));
    }
    Ok(())
}

/// Decode Circle `MuxedAddress` wire in contract event fields to canonical strkey.
pub fn muxed_recipient_from_scval(val: &ScVal) -> Result<String, VerifierError> {
    match val {
        ScVal::Address(addr) => sc_address_to_forward_recipient_strkey(addr),
        _ => Err(VerifierError::Failed("forward_recipient type".into())),
    }
}

pub fn sc_address_to_forward_recipient_strkey(addr: &ScAddress) -> Result<String, VerifierError> {
    match addr {
        ScAddress::Account(account_id) => account_id_to_g_strkey(account_id),
        ScAddress::Contract(_) => Err(VerifierError::Failed(
            "contract forward recipient unsupported".into(),
        )),
        ScAddress::MuxedAccount(muxed) => Ok(format!(
            "{}",
            stellar_strkey::ed25519::MuxedAccount {
                ed25519: muxed.ed25519.0,
                id: muxed.id,
            }
        )),
        ScAddress::ClaimableBalance(_) | ScAddress::LiquidityPool(_) => Err(VerifierError::Failed(
            "unsupported forward recipient address type".into(),
        )),
    }
}

fn account_id_to_g_strkey(account_id: &AccountId) -> Result<String, VerifierError> {
    let PublicKey::PublicKeyTypeEd25519(Uint256(bytes)) = account_id.0;
    Ok(format!("{}", stellar_strkey::ed25519::PublicKey(bytes)))
}

pub fn muxed_account_xdr_to_strkey(
    account: &stellar_xdr::curr::MuxedAccount,
) -> Result<String, VerifierError> {
    use stellar_xdr::curr::MuxedAccount;
    match account {
        MuxedAccount::Ed25519(Uint256(bytes)) => {
            Ok(format!("{}", stellar_strkey::ed25519::PublicKey(*bytes)))
        }
        MuxedAccount::MuxedEd25519(muxed) => Ok(format!(
            "{}",
            stellar_strkey::ed25519::MuxedAccount {
                ed25519: muxed.ed25519.0,
                id: muxed.id,
            }
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stellar_xdr::curr::{
        ContractId, Hash, Limits, MuxedEd25519Account, ScBytes, ScMap, ScMapEntry, ScSymbol,
        WriteXdr,
    };

    const TEST_G: &str = "GA3D5KRYM6CB7OWQ6TWYRR3Z4T7GNZLKERYNZGGA5SOAOPIFY6YQHES5";
    const TEST_M: &str = "MA3D5KRYM6CB7OWQ6TWYRR3Z4T7GNZLKERYNZGGA5SOAOPIFY6YQGAAAAAAAAAPCICBKU";
    const TEST_C: &str = "CA3D5KRYM6CB7OWQ6TWYRR3Z4T7GNZLKERYNZGGA5SOAOPIFY6YQGAXE";

    #[test]
    fn g_strkey_roundtrip_via_scval() {
        let pk = stellar_strkey::ed25519::PublicKey::from_string(TEST_G).unwrap();
        let addr = ScAddress::Account(AccountId(PublicKey::PublicKeyTypeEd25519(Uint256(pk.0))));
        let out = muxed_recipient_from_scval(&ScVal::Address(addr)).unwrap();
        assert_eq!(out, TEST_G);
        stellar_recipients_match(TEST_G, &out).unwrap();
    }

    #[test]
    fn m_strkey_nonzero_id_roundtrip_via_scval() {
        let muxed = stellar_strkey::ed25519::MuxedAccount::from_string(TEST_M).unwrap();
        assert_ne!(muxed.id, 0);
        let addr = ScAddress::MuxedAccount(MuxedEd25519Account {
            id: muxed.id,
            ed25519: Uint256(muxed.ed25519),
        });
        let out = muxed_recipient_from_scval(&ScVal::Address(addr)).unwrap();
        assert_eq!(out, TEST_M);
        stellar_recipients_match(TEST_M, &out).unwrap();
    }

    #[test]
    fn m_max_u64_id_roundtrip() {
        let pk = stellar_strkey::ed25519::PublicKey::from_string(TEST_G).unwrap();
        let addr = ScAddress::MuxedAccount(MuxedEd25519Account {
            id: u64::MAX,
            ed25519: Uint256(pk.0),
        });
        let out = muxed_recipient_from_scval(&ScVal::Address(addr)).unwrap();
        let parsed = stellar_strkey::ed25519::MuxedAccount::from_string(&out).unwrap();
        assert_eq!(parsed.id, u64::MAX);
        assert_eq!(parsed.ed25519, pk.0);
    }

    #[test]
    fn contract_forward_recipient_rejected() {
        let contract = stellar_strkey::Contract::from_string(TEST_C).unwrap();
        let addr = ScAddress::Contract(ContractId(Hash(contract.0)));
        assert!(matches!(
            muxed_recipient_from_scval(&ScVal::Address(addr)),
            Err(VerifierError::Failed(ref m)) if m.contains("contract")
        ));
    }

    #[test]
    fn malformed_map_rejected() {
        let entries = vec![ScMapEntry {
            key: ScVal::Symbol(ScSymbol::try_from("ed25519").unwrap()),
            val: ScVal::Bytes(ScBytes(vec![1, 2, 3].try_into().unwrap())),
        }];
        let map = ScVal::Map(Some(ScMap(entries.try_into().unwrap())));
        assert!(muxed_recipient_from_scval(&map).is_err());
    }

    #[test]
    fn wrong_m_id_does_not_match_g_expected() {
        let pk = stellar_strkey::ed25519::PublicKey::from_string(TEST_G).unwrap();
        let addr = ScAddress::MuxedAccount(MuxedEd25519Account {
            id: 42,
            ed25519: Uint256(pk.0),
        });
        let out = muxed_recipient_from_scval(&ScVal::Address(addr)).unwrap();
        assert!(stellar_recipients_match(TEST_G, &out).is_err());
    }

    #[test]
    fn checksum_invalid_strkey_rejected() {
        assert!(parse_recipient_strkey("GINVALID").is_err());
        assert!(parse_recipient_strkey("MINVALID").is_err());
    }

    #[test]
    fn muxed_strkey_encoding_roundtrip() {
        let key = StellarRecipientKey::Muxed {
            ed25519: [7u8; 32],
            id: 99,
        };
        let s = key.to_strkey();
        assert_eq!(parse_recipient_strkey(&s).unwrap(), key);
    }
}
