//! Automated stale-quote purger for replay_artifacts and route_audit_log
//!
//! Provides:
//! - Configurable retention-based purging on a schedule
//! - Observability hooks with detailed metrics (age distributions, deleted counts)
//! - Safe guardrails to prevent over-aggressive deletion (batch limits, iteration limits)
//! - Comprehensive logging and alerting

pub mod config;

use sqlx::PgPool;
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

pub use config::PurgerConfig;

use crate::error::Result;

/// Purge operation result
#[derive(Debug, Clone)]
pub struct PurgeResult {
    pub purge_type: String,
    pub deleted_count: i64,
    pub scanned_count: i64,
    pub rows_retained: i64,
    pub duration_ms: i32,
    pub age_min_days: Option<f64>,
    pub age_max_days: Option<f64>,
    pub age_p50_days: Option<f64>,
    pub age_p95_days: Option<f64>,
    pub age_p99_days: Option<f64>,
    pub was_rate_limited: bool,
}

impl PurgeResult {
    /// Check if this result indicates an alert condition
    pub fn should_alert(&self, config: &PurgerConfig) -> bool {
        let duration_secs = (self.duration_ms as f64) / 1000.0;
        
        (duration_secs > config.slow_purge_threshold_secs as f64) 
            || (self.deleted_count > config.alert_deletion_threshold)
            || self.was_rate_limited
    }

    /// Get alert reason if one applies
    pub fn alert_reason(&self, config: &PurgerConfig) -> Option<String> {
        let duration_secs = (self.duration_ms as f64) / 1000.0;
        
        if self.was_rate_limited {
            return Some("Purge was rate-limited due to large volume".to_string());
        }
        
        if duration_secs > config.slow_purge_threshold_secs as f64 {
            return Some(format!(
                "Purge took {:.1}s (threshold: {}s)",
                duration_secs, config.slow_purge_threshold_secs
            ));
        }
        
        if self.deleted_count > config.alert_deletion_threshold {
            return Some(format!(
                "Deleted {} rows (threshold: {})",
                self.deleted_count, config.alert_deletion_threshold
            ));
        }
        
        None
    }
}

/// Quote artifact purger
pub struct QuoteArtifactPurger {
    pool: PgPool,
    config: PurgerConfig,
}

impl QuoteArtifactPurger {
    /// Create a new purger
    pub fn new(pool: PgPool, config: PurgerConfig) -> Self {
        Self { pool, config }
    }

    /// Run a single purge cycle
    pub async fn run(&self) -> Result<Vec<PurgeResult>> {
        let mut results = Vec::new();

        if !self.config.enabled {
            info!("Quote purger is disabled");
            return Ok(results);
        }

        if self.config.purge_replay_artifacts {
            match self.purge_replay_artifacts().await {
                Ok(result) => {
                    self.log_purge_result(&result);
                    results.push(result);
                }
                Err(e) => {
                    error!(
                        target: "stellarroute.api.purger",
                        error = %e,
                        "Failed to purge replay_artifacts"
                    );
                }
            }
        }

        if self.config.purge_audit_log {
            match self.purge_route_audit_log().await {
                Ok(result) => {
                    self.log_purge_result(&result);
                    results.push(result);
                }
                Err(e) => {
                    error!(
                        target: "stellarroute.api.purger",
                        error = %e,
                        "Failed to purge route_audit_log"
                    );
                }
            }
        }

        Ok(results)
    }

    /// Purge stale replay_artifacts
    async fn purge_replay_artifacts(&self) -> Result<PurgeResult> {
        let start = Instant::now();

        info!(
            target: "stellarroute.api.purger",
            retention_days = self.config.replay_artifacts_retention_days,
            batch_size = self.config.replay_artifacts_batch_size,
            "Starting replay_artifacts purge"
        );

        let result = sqlx::query_as::<_, (i64, i64, i64, Option<f64>, Option<f64>, Option<f64>, Option<f64>, Option<f64>, bool, i32)>(
            r#"
            SELECT * FROM purge_replay_artifacts_older_than(
                $1::INTEGER,
                $2::INTEGER,
                $3::INTEGER
            )
            "#
        )
        .bind(self.config.replay_artifacts_retention_days)
        .bind(self.config.replay_artifacts_batch_size)
        .bind(self.config.max_iterations)
        .fetch_one(&self.pool)
        .await?;

        Ok(PurgeResult {
            purge_type: "replay_artifacts".to_string(),
            deleted_count: result.0,
            scanned_count: result.1,
            rows_retained: result.2,
            age_min_days: result.3,
            age_max_days: result.4,
            age_p50_days: result.5,
            age_p95_days: result.6,
            age_p99_days: result.7,
            was_rate_limited: result.8,
            duration_ms: result.9,
        })
    }

    /// Purge stale route_audit_log entries
    async fn purge_route_audit_log(&self) -> Result<PurgeResult> {
        let start = Instant::now();

        info!(
            target: "stellarroute.api.purger",
            retention_days = self.config.audit_log_retention_days,
            batch_size = self.config.audit_log_batch_size,
            "Starting route_audit_log purge"
        );

        let result = sqlx::query_as::<_, (i64, i64, i64, Option<f64>, Option<f64>, Option<f64>, Option<f64>, Option<f64>, bool, i32)>(
            r#"
            SELECT * FROM purge_route_audit_log_older_than(
                $1::INTEGER,
                $2::INTEGER,
                $3::INTEGER
            )
            "#
        )
        .bind(self.config.audit_log_retention_days)
        .bind(self.config.audit_log_batch_size)
        .bind(self.config.max_iterations)
        .fetch_one(&self.pool)
        .await?;

        Ok(PurgeResult {
            purge_type: "route_audit_log".to_string(),
            deleted_count: result.0,
            scanned_count: result.1,
            rows_retained: result.2,
            age_min_days: result.3,
            age_max_days: result.4,
            age_p50_days: result.5,
            age_p95_days: result.6,
            age_p99_days: result.7,
            was_rate_limited: result.8,
            duration_ms: result.9,
        })
    }

    /// Log structured metrics for a purge result
    fn log_purge_result(&self, result: &PurgeResult) {
        if !self.config.log_metrics {
            return;
        }

        let alert_reason = result.alert_reason(&self.config);
        let level = if alert_reason.is_some() {
            "warn"
        } else {
            "info"
        };

        info!(
            target: "stellarroute.api.purger",
            metric = "stellarroute.api.quote_purge",
            purge_type = %result.purge_type,
            deleted_count = result.deleted_count,
            scanned_count = result.scanned_count,
            rows_retained = result.rows_retained,
            duration_ms = result.duration_ms,
            age_p99_days = result.age_p99_days,
            was_rate_limited = result.was_rate_limited,
            alert = alert_reason.is_some(),
            alert_reason = alert_reason.as_deref().unwrap_or("none"),
            "Quote purge completed"
        );
    }

    /// Get latest purge metrics for dashboarding
    pub async fn get_purge_status(&self) -> Result<Vec<(String, String)>> {
        let rows = sqlx::query_as::<_, (String, Option<String>, Option<i64>, Option<i32>, i64, Option<f64>)>(
            "SELECT * FROM get_quote_purge_status()"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut status = Vec::new();
        for (purge_type, last_purge_at, deleted, duration, retained, age_p99) in rows {
            let last_purge = last_purge_at.unwrap_or_else(|| "never".to_string());
            let deleted_str = deleted.map(|d| d.to_string()).unwrap_or_else(|| "N/A".to_string());
            let duration_str = duration.map(|d| format!("{}ms", d)).unwrap_or_else(|| "N/A".to_string());
            let age_str = age_p99.map(|a| format!("{:.1}d", a)).unwrap_or_else(|| "N/A".to_string());
            
            status.push((
                purge_type.clone(),
                format!(
                    "last_purge={}, deleted={}, duration={}, retained={}, age_p99={}",
                    last_purge, deleted_str, duration_str, retained, age_str
                ),
            ));
        }

        Ok(status)
    }
}

/// Background purger task
pub async fn run_purger_task(pool: PgPool, config: PurgerConfig) {
    if !config.enabled {
        info!("Quote purger task disabled");
        return;
    }

    let purger = QuoteArtifactPurger::new(pool, config.clone());
    let interval = Duration::from_secs(config.interval_secs);

    info!(
        interval_secs = config.interval_secs,
        "Starting quote purger background task"
    );

    loop {
        tokio::time::sleep(interval).await;

        match purger.run().await {
            Ok(results) => {
                for result in results {
                    if let Some(reason) = result.alert_reason(&config) {
                        warn!(
                            target: "stellarroute.api.purger",
                            purge_type = %result.purge_type,
                            reason = %reason,
                            "Quote purge alert"
                        );
                    }
                }
            }
            Err(e) => {
                error!(
                    target: "stellarroute.api.purger",
                    error = %e,
                    "Quote purger task error"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(result.alert_reason(&config).is_some());
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
        assert!(result.alert_reason(&config).is_some());
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
}
