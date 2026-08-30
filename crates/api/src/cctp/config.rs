//! Typed CCTP testnet configuration with fail-closed validation.

use std::fmt;

use crate::cctp::rpc_env::{
    require_sepolia_rpc_when_cctp_enabled, resolve_sepolia_rpc_primary,
    resolve_stellar_rpc_primary, DEFAULT_STELLAR_TESTNET_RPC,
};
use crate::env_profile::{is_production, parse_bool_env};
use crate::models::v2_cctp::{
    CctpFinality, CCTP_PROVIDER_ID, CCTP_TESTNET_CORRIDOR_ID, SEPOLIA_USDC_ASSET,
    STELLAR_TESTNET_CHAIN_ID, STELLAR_TESTNET_USDC_ASSET,
};

pub const STELLAR_TESTNET_DOMAIN: u32 = 27;
pub const SEPOLIA_DOMAIN: u32 = 0;
pub const STELLAR_TESTNET_PASSPHRASE: &str = "Test SDF Network ; September 2015";
pub const DEFAULT_IRIS_SANDBOX_URL: &str = "https://iris-api-sandbox.circle.com";
pub const IRIS_SANDBOX_HOST: &str = "iris-api-sandbox.circle.com";

pub const STELLAR_TOKEN_MESSENGER: &str =
    "CDNG7HXAPBWICI2E3AUBP3YZWZELJLYSB6F5CC7WLDTLTHVM74SLRTHP";
pub const STELLAR_MESSAGE_TRANSMITTER: &str =
    "CBJ6MTCKKZG73PMDZCJMSFRD7DQEMI4FKDH7CGDSV4W6FHCRBCQAVVJY";
pub const STELLAR_CCTP_FORWARDER: &str = "CA66Q2WFBND6V4UEB7RD4SAXSVIWMD6RA4X3U32ELVFGXV5PJK4T4VSZ";
pub const STELLAR_USDC_CONTRACT: &str = "CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA";

pub const SEPOLIA_TOKEN_MESSENGER: &str = "0x8FE6B999Dc680CcFDD5Bf7EB0974218be2542DAA";
pub const SEPOLIA_MESSAGE_TRANSMITTER: &str = "0xE737e5cEBEEBa77EFE34D4aa090756590b1CE275";
pub const SEPOLIA_USDC: &str = "0x1c7d4b196cb0c7b01d743fbc6116a902379c7238";

/// Finality threshold constants from Circle CCTP v2 technical guide.
pub const FINALITY_STANDARD: u32 = 2000;
pub const FINALITY_FAST: u32 = 1000;
/// Stellar USDC on testnet uses 7 decimal places (Circle SEP-41 token).
pub const STELLAR_USDC_DECIMALS: u32 = 7;
/// Sepolia USDC (ERC-20) uses 6 decimal places.
pub const SEPOLIA_USDC_DECIMALS: u32 = 6;

pub fn corridor_min_finality(finality: CctpFinality) -> u32 {
    match finality {
        CctpFinality::Standard => FINALITY_STANDARD,
        CctpFinality::Fast => FINALITY_FAST,
    }
}

#[derive(Clone)]
pub struct CctpConfig {
    pub enabled: bool,
    pub iris_base_url: String,
    pub stellar_domain: u32,
    pub sepolia_domain: u32,
    pub stellar_rpc_url: String,
    pub stellar_horizon_url: String,
    pub stellar_network_passphrase: String,
    pub sepolia_rpc_url: String,
    pub amount_cap: String,
    pub quote_ttl_secs: u64,
    /// Max mint payload TTL (seconds); actual expiry is min(quote expiry, this).
    pub mint_payload_ttl_secs: u64,
    pub poll_interval_secs: u64,
    pub poll_timeout_secs: u64,
    pub iris_timeout_secs: u64,
    pub iris_max_retries: u32,
    /// Iris `/v2/publicKeys` cache TTL (default 15m).
    pub iris_keys_ttl_secs: u64,
    /// Max staleness before fail-closed (default 24h).
    pub iris_keys_stale_max_secs: u64,
    /// On-chain attester snapshot TTL (default 15m).
    pub attester_snapshot_ttl_secs: u64,
    /// On-chain snapshot max staleness (default 24h).
    pub attester_snapshot_stale_max_secs: u64,
    pub contracts: CctpContractAddresses,
}

#[derive(Clone)]
pub struct CctpContractAddresses {
    pub stellar_token_messenger: String,
    pub stellar_message_transmitter: String,
    pub stellar_cctp_forwarder: String,
    pub stellar_usdc: String,
    pub sepolia_token_messenger: String,
    pub sepolia_message_transmitter: String,
    pub sepolia_usdc: String,
}

impl fmt::Debug for CctpConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CctpConfig")
            .field("enabled", &self.enabled)
            .field("iris_base_url", &redact_url(&self.iris_base_url))
            .field("stellar_domain", &self.stellar_domain)
            .field("sepolia_domain", &self.sepolia_domain)
            .field("stellar_rpc_url", &redact_url(&self.stellar_rpc_url))
            .field(
                "stellar_horizon_url",
                &redact_url(&self.stellar_horizon_url),
            )
            .field("sepolia_rpc_url", &redact_url(&self.sepolia_rpc_url))
            .field("amount_cap", &self.amount_cap)
            .field("quote_ttl_secs", &self.quote_ttl_secs)
            .field("poll_interval_secs", &self.poll_interval_secs)
            .field("poll_timeout_secs", &self.poll_timeout_secs)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CctpConfigError {
    MalformedAddress(String),
    WrongDomainPair,
    NonHttpsUrl(String),
    MainnetPassphraseOnTestnet,
    ZeroOrUnsafeLimit(String),
    FastModeUnsupported,
    InvalidEnv(String),
}

impl fmt::Display for CctpConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedAddress(a) => write!(f, "malformed address: {a}"),
            Self::WrongDomainPair => write!(f, "domain/chain pair mismatch"),
            Self::NonHttpsUrl(u) => write!(f, "URL must be HTTPS in non-test builds: {u}"),
            Self::MainnetPassphraseOnTestnet => {
                write!(f, "mainnet passphrase on testnet configuration")
            }
            Self::ZeroOrUnsafeLimit(l) => write!(f, "unsafe limit: {l}"),
            Self::FastModeUnsupported => write!(f, "fast finality not verified for this corridor"),
            Self::InvalidEnv(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for CctpConfigError {}

/// Redact URL userinfo and query for Debug/logging.
pub fn redact_url(url: &str) -> String {
    let scheme_end = url.find("//").map(|i| i + 2).unwrap_or(0);
    let (scheme, rest) = url.split_at(scheme_end);
    let host_path = rest.rsplit('@').next().unwrap_or(rest);
    let host_path = host_path.split('?').next().unwrap_or(host_path);
    format!("{scheme}{host_path}")
}

impl CctpConfig {
    pub fn default_testnet() -> Self {
        Self {
            enabled: false,
            iris_base_url: DEFAULT_IRIS_SANDBOX_URL.into(),
            stellar_domain: STELLAR_TESTNET_DOMAIN,
            sepolia_domain: SEPOLIA_DOMAIN,
            stellar_rpc_url: DEFAULT_STELLAR_TESTNET_RPC.into(),
            stellar_horizon_url: "https://horizon-testnet.stellar.org".into(),
            stellar_network_passphrase: STELLAR_TESTNET_PASSPHRASE.into(),
            sepolia_rpc_url: String::new(),
            amount_cap: "100000".into(),
            quote_ttl_secs: 300,
            mint_payload_ttl_secs: 600,
            poll_interval_secs: 5,
            // Circle Standard on Ethereum/Sepolia is ~15–19m (~65 blocks); keep margin.
            poll_timeout_secs: 1_800,
            iris_timeout_secs: 10,
            iris_max_retries: 2,
            iris_keys_ttl_secs: 900,
            iris_keys_stale_max_secs: 86_400,
            attester_snapshot_ttl_secs: 900,
            attester_snapshot_stale_max_secs: 86_400,
            contracts: CctpContractAddresses {
                stellar_token_messenger: STELLAR_TOKEN_MESSENGER.into(),
                stellar_message_transmitter: STELLAR_MESSAGE_TRANSMITTER.into(),
                stellar_cctp_forwarder: STELLAR_CCTP_FORWARDER.into(),
                stellar_usdc: STELLAR_USDC_CONTRACT.into(),
                sepolia_token_messenger: SEPOLIA_TOKEN_MESSENGER.into(),
                sepolia_message_transmitter: SEPOLIA_MESSAGE_TRANSMITTER.into(),
                sepolia_usdc: SEPOLIA_USDC.into(),
            },
        }
    }

    pub fn from_env() -> Result<Self, CctpConfigError> {
        let mut cfg = Self::default_testnet();
        cfg.enabled = parse_bool_env("CCTP_ENABLED");

        // Compose often injects `VAR=` for optional keys. Treat empty as unset so
        // defaults (Iris sandbox, etc.) are not wiped to invalid URLs.
        if let Some(v) = env_nonempty("CCTP_IRIS_BASE_URL") {
            cfg.iris_base_url = v;
        }
        cfg.stellar_rpc_url = resolve_stellar_rpc_primary();
        cfg.sepolia_rpc_url = resolve_sepolia_rpc_primary();
        if let Some(v) = env_nonempty("STELLAR_HORIZON_URL") {
            cfg.stellar_horizon_url = v;
        }
        if let Some(v) = env_nonempty("STELLAR_NETWORK_PASSPHRASE") {
            cfg.stellar_network_passphrase = v;
        }
        if let Some(v) = env_nonempty("CCTP_AMOUNT_CAP") {
            cfg.amount_cap = v;
        }
        if let Ok(v) = std::env::var("CCTP_QUOTE_TTL_SECS") {
            cfg.quote_ttl_secs = v.parse().map_err(|_| {
                CctpConfigError::InvalidEnv("CCTP_QUOTE_TTL_SECS must be u64".into())
            })?;
        }
        if let Ok(v) = std::env::var("CCTP_MINT_PAYLOAD_TTL_SECS") {
            cfg.mint_payload_ttl_secs = v.parse().map_err(|_| {
                CctpConfigError::InvalidEnv("CCTP_MINT_PAYLOAD_TTL_SECS must be u64".into())
            })?;
        }
        if let Ok(v) = std::env::var("CCTP_POLL_INTERVAL_SECS") {
            cfg.poll_interval_secs = v.parse().map_err(|_| {
                CctpConfigError::InvalidEnv("CCTP_POLL_INTERVAL_SECS must be u64".into())
            })?;
        }
        if let Ok(v) = std::env::var("CCTP_POLL_TIMEOUT_SECS") {
            cfg.poll_timeout_secs = v.parse().map_err(|_| {
                CctpConfigError::InvalidEnv("CCTP_POLL_TIMEOUT_SECS must be u64".into())
            })?;
        }
        if let Ok(v) = std::env::var("CCTP_IRIS_KEYS_TTL_SECS") {
            cfg.iris_keys_ttl_secs = v.parse().map_err(|_| {
                CctpConfigError::InvalidEnv("CCTP_IRIS_KEYS_TTL_SECS must be u64".into())
            })?;
        }
        if let Ok(v) = std::env::var("CCTP_IRIS_KEYS_STALE_MAX_SECS") {
            cfg.iris_keys_stale_max_secs = v.parse().map_err(|_| {
                CctpConfigError::InvalidEnv("CCTP_IRIS_KEYS_STALE_MAX_SECS must be u64".into())
            })?;
        }
        if let Ok(v) = std::env::var("CCTP_ATTESTER_SNAPSHOT_TTL_SECS") {
            cfg.attester_snapshot_ttl_secs = v.parse().map_err(|_| {
                CctpConfigError::InvalidEnv("CCTP_ATTESTER_SNAPSHOT_TTL_SECS must be u64".into())
            })?;
        }
        if let Ok(v) = std::env::var("CCTP_ATTESTER_SNAPSHOT_STALE_MAX_SECS") {
            cfg.attester_snapshot_stale_max_secs = v.parse().map_err(|_| {
                CctpConfigError::InvalidEnv(
                    "CCTP_ATTESTER_SNAPSHOT_STALE_MAX_SECS must be u64".into(),
                )
            })?;
        }

        require_sepolia_rpc_when_cctp_enabled()?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> Result<(), CctpConfigError> {
        if self.stellar_domain != STELLAR_TESTNET_DOMAIN || self.sepolia_domain != SEPOLIA_DOMAIN {
            return Err(CctpConfigError::WrongDomainPair);
        }

        validate_stellar_contract(&self.contracts.stellar_token_messenger)?;
        validate_stellar_contract(&self.contracts.stellar_message_transmitter)?;
        validate_stellar_contract(&self.contracts.stellar_cctp_forwarder)?;
        validate_stellar_contract(&self.contracts.stellar_usdc)?;
        validate_evm_address(&self.contracts.sepolia_token_messenger)?;
        validate_evm_address(&self.contracts.sepolia_message_transmitter)?;
        validate_evm_address(&self.contracts.sepolia_usdc)?;

        if self
            .stellar_network_passphrase
            .contains("Public Global Stellar Network")
        {
            return Err(CctpConfigError::MainnetPassphraseOnTestnet);
        }
        if self.stellar_network_passphrase != STELLAR_TESTNET_PASSPHRASE {
            return Err(CctpConfigError::WrongDomainPair);
        }

        validate_iris_url(&self.iris_base_url)?;

        validate_https_url(&self.stellar_rpc_url)?;
        validate_https_url(&self.stellar_horizon_url)?;
        if self.enabled {
            if self.sepolia_rpc_url.trim().is_empty() {
                return Err(CctpConfigError::InvalidEnv(
                    "CCTP_ENABLED=true requires CCTP_SEPOLIA_RPC_URL or SEPOLIA_RPC_URL".into(),
                ));
            }
            validate_https_url(&self.sepolia_rpc_url)?;
        } else if !self.sepolia_rpc_url.trim().is_empty() {
            validate_https_url(&self.sepolia_rpc_url)?;
        }

        if self.amount_cap.is_empty()
            || self.quote_ttl_secs == 0
            || self.poll_interval_secs == 0
            || self.poll_timeout_secs == 0
            || self.iris_timeout_secs == 0
            || self.iris_keys_ttl_secs == 0
            || self.iris_keys_stale_max_secs == 0
            || self.attester_snapshot_ttl_secs == 0
            || self.attester_snapshot_stale_max_secs == 0
        {
            return Err(CctpConfigError::ZeroOrUnsafeLimit("timing or cap".into()));
        }

        Ok(())
    }

    /// Config is structurally valid. Distinct from public executability.
    pub fn is_configured(&self) -> bool {
        self.validate().is_ok()
    }

    /// Public corridor executability — requires all readiness components.
    pub fn is_executable(&self) -> bool {
        false
    }

    pub fn corridor_id(&self) -> &'static str {
        CCTP_TESTNET_CORRIDOR_ID
    }

    pub fn provider_id(&self) -> &'static str {
        CCTP_PROVIDER_ID
    }

    pub fn request_url_matches_allowed_host(
        &self,
        url: &str,
        allowed_host: &str,
    ) -> Result<(), CctpConfigError> {
        let parsed = parse_service_url(url)?;
        if parsed.host != allowed_host.to_ascii_lowercase() {
            return Err(CctpConfigError::NonHttpsUrl(redact_url(url)));
        }
        if parsed.scheme != "https" {
            return Err(CctpConfigError::NonHttpsUrl(redact_url(url)));
        }
        if !parsed.path.starts_with("/v2") {
            return Err(CctpConfigError::NonHttpsUrl(redact_url(url)));
        }
        Ok(())
    }
}

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn validate_iris_url(url: &str) -> Result<(), CctpConfigError> {
    let parsed = parse_service_url(url)?;
    if parsed.host != IRIS_SANDBOX_HOST {
        return Err(CctpConfigError::NonHttpsUrl(redact_url(url)));
    }
    if parsed.scheme != "https" {
        return Err(CctpConfigError::NonHttpsUrl(redact_url(url)));
    }
    if parsed.path != "/" && !parsed.path.starts_with("/v2") {
        return Err(CctpConfigError::NonHttpsUrl(redact_url(url)));
    }
    Ok(())
}

fn validate_https_url(url: &str) -> Result<(), CctpConfigError> {
    let parsed = parse_service_url(url)?;
    if is_production() && parsed.scheme != "https" {
        return Err(CctpConfigError::NonHttpsUrl(redact_url(url)));
    }
    if parsed.scheme != "https" && parsed.scheme != "http" {
        return Err(CctpConfigError::NonHttpsUrl(redact_url(url)));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedServiceUrl {
    pub scheme: String,
    pub host: String,
    pub port: Option<u16>,
    pub path: String,
}

pub fn parse_service_url(url_str: &str) -> Result<ParsedServiceUrl, CctpConfigError> {
    let trimmed = url_str.trim();
    if trimmed.contains('#') || trimmed.contains('@') {
        return Err(CctpConfigError::NonHttpsUrl(redact_url(url_str)));
    }
    let (scheme, rest) = trimmed
        .split_once("//")
        .map(|(s, r)| (s.trim_end_matches(':').to_ascii_lowercase(), r))
        .unwrap_or(("".into(), trimmed));
    if scheme != "https" && scheme != "http" {
        return Err(CctpConfigError::NonHttpsUrl(redact_url(url_str)));
    }
    let (host_part, path) = rest
        .split_once('/')
        .map(|(h, p)| (h, format!("/{}", p.split('?').next().unwrap_or(p))))
        .unwrap_or((rest, "/".into()));
    if host_part.contains('?') {
        return Err(CctpConfigError::NonHttpsUrl(redact_url(url_str)));
    }
    let (host, port) = if let Some((h, p)) = host_part.rsplit_once(':') {
        let port = p
            .parse::<u16>()
            .map_err(|_| CctpConfigError::NonHttpsUrl(redact_url(url_str)))?;
        (h.to_ascii_lowercase(), Some(port))
    } else {
        (host_part.to_ascii_lowercase(), None)
    };
    Ok(ParsedServiceUrl {
        scheme,
        host,
        port,
        path,
    })
}

fn validate_stellar_contract(addr: &str) -> Result<(), CctpConfigError> {
    if stellar_strkey::Contract::from_string(addr.trim()).is_err() {
        return Err(CctpConfigError::MalformedAddress(addr.to_string()));
    }
    Ok(())
}

fn validate_evm_address(addr: &str) -> Result<(), CctpConfigError> {
    let trimmed = addr.trim();
    if trimmed.len() != 42 || !trimmed.starts_with("0x") {
        return Err(CctpConfigError::MalformedAddress(addr.to_string()));
    }
    if !trimmed[2..].chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(CctpConfigError::MalformedAddress(addr.to_string()));
    }
    Ok(())
}

pub fn frozen_source_asset(chain: &str) -> (String, String) {
    if chain == STELLAR_TESTNET_CHAIN_ID {
        (
            STELLAR_TESTNET_USDC_ASSET.into(),
            format!("{}/{}", chain, STELLAR_TESTNET_USDC_ASSET),
        )
    } else {
        (
            SEPOLIA_USDC_ASSET.into(),
            format!("{}/{}", chain, SEPOLIA_USDC_ASSET),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_disabled_and_valid_testnet() {
        let cfg = CctpConfig::default_testnet();
        assert!(!cfg.enabled);
        assert!(cfg.validate().is_ok());
        assert!(cfg.is_configured());
        assert!(!cfg.is_executable());
    }

    #[test]
    fn wrong_passphrase_fails() {
        let mut cfg = CctpConfig::default_testnet();
        cfg.stellar_network_passphrase = "Public Global Stellar Network ; September 2015".into();
        assert_eq!(
            cfg.validate(),
            Err(CctpConfigError::MainnetPassphraseOnTestnet)
        );
    }

    #[test]
    fn malformed_evm_address_fails() {
        let mut cfg = CctpConfig::default_testnet();
        cfg.contracts.sepolia_usdc = "not-an-address".into();
        assert!(matches!(
            cfg.validate(),
            Err(CctpConfigError::MalformedAddress(_))
        ));
    }

    #[test]
    fn redact_url_strips_credentials_and_query() {
        let redacted = redact_url("https://user:secret@rpc.example.com/path?token=abc");
        assert!(!redacted.contains("secret"));
        assert!(!redacted.contains("token"));
        assert!(redacted.contains("rpc.example.com"));
    }

    #[test]
    fn rejects_evil_iris_host_variants() {
        for url in [
            "https://evil.com",
            "https://iris-api-sandbox.circle.com.evil.com",
            "http://iris-api-sandbox.circle.com",
            "https://user:pass@iris-api-sandbox.circle.com",
            "https://iris-api-sandbox.circle.com?token=secret",
        ] {
            let mut cfg = CctpConfig::default_testnet();
            cfg.iris_base_url = url.into();
            assert!(cfg.validate().is_err(), "must reject {url}");
        }
    }

    #[test]
    fn iris_poll_url_with_query_matches_allowed_host() {
        let cfg = CctpConfig::default_testnet();
        let hash = "26514bc123354d8c2ff72f73ad56da48824b03e851c33b2772f2df0f13a96c3d";
        let url = format!(
            "{}/v2/messages/27?transactionHash={hash}",
            DEFAULT_IRIS_SANDBOX_URL
        );
        cfg.request_url_matches_allowed_host(&url, IRIS_SANDBOX_HOST)
            .expect("Iris poll URLs carry query params");
    }

    #[test]
    fn iris_outbound_url_rejects_non_v2_path() {
        let cfg = CctpConfig::default_testnet();
        let url = format!("{}/v1/messages/27", DEFAULT_IRIS_SANDBOX_URL);
        assert!(cfg
            .request_url_matches_allowed_host(&url, IRIS_SANDBOX_HOST)
            .is_err());
    }

    #[test]
    fn env_parsing_respects_cctp_enabled() {
        let previous_enabled = std::env::var("CCTP_ENABLED").ok();
        let previous_sepolia = std::env::var("SEPOLIA_RPC_URL").ok();
        std::env::set_var("CCTP_ENABLED", "true");
        std::env::set_var("SEPOLIA_RPC_URL", "https://sepolia.drpc.org");
        let cfg = CctpConfig::from_env().expect("valid env config");
        assert!(cfg.enabled);
        match previous_enabled {
            Some(v) => std::env::set_var("CCTP_ENABLED", v),
            None => std::env::remove_var("CCTP_ENABLED"),
        }
        match previous_sepolia {
            Some(v) => std::env::set_var("SEPOLIA_RPC_URL", v),
            None => std::env::remove_var("SEPOLIA_RPC_URL"),
        }
    }

    #[test]
    fn enabled_without_sepolia_rpc_fails() {
        let previous_enabled = std::env::var("CCTP_ENABLED").ok();
        let previous_cctp_sepolia = std::env::var("CCTP_SEPOLIA_RPC_URL").ok();
        let previous_sepolia = std::env::var("SEPOLIA_RPC_URL").ok();
        std::env::set_var("CCTP_ENABLED", "true");
        std::env::remove_var("CCTP_SEPOLIA_RPC_URL");
        std::env::remove_var("SEPOLIA_RPC_URL");
        assert!(CctpConfig::from_env().is_err());
        match previous_enabled {
            Some(v) => std::env::set_var("CCTP_ENABLED", v),
            None => std::env::remove_var("CCTP_ENABLED"),
        }
        match previous_cctp_sepolia {
            Some(v) => std::env::set_var("CCTP_SEPOLIA_RPC_URL", v),
            None => std::env::remove_var("CCTP_SEPOLIA_RPC_URL"),
        }
        match previous_sepolia {
            Some(v) => std::env::set_var("SEPOLIA_RPC_URL", v),
            None => std::env::remove_var("SEPOLIA_RPC_URL"),
        }
    }

    #[test]
    fn disabled_allows_empty_sepolia_rpc() {
        let mut cfg = CctpConfig::default_testnet();
        cfg.sepolia_rpc_url.clear();
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn empty_optional_url_envs_keep_defaults() {
        let previous_iris = std::env::var("CCTP_IRIS_BASE_URL").ok();
        let previous_horizon = std::env::var("STELLAR_HORIZON_URL").ok();
        let previous_enabled = std::env::var("CCTP_ENABLED").ok();
        let previous_sepolia = std::env::var("CCTP_SEPOLIA_RPC_URL").ok();
        std::env::set_var("CCTP_IRIS_BASE_URL", "");
        std::env::set_var("STELLAR_HORIZON_URL", "   ");
        std::env::set_var("CCTP_ENABLED", "true");
        std::env::set_var("CCTP_SEPOLIA_RPC_URL", "https://sepolia.drpc.org");
        let cfg = CctpConfig::from_env().expect("empty optional URLs must not invalidate config");
        assert_eq!(cfg.iris_base_url, DEFAULT_IRIS_SANDBOX_URL);
        assert!(!cfg.stellar_horizon_url.trim().is_empty());
        match previous_iris {
            Some(v) => std::env::set_var("CCTP_IRIS_BASE_URL", v),
            None => std::env::remove_var("CCTP_IRIS_BASE_URL"),
        }
        match previous_horizon {
            Some(v) => std::env::set_var("STELLAR_HORIZON_URL", v),
            None => std::env::remove_var("STELLAR_HORIZON_URL"),
        }
        match previous_enabled {
            Some(v) => std::env::set_var("CCTP_ENABLED", v),
            None => std::env::remove_var("CCTP_ENABLED"),
        }
        match previous_sepolia {
            Some(v) => std::env::set_var("CCTP_SEPOLIA_RPC_URL", v),
            None => std::env::remove_var("CCTP_SEPOLIA_RPC_URL"),
        }
    }

    #[test]
    fn stellar_rpc_alias_precedence_from_env() {
        let previous = std::env::var("CCTP_STELLAR_RPC_URL").ok();
        std::env::set_var("CCTP_STELLAR_RPC_URL", "https://cctp-stellar.example");
        let cfg = CctpConfig::from_env().expect("valid env config");
        assert_eq!(cfg.stellar_rpc_url, "https://cctp-stellar.example");
        match previous {
            Some(v) => std::env::set_var("CCTP_STELLAR_RPC_URL", v),
            None => std::env::remove_var("CCTP_STELLAR_RPC_URL"),
        }
    }
}
