//! Configuration for automated stale-quote purger

use serde::{Deserialize, Serialize};

/// Purger configuration for controlling purge behavior, retention policies, and safeguards
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PurgerConfig {
    /// Enable automated purging on startup (default: true)
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Interval between purge runs in seconds (default: 3600 = 1 hour)
    #[serde(default = "default_interval_secs")]
    pub interval_secs: u64,

    /// Retention for replay_artifacts in days (default: 30)
    #[serde(default = "default_replay_retention_days")]
    pub replay_artifacts_retention_days: i32,

    /// Retention for route_audit_log in days (default: 30)
    #[serde(default = "default_audit_log_retention_days")]
    pub audit_log_retention_days: i32,

    /// Maximum rows to delete per batch (default: 1000 for replay_artifacts)
    /// Smaller batches reduce lock contention but take longer overall
    #[serde(default = "default_replay_batch_size")]
    pub replay_artifacts_batch_size: i32,

    /// Maximum rows to delete per batch for audit log (default: 5000)
    /// Audit log batches can be larger since they're append-only
    #[serde(default = "default_audit_log_batch_size")]
    pub audit_log_batch_size: i32,

    /// Maximum number of delete iterations before rate-limiting (default: 100)
    /// Prevents purger from running indefinitely if table has millions of rows
    #[serde(default = "default_max_iterations")]
    pub max_iterations: i32,

    /// Enable purging of replay_artifacts (default: true)
    #[serde(default = "default_purge_replay_artifacts")]
    pub purge_replay_artifacts: bool,

    /// Enable purging of route_audit_log (default: true)
    #[serde(default = "default_purge_audit_log")]
    pub purge_audit_log: bool,

    /// Log purge metrics to tracing (default: true)
    #[serde(default = "default_log_metrics")]
    pub log_metrics: bool,

    /// Alert if purge takes longer than this many seconds (default: 60)
    #[serde(default = "default_slow_purge_threshold_secs")]
    pub slow_purge_threshold_secs: u64,

    /// Alert if deleted count exceeds this threshold (default: 1_000_000)
    /// High numbers might indicate retention policy drift
    #[serde(default = "default_alert_deletion_threshold")]
    pub alert_deletion_threshold: i64,
}

fn default_enabled() -> bool {
    true
}

fn default_interval_secs() -> u64 {
    3600  // 1 hour
}

fn default_replay_retention_days() -> i32 {
    30
}

fn default_audit_log_retention_days() -> i32 {
    30
}

fn default_replay_batch_size() -> i32 {
    1000
}

fn default_audit_log_batch_size() -> i32 {
    5000
}

fn default_max_iterations() -> i32 {
    100
}

fn default_purge_replay_artifacts() -> bool {
    true
}

fn default_purge_audit_log() -> bool {
    true
}

fn default_log_metrics() -> bool {
    true
}

fn default_slow_purge_threshold_secs() -> u64 {
    60
}

fn default_alert_deletion_threshold() -> i64 {
    1_000_000
}

impl Default for PurgerConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            interval_secs: default_interval_secs(),
            replay_artifacts_retention_days: default_replay_retention_days(),
            audit_log_retention_days: default_audit_log_retention_days(),
            replay_artifacts_batch_size: default_replay_batch_size(),
            audit_log_batch_size: default_audit_log_batch_size(),
            max_iterations: default_max_iterations(),
            purge_replay_artifacts: default_purge_replay_artifacts(),
            purge_audit_log: default_purge_audit_log(),
            log_metrics: default_log_metrics(),
            slow_purge_threshold_secs: default_slow_purge_threshold_secs(),
            alert_deletion_threshold: default_alert_deletion_threshold(),
        }
    }
}

impl PurgerConfig {
    /// Load from environment variables with `QUOTE_PURGER_` prefix
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(v) = std::env::var("QUOTE_PURGER_ENABLED") {
            config.enabled = v.trim().eq_ignore_ascii_case("true");
        }

        if let Ok(v) = std::env::var("QUOTE_PURGER_INTERVAL_SECS") {
            if let Ok(interval) = v.parse() {
                config.interval_secs = interval;
            }
        }

        if let Ok(v) = std::env::var("QUOTE_PURGER_REPLAY_RETENTION_DAYS") {
            if let Ok(days) = v.parse() {
                config.replay_artifacts_retention_days = days;
            }
        }

        if let Ok(v) = std::env::var("QUOTE_PURGER_AUDIT_LOG_RETENTION_DAYS") {
            if let Ok(days) = v.parse() {
                config.audit_log_retention_days = days;
            }
        }

        if let Ok(v) = std::env::var("QUOTE_PURGER_REPLAY_BATCH_SIZE") {
            if let Ok(size) = v.parse() {
                config.replay_artifacts_batch_size = size;
            }
        }

        if let Ok(v) = std::env::var("QUOTE_PURGER_AUDIT_LOG_BATCH_SIZE") {
            if let Ok(size) = v.parse() {
                config.audit_log_batch_size = size;
            }
        }

        if let Ok(v) = std::env::var("QUOTE_PURGER_MAX_ITERATIONS") {
            if let Ok(iterations) = v.parse() {
                config.max_iterations = iterations;
            }
        }

        if let Ok(v) = std::env::var("QUOTE_PURGER_PURGE_REPLAY_ARTIFACTS") {
            config.purge_replay_artifacts = v.trim().eq_ignore_ascii_case("true");
        }

        if let Ok(v) = std::env::var("QUOTE_PURGER_PURGE_AUDIT_LOG") {
            config.purge_audit_log = v.trim().eq_ignore_ascii_case("true");
        }

        if let Ok(v) = std::env::var("QUOTE_PURGER_LOG_METRICS") {
            config.log_metrics = v.trim().eq_ignore_ascii_case("true");
        }

        if let Ok(v) = std::env::var("QUOTE_PURGER_SLOW_PURGE_THRESHOLD_SECS") {
            if let Ok(secs) = v.parse() {
                config.slow_purge_threshold_secs = secs;
            }
        }

        if let Ok(v) = std::env::var("QUOTE_PURGER_ALERT_DELETION_THRESHOLD") {
            if let Ok(threshold) = v.parse() {
                config.alert_deletion_threshold = threshold;
            }
        }

        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = PurgerConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.interval_secs, 3600);
        assert_eq!(cfg.replay_artifacts_retention_days, 30);
        assert_eq!(cfg.audit_log_retention_days, 30);
    }

    #[test]
    fn test_config_serialization() {
        let cfg = PurgerConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let deserialized: PurgerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg.enabled, deserialized.enabled);
        assert_eq!(cfg.interval_secs, deserialized.interval_secs);
    }
}
