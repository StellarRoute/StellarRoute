//! Chain-scoped asset identifiers for multi-chain routing foundations.
//!
//! # Wire format (CAIP-inspired, not strict CAIP everywhere)
//!
//! Identifiers use the CAIP-19 shape `{chain_id}/{asset_namespace}:{asset_reference}`:
//!
//! | Chain | `chain_id` form | Notes |
//! |-------|-----------------|-------|
//! | Stellar | `stellar:{network}` | CAIP-2 compatible (`pubnet` / `testnet`) |
//! | EVM | `eip155:{chain_id}` | Strict CAIP-2 |
//! | Bitcoin | `bip122:{genesis_hash}` | 64 lowercase hex chars (structural) |
//! | Solana | `solana:{cluster}` | **Internal label** (`mainnet` / `devnet` / `testnet`), not a genesis-hash CAIP-2 |
//! | TRON | `tron:{network}` | **Internal label** (`mainnet` / `nile` / `shasta`), not full CAIP-2 |
//!
//! Native assets use numeric SLIP-44 coin types (never `slip44:native`):
//! BTC=`0`, ETH=`60`, XLM=`148`, TRX=`195`, SOL=`501`.
//!
//! Token/address validation is **structural only** (no checksum verification).
//!
//! Legacy Stellar API ids (`native`, `XLM`, `CODE:ISSUER`) map via
//! [`ChainAsset::from_stellar_legacy`]. Issuer case is preserved in chain form;
//! asset codes are uppercased. v1 cache helpers keep historical
//! [`crate::normalize_asset`] byte-for-byte behaviour separately.

use crate::error::{Result, RoutingError};
use serde::{Deserialize, Serialize};
use std::fmt;

/// SLIP-44 coin type for Bitcoin.
pub const SLIP44_BTC: u32 = 0;
/// SLIP-44 coin type for Ethereum.
pub const SLIP44_ETH: u32 = 60;
/// SLIP-44 coin type for Stellar Lumens.
pub const SLIP44_XLM: u32 = 148;
/// SLIP-44 coin type for TRON.
pub const SLIP44_TRX: u32 = 195;
/// SLIP-44 coin type for Solana.
pub const SLIP44_SOL: u32 = 501;

const STELLAR_ISSUER_ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
const BASE58_ALPHABET: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// Well-known chain namespaces supported by the foundation layer.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChainId {
    /// Stellar network (`stellar:pubnet`, `stellar:testnet`, …).
    Stellar { network: String },
    /// EVM chain via EIP-155 (`eip155:1`, …).
    Eip155 { chain_id: u64 },
    /// Solana cluster label (`solana:mainnet`, …) — internal, not genesis-hash CAIP-2.
    Solana { cluster: String },
    /// Bitcoin via BIP-122 genesis hash reference (64 hex chars).
    Bitcoin { genesis_hash: String },
    /// TRON network label (`tron:mainnet`, …) — internal convenience id.
    Tron { network: String },
}

impl ChainId {
    pub fn stellar_pubnet() -> Self {
        Self::Stellar {
            network: "pubnet".to_string(),
        }
    }

    pub fn ethereum_mainnet() -> Self {
        Self::Eip155 { chain_id: 1 }
    }

    pub fn solana_mainnet() -> Self {
        Self::Solana {
            cluster: "mainnet".to_string(),
        }
    }

    pub fn bitcoin_mainnet() -> Self {
        // Full BIP-122 genesis hash (64 hex chars). Structural only — not a
        // proof of chain identity beyond format.
        Self::Bitcoin {
            genesis_hash: "000000000019d6689c085ae165831e9345904ddf3418b7bb805d31aeb52cafd2"
                .to_string(),
        }
    }

    pub fn tron_mainnet() -> Self {
        Self::Tron {
            network: "mainnet".to_string(),
        }
    }

    /// Expected SLIP-44 coin type for this chain's native asset.
    pub fn native_slip44(&self) -> u32 {
        match self {
            Self::Bitcoin { .. } => SLIP44_BTC,
            Self::Eip155 { .. } => SLIP44_ETH,
            Self::Stellar { .. } => SLIP44_XLM,
            Self::Tron { .. } => SLIP44_TRX,
            Self::Solana { .. } => SLIP44_SOL,
        }
    }

    /// Chain id string (CAIP-2 where applicable; see module docs for exceptions).
    pub fn to_caip2(&self) -> String {
        match self {
            Self::Stellar { network } => format!("stellar:{network}"),
            Self::Eip155 { chain_id } => format!("eip155:{chain_id}"),
            Self::Solana { cluster } => format!("solana:{cluster}"),
            Self::Bitcoin { genesis_hash } => format!("bip122:{genesis_hash}"),
            Self::Tron { network } => format!("tron:{network}"),
        }
    }

    pub fn parse_caip2(input: &str) -> Result<Self> {
        let (namespace, reference) = split_once_colon(input)
            .ok_or_else(|| RoutingError::InvalidAsset(format!("invalid chain id: {input}")))?;

        match namespace {
            "stellar" => {
                let network = reference.to_ascii_lowercase();
                if network != "pubnet" && network != "testnet" {
                    return Err(RoutingError::InvalidAsset(format!(
                        "unsupported stellar network: {reference}"
                    )));
                }
                Ok(Self::Stellar { network })
            }
            "eip155" => {
                let chain_id = reference.parse::<u64>().map_err(|_| {
                    RoutingError::InvalidAsset(format!("invalid eip155 chain id: {reference}"))
                })?;
                Ok(Self::Eip155 { chain_id })
            }
            "solana" => {
                let cluster = reference.to_ascii_lowercase();
                if !matches!(cluster.as_str(), "mainnet" | "devnet" | "testnet") {
                    return Err(RoutingError::InvalidAsset(format!(
                        "unsupported solana cluster label: {reference}"
                    )));
                }
                Ok(Self::Solana { cluster })
            }
            "bip122" => {
                validate_bip122_genesis(reference)?;
                Ok(Self::Bitcoin {
                    genesis_hash: reference.to_ascii_lowercase(),
                })
            }
            "tron" => {
                let network = reference.to_ascii_lowercase();
                if !matches!(network.as_str(), "mainnet" | "nile" | "shasta") {
                    return Err(RoutingError::InvalidAsset(format!(
                        "unsupported tron network label: {reference}"
                    )));
                }
                Ok(Self::Tron { network })
            }
            other => Err(RoutingError::InvalidAsset(format!(
                "unsupported chain namespace: {other}"
            ))),
        }
    }
}

impl fmt::Display for ChainId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_caip2())
    }
}

/// Asset reference within a chain.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AssetReference {
    /// Native gas/settlement asset (XLM, ETH, SOL, BTC, TRX).
    Native,
    /// Stellar issued credit asset.
    StellarCredit { code: String, issuer: String },
    /// Ethereum / EVM ERC-20 (structural `0x` + 40 hex; no checksum verify).
    Erc20 { address: String },
    /// Solana SPL token mint (structural base58).
    SplToken { mint: String },
    /// TRON TRC-20 contract (structural Base58 `T…` address).
    Trc20 { address: String },
}

impl AssetReference {
    /// `{asset_namespace}:{asset_reference}` suffix for the given chain.
    pub fn to_caip19_suffix(&self, chain: &ChainId) -> String {
        match self {
            Self::Native => format!("slip44:{}", chain.native_slip44()),
            Self::StellarCredit { code, issuer } => {
                format!("stellar:{}:{}", code, issuer)
            }
            Self::Erc20 { address } => format!("erc20:{}", address),
            Self::SplToken { mint } => format!("token:{}", mint),
            Self::Trc20 { address } => format!("trc20:{}", address),
        }
    }

    fn parse_for_chain(
        chain: &ChainId,
        asset_namespace: &str,
        asset_reference: &str,
    ) -> Result<Self> {
        match asset_namespace {
            "slip44" => {
                let slip44 = asset_reference.parse::<u32>().map_err(|_| {
                    RoutingError::InvalidAsset(format!(
                        "invalid slip44 coin type: {asset_reference}"
                    ))
                })?;
                let expected = chain.native_slip44();
                if slip44 != expected {
                    return Err(RoutingError::InvalidAsset(format!(
                        "slip44:{slip44} is not valid for chain {} (expected slip44:{expected})",
                        chain.to_caip2()
                    )));
                }
                Ok(Self::Native)
            }
            "native" => Err(RoutingError::InvalidAsset(
                "bare 'native' asset namespace is not allowed; use slip44:<coin_type>".to_string(),
            )),
            "stellar" => {
                if !matches!(chain, ChainId::Stellar { .. }) {
                    return Err(RoutingError::InvalidAsset(
                        "stellar credit assets require a stellar:* chain id".to_string(),
                    ));
                }
                let (code, issuer) = split_once_colon(asset_reference).ok_or_else(|| {
                    RoutingError::InvalidAsset(format!(
                        "stellar asset requires CODE:ISSUER, got: {asset_reference}"
                    ))
                })?;
                let code = normalize_stellar_code(code)?;
                let issuer = validate_stellar_issuer(issuer)?;
                Ok(Self::StellarCredit { code, issuer })
            }
            "erc20" => {
                if !matches!(chain, ChainId::Eip155 { .. }) {
                    return Err(RoutingError::InvalidAsset(
                        "erc20 assets require an eip155:* chain id".to_string(),
                    ));
                }
                let address = validate_erc20_address(asset_reference)?;
                Ok(Self::Erc20 { address })
            }
            "token" => {
                if !matches!(chain, ChainId::Solana { .. }) {
                    return Err(RoutingError::InvalidAsset(
                        "solana token assets require a solana:* chain id".to_string(),
                    ));
                }
                let mint = validate_solana_address(asset_reference)?;
                Ok(Self::SplToken { mint })
            }
            "trc20" => {
                if !matches!(chain, ChainId::Tron { .. }) {
                    return Err(RoutingError::InvalidAsset(
                        "trc20 assets require a tron:* chain id".to_string(),
                    ));
                }
                let address = validate_tron_address(asset_reference)?;
                Ok(Self::Trc20 { address })
            }
            other => Err(RoutingError::InvalidAsset(format!(
                "unsupported asset namespace: {other}"
            ))),
        }
    }
}

/// Fully qualified chain-scoped asset.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChainAsset {
    pub chain: ChainId,
    pub asset: AssetReference,
}

impl ChainAsset {
    pub fn new(chain: ChainId, asset: AssetReference) -> Result<Self> {
        let value = Self { chain, asset };
        value.validate()?;
        Ok(value)
    }

    pub fn stellar_native(network: impl Into<String>) -> Self {
        Self {
            chain: ChainId::Stellar {
                network: network.into(),
            },
            asset: AssetReference::Native,
        }
    }

    pub fn stellar_credit(
        network: impl Into<String>,
        code: impl Into<String>,
        issuer: impl Into<String>,
    ) -> Result<Self> {
        let code = normalize_stellar_code(&code.into())?;
        let issuer = validate_stellar_issuer(&issuer.into())?;
        Ok(Self {
            chain: ChainId::Stellar {
                network: network.into(),
            },
            asset: AssetReference::StellarCredit { code, issuer },
        })
    }

    fn validate(&self) -> Result<()> {
        match (&self.chain, &self.asset) {
            (_, AssetReference::Native) => Ok(()),
            (ChainId::Stellar { .. }, AssetReference::StellarCredit { code, issuer }) => {
                normalize_stellar_code(code)?;
                validate_stellar_issuer(issuer)?;
                Ok(())
            }
            (ChainId::Eip155 { .. }, AssetReference::Erc20 { address }) => {
                validate_erc20_address(address)?;
                Ok(())
            }
            (ChainId::Solana { .. }, AssetReference::SplToken { mint }) => {
                validate_solana_address(mint)?;
                Ok(())
            }
            (ChainId::Tron { .. }, AssetReference::Trc20 { address }) => {
                validate_tron_address(address)?;
                Ok(())
            }
            (chain, asset) => Err(RoutingError::InvalidAsset(format!(
                "asset kind {:?} is incompatible with chain {}",
                asset_kind_name(asset),
                chain.to_caip2()
            ))),
        }
    }

    /// Canonical wire identifier.
    pub fn to_canonical(&self) -> String {
        format!(
            "{}/{}",
            self.chain.to_caip2(),
            self.asset.to_caip19_suffix(&self.chain)
        )
    }

    /// Parse a chain-scoped identifier. Fails closed on malformed input.
    pub fn parse(input: &str) -> Result<Self> {
        let input = input.trim();
        if input.is_empty() {
            return Err(RoutingError::InvalidAsset(
                "asset identifier is empty".to_string(),
            ));
        }

        let (caip2, asset_part) = input.split_once('/').ok_or_else(|| {
            RoutingError::InvalidAsset(format!("expected chain/asset form, got: {input}"))
        })?;

        let chain = ChainId::parse_caip2(caip2)?;

        // Reject collapsing / bare native forms.
        if asset_part.eq_ignore_ascii_case("native")
            || asset_part.eq_ignore_ascii_case("slip44:native")
        {
            return Err(RoutingError::InvalidAsset(
                "slip44:native / bare native are not allowed; use slip44:<coin_type>".to_string(),
            ));
        }

        let (asset_namespace, asset_reference) = split_once_colon(asset_part).ok_or_else(|| {
            RoutingError::InvalidAsset(format!(
                "expected asset_namespace:reference, got: {asset_part}"
            ))
        })?;

        let asset = AssetReference::parse_for_chain(&chain, asset_namespace, asset_reference)?;
        let parsed = Self { chain, asset };
        parsed.validate()?;
        Ok(parsed)
    }

    /// Map legacy Stellar API identifiers into chain-scoped form (pubnet default).
    ///
    /// Issuer case is preserved. Asset codes are uppercased. Does **not** use
    /// [`crate::normalize_asset`] (which uppercases the issuer for v1 cache).
    pub fn from_stellar_legacy(legacy: &str) -> Result<Self> {
        let trimmed = legacy.trim();
        if trimmed.is_empty() {
            return Err(RoutingError::InvalidAsset(
                "asset identifier is empty".to_string(),
            ));
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower == "native" || lower == "xlm" {
            return Ok(Self::stellar_native("pubnet"));
        }

        if let Some((code, issuer)) = trimmed.split_once(':') {
            return Self::stellar_credit("pubnet", code, issuer);
        }

        if trimmed.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(RoutingError::InvalidAsset(format!(
                "stellar issued asset requires issuer (CODE:ISSUER), got: {legacy}"
            )));
        }

        Err(RoutingError::InvalidAsset(format!(
            "unrecognized stellar legacy asset: {legacy}"
        )))
    }

    pub fn is_same_chain(&self, other: &Self) -> bool {
        self.chain == other.chain
    }

    pub fn chain_namespace(&self) -> &'static str {
        match self.chain {
            ChainId::Stellar { .. } => "stellar",
            ChainId::Eip155 { .. } => "eip155",
            ChainId::Solana { .. } => "solana",
            ChainId::Bitcoin { .. } => "bip122",
            ChainId::Tron { .. } => "tron",
        }
    }
}

impl fmt::Display for ChainAsset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_canonical())
    }
}

/// Returns true when `input` looks like a chain-scoped identifier.
pub fn looks_like_caip(input: &str) -> bool {
    let lower = input.to_ascii_lowercase();
    lower.starts_with("stellar:")
        || lower.starts_with("eip155:")
        || lower.starts_with("solana:")
        || lower.starts_with("bip122:")
        || lower.starts_with("tron:")
}

/// Canonicalize a chain-scoped or legacy Stellar asset id.
///
/// Returns a typed error for malformed chain-scoped ids — never echoes the raw
/// invalid string as "canonical" material.
pub fn canonicalize_asset_id(input: &str) -> Result<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(RoutingError::InvalidAsset(
            "asset identifier is empty".to_string(),
        ));
    }
    if looks_like_caip(trimmed) {
        return Ok(ChainAsset::parse(trimmed)?.to_canonical());
    }
    Ok(ChainAsset::from_stellar_legacy(trimmed)?.to_canonical())
}

/// v1 cache helper: preserves pure Stellar legacy form (`native` / `CODE:ISSUER`)
/// via [`crate::normalize_asset`] for byte-for-byte compatibility.
///
/// Chain-scoped inputs are canonicalized with [`canonicalize_asset_id`] and
/// fail closed on malformed ids.
pub fn canonicalize_for_v1_cache(input: &str) -> Result<String> {
    if looks_like_caip(input) {
        return canonicalize_asset_id(input);
    }
    Ok(crate::normalize_asset(input))
}

fn asset_kind_name(asset: &AssetReference) -> &'static str {
    match asset {
        AssetReference::Native => "native",
        AssetReference::StellarCredit { .. } => "stellar_credit",
        AssetReference::Erc20 { .. } => "erc20",
        AssetReference::SplToken { .. } => "spl_token",
        AssetReference::Trc20 { .. } => "trc20",
    }
}

fn split_once_colon(input: &str) -> Option<(&str, &str)> {
    input.split_once(':')
}

fn normalize_stellar_code(code: &str) -> Result<String> {
    if code.is_empty() || code.len() > 12 || !code.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(RoutingError::InvalidAsset(format!(
            "invalid stellar asset code: {code}"
        )));
    }
    Ok(code.to_ascii_uppercase())
}

fn validate_stellar_issuer(issuer: &str) -> Result<String> {
    // Structural StrKey account id: 'G' + 55 base32 chars (no checksum verify).
    if issuer.len() != 56 || !issuer.starts_with('G') {
        return Err(RoutingError::InvalidAsset(format!(
            "invalid stellar issuer (expected G… length 56): {issuer}"
        )));
    }
    if !issuer
        .chars()
        .skip(1)
        .all(|c| STELLAR_ISSUER_ALPHABET.contains(c))
    {
        return Err(RoutingError::InvalidAsset(format!(
            "invalid stellar issuer alphabet: {issuer}"
        )));
    }
    Ok(issuer.to_string())
}

fn validate_erc20_address(address: &str) -> Result<String> {
    let lower = address.to_ascii_lowercase();
    if !(lower.starts_with("0x")
        && lower.len() == 42
        && lower[2..].chars().all(|c| c.is_ascii_hexdigit()))
    {
        return Err(RoutingError::InvalidAsset(format!(
            "invalid erc20 address (expected 0x + 40 hex): {address}"
        )));
    }
    Ok(lower)
}

fn validate_bip122_genesis(reference: &str) -> Result<()> {
    if reference.len() != 64 || !reference.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(RoutingError::InvalidAsset(format!(
            "invalid bip122 genesis hash (expected 64 hex chars): {reference}"
        )));
    }
    Ok(())
}

fn validate_solana_address(address: &str) -> Result<String> {
    // Structural base58 pubkey/mint: typical length 32–44, no checksum verify.
    if address.len() < 32 || address.len() > 44 {
        return Err(RoutingError::InvalidAsset(format!(
            "invalid solana address length: {address}"
        )));
    }
    if !address.chars().all(|c| BASE58_ALPHABET.contains(c)) {
        return Err(RoutingError::InvalidAsset(format!(
            "invalid solana address alphabet: {address}"
        )));
    }
    Ok(address.to_string())
}

fn validate_tron_address(address: &str) -> Result<String> {
    // Structural Base58Check mainnet address: 'T' + 33 chars (no checksum verify).
    if address.len() != 34 || !address.starts_with('T') {
        return Err(RoutingError::InvalidAsset(format!(
            "invalid tron address (expected T… length 34): {address}"
        )));
    }
    if !address.chars().all(|c| BASE58_ALPHABET.contains(c)) {
        return Err(RoutingError::InvalidAsset(format!(
            "invalid tron address alphabet: {address}"
        )));
    }
    Ok(address.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_ISSUER: &str = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";

    #[test]
    fn stellar_native_uses_slip44_148() {
        let asset = ChainAsset::stellar_native("pubnet");
        assert_eq!(asset.to_canonical(), "stellar:pubnet/slip44:148");
        assert_eq!(
            ChainAsset::parse("stellar:pubnet/slip44:148").unwrap(),
            asset
        );
    }

    #[test]
    fn rejects_slip44_native_and_bare_native() {
        assert!(ChainAsset::parse("stellar:pubnet/slip44:native").is_err());
        assert!(ChainAsset::parse("stellar:pubnet/native").is_err());
        assert!(ChainAsset::parse("eip155:1/slip44:native").is_err());
        assert!(canonicalize_asset_id("eip155:1/slip44:native").is_err());
    }

    #[test]
    fn rejects_wrong_slip44_for_chain() {
        assert!(ChainAsset::parse("stellar:pubnet/slip44:60").is_err());
        assert!(ChainAsset::parse("eip155:1/slip44:148").is_err());
        assert!(ChainAsset::parse(
            "bip122:000000000019d6689c085ae165831e9345904ddf3418b7bb805d31aeb52cafd2/slip44:60"
        )
        .is_err());
    }

    #[test]
    fn multi_chain_natives_use_distinct_slip44() {
        let ids = [
            ChainAsset::stellar_native("pubnet").to_canonical(),
            ChainAsset::new(ChainId::ethereum_mainnet(), AssetReference::Native)
                .unwrap()
                .to_canonical(),
            ChainAsset::new(ChainId::solana_mainnet(), AssetReference::Native)
                .unwrap()
                .to_canonical(),
            ChainAsset::new(ChainId::bitcoin_mainnet(), AssetReference::Native)
                .unwrap()
                .to_canonical(),
            ChainAsset::new(ChainId::tron_mainnet(), AssetReference::Native)
                .unwrap()
                .to_canonical(),
        ];
        assert_eq!(
            ids,
            [
                "stellar:pubnet/slip44:148",
                "eip155:1/slip44:60",
                "solana:mainnet/slip44:501",
                "bip122:000000000019d6689c085ae165831e9345904ddf3418b7bb805d31aeb52cafd2/slip44:0",
                "tron:mainnet/slip44:195",
            ]
        );
    }

    #[test]
    fn preserves_stellar_issuer_casing_in_caip_form() {
        let asset = ChainAsset::from_stellar_legacy(&format!("usdc:{VALID_ISSUER}")).unwrap();
        assert_eq!(
            asset.to_canonical(),
            format!("stellar:pubnet/stellar:USDC:{VALID_ISSUER}")
        );
    }

    #[test]
    fn canonicalize_fails_closed_on_malformed_caip() {
        assert!(canonicalize_asset_id("eip155:1/erc20:not-an-address").is_err());
        assert!(canonicalize_asset_id("stellar:pubnet/slip44:native").is_err());
        assert!(canonicalize_asset_id("totally-bogus").is_err());
    }

    #[test]
    fn v1_cache_preserves_legacy_normalize_bytes() {
        assert_eq!(canonicalize_for_v1_cache("XLM").unwrap(), "native");
        assert_eq!(canonicalize_for_v1_cache("usdc:ga").unwrap(), "USDC:GA");
        assert_eq!(
            canonicalize_for_v1_cache("eip155:1/slip44:60").unwrap(),
            "eip155:1/slip44:60"
        );
    }

    #[test]
    fn erc20_must_be_0x_plus_40_hex() {
        assert!(
            ChainAsset::parse("eip155:1/erc20:0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").is_ok()
        );
        assert!(ChainAsset::parse("eip155:1/erc20:0xabc").is_err());
        assert!(
            ChainAsset::parse("eip155:1/erc20:a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").is_err()
        );
    }

    #[test]
    fn bip122_requires_64_hex() {
        assert!(ChainId::parse_caip2(
            "bip122:000000000019d6689c085ae165831e9345904ddf3418b7bb805d31aeb52cafd2"
        )
        .is_ok());
        assert!(ChainId::parse_caip2("bip122:000000000019d6689c085ae165831e93").is_err());
        assert!(ChainId::parse_caip2("bip122:deadbeef").is_err());
    }

    #[test]
    fn serialize_round_trip() {
        let asset = ChainAsset::new(
            ChainId::Solana {
                cluster: "mainnet".into(),
            },
            AssetReference::SplToken {
                mint: "So11111111111111111111111111111111111111112".into(),
            },
        )
        .unwrap();
        let json = serde_json::to_string(&asset).unwrap();
        let parsed: ChainAsset = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, asset);
    }
}
