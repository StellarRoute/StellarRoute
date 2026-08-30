//! Canonical CCTP / dependency-health RPC URL resolution from environment.
//!
//! Precedence (documented in `docs/development/environment-variables.md`):
//! - Stellar: `CCTP_STELLAR_RPC_URL` → `STELLAR_RPC_URL` → `SOROBAN_RPC_URL` → official testnet default
//! - Sepolia: `CCTP_SEPOLIA_RPC_URL` → `SEPOLIA_RPC_URL` (+ explicit fallback CSV env vars)
//!
//! Disabled CCTP must never silently fall back to broken public defaults such as `rpc.sepolia.org`.

use crate::cctp::config::{parse_service_url, redact_url, CctpConfigError};
use crate::env_profile::parse_bool_env;

pub const DEFAULT_STELLAR_TESTNET_RPC: &str = "https://soroban-testnet.stellar.org";
pub const SEPOLIA_CHAIN_ID_DECIMAL: u64 = 11155111;
pub const SEPOLIA_CHAIN_ID_HEX: &str = "0xaa36a7";

const STELLAR_PRIMARY_ENV: &[&str] =
    &["CCTP_STELLAR_RPC_URL", "STELLAR_RPC_URL", "SOROBAN_RPC_URL"];
const SEPOLIA_PRIMARY_ENV: &[&str] = &["CCTP_SEPOLIA_RPC_URL", "SEPOLIA_RPC_URL"];
const STELLAR_FALLBACK_ENV: &str = "CCTP_STELLAR_RPC_FALLBACK_URLS";
const SEPOLIA_FALLBACK_ENV: &str = "CCTP_SEPOLIA_RPC_FALLBACK_URLS";
const MAX_RPC_URL_COUNT: usize = 8;

fn trim_url(url: &str) -> String {
    url.trim().trim_end_matches('/').to_string()
}

fn read_env_nonempty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| trim_url(&v))
        .filter(|v| !v.is_empty())
}

fn read_first_env(keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|k| read_env_nonempty(k))
}

fn parse_csv_urls(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(trim_url)
        .filter(|u| !u.is_empty())
        .collect()
}

fn read_fallback_urls(env_key: &str) -> Vec<String> {
    std::env::var(env_key)
        .ok()
        .map(|v| parse_csv_urls(&v))
        .unwrap_or_default()
}

fn dedupe_preserve_order(urls: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for url in urls {
        if !out.iter().any(|existing| existing == &url) {
            out.push(url);
        }
    }
    out
}

fn bounded_urls(mut urls: Vec<String>) -> Result<Vec<String>, CctpConfigError> {
    urls = dedupe_preserve_order(urls);
    if urls.len() > MAX_RPC_URL_COUNT {
        return Err(CctpConfigError::InvalidEnv(format!(
            "too many RPC URLs (max {MAX_RPC_URL_COUNT})"
        )));
    }
    Ok(urls)
}

/// Validate RPC URL shape: no credentials/query fragments; HTTPS required outside tests.
pub fn validate_rpc_url(url: &str) -> Result<(), CctpConfigError> {
    if url.trim().is_empty() {
        return Err(CctpConfigError::InvalidEnv(
            "RPC URL must not be empty".into(),
        ));
    }
    let parsed = parse_service_url(url)?;
    if parsed.scheme != "https" && parsed.scheme != "http" {
        return Err(CctpConfigError::NonHttpsUrl(redact_url(url)));
    }
    if !cfg!(test) && parsed.scheme != "https" {
        return Err(CctpConfigError::NonHttpsUrl(redact_url(url)));
    }
    Ok(())
}

/// Hostname only — safe for structured logs (never log path/query/credentials).
pub fn rpc_url_host(url: &str) -> Option<String> {
    parse_service_url(url).ok().map(|parsed| parsed.host)
}

/// Resolve the primary Stellar/Soroban RPC URL for CCTP runtime and dependency probes.
pub fn resolve_stellar_rpc_primary() -> String {
    read_first_env(STELLAR_PRIMARY_ENV).unwrap_or_else(|| DEFAULT_STELLAR_TESTNET_RPC.into())
}

/// Ordered Stellar RPC URLs: primary + explicit `CCTP_STELLAR_RPC_FALLBACK_URLS`.
pub fn resolve_stellar_rpc_urls() -> Result<Vec<String>, CctpConfigError> {
    let mut urls = vec![resolve_stellar_rpc_primary()];
    urls.extend(read_fallback_urls(STELLAR_FALLBACK_ENV));
    let urls = bounded_urls(urls)?;
    for url in &urls {
        validate_rpc_url(url)?;
    }
    Ok(urls)
}

/// Resolve primary Sepolia RPC URL when explicitly configured; empty when unset.
pub fn resolve_sepolia_rpc_primary() -> String {
    read_first_env(SEPOLIA_PRIMARY_ENV).unwrap_or_default()
}

/// Ordered Sepolia RPC URLs: primary (if any) + `CCTP_SEPOLIA_RPC_FALLBACK_URLS`.
pub fn resolve_sepolia_rpc_urls() -> Result<Vec<String>, CctpConfigError> {
    let mut urls = Vec::new();
    if let Some(primary) = read_first_env(SEPOLIA_PRIMARY_ENV) {
        urls.push(primary);
    }
    urls.extend(read_fallback_urls(SEPOLIA_FALLBACK_ENV));
    let urls = bounded_urls(urls)?;
    for url in &urls {
        validate_rpc_url(url)?;
    }
    Ok(urls)
}

/// When CCTP is enabled, Sepolia RPC must be explicitly configured (no implicit default).
pub fn require_sepolia_rpc_when_cctp_enabled() -> Result<(), CctpConfigError> {
    if !parse_bool_env("CCTP_ENABLED") {
        return Ok(());
    }
    let primary = resolve_sepolia_rpc_primary();
    if primary.is_empty() {
        return Err(CctpConfigError::InvalidEnv(
            "CCTP_ENABLED=true requires CCTP_SEPOLIA_RPC_URL or SEPOLIA_RPC_URL (no implicit Sepolia default)"
                .into(),
        ));
    }
    validate_rpc_url(&primary)
}

pub fn is_sepolia_chain_id(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.eq_ignore_ascii_case(SEPOLIA_CHAIN_ID_HEX) {
        return true;
    }
    trimmed == SEPOLIA_CHAIN_ID_DECIMAL.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn reset_env() {
        for key in [
            "CCTP_ENABLED",
            "CCTP_STELLAR_RPC_URL",
            "STELLAR_RPC_URL",
            "SOROBAN_RPC_URL",
            "CCTP_STELLAR_RPC_FALLBACK_URLS",
            "CCTP_SEPOLIA_RPC_URL",
            "SEPOLIA_RPC_URL",
            "CCTP_SEPOLIA_RPC_FALLBACK_URLS",
        ] {
            std::env::remove_var(key);
        }
    }

    #[test]
    fn stellar_precedence_cctp_over_stellar_over_soroban() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_env();
        std::env::set_var("SOROBAN_RPC_URL", "https://soroban.example");
        std::env::set_var("STELLAR_RPC_URL", "https://stellar.example");
        std::env::set_var("CCTP_STELLAR_RPC_URL", "https://cctp-stellar.example");
        assert_eq!(
            resolve_stellar_rpc_primary(),
            "https://cctp-stellar.example"
        );
        reset_env();
    }

    #[test]
    fn stellar_falls_back_to_official_testnet_default() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_env();
        assert_eq!(resolve_stellar_rpc_primary(), DEFAULT_STELLAR_TESTNET_RPC);
        reset_env();
    }

    #[test]
    fn sepolia_precedence_cctp_over_generic() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_env();
        std::env::set_var("SEPOLIA_RPC_URL", "https://sepolia-a.example");
        std::env::set_var("CCTP_SEPOLIA_RPC_URL", "https://sepolia-b.example");
        assert_eq!(resolve_sepolia_rpc_primary(), "https://sepolia-b.example");
        reset_env();
    }

    #[test]
    fn sepolia_empty_when_unset() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_env();
        assert!(resolve_sepolia_rpc_primary().is_empty());
        reset_env();
    }

    #[test]
    fn enabled_cctp_requires_explicit_sepolia_rpc() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_env();
        std::env::set_var("CCTP_ENABLED", "true");
        assert!(require_sepolia_rpc_when_cctp_enabled().is_err());
        std::env::set_var("SEPOLIA_RPC_URL", "https://sepolia.drpc.org");
        assert!(require_sepolia_rpc_when_cctp_enabled().is_ok());
        reset_env();
    }

    #[test]
    fn rejects_credential_urls() {
        let err = validate_rpc_url("https://user:secret@rpc.example.com");
        assert!(err.is_err());
    }

    #[test]
    fn rejects_overlong_url_lists() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_env();
        let many = (0..9)
            .map(|i| format!("https://rpc{i}.example"))
            .collect::<Vec<_>>()
            .join(",");
        std::env::set_var("CCTP_SEPOLIA_RPC_URL", "https://primary.example");
        std::env::set_var("CCTP_SEPOLIA_RPC_FALLBACK_URLS", many);
        assert!(resolve_sepolia_rpc_urls().is_err());
        reset_env();
    }

    #[test]
    fn sepolia_chain_id_aliases() {
        assert!(is_sepolia_chain_id("0xaa36a7"));
        assert!(is_sepolia_chain_id("11155111"));
        assert!(!is_sepolia_chain_id("0x1"));
    }

    #[test]
    fn rpc_url_host_redacts_path() {
        assert_eq!(
            rpc_url_host("https://soroban-testnet.stellar.org/soroban/rpc"),
            Some("soroban-testnet.stellar.org".into())
        );
    }
}
