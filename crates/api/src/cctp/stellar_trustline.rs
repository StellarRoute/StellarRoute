//! Horizon USDC trustline probe + classic ChangeTrust XDR for EVM→Stellar mint.

use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use stellar_xdr::curr::{
    AlphaNum4, AssetCode4, ChangeTrustAsset, ChangeTrustOp, Limits, Memo, MuxedAccount, Operation,
    OperationBody, Preconditions, PublicKey, SequenceNumber, TimeBounds, TimePoint, Transaction,
    TransactionEnvelope, TransactionExt, TransactionV1Envelope, Uint256, VecM, WriteXdr,
};

use crate::cctp::builders::BuilderError;
use crate::cctp::stellar_muxed::{parse_recipient_strkey, StellarRecipientKey};
use crate::swap::tx::{network_id, transaction_hash_hex, DEFAULT_BASE_FEE, DEFAULT_TIMEOUT_SECS};

/// Circle Testnet USDC classic issuer (SEP-0007 / Circle faucet).
pub const CIRCLE_TESTNET_USDC_ISSUER: &str =
    "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";
pub const CIRCLE_TESTNET_USDC_CODE: &str = "USDC";

/// Max classic trustline limit (INT64_MAX stroops).
pub const CHANGE_TRUST_MAX_LIMIT: i64 = i64::MAX;

/// Underlying G-account that must hold the USDC trustline (demux M→G).
pub fn recipient_trustline_account(recipient: &str) -> Result<String, BuilderError> {
    match parse_recipient_strkey(recipient) {
        Ok(StellarRecipientKey::Account(bytes)) => {
            Ok(format!("{}", stellar_strkey::ed25519::PublicKey(bytes)))
        }
        Ok(StellarRecipientKey::Muxed { ed25519, .. }) => {
            Ok(format!("{}", stellar_strkey::ed25519::PublicKey(ed25519)))
        }
        Ok(StellarRecipientKey::Contract(_)) => Err(BuilderError::Validation(
            "contract recipient not allowed for corridor".into(),
        )),
        Err(_) => Err(BuilderError::Validation(
            "mint recipient must be G or M address".into(),
        )),
    }
}

#[async_trait]
pub trait UsdcTrustlineProbe: Send + Sync {
    async fn has_usdc_trustline(&self, account_g: &str) -> Result<bool, BuilderError>;
}

/// Test double — fixed trustline presence.
pub struct FixedUsdcTrustlineProbe {
    pub present: bool,
}

#[async_trait]
impl UsdcTrustlineProbe for FixedUsdcTrustlineProbe {
    async fn has_usdc_trustline(&self, _account_g: &str) -> Result<bool, BuilderError> {
        Ok(self.present)
    }
}

pub struct HorizonUsdcTrustlineProbe {
    client: Client,
    horizon_urls: Vec<String>,
    asset_code: String,
    asset_issuer: String,
}

impl HorizonUsdcTrustlineProbe {
    pub fn new(horizon_base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        let base = horizon_base_url.trim().trim_end_matches('/').to_string();
        Self {
            client,
            horizon_urls: if base.is_empty() {
                vec!["https://horizon-testnet.stellar.org".into()]
            } else {
                vec![base]
            },
            asset_code: CIRCLE_TESTNET_USDC_CODE.into(),
            asset_issuer: CIRCLE_TESTNET_USDC_ISSUER.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct HorizonAccountBalances {
    balances: Vec<HorizonBalance>,
}

#[derive(Debug, Deserialize)]
struct HorizonBalance {
    #[serde(default)]
    asset_type: String,
    #[serde(default)]
    asset_code: Option<String>,
    #[serde(default)]
    asset_issuer: Option<String>,
}

#[async_trait]
impl UsdcTrustlineProbe for HorizonUsdcTrustlineProbe {
    async fn has_usdc_trustline(&self, account_g: &str) -> Result<bool, BuilderError> {
        let mut last = BuilderError::AccountLookup("no horizon URLs configured".into());
        for base in &self.horizon_urls {
            let url = format!("{base}/accounts/{account_g}");
            match self.client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    let body: HorizonAccountBalances = resp
                        .json()
                        .await
                        .map_err(|e| BuilderError::AccountLookup(e.to_string()))?;
                    return Ok(body.balances.iter().any(|b| {
                        b.asset_type != "native"
                            && b.asset_code.as_deref() == Some(self.asset_code.as_str())
                            && b.asset_issuer.as_deref() == Some(self.asset_issuer.as_str())
                    }));
                }
                Ok(resp) if resp.status().as_u16() == 404 => {
                    return Err(BuilderError::AccountLookup(format!(
                        "account {account_g} not found on Horizon"
                    )));
                }
                Ok(resp) => {
                    last = BuilderError::AccountLookup(format!(
                        "HTTP {} fetching account",
                        resp.status()
                    ));
                }
                Err(e) => last = BuilderError::AccountLookup(e.to_string()),
            }
        }
        Err(last)
    }
}

fn decode_ed25519(account: &str) -> Result<[u8; 32], BuilderError> {
    stellar_strkey::ed25519::PublicKey::from_string(account.trim())
        .map(|pk| pk.0)
        .map_err(|_| BuilderError::Validation(format!("invalid Stellar account: {account}")))
}

fn muxed_account(account: &str) -> Result<MuxedAccount, BuilderError> {
    Ok(MuxedAccount::Ed25519(Uint256(decode_ed25519(account)?)))
}

fn account_id(account: &str) -> Result<stellar_xdr::curr::AccountId, BuilderError> {
    Ok(stellar_xdr::curr::AccountId(
        PublicKey::PublicKeyTypeEd25519(Uint256(decode_ed25519(account)?)),
    ))
}

fn usdc_change_trust_asset() -> Result<ChangeTrustAsset, BuilderError> {
    let mut code = [0u8; 4];
    let bytes = CIRCLE_TESTNET_USDC_CODE.as_bytes();
    code[..bytes.len()].copy_from_slice(bytes);
    Ok(ChangeTrustAsset::CreditAlphanum4(AlphaNum4 {
        asset_code: AssetCode4(code),
        issuer: account_id(CIRCLE_TESTNET_USDC_ISSUER)?,
    }))
}

/// Build an unsigned classic ChangeTrust envelope for Circle Testnet USDC.
pub fn build_unsigned_change_trust_xdr(
    source_g: &str,
    account_sequence: i64,
    network_passphrase: &str,
    base_fee: u32,
    timeout_secs: u64,
) -> Result<(String, String), BuilderError> {
    let source = muxed_account(source_g)?;
    let source_sequence = account_sequence.saturating_add(1);
    let now = Utc::now().timestamp().max(0) as u64;
    let timebounds_max = now.saturating_add(timeout_secs.max(1));
    let fee = if base_fee == 0 {
        DEFAULT_BASE_FEE
    } else {
        base_fee
    };

    let op = Operation {
        source_account: None,
        body: OperationBody::ChangeTrust(ChangeTrustOp {
            line: usdc_change_trust_asset()?,
            limit: CHANGE_TRUST_MAX_LIMIT,
        }),
    };

    let tx = Transaction {
        source_account: source,
        fee,
        seq_num: SequenceNumber(source_sequence),
        cond: Preconditions::Time(TimeBounds {
            min_time: TimePoint(0),
            max_time: TimePoint(timebounds_max),
        }),
        memo: Memo::None,
        operations: vec![op]
            .try_into()
            .map_err(|_| BuilderError::Encoding("operations vec".into()))?,
        ext: TransactionExt::V0,
    };

    let tx_hash = transaction_hash_hex(&tx, network_passphrase)
        .map_err(|e| BuilderError::Encoding(e.to_string()))?;
    let _ = network_id(network_passphrase);
    let envelope = TransactionEnvelope::Tx(TransactionV1Envelope {
        tx,
        signatures: VecM::default(),
    });
    let xdr_envelope = envelope
        .to_xdr_base64(Limits::none())
        .map_err(|e| BuilderError::Encoding(e.to_string()))?;
    Ok((xdr_envelope, tx_hash))
}

pub fn default_change_trust_timeout_secs() -> u64 {
    DEFAULT_TIMEOUT_SECS
}

/// Confirm the unsigned envelope is a single ChangeTrust for USDC (tests / gates).
pub fn is_usdc_change_trust_envelope(xdr: &str) -> bool {
    use stellar_xdr::curr::ReadXdr;
    let Ok(envelope) = TransactionEnvelope::from_xdr_base64(xdr.trim(), Limits::none()) else {
        return false;
    };
    let TransactionEnvelope::Tx(v1) = envelope else {
        return false;
    };
    if v1.tx.operations.len() != 1 {
        return false;
    }
    match &v1.tx.operations[0].body {
        OperationBody::ChangeTrust(op) => match &op.line {
            ChangeTrustAsset::CreditAlphanum4(a) => {
                let code = {
                    let raw = a.asset_code.0;
                    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
                    String::from_utf8_lossy(&raw[..end]).into_owned()
                };
                code == CIRCLE_TESTNET_USDC_CODE
                    && {
                        let issuer = match &a.issuer.0 {
                            PublicKey::PublicKeyTypeEd25519(u) => {
                                format!("{}", stellar_strkey::ed25519::PublicKey(u.0))
                            }
                        };
                        issuer == CIRCLE_TESTNET_USDC_ISSUER
                    }
                    && op.limit == CHANGE_TRUST_MAX_LIMIT
            }
            _ => false,
        },
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cctp::config::STELLAR_TESTNET_PASSPHRASE;

    #[test]
    fn demux_g_and_m_to_underlying_g() {
        let g = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";
        assert_eq!(recipient_trustline_account(g).unwrap(), g);

        let pk = stellar_strkey::ed25519::PublicKey::from_string(g).unwrap();
        let m = stellar_strkey::ed25519::MuxedAccount {
            ed25519: pk.0,
            id: 42,
        }
        .to_string();
        assert_eq!(recipient_trustline_account(&m).unwrap(), g);

        assert!(matches!(
            recipient_trustline_account("CA66Q2WFBND6V4UEB7RD4SAXSVIWMD6RA4X3U32ELVFGXV5PJK4T4VSZ"),
            Err(BuilderError::Validation(_))
        ));
    }

    #[test]
    fn change_trust_xdr_is_usdc_change_trust() {
        let (xdr, hash) = build_unsigned_change_trust_xdr(
            "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            100,
            STELLAR_TESTNET_PASSPHRASE,
            DEFAULT_BASE_FEE,
            120,
        )
        .unwrap();
        assert!(!hash.is_empty());
        assert!(is_usdc_change_trust_envelope(&xdr));
    }

    #[tokio::test]
    async fn fixed_probe_reports_presence() {
        let probe = FixedUsdcTrustlineProbe { present: false };
        assert!(!probe
            .has_usdc_trustline("GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF")
            .await
            .unwrap());
        let probe = FixedUsdcTrustlineProbe { present: true };
        assert!(probe
            .has_usdc_trustline("GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF")
            .await
            .unwrap());
    }
}
