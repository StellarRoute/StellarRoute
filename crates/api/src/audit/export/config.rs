//! Configuration for the audit log export pipeline.

use serde::{Deserialize, Serialize};

/// Export pipeline configuration loaded from `AUDIT_EXPORT_*` environment variables.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditExportConfig {
    /// Enable the scheduled export background task.
    pub enabled: bool,
    /// Interval between export runs in seconds.
    pub interval_secs: u64,
    /// Maximum rows exported per batch.
    pub batch_size: i32,
    /// Root prefix for object keys (e.g. `stellarroute/audit-logs`).
    pub object_prefix: String,
    /// Local filesystem directory for exported objects.
    pub storage_path: String,
    /// Alert when an export run exceeds this duration in seconds.
    pub slow_export_threshold_secs: u64,
    /// Alert when consecutive failures exceed this count.
    pub alert_failure_streak: u32,
}

impl Default for AuditExportConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_secs: 900,
            batch_size: 5_000,
            object_prefix: "stellarroute/audit-logs".to_string(),
            storage_path: "./data/audit-exports".to_string(),
            slow_export_threshold_secs: 120,
            alert_failure_streak: 3,
        }
    }
}

impl AuditExportConfig {
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(v) = std::env::var("AUDIT_EXPORT_ENABLED") {
            config.enabled = v.trim().eq_ignore_ascii_case("true");
        }
        if let Ok(v) = std::env::var("AUDIT_EXPORT_INTERVAL_SECS") {
            if let Ok(n) = v.parse() {
                config.interval_secs = n;
            }
        }
        if let Ok(v) = std::env::var("AUDIT_EXPORT_BATCH_SIZE") {
            if let Ok(n) = v.parse() {
                config.batch_size = n;
            }
        }
        if let Ok(v) = std::env::var("AUDIT_EXPORT_OBJECT_PREFIX") {
            if !v.trim().is_empty() {
                config.object_prefix = v.trim().trim_matches('/').to_string();
            }
        }
        if let Ok(v) = std::env::var("AUDIT_EXPORT_STORAGE_PATH") {
            if !v.trim().is_empty() {
                config.storage_path = v.trim().to_string();
            }
        }
        if let Ok(v) = std::env::var("AUDIT_EXPORT_SLOW_THRESHOLD_SECS") {
            if let Ok(n) = v.parse() {
                config.slow_export_threshold_secs = n;
            }
        }
        if let Ok(v) = std::env::var("AUDIT_EXPORT_ALERT_FAILURE_STREAK") {
            if let Ok(n) = v.parse() {
                config.alert_failure_streak = n;
            }
        }

        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_disabled() {
        let cfg = AuditExportConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(cfg.batch_size, 5_000);
    }
}
