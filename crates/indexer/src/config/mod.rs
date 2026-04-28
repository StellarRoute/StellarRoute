use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct IndexerConfig {
    /// Horizon base URL, e.g. `https://horizon.stellar.org` or `https://horizon-testnet.stellar.org`
    pub stellar_horizon_url: String,

    /// Soroban RPC base URL
    pub soroban_rpc_url: String,

    /// Router contract address for AMM pool discovery
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
}

impl std::fmt::Debug for IndexerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexerConfig")
            .field("stellar_horizon_url", &self.stellar_horizon_url)
            .field("soroban_rpc_url", &self.soroban_rpc_url)
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

impl IndexerConfig {
    pub fn load() -> std::result::Result<Self, config::ConfigError> {
        let cfg = config::Config::builder()
            .add_source(config::Environment::default())
            .build()?;
        cfg.try_deserialize()
    }

    /// Convenience constructor from environment variables.
    pub fn from_env() -> std::result::Result<Self, config::ConfigError> {
        let required = [
            "DATABASE_URL",
            "STELLAR_HORIZON_URL",
            "SOROBAN_RPC_URL",
            "ROUTER_CONTRACT_ADDRESS",
        ];
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

        Self::load()
    }
}

// Optional alias if you still want it:
pub type Config = IndexerConfig;
