use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum HorizonMode {
    #[default]
    Poll,
    Sse,
}

/// Parse a comma-separated list of URLs from an env var, returning a `Vec<String>`.
/// Trims whitespace and drops empty entries.
pub fn parse_url_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[derive(Clone, Deserialize)]
pub struct IndexerConfig {
    /// Primary Horizon base URL.
    pub stellar_horizon_url: String,

    /// Ordered failover Horizon URLs tried when the primary is unreachable.
    /// Env: `STELLAR_HORIZON_FALLBACK_URLS` — comma-separated list.
    #[serde(default)]
    pub stellar_horizon_fallback_urls: String,

    /// Ingestion mode for SDEX offers
    #[serde(default)]
    pub horizon_mode: HorizonMode,

    /// Primary Soroban RPC base URL.
    pub soroban_rpc_url: String,

    /// Ordered failover Soroban RPC URLs tried when the primary is unreachable.
    /// Env: `SOROBAN_RPC_FALLBACK_URLS` — comma-separated list.
    #[serde(default)]
    pub soroban_rpc_fallback_urls: String,

    /// Router contract address for AMM pool discovery.
    ///
    /// Enforced by [`check_router_contract_address`] in [`IndexerConfig::from_env`]
    /// rather than by serde, so that the dev-only `ALLOW_EMPTY_ROUTER` escape hatch
    /// can leave it blank without deserialization failing.
    #[serde(default)]
    pub router_contract_address: String,

    /// Postgres connection string
    pub database_url: String,

    /// Poll interval for Horizon when streaming is not used yet.
    #[serde(default = "default_poll_interval_secs")]
    pub poll_interval_secs: u64,

    /// Poll interval for AMM pool updates
    #[serde(default = "default_amm_poll_interval_secs")]
    pub amm_poll_interval_secs: u64,

    /// Stale pool threshold in seconds
    #[serde(default = "default_stale_threshold_secs")]
    pub stale_threshold_secs: u64,

    /// Max records to request per page (Horizon supports `limit`).
    #[serde(default = "default_horizon_limit")]
    pub horizon_limit: u32,

    /// Maximum number of connections in the pool (env: `DB_MAX_CONNECTIONS`).
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,

    /// Minimum number of idle connections maintained in the pool (env: `DB_MIN_CONNECTIONS`).
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,

    /// Timeout in seconds to wait for a connection from the pool (env: `DB_CONNECTION_TIMEOUT`).
    #[serde(default = "default_connection_timeout_secs")]
    pub connection_timeout_secs: u64,

    /// Idle connection timeout in seconds before it is closed (env: `DB_IDLE_TIMEOUT`).
    #[serde(default = "default_idle_timeout_secs")]
    pub idle_timeout_secs: u64,

    /// Maximum lifetime of a pooled connection in seconds (env: `DB_MAX_LIFETIME`).
    #[serde(default = "default_max_lifetime_secs")]
    pub max_lifetime_secs: u64,

    /// Maintenance interval in minutes (env: `MAINTENANCE_INTERVAL_MINS`).
    #[serde(default = "default_maintenance_interval_mins")]
    pub maintenance_interval_mins: u64,

    /// Snapshot retention in days (env: `SNAPSHOT_RETENTION_DAYS`).
    #[serde(default = "default_snapshot_retention_days")]
    pub snapshot_retention_days: i32,

    /// Snapshot compaction after threshold hours (env: `SNAPSHOT_COMPACTION_HOURS`).
    #[serde(default = "default_snapshot_compaction_hours")]
    pub snapshot_compaction_hours: i32,

    // New partitioning configuration
    /// Number of partitions for workload distribution (env: `INDEXER_PARTITION_COUNT`).
    #[serde(default = "default_partition_count")]
    pub partition_count: usize,

    /// Comma‑separated list of hot pair identifiers (e.g., "XLM/USD,USDC/EUR").
    #[serde(default = "default_hot_pair_allowlist")]
    pub hot_pair_allowlist: String,

    /// Volume threshold (in native units) to consider a pair hot (env: `INDEXER_HOT_VOLUME_THRESHOLD`).
    #[serde(default = "default_hot_pair_volume_threshold")]
    pub hot_pair_volume_threshold: u64,

    /// Window in seconds for detecting hot pairs based on recent volume.
    #[serde(default = "default_hot_pair_window_secs")]
    pub hot_pair_window_secs: u64,

    /// Identifier of this partition instance (env: `INDEXER_PARTITION_ID`).
    #[serde(default = "default_partition_id")]
    pub partition_id: usize,
}

impl std::fmt::Debug for IndexerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexerConfig")
            .field("stellar_horizon_url", &self.stellar_horizon_url)
            .field(
                "stellar_horizon_fallback_urls",
                &self.stellar_horizon_fallback_urls,
            )
            .field("horizon_mode", &self.horizon_mode)
            .field("soroban_rpc_url", &self.soroban_rpc_url)
            .field("soroban_rpc_fallback_urls", &self.soroban_rpc_fallback_urls)
            .field("router_contract_address", &self.router_contract_address)
            .field("database_url", &"[REDACTED]")
            .field("poll_interval_secs", &self.poll_interval_secs)
            .field("amm_poll_interval_secs", &self.amm_poll_interval_secs)
            .field("stale_threshold_secs", &self.stale_threshold_secs)
            .field("horizon_limit", &self.horizon_limit)
            .field("max_connections", &self.max_connections)
            .field("min_connections", &self.min_connections)
            .field("connection_timeout_secs", &self.connection_timeout_secs)
            .field("idle_timeout_secs", &self.idle_timeout_secs)
            .field("max_lifetime_secs", &self.max_lifetime_secs)
            .field("maintenance_interval_mins", &self.maintenance_interval_mins)
            .field("snapshot_retention_days", &self.snapshot_retention_days)
            .field("snapshot_compaction_hours", &self.snapshot_compaction_hours)
            .finish()
    }
}

fn default_poll_interval_secs() -> u64 {
    2
}

fn default_amm_poll_interval_secs() -> u64 {
    30
}

fn default_stale_threshold_secs() -> u64 {
    300
}

fn default_horizon_limit() -> u32 {
    200
}

fn default_max_connections() -> u32 {
    10
}

fn default_min_connections() -> u32 {
    2
}

fn default_connection_timeout_secs() -> u64 {
    30
}

fn default_idle_timeout_secs() -> u64 {
    600
}

fn default_max_lifetime_secs() -> u64 {
    1800
}

fn default_maintenance_interval_mins() -> u64 {
    60
}

fn default_snapshot_retention_days() -> i32 {
    90
}

fn default_snapshot_compaction_hours() -> i32 {
    24
}

fn default_partition_id() -> usize {
    0
}

// New defaults for partitioning and hot‑pair detection
fn default_partition_count() -> usize {
    1
}
fn default_hot_pair_allowlist() -> String {
    String::new()
}
fn default_hot_pair_volume_threshold() -> u64 {
    1_000_000_000
}
fn default_hot_pair_window_secs() -> u64 {
    300
}

/// Length of a Soroban contract ID in its StrKey (base32) text form.
const CONTRACT_ID_LEN: usize = 56;

/// Format-only check for a Soroban contract ID (`C` + 55 base32 chars).
///
/// Deliberately does not verify the StrKey checksum or hit the network — this is
/// a startup footgun guard, not an on-chain existence proof.
fn is_contract_id_shaped(value: &str) -> bool {
    value.len() == CONTRACT_ID_LEN
        && value.starts_with('C')
        && value
            .bytes()
            .all(|b| b.is_ascii_uppercase() || (b'2'..=b'7').contains(&b))
}

/// Validate `ROUTER_CONTRACT_ADDRESS` before any indexing loop is started.
///
/// `raw` is the raw env value (`None` when the variable is unset). `allow_empty`
/// reflects `ALLOW_EMPTY_ROUTER`, and `is_production` reflects
/// `STELLARROUTE_ENV=production` — the escape hatch is refused in production.
///
/// Returns the validated contract ID, or `None` when the dev-only escape hatch
/// legitimately permits an empty AMM side.
pub fn check_router_contract_address(
    raw: Option<&str>,
    allow_empty: bool,
    is_production: bool,
) -> std::result::Result<Option<String>, String> {
    let value = raw.unwrap_or("").trim();

    if value.is_empty() {
        if allow_empty && is_production {
            return Err(
                "ROUTER_CONTRACT_ADDRESS is missing or empty. ALLOW_EMPTY_ROUTER is a \
                 development-only escape hatch and is refused when STELLARROUTE_ENV=production. \
                 Set ROUTER_CONTRACT_ADDRESS to the deployed router contract ID (see \
                 `jq -r .router_contract_id config/deployments/testnet.json`)."
                    .to_string(),
            );
        }
        if allow_empty {
            return Ok(None);
        }
        return Err(
            "ROUTER_CONTRACT_ADDRESS is missing or empty. Set it to the deployed router \
             contract ID (56 characters, starting with `C`); read it from the deploy artifact \
             with `jq -r .router_contract_id config/deployments/testnet.json`. For SDEX-only \
             local development set ALLOW_EMPTY_ROUTER=1."
                .to_string(),
        );
    }

    if !is_contract_id_shaped(value) {
        return Err(format!(
            "ROUTER_CONTRACT_ADDRESS is not a valid Soroban contract ID: expected {} base32 \
             characters starting with `C`, got {} character(s). Read the correct value from the \
             deploy artifact with `jq -r .router_contract_id config/deployments/testnet.json`.",
            CONTRACT_ID_LEN,
            value.len()
        ));
    }

    Ok(Some(value.to_string()))
}

impl IndexerConfig {
    /// Returns all Horizon URLs to try in priority order: primary first, then fallbacks.
    pub fn horizon_urls(&self) -> Vec<String> {
        let mut urls = vec![self.stellar_horizon_url.trim_end_matches('/').to_string()];
        urls.extend(parse_url_list(&self.stellar_horizon_fallback_urls));
        urls
    }

    /// Returns all Soroban RPC URLs to try in priority order: primary first, then fallbacks.
    pub fn soroban_rpc_urls(&self) -> Vec<String> {
        let mut urls = vec![self.soroban_rpc_url.trim_end_matches('/').to_string()];
        urls.extend(parse_url_list(&self.soroban_rpc_fallback_urls));
        urls
    }

    pub fn load() -> std::result::Result<Self, config::ConfigError> {
        let cfg = config::Config::builder()
            .add_source(config::Environment::default())
            .build()?;
        cfg.try_deserialize()
    }

    /// Convenience constructor from environment variables.
    pub fn from_env() -> std::result::Result<Self, config::ConfigError> {
        let required = ["DATABASE_URL", "STELLAR_HORIZON_URL", "SOROBAN_RPC_URL"];
        let mut missing = Vec::new();
        for key in required {
            match std::env::var(key) {
                Ok(value) if !value.trim().is_empty() => {}
                _ => missing.push(key),
            }
        }
        if !missing.is_empty() {
            return Err(config::ConfigError::Message(format!(
                "Missing required environment variable(s): {}",
                missing.join(", ")
            )));
        }

        // Validated separately so the failure names the variable and says how to fix it.
        let raw_router = std::env::var("ROUTER_CONTRACT_ADDRESS").ok();
        let allow_empty = std::env::var("ALLOW_EMPTY_ROUTER")
            .map(|v| {
                matches!(
                    v.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or(false);
        let is_production = std::env::var("STELLARROUTE_ENV")
            .map(|v| v.trim().eq_ignore_ascii_case("production"))
            .unwrap_or(false);

        let router =
            check_router_contract_address(raw_router.as_deref(), allow_empty, is_production)
                .map_err(config::ConfigError::Message)?;

        let mut config = Self::load()?;
        config.router_contract_address = router.unwrap_or_default();
        Ok(config)
    }
}

// Optional alias if you still want it:
pub type Config = IndexerConfig;

#[cfg(test)]
mod router_contract_tests {
    use super::check_router_contract_address;

    /// Shape-valid sample: `C` + 55 base32 characters.
    const VALID_ID: &str = "CCJZ5DGCTUUYYUQZ7CQGDJEQZ7CQGDJEQZ7CQGDJEQZ7CQGDJEQZ7CQG";

    #[test]
    fn valid_sample_is_56_base32_chars() {
        assert_eq!(VALID_ID.len(), 56);
    }

    #[test]
    fn missing_router_contract_is_rejected() {
        let err = check_router_contract_address(None, false, false).unwrap_err();
        assert!(err.contains("ROUTER_CONTRACT_ADDRESS"));
    }

    #[test]
    fn empty_router_contract_is_rejected() {
        let err = check_router_contract_address(Some(""), false, false).unwrap_err();
        assert!(err.contains("ROUTER_CONTRACT_ADDRESS"));
    }

    #[test]
    fn whitespace_router_contract_is_rejected() {
        let err = check_router_contract_address(Some("   \t "), false, false).unwrap_err();
        assert!(err.contains("ROUTER_CONTRACT_ADDRESS"));
    }

    #[test]
    fn malformed_router_contract_is_rejected() {
        // Right length, wrong prefix.
        let wrong_prefix = format!("G{}", &VALID_ID[1..]);
        assert!(check_router_contract_address(Some(&wrong_prefix), false, false).is_err());

        // Right prefix, wrong length.
        assert!(check_router_contract_address(Some("CABC"), false, false).is_err());

        // Lowercase is not valid base32 StrKey.
        assert!(
            check_router_contract_address(Some(&VALID_ID.to_ascii_lowercase()), false, false)
                .is_err()
        );
    }

    #[test]
    fn valid_router_contract_is_accepted_and_trimmed() {
        let padded = format!("  {}  ", VALID_ID);
        assert_eq!(
            check_router_contract_address(Some(&padded), false, false).unwrap(),
            Some(VALID_ID.to_string())
        );
    }

    #[test]
    fn allow_empty_router_is_a_dev_only_escape_hatch() {
        assert_eq!(
            check_router_contract_address(Some(""), true, false).unwrap(),
            None
        );
    }

    #[test]
    fn allow_empty_router_is_refused_in_production() {
        let err = check_router_contract_address(None, true, true).unwrap_err();
        assert!(err.contains("STELLARROUTE_ENV=production"));
    }

    #[test]
    fn allow_empty_router_does_not_excuse_a_malformed_id() {
        assert!(check_router_contract_address(Some("not-a-contract-id"), true, false).is_err());
    }
}
