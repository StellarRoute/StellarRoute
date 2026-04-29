//! Integration tests for the quote purger

#[cfg(test)]
mod tests {
    use stellarroute_api::purger::{PurgerConfig, QuoteArtifactPurger, PurgeResult};

    #[test]
    fn test_purger_config_default() {
        let cfg = PurgerConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.interval_secs, 3600);
        assert_eq!(cfg.replay_artifacts_retention_days, 30);
        assert_eq!(cfg.audit_log_retention_days, 30);
        assert_eq!(cfg.replay_artifacts_batch_size, 1000);
        assert_eq!(cfg.audit_log_batch_size, 5000);
        assert_eq!(cfg.max_iterations, 100);
    }

    #[test]
    fn test_purger_config_from_env() {
        // Set environment variables
        std::env::set_var("QUOTE_PURGER_ENABLED", "true");
        std::env::set_var("QUOTE_PURGER_INTERVAL_SECS", "1800");
        std::env::set_var("QUOTE_PURGER_REPLAY_RETENTION_DAYS", "14");
        std::env::set_var("QUOTE_PURGER_AUDIT_LOG_RETENTION_DAYS", "7");

        let cfg = PurgerConfig::from_env();
        assert!(cfg.enabled);
        assert_eq!(cfg.interval_secs, 1800);
        assert_eq!(cfg.replay_artifacts_retention_days, 14);
        assert_eq!(cfg.audit_log_retention_days, 7);

        // Cleanup
        std::env::remove_var("QUOTE_PURGER_ENABLED");
        std::env::remove_var("QUOTE_PURGER_INTERVAL_SECS");
        std::env::remove_var("QUOTE_PURGER_REPLAY_RETENTION_DAYS");
        std::env::remove_var("QUOTE_PURGER_AUDIT_LOG_RETENTION_DAYS");
    }

    #[test]
    fn test_purger_config_disabled() {
        std::env::set_var("QUOTE_PURGER_ENABLED", "false");
        let cfg = PurgerConfig::from_env();
        assert!(!cfg.enabled);
        std::env::remove_var("QUOTE_PURGER_ENABLED");
    }

    #[test]
    fn test_purge_result_alert_rate_limited() {
        let config = PurgerConfig::default();
        let result = PurgeResult {
            purge_type: "test".to_string(),
            deleted_count: 100,
            scanned_count: 1000,
            rows_retained: 0,
            duration_ms: 1000,
            age_min_days: Some(1.0),
            age_max_days: Some(30.0),
            age_p50_days: Some(15.0),
            age_p95_days: Some(28.0),
            age_p99_days: Some(29.0),
            was_rate_limited: true,
        };

        assert!(result.should_alert(&config));
        let reason = result.alert_reason(&config);
        assert!(reason.is_some());
        assert!(reason.unwrap().contains("rate-limited"));
    }

    #[test]
    fn test_purge_result_alert_slow() {
        let config = PurgerConfig {
            slow_purge_threshold_secs: 10,
            ..Default::default()
        };
        let result = PurgeResult {
            purge_type: "test".to_string(),
            deleted_count: 100,
            scanned_count: 1000,
            rows_retained: 0,
            duration_ms: 60_000,  // 60 seconds
            age_min_days: Some(1.0),
            age_max_days: Some(30.0),
            age_p50_days: Some(15.0),
            age_p95_days: Some(28.0),
            age_p99_days: Some(29.0),
            was_rate_limited: false,
        };

        assert!(result.should_alert(&config));
        let reason = result.alert_reason(&config);
        assert!(reason.is_some());
        assert!(reason.unwrap().contains("took"));
    }

    #[test]
    fn test_purge_result_alert_high_deletion_threshold() {
        let config = PurgerConfig {
            alert_deletion_threshold: 100,
            ..Default::default()
        };
        let result = PurgeResult {
            purge_type: "test".to_string(),
            deleted_count: 1000,  // exceeds threshold
            scanned_count: 1000,
            rows_retained: 0,
            duration_ms: 1000,
            age_min_days: Some(1.0),
            age_max_days: Some(30.0),
            age_p50_days: Some(15.0),
            age_p95_days: Some(28.0),
            age_p99_days: Some(29.0),
            was_rate_limited: false,
        };

        assert!(result.should_alert(&config));
        let reason = result.alert_reason(&config);
        assert!(reason.is_some());
        assert!(reason.unwrap().contains("Deleted"));
    }

    #[test]
    fn test_purge_result_no_alert() {
        let config = PurgerConfig::default();
        let result = PurgeResult {
            purge_type: "test".to_string(),
            deleted_count: 100,
            scanned_count: 1000,
            rows_retained: 0,
            duration_ms: 1000,
            age_min_days: Some(1.0),
            age_max_days: Some(30.0),
            age_p50_days: Some(15.0),
            age_p95_days: Some(28.0),
            age_p99_days: Some(29.0),
            was_rate_limited: false,
        };

        assert!(!result.should_alert(&config));
        assert!(result.alert_reason(&config).is_none());
    }

    #[test]
    fn test_purge_result_alert_reason_priority() {
        // Rate-limited takes priority over slow
        let config = PurgerConfig {
            slow_purge_threshold_secs: 10,
            ..Default::default()
        };
        let result = PurgeResult {
            purge_type: "test".to_string(),
            deleted_count: 100,
            scanned_count: 1000,
            rows_retained: 0,
            duration_ms: 60_000,
            age_min_days: Some(1.0),
            age_max_days: Some(30.0),
            age_p50_days: Some(15.0),
            age_p95_days: Some(28.0),
            age_p99_days: Some(29.0),
            was_rate_limited: true,
        };

        let reason = result.alert_reason(&config).unwrap();
        assert!(reason.contains("rate-limited"));
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let original = PurgerConfig {
            enabled: true,
            interval_secs: 1800,
            replay_artifacts_retention_days: 14,
            audit_log_retention_days: 7,
            replay_artifacts_batch_size: 500,
            audit_log_batch_size: 2000,
            max_iterations: 50,
            purge_replay_artifacts: true,
            purge_audit_log: false,
            log_metrics: true,
            slow_purge_threshold_secs: 30,
            alert_deletion_threshold: 500000,
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: PurgerConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(original.enabled, deserialized.enabled);
        assert_eq!(original.interval_secs, deserialized.interval_secs);
        assert_eq!(original.replay_artifacts_retention_days, deserialized.replay_artifacts_retention_days);
        assert_eq!(original.audit_log_retention_days, deserialized.audit_log_retention_days);
        assert_eq!(original.replay_artifacts_batch_size, deserialized.replay_artifacts_batch_size);
        assert_eq!(original.audit_log_batch_size, deserialized.audit_log_batch_size);
        assert_eq!(original.max_iterations, deserialized.max_iterations);
        assert_eq!(original.purge_replay_artifacts, deserialized.purge_replay_artifacts);
        assert_eq!(original.purge_audit_log, deserialized.purge_audit_log);
        assert_eq!(original.log_metrics, deserialized.log_metrics);
        assert_eq!(original.slow_purge_threshold_secs, deserialized.slow_purge_threshold_secs);
        assert_eq!(original.alert_deletion_threshold, deserialized.alert_deletion_threshold);
    }

    #[test]
    fn test_config_env_overrides_defaults() {
        // Verify that setting specific variables overrides defaults while leaving others untouched
        std::env::set_var("QUOTE_PURGER_REPLAY_RETENTION_DAYS", "60");
        std::env::set_var("QUOTE_PURGER_AUDIT_LOG_BATCH_SIZE", "10000");

        let cfg = PurgerConfig::from_env();

        // These should be overridden
        assert_eq!(cfg.replay_artifacts_retention_days, 60);
        assert_eq!(cfg.audit_log_batch_size, 10000);

        // These should use defaults
        assert_eq!(cfg.interval_secs, 3600);
        assert_eq!(cfg.audit_log_retention_days, 30);

        std::env::remove_var("QUOTE_PURGER_REPLAY_RETENTION_DAYS");
        std::env::remove_var("QUOTE_PURGER_AUDIT_LOG_BATCH_SIZE");
    }

    #[test]
    fn test_purge_result_with_missing_age_distribution() {
        let config = PurgerConfig::default();
        let result = PurgeResult {
            purge_type: "test".to_string(),
            deleted_count: 0,  // No rows deleted, so no age data
            scanned_count: 0,
            rows_retained: 1000,
            duration_ms: 100,
            age_min_days: None,
            age_max_days: None,
            age_p50_days: None,
            age_p95_days: None,
            age_p99_days: None,
            was_rate_limited: false,
        };

        assert!(!result.should_alert(&config));
    }

    #[test]
    fn test_config_boolean_parsing() {
        for val in &["true", "True", "TRUE", "yes", "1", "on"] {
            std::env::set_var("QUOTE_PURGER_ENABLED", val);
            let cfg = PurgerConfig::from_env();
            assert!(cfg.enabled, "Failed for value: {}", val);
            std::env::remove_var("QUOTE_PURGER_ENABLED");
        }

        for val in &["false", "False", "FALSE", "no", "0", "off", ""] {
            std::env::set_var("QUOTE_PURGER_ENABLED", val);
            let cfg = PurgerConfig::from_env();
            assert!(!cfg.enabled, "Failed for value: {}", val);
            std::env::remove_var("QUOTE_PURGER_ENABLED");
        }
    }
}
