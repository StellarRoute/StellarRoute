//! Scheduled audit log export pipeline with checkpointing and redaction.

use std::sync::Arc;
use std::time::Instant;
use std::io::Write;

use flate2::write::GzEncoder;
use flate2::Compression;
use sqlx::PgPool;
use tracing::{error, info, warn};

use super::config::AuditExportConfig;
use super::storage::{
    build_manifest_key, build_object_key, LocalObjectStorage,
};
use crate::audit::redactor::AuditRedactor;
use crate::audit::schema::AUDIT_SCHEMA_VERSION;
use crate::audit::store::AuditStore;
use crate::error::Result;

/// Result of a single export run.
#[derive(Debug, Clone)]
pub struct ExportRunResult {
    pub rows_exported: i64,
    pub checkpoint_before: i64,
    pub checkpoint_after: i64,
    pub object_key: Option<String>,
    pub duration_ms: i64,
}

impl ExportRunResult {
    pub fn should_alert(&self, config: &AuditExportConfig) -> Option<String> {
        let duration_secs = self.duration_ms as f64 / 1000.0;
        if duration_secs > config.slow_export_threshold_secs as f64 {
            return Some(format!(
                "Audit export took {:.1}s (threshold: {}s)",
                duration_secs, config.slow_export_threshold_secs
            ));
        }
        None
    }
}

/// Incrementally exports `route_audit_log` rows to object storage.
pub struct AuditExportPipeline {
    store: AuditStore,
    storage: LocalObjectStorage,
    config: AuditExportConfig,
}

impl AuditExportPipeline {
    pub fn new(pool: PgPool, config: AuditExportConfig) -> Self {
        Self {
            store: AuditStore::new(pool),
            storage: LocalObjectStorage::new(&config.storage_path),
            config,
        }
    }

    /// Run one export cycle.
    pub async fn run_once(&self) -> Result<Option<ExportRunResult>> {
        let start = Instant::now();
        let checkpoint_before = self.store.get_export_checkpoint().await?;
        let run_id = self
            .store
            .begin_export_run(checkpoint_before)
            .await?;

        let export_outcome = self.export_batch(checkpoint_before).await;

        match export_outcome {
            Ok(result) => {
                self.store
                    .finish_export_run(run_id, "success", result.checkpoint_after, result.rows_exported, result.object_key.as_deref(), None)
                    .await?;
                if result.rows_exported > 0 {
                    self.store
                        .update_export_checkpoint(
                            result.checkpoint_after,
                            result.object_key.as_deref(),
                            result.rows_exported,
                        )
                        .await?;
                }
                let duration_ms = start.elapsed().as_millis() as i64;
                let final_result = ExportRunResult {
                    duration_ms,
                    ..result
                };
                if let Some(reason) = final_result.should_alert(&self.config) {
                    warn!(
                        target: "stellarroute.api.audit_export",
                        alert_reason = %reason,
                        rows_exported = final_result.rows_exported,
                        "Audit export slow-run alert"
                    );
                    crate::metrics::record_audit_export_alert("slow_run");
                }
                crate::metrics::record_audit_export_success(final_result.rows_exported);
                Ok(Some(final_result))
            }
            Err(e) => {
                let msg = e.to_string();
                self.store
                    .finish_export_run(run_id, "failed", checkpoint_before, 0, None, Some(&msg))
                    .await?;
                crate::metrics::record_audit_export_failure();
                error!(
                    target: "stellarroute.api.audit_export",
                    error = %msg,
                    checkpoint = checkpoint_before,
                    "Audit export run failed"
                );
                Err(e)
            }
        }
    }

    async fn export_batch(&self, checkpoint_before: i64) -> Result<ExportRunResult> {
        let entries = self
            .store
            .fetch_batch_after_id(checkpoint_before, self.config.batch_size)
            .await?;

        if entries.is_empty() {
            return Ok(ExportRunResult {
                rows_exported: 0,
                checkpoint_before,
                checkpoint_after: checkpoint_before,
                object_key: None,
                duration_ms: 0,
            });
        }

        let first_id = entries.first().map(|e| e.id).unwrap_or(checkpoint_before);
        let last_id = entries.last().map(|e| e.id).unwrap_or(checkpoint_before);
        let unix_ts = chrono::Utc::now().timestamp();
        let object_key = build_object_key(
            &self.config.object_prefix,
            first_id,
            last_id,
            unix_ts,
        );

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        let mut min_logged_at = entries[0].entry.logged_at;
        let mut max_logged_at = entries[0].entry.logged_at;

        for mut wrapped in entries {
            AuditRedactor::redact(&mut wrapped.entry);
            min_logged_at = min_logged_at.min(wrapped.entry.logged_at);
            max_logged_at = max_logged_at.max(wrapped.entry.logged_at);
            let line = serde_json::to_string(&wrapped).map_err(|e| {
                crate::error::ApiError::Internal(Arc::new(anyhow::anyhow!(
                    "Failed to serialize audit entry: {}",
                    e
                )))
            })?;
            writeln!(encoder, "{}", line).map_err(|e| {
                crate::error::ApiError::Internal(Arc::new(anyhow::anyhow!(
                    "Failed to encode audit export batch: {}",
                    e
                )))
            })?;
        }

        let payload = encoder.finish().map_err(|e| {
            crate::error::ApiError::Internal(Arc::new(anyhow::anyhow!(
                "Failed to finalize gzip export batch: {}",
                e
            )))
        })?;

        self.storage.put_object(&object_key, &payload).await?;

        let manifest_key = build_manifest_key(&object_key);
        let manifest = serde_json::json!({
            "schema_version": AUDIT_SCHEMA_VERSION,
            "redaction_rules_version": 1,
            "redaction_rules_doc": "docs/AUDIT_LOG_EXPORT_RUNBOOK.md#redaction-rules",
            "format": "jsonl.gz",
            "first_id": first_id,
            "last_id": last_id,
            "row_count": last_id - first_id + 1,
            "time_range": {
                "start": min_logged_at.to_rfc3339(),
                "end": max_logged_at.to_rfc3339(),
            },
            "replay_hint": "Use request_id or trace_id from entries to correlate with incident-replay-workflow.md",
        });
        self.storage
            .put_object(
                &manifest_key,
                serde_json::to_vec_pretty(&manifest)
                    .unwrap_or_default()
                    .as_slice(),
            )
            .await?;

        Ok(ExportRunResult {
            rows_exported: (last_id - first_id + 1) as i64,
            checkpoint_before,
            checkpoint_after: last_id,
            object_key: Some(object_key),
            duration_ms: 0,
        })
    }
}

/// Background task that runs export on a schedule.
pub async fn run_export_task(pool: PgPool, config: AuditExportConfig) {
    if !config.enabled {
        info!("Audit log export task disabled");
        return;
    }

    let pipeline = AuditExportPipeline::new(pool, config.clone());
    let interval = std::time::Duration::from_secs(config.interval_secs);
    let mut failure_streak: u32 = 0;

    info!(
        interval_secs = config.interval_secs,
        batch_size = config.batch_size,
        storage_path = %config.storage_path,
        "Starting audit log export background task"
    );

    loop {
        tokio::time::sleep(interval).await;

        match pipeline.run_once().await {
            Ok(Some(result)) => {
                failure_streak = 0;
                if result.rows_exported > 0 {
                    info!(
                        target: "stellarroute.api.audit_export",
                        rows_exported = result.rows_exported,
                        checkpoint_after = result.checkpoint_after,
                        object_key = result.object_key.as_deref().unwrap_or("none"),
                        duration_ms = result.duration_ms,
                        "Audit log export completed"
                    );
                }
            }
            Ok(None) => {
                failure_streak = 0;
            }
            Err(_) => {
                failure_streak += 1;
                if failure_streak >= config.alert_failure_streak {
                    warn!(
                        target: "stellarroute.api.audit_export",
                        failure_streak,
                        threshold = config.alert_failure_streak,
                        "Audit export failure streak alert"
                    );
                    crate::metrics::record_audit_export_alert("failure_streak");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::schema::{
        AuditExclusion, AuditInputs, AuditOutcome, AuditPathStep, AuditSelected, RouteAuditEntry,
    };

    const ISSUER: &str = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";

    fn sample_entry(id_hint: &str) -> RouteAuditEntry {
        RouteAuditEntry::new(
            format!("req-{}", id_hint),
            "trace-001".to_string(),
            12,
            AuditOutcome::Success,
            false,
            AuditInputs {
                base: format!("USDC:{}", ISSUER),
                quote: "native".to_string(),
                amount: "10.0000000".to_string(),
                slippage_bps: 50,
                quote_type: "sell".to_string(),
            },
            Some(AuditSelected {
                venue_type: "sdex".to_string(),
                venue_ref: "offer1".to_string(),
                price: "1.0000000".to_string(),
                path: vec![AuditPathStep {
                    from: format!("USDC:{}", ISSUER),
                    to: "native".to_string(),
                    price: "1.0000000".to_string(),
                    source: "sdex".to_string(),
                }],
                strategy: "single_hop".to_string(),
            }),
            vec![AuditExclusion {
                venue_ref: "pool1".to_string(),
                reason: "stale_data".to_string(),
            }],
        )
    }

    #[test]
    fn exported_payload_contains_no_raw_issuer() {
        let mut entry = sample_entry("export-test");
        AuditRedactor::redact(&mut entry);
        let json = serde_json::to_string(&entry).unwrap();
        assert!(!json.contains(ISSUER));
        assert!(json.contains("[REDACTED]"));
    }
}
