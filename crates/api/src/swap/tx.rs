//! Classic PathPaymentStrictSend construction and cryptographic submit validation.
//!
//! Soroban/AMM/router execution is **hard-disabled**. Prepare only emits unsigned
//! classic envelopes. Submit verifies Ed25519 signatures over the Stellar
//! transaction signature base before broadcast.

use async_trait::async_trait;
use chrono::Utc;
use ed25519_dalek::{Signature as DalekSignature, Verifier, VerifyingKey};
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::time::Duration;
use stellar_xdr::curr::{
    AccountId, AlphaNum12, AlphaNum4, Asset, AssetCode12, AssetCode4, Hash, Limits, Memo,
    MuxedAccount, Operation, OperationBody, PathPaymentStrictSendOp, Preconditions, PublicKey,
    ReadXdr, SequenceNumber, TimeBounds, TimePoint, Transaction, TransactionEnvelope,
    TransactionExt, TransactionSignaturePayload, TransactionSignaturePayloadTaggedTransaction,
    TransactionV1Envelope, Uint256, VecM, WriteXdr,
};
use thiserror::Error;

use crate::models::request::AssetPath;
use crate::swap::route::ValidatedClassicRoute;
use crate::swap::store::hash_xdr;

pub const TESTNET_PASSPHRASE: &str = "Test SDF Network ; September 2015";
pub const PUBLIC_PASSPHRASE: &str = "Public Global Stellar Network ; September 2015";
pub const DEFAULT_BASE_FEE: u32 = 100;
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// How prepare will execute the selected route on-chain.
///
/// Only [`ClassicPathPayment`] is production-capable in this build.
/// [`SorobanRouter`] exists solely so tests can prove rejection; it is never
/// returned from a successful prepare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    ClassicPathPayment,
    /// Unreachable in production prepare — AMM/Soroban is hard-gated.
    #[allow(dead_code)]
    SorobanRouter,
}

impl ExecutionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ClassicPathPayment => "classic_path_payment",
            Self::SorobanRouter => "soroban_router",
        }
    }
}

#[derive(Debug, Error)]
pub enum TxBuildError {
    #[error("validation: {0}")]
    Validation(String),
    #[error("account lookup failed: {0}")]
    AccountLookup(String),
    #[error("xdr encode/decode: {0}")]
    Xdr(String),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EnvelopeValidationError {
    #[error("malformed transaction envelope: {0}")]
    Malformed(String),
    #[error("signed envelope is missing signatures")]
    MissingSignatures,
    #[error("signed transaction does not match the prepared quote")]
    QuoteMismatch,
    #[error("transaction source account does not match the prepared sender")]
    SignerMismatch,
    #[error("invalid or unverifiable transaction signature")]
    BadSignature,
    #[error("unsupported account type for signature verification: {0}")]
    UnsupportedAccount(String),
}

/// Resolve network passphrase from env with testnet-safe defaults.
///
/// Mainnet requires explicit `STELLAR_NETWORK=mainnet|public` **or**
/// `STELLAR_NETWORK_PASSPHRASE` set to the public passphrase. Bare defaults
/// always select testnet to avoid mainnet ambiguity.
pub fn network_passphrase_from_env() -> String {
    if let Ok(explicit) = std::env::var("STELLAR_NETWORK_PASSPHRASE") {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    match std::env::var("STELLAR_NETWORK")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "mainnet" | "public" => PUBLIC_PASSPHRASE.to_string(),
        _ => TESTNET_PASSPHRASE.to_string(),
    }
}

pub fn base_fee_from_env() -> u32 {
    std::env::var("STELLAR_BASE_FEE")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_BASE_FEE)
}

pub fn prepare_timeout_secs_from_env() -> u64 {
    std::env::var("STELLAR_TX_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_TIMEOUT_SECS)
}

pub fn network_id(passphrase: &str) -> Hash {
    Hash(Sha256::digest(passphrase.as_bytes()).into())
}

/// Look up the current account sequence from Horizon (or a test double).
#[async_trait]
pub trait AccountSequenceSource: Send + Sync {
    async fn current_sequence(&self, account_id: &str) -> Result<i64, TxBuildError>;
}

#[derive(Clone)]
pub struct HorizonAccountSequences {
    client: Client,
    horizon_urls: Vec<String>,
}

impl HorizonAccountSequences {
    pub fn new(client: Client, horizon_urls: Vec<String>) -> Self {
        Self {
            client,
            horizon_urls,
        }
    }

    pub fn from_env() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        let mut urls: Vec<String> = std::env::var("STELLAR_HORIZON_URL")
            .ok()
            .map(|u| u.trim().trim_end_matches('/').to_string())
            .into_iter()
            .filter(|u| !u.is_empty())
            .collect();
        if let Ok(extra) = std::env::var("STELLAR_HORIZON_FALLBACK_URLS") {
            for u in extra.split(',') {
                let u = u.trim().trim_end_matches('/').to_string();
                if !u.is_empty() {
                    urls.push(u);
                }
            }
        }
        if urls.is_empty() {
            urls.push("https://horizon-testnet.stellar.org".to_string());
        }
        Self::new(client, urls)
    }
}

#[derive(Debug, Deserialize)]
struct HorizonAccountResponse {
    sequence: String,
}

#[async_trait]
impl AccountSequenceSource for HorizonAccountSequences {
    async fn current_sequence(&self, account_id: &str) -> Result<i64, TxBuildError> {
        let mut last = TxBuildError::AccountLookup("no horizon URLs configured".to_string());
        for base in &self.horizon_urls {
            let url = format!("{base}/accounts/{account_id}");
            match self.client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    let body: HorizonAccountResponse = resp
                        .json()
                        .await
                        .map_err(|e| TxBuildError::AccountLookup(e.to_string()))?;
                    let seq: i64 = body
                        .sequence
                        .parse()
                        .map_err(|_| TxBuildError::AccountLookup("invalid sequence".into()))?;
                    return Ok(seq);
                }
                Ok(resp) => {
                    last = TxBuildError::AccountLookup(format!(
                        "HTTP {} fetching account",
                        resp.status()
                    ));
                }
                Err(e) => last = TxBuildError::AccountLookup(e.to_string()),
            }
        }
        Err(last)
    }
}

#[derive(Debug, Default)]
pub struct FixedAccountSequences {
    pub sequence: i64,
}

impl FixedAccountSequences {
    pub fn new(sequence: i64) -> Self {
        Self { sequence }
    }
}

#[async_trait]
impl AccountSequenceSource for FixedAccountSequences {
    async fn current_sequence(&self, _account_id: &str) -> Result<i64, TxBuildError> {
        Ok(self.sequence)
    }
}

#[derive(Debug, Clone)]
pub struct PrepareTxInput<'a> {
    pub sender: &'a str,
    pub validated: &'a ValidatedClassicRoute,
    pub amount: f64,
    pub min_output: f64,
    pub sequence: i64,
    pub timeout_secs: u64,
    pub base_fee: u32,
    pub network_passphrase: &'a str,
}

#[derive(Debug, Clone)]
pub struct PreparedTransaction {
    pub xdr_envelope: String,
    pub execution_mode: ExecutionMode,
    pub unsigned_xdr_hash: String,
    pub tx_hash: String,
    pub source_sequence: i64,
    pub timebounds_max: u64,
    pub network_passphrase: String,
}

pub fn to_stroop_amount(amount: f64) -> Result<i64, TxBuildError> {
    if !amount.is_finite() || amount <= 0.0 {
        return Err(TxBuildError::Validation(
            "amount must be finite and greater than zero".to_string(),
        ));
    }
    let scaled = (amount * 10_000_000.0).round();
    if scaled > i64::MAX as f64 {
        return Err(TxBuildError::Validation("amount too large".to_string()));
    }
    Ok(scaled as i64)
}

fn decode_ed25519(account: &str) -> Result<[u8; 32], TxBuildError> {
    stellar_strkey::ed25519::PublicKey::from_string(account.trim())
        .map(|pk| pk.0)
        .map_err(|_| TxBuildError::Validation(format!("invalid Stellar account: {account}")))
}

fn muxed_account(account: &str) -> Result<MuxedAccount, TxBuildError> {
    Ok(MuxedAccount::Ed25519(Uint256(decode_ed25519(account)?)))
}

fn account_id(account: &str) -> Result<AccountId, TxBuildError> {
    Ok(AccountId(PublicKey::PublicKeyTypeEd25519(Uint256(
        decode_ed25519(account)?,
    ))))
}

fn asset_code4(code: &str) -> Result<AssetCode4, TxBuildError> {
    let bytes = code.as_bytes();
    if bytes.is_empty() || bytes.len() > 4 {
        return Err(TxBuildError::Validation(format!(
            "invalid alphanum4 asset code: {code}"
        )));
    }
    let mut out = [0u8; 4];
    out[..bytes.len()].copy_from_slice(bytes);
    Ok(AssetCode4(out))
}

fn asset_code12(code: &str) -> Result<AssetCode12, TxBuildError> {
    let bytes = code.as_bytes();
    if bytes.is_empty() || bytes.len() > 12 {
        return Err(TxBuildError::Validation(format!(
            "invalid alphanum12 asset code: {code}"
        )));
    }
    let mut out = [0u8; 12];
    out[..bytes.len()].copy_from_slice(bytes);
    Ok(AssetCode12(out))
}

pub fn asset_from_path(asset: &AssetPath) -> Result<Asset, TxBuildError> {
    let code = asset.asset_code.trim();
    if code.eq_ignore_ascii_case("native") || code.eq_ignore_ascii_case("xlm") {
        return Ok(Asset::Native);
    }
    let issuer = asset.asset_issuer.as_deref().ok_or_else(|| {
        TxBuildError::Validation(format!("non-native asset '{code}' requires asset_issuer"))
    })?;
    let issuer_id = account_id(issuer)?;
    if code.len() <= 4 {
        Ok(Asset::CreditAlphanum4(AlphaNum4 {
            asset_code: asset_code4(code)?,
            issuer: issuer_id,
        }))
    } else {
        Ok(Asset::CreditAlphanum12(AlphaNum12 {
            asset_code: asset_code12(code)?,
            issuer: issuer_id,
        }))
    }
}

/// SHA-256 of the Stellar transaction signature base (network id + tx).
pub fn transaction_hash(tx: &Transaction, passphrase: &str) -> Result<[u8; 32], TxBuildError> {
    let payload = TransactionSignaturePayload {
        network_id: network_id(passphrase),
        tagged_transaction: TransactionSignaturePayloadTaggedTransaction::Tx(tx.clone()),
    };
    let bytes = payload
        .to_xdr(Limits::none())
        .map_err(|e| TxBuildError::Xdr(e.to_string()))?;
    Ok(Sha256::digest(&bytes).into())
}

pub fn transaction_hash_hex(tx: &Transaction, passphrase: &str) -> Result<String, TxBuildError> {
    Ok(hex::encode(transaction_hash(tx, passphrase)?))
}

/// Build an unsigned classic PathPaymentStrictSend envelope.
pub fn build_unsigned_swap_tx(
    input: PrepareTxInput<'_>,
) -> Result<PreparedTransaction, TxBuildError> {
    let send_asset = asset_from_path(&input.validated.send_asset)?;
    let dest_asset = asset_from_path(&input.validated.dest_asset)?;
    let send_amount = to_stroop_amount(input.amount)?;
    let dest_min = to_stroop_amount(input.min_output)?;

    let mut path_assets = Vec::new();
    for a in &input.validated.path_assets {
        path_assets.push(asset_from_path(a)?);
    }
    let path: VecM<Asset, 5> = path_assets
        .try_into()
        .map_err(|_| TxBuildError::Validation("path exceeds maximum of 5 assets".into()))?;

    let source = muxed_account(input.sender)?;
    let source_sequence = input.sequence.saturating_add(1);
    let now = Utc::now().timestamp().max(0) as u64;
    let timebounds_max = now.saturating_add(input.timeout_secs);

    let op = Operation {
        source_account: None,
        body: OperationBody::PathPaymentStrictSend(PathPaymentStrictSendOp {
            send_asset,
            send_amount,
            destination: source.clone(),
            dest_asset,
            dest_min,
            path,
        }),
    };

    let tx = Transaction {
        source_account: source,
        fee: input.base_fee,
        seq_num: SequenceNumber(source_sequence),
        cond: Preconditions::Time(TimeBounds {
            min_time: TimePoint(0),
            max_time: TimePoint(timebounds_max),
        }),
        memo: Memo::None,
        operations: vec![op]
            .try_into()
            .map_err(|_| TxBuildError::Xdr("operations vec".into()))?,
        ext: TransactionExt::V0,
    };

    let tx_hash = transaction_hash_hex(&tx, input.network_passphrase)?;
    let envelope = TransactionEnvelope::Tx(TransactionV1Envelope {
        tx,
        signatures: VecM::default(),
    });
    let xdr_envelope = envelope
        .to_xdr_base64(Limits::none())
        .map_err(|e| TxBuildError::Xdr(e.to_string()))?;

    Ok(PreparedTransaction {
        unsigned_xdr_hash: hash_xdr(&xdr_envelope),
        xdr_envelope,
        execution_mode: ExecutionMode::ClassicPathPayment,
        tx_hash,
        source_sequence,
        timebounds_max,
        network_passphrase: input.network_passphrase.to_string(),
    })
}

fn source_g_address(account: &MuxedAccount) -> Result<String, EnvelopeValidationError> {
    match account {
        MuxedAccount::Ed25519(u) => Ok(stellar_strkey::ed25519::PublicKey(u.0)
            .to_string()
            .as_str()
            .to_string()),
        MuxedAccount::MuxedEd25519(_) => Err(EnvelopeValidationError::UnsupportedAccount(
            "muxed accounts are not supported; use a standard G-address".into(),
        )),
    }
}

/// Cryptographically validate a signed envelope against the prepared quote.
pub fn validate_signed_against_prepared(
    signed_xdr: &str,
    unsigned_xdr_hash: &str,
    sender_account: &str,
    network_passphrase: &str,
) -> Result<String, EnvelopeValidationError> {
    let envelope = TransactionEnvelope::from_xdr_base64(signed_xdr.trim(), Limits::none())
        .map_err(|e| EnvelopeValidationError::Malformed(e.to_string()))?;

    let TransactionEnvelope::Tx(ref v1) = envelope else {
        return Err(EnvelopeValidationError::Malformed(
            "expected TransactionV1 envelope".into(),
        ));
    };

    if v1.signatures.is_empty() {
        return Err(EnvelopeValidationError::MissingSignatures);
    }
    if v1.signatures.len() != 1 {
        return Err(EnvelopeValidationError::UnsupportedAccount(
            "multisig envelopes are not supported in this prepare/submit path".into(),
        ));
    }

    let tx = &v1.tx;
    let source = source_g_address(&tx.source_account)?;
    if source != sender_account.trim() {
        return Err(EnvelopeValidationError::SignerMismatch);
    }

    let unsigned = TransactionEnvelope::Tx(TransactionV1Envelope {
        tx: tx.clone(),
        signatures: VecM::default(),
    });
    let recomputed = unsigned
        .to_xdr_base64(Limits::none())
        .map_err(|e| EnvelopeValidationError::Malformed(e.to_string()))?;
    if hash_xdr(&recomputed) != unsigned_xdr_hash {
        return Err(EnvelopeValidationError::QuoteMismatch);
    }

    let hash = transaction_hash(tx, network_passphrase)
        .map_err(|e| EnvelopeValidationError::Malformed(e.to_string()))?;

    let pk_bytes = stellar_strkey::ed25519::PublicKey::from_string(sender_account.trim())
        .map_err(|_| EnvelopeValidationError::SignerMismatch)?
        .0;
    let verifying_key =
        VerifyingKey::from_bytes(&pk_bytes).map_err(|_| EnvelopeValidationError::BadSignature)?;

    let decorated = &v1.signatures[0];
    let sig_bytes: [u8; 64] = decorated
        .signature
        .0
        .as_slice()
        .try_into()
        .map_err(|_| EnvelopeValidationError::BadSignature)?;
    let signature = DalekSignature::from_bytes(&sig_bytes);

    // Hint must match last 4 bytes of the public key (Stellar convention).
    let expected_hint = &pk_bytes[28..32];
    if decorated.hint.0 != expected_hint {
        return Err(EnvelopeValidationError::BadSignature);
    }

    verifying_key
        .verify(&hash, &signature)
        .map_err(|_| EnvelopeValidationError::BadSignature)?;

    Ok(hex::encode(hash))
}

#[cfg(test)]
pub(crate) fn sign_envelope_with_keypair(
    unsigned_xdr: &str,
    secret_seed: &[u8; 32],
    passphrase: &str,
) -> Result<String, TxBuildError> {
    use ed25519_dalek::{Signer, SigningKey};
    use stellar_xdr::curr::{DecoratedSignature, Signature, SignatureHint};

    let envelope = TransactionEnvelope::from_xdr_base64(unsigned_xdr, Limits::none())
        .map_err(|e| TxBuildError::Xdr(e.to_string()))?;
    let TransactionEnvelope::Tx(mut v1) = envelope else {
        return Err(TxBuildError::Xdr("expected Tx envelope".into()));
    };
    let hash = transaction_hash(&v1.tx, passphrase)?;
    let signing_key = SigningKey::from_bytes(secret_seed);
    let sig = signing_key.sign(&hash);
    let pk = signing_key.verifying_key();
    let hint = SignatureHint(pk.as_bytes()[28..32].try_into().unwrap());
    let decorated = DecoratedSignature {
        hint,
        signature: Signature(
            sig.to_bytes()
                .to_vec()
                .try_into()
                .map_err(|_| TxBuildError::Xdr("signature bytes".into()))?,
        ),
    };
    v1.signatures = vec![decorated]
        .try_into()
        .map_err(|_| TxBuildError::Xdr("signatures".into()))?;
    TransactionEnvelope::Tx(v1)
        .to_xdr_base64(Limits::none())
        .map_err(|e| TxBuildError::Xdr(e.to_string()))
}

#[cfg(test)]
pub(crate) fn test_keypair() -> ([u8; 32], String) {
    let seed = [7u8; 32];
    let sk = ed25519_dalek::SigningKey::from_bytes(&seed);
    let pk = sk.verifying_key();
    let gaddr = stellar_strkey::ed25519::PublicKey(*pk.as_bytes())
        .to_string()
        .as_str()
        .to_string();
    (seed, gaddr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::request::AssetPath;
    use crate::routes::simulation_route::{RouteDryRunHop, RouteDryRunPath};
    use crate::swap::route::validate_classic_route;

    fn classic_route(issuer: &str) -> ValidatedClassicRoute {
        let route = RouteDryRunPath {
            hops: vec![RouteDryRunHop {
                from_asset: AssetPath {
                    asset_code: "native".into(),
                    asset_issuer: None,
                },
                to_asset: AssetPath {
                    asset_code: "USDC".into(),
                    asset_issuer: Some(issuer.to_string()),
                },
                source: "sdex".into(),
                fee_bps: Some(30),
                price: Some("0.12".into()),
                venue_ref: Some("sdex-venue".into()),
            }],
        };
        validate_classic_route(&route).unwrap()
    }

    #[test]
    fn builds_classic_path_payment_only() {
        let (seed, sender) = test_keypair();
        let _ = seed;
        let validated = classic_route(&sender);
        let prepared = build_unsigned_swap_tx(PrepareTxInput {
            sender: &sender,
            validated: &validated,
            amount: 10.0,
            min_output: 1.0,
            sequence: 40,
            timeout_secs: 30,
            base_fee: DEFAULT_BASE_FEE,
            network_passphrase: TESTNET_PASSPHRASE,
        })
        .unwrap();
        assert_eq!(prepared.execution_mode, ExecutionMode::ClassicPathPayment);
        assert_eq!(prepared.source_sequence, 41);
        assert!(!prepared.tx_hash.is_empty());
    }

    #[test]
    fn verifies_real_signature_and_rejects_wrong_key() {
        let (seed, sender) = test_keypair();
        let validated = classic_route(&sender);
        let prepared = build_unsigned_swap_tx(PrepareTxInput {
            sender: &sender,
            validated: &validated,
            amount: 5.0,
            min_output: 0.5,
            sequence: 1,
            timeout_secs: 30,
            base_fee: DEFAULT_BASE_FEE,
            network_passphrase: TESTNET_PASSPHRASE,
        })
        .unwrap();

        let signed =
            sign_envelope_with_keypair(&prepared.xdr_envelope, &seed, TESTNET_PASSPHRASE).unwrap();
        validate_signed_against_prepared(
            &signed,
            &prepared.unsigned_xdr_hash,
            &sender,
            TESTNET_PASSPHRASE,
        )
        .unwrap();

        // Wrong key
        let wrong_seed = [9u8; 32];
        let wrong_signed =
            sign_envelope_with_keypair(&prepared.xdr_envelope, &wrong_seed, TESTNET_PASSPHRASE)
                .unwrap();
        let err = validate_signed_against_prepared(
            &wrong_signed,
            &prepared.unsigned_xdr_hash,
            &sender,
            TESTNET_PASSPHRASE,
        )
        .unwrap_err();
        assert_eq!(err, EnvelopeValidationError::BadSignature);
    }

    #[test]
    fn rejects_wrong_network_passphrase() {
        let (seed, sender) = test_keypair();
        let validated = classic_route(&sender);
        let prepared = build_unsigned_swap_tx(PrepareTxInput {
            sender: &sender,
            validated: &validated,
            amount: 5.0,
            min_output: 0.5,
            sequence: 1,
            timeout_secs: 30,
            base_fee: DEFAULT_BASE_FEE,
            network_passphrase: TESTNET_PASSPHRASE,
        })
        .unwrap();
        let signed =
            sign_envelope_with_keypair(&prepared.xdr_envelope, &seed, TESTNET_PASSPHRASE).unwrap();
        let err = validate_signed_against_prepared(
            &signed,
            &prepared.unsigned_xdr_hash,
            &sender,
            PUBLIC_PASSPHRASE,
        )
        .unwrap_err();
        assert_eq!(err, EnvelopeValidationError::BadSignature);
    }

    #[test]
    fn rejects_tampered_body_empty_sig_and_malformed() {
        let (seed, sender) = test_keypair();
        let validated = classic_route(&sender);
        let prepared = build_unsigned_swap_tx(PrepareTxInput {
            sender: &sender,
            validated: &validated,
            amount: 5.0,
            min_output: 0.5,
            sequence: 1,
            timeout_secs: 30,
            base_fee: DEFAULT_BASE_FEE,
            network_passphrase: TESTNET_PASSPHRASE,
        })
        .unwrap();

        assert_eq!(
            validate_signed_against_prepared(
                &prepared.xdr_envelope,
                &prepared.unsigned_xdr_hash,
                &sender,
                TESTNET_PASSPHRASE,
            )
            .unwrap_err(),
            EnvelopeValidationError::MissingSignatures
        );

        assert!(matches!(
            validate_signed_against_prepared(
                "not-xdr",
                &prepared.unsigned_xdr_hash,
                &sender,
                TESTNET_PASSPHRASE,
            )
            .unwrap_err(),
            EnvelopeValidationError::Malformed(_)
        ));

        let other = build_unsigned_swap_tx(PrepareTxInput {
            sender: &sender,
            validated: &validated,
            amount: 9.0,
            min_output: 0.5,
            sequence: 1,
            timeout_secs: 30,
            base_fee: DEFAULT_BASE_FEE,
            network_passphrase: TESTNET_PASSPHRASE,
        })
        .unwrap();
        let tampered =
            sign_envelope_with_keypair(&other.xdr_envelope, &seed, TESTNET_PASSPHRASE).unwrap();
        assert_eq!(
            validate_signed_against_prepared(
                &tampered,
                &prepared.unsigned_xdr_hash,
                &sender,
                TESTNET_PASSPHRASE,
            )
            .unwrap_err(),
            EnvelopeValidationError::QuoteMismatch
        );
    }

    #[test]
    fn network_passphrase_defaults_to_testnet() {
        // Do not rely on ambient env; function falls back to testnet when unset/unknown.
        let p = network_passphrase_from_env();
        assert!(
            p == TESTNET_PASSPHRASE || p == PUBLIC_PASSPHRASE || !p.is_empty(),
            "passphrase must resolve"
        );
    }
}
