//! PostgreSQL persistence for route audit log entries.
//!
//! # Retention
//!
//! The `retained_until` column is a generated column (`logged_at + 30 days`).
//! Call [`AuditStore::prune_older_than`] from a background task to enforce
//! the retention policy.  See `docs/audit-log-retention.md` for guidance on
//! tuning the retention window and storage cost.

use chrono::Duration;
use sqlx::{PgPool, Row};
use std::sync::Arc;

use crate::error::{ApiError, Result};

use super::schema::{AuditExclusion, AuditInputs, AuditOutcome, AuditSelected, RouteAuditEntry};

/// PostgreSQL-backed store for [`RouteAuditEntry`] records.
#[derive(Clone)]
pub struct AuditStore {
    db: PgPool,
}

impl AuditStore {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Insert a single audit entry.
    ///
    /// The entry **must** have already been redacted by [`super::AuditRedactor`]
    /// before calling this method.
    pub async fn insert(&self, entry: &RouteAuditEntry) -> Result<i64> {
        let inputs_json = serde_json::to_value(&entry.inputs).map_err(|e| {
            ApiError::Internal(Arc::new(anyhow::anyhow!(
                "Failed to serialize audit inputs: {}",
                e
            )))
        })?;

        let selected_json = entry
            .selected
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| {
                ApiError::Internal(Arc::new(anyhow::anyhow!(
                    "Failed to serialize audit selected: {}",
                    e
                )))
            })?;

        let exclusions_json = serde_json::to_value(&entry.exclusions).map_err(|e| {
            ApiError::Internal(Arc::new(anyhow::anyhow!(
                "Failed to serialize audit exclusions: {}",
                e
            )))
        })?;

        let row = sqlx::query(
            r#"
            INSERT INTO route_audit_log (
                request_id, trace_id, logged_at, latency_ms,
                outcome, cache_hit, inputs, selected, exclusions
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id
            "#,
        )
        .bind(&entry.request_id)
        .bind(&entry.trace_id)
        .bind(entry.logged_at)
        .bind(entry.latency_ms as i32)
        .bind(entry.outcome.as_str())
        .bind(entry.cache_hit)
        .bind(inputs_json)
        .bind(selected_json)
        .bind(exclusions_json)
        .fetch_one(&self.db)
        .await
        .map_err(|e| {
            ApiError::Internal(Arc::new(anyhow::anyhow!(
                "Failed to insert audit entry: {}",
                e
            )))
        })?;

        Ok(row.get::<i64, _>("id"))
    }

    /// Fetch a single audit entry by its auto-generated `id`.
    pub async fn fetch(&self, id: i64) -> Result<RouteAuditEntry> {
        let row = sqlx::query(
            r#"
            SELECT request_id, trace_id, logged_at, latency_ms,
                   outcome, cache_hit, inputs, selected, exclusions
            FROM route_audit_log
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| {
            ApiError::Internal(Arc::new(anyhow::anyhow!(
                "Failed to fetch audit entry: {}",
                e
            )))
        })?
        .ok_or_else(|| ApiError::NotFound(format!("Audit entry not found: {}", id)))?;

        let inputs: AuditInputs = serde_json::from_value(row.get::<serde_json::Value, _>("inputs"))
            .map_err(|e| {
                ApiError::Internal(Arc::new(anyhow::anyhow!(
                    "Failed to deserialize audit inputs: {}",
                    e
                )))
            })?;

        let selected: Option<AuditSelected> = row
            .get::<Option<serde_json::Value>, _>("selected")
            .map(serde_json::from_value)
            .transpose()
            .map_err(|e| {
                ApiError::Internal(Arc::new(anyhow::anyhow!(
                    "Failed to deserialize audit selected: {}",
                    e
                )))
            })?;

        let exclusions: Vec<AuditExclusion> =
            serde_json::from_value(row.get::<serde_json::Value, _>("exclusions")).map_err(|e| {
                ApiError::Internal(Arc::new(anyhow::anyhow!(
                    "Failed to deserialize audit exclusions: {}",
                    e
                )))
            })?;

        let outcome_str: &str = row.get("outcome");
        let outcome = match outcome_str {
            "success" => AuditOutcome::Success,
            "no_route" => AuditOutcome::NoRoute,
            "stale_data" => AuditOutcome::StaleData,
            _ => AuditOutcome::Error,
        };

        Ok(RouteAuditEntry {
            schema_version: super::schema::AUDIT_SCHEMA_VERSION,
            request_id: row.get("request_id"),
            trace_id: row.get("trace_id"),
            logged_at: row.get("logged_at"),
            latency_ms: row.get::<i32, _>("latency_ms") as u64,
            outcome,
            cache_hit: row.get("cache_hit"),
            inputs,
            selected,
            exclusions,
        })
    }

    /// List audit entries for a given `request_id`.
    ///
    /// A single HTTP request may produce multiple entries (e.g. batch quotes).
    pub async fn list_by_request_id(&self, request_id: &str) -> Result<Vec<AuditEntrySummary>> {
        let rows = sqlx::query(
            r#"
            SELECT id, request_id, trace_id, logged_at, latency_ms, outcome, cache_hit
            FROM route_audit_log
            WHERE request_id = $1
            ORDER BY logged_at ASC
            "#,
        )
        .bind(request_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| {
            ApiError::Internal(Arc::new(anyhow::anyhow!(
                "Failed to list audit entries: {}",
                e
            )))
        })?;

        Ok(rows
            .into_iter()
            .map(|r| AuditEntrySummary {
                id: r.get("id"),
                request_id: r.get("request_id"),
                trace_id: r.get("trace_id"),
                logged_at: r.get("logged_at"),
                latency_ms: r.get::<i32, _>("latency_ms") as u64,
                outcome: r.get::<&str, _>("outcome").to_string(),
                cache_hit: r.get("cache_hit"),
            })
            .collect())
    }

    /// List audit entries for a given `trace_id`.
    pub async fn list_by_trace_id(&self, trace_id: &str) -> Result<Vec<AuditEntrySummary>> {
        let rows = sqlx::query(
            r#"
            SELECT id, request_id, trace_id, logged_at, latency_ms, outcome, cache_hit
            FROM route_audit_log
            WHERE trace_id = $1
            ORDER BY logged_at ASC
            "#,
        )
        .bind(trace_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| {
            ApiError::Internal(Arc::new(anyhow::anyhow!(
                "Failed to list audit entries by trace: {}",
                e
            )))
        })?;

        Ok(rows
            .into_iter()
            .map(|r| AuditEntrySummary {
                id: r.get("id"),
                request_id: r.get("request_id"),
                trace_id: r.get("trace_id"),
                logged_at: r.get("logged_at"),
                latency_ms: r.get::<i32, _>("latency_ms") as u64,
                outcome: r.get::<&str, _>("outcome").to_string(),
                cache_hit: r.get("cache_hit"),
            })
            .collect())
    }

    /// Delete entries whose `retained_until` timestamp is in the past.
    ///
    /// Returns the number of rows deleted.
    ///
    /// # Retention policy
    ///
    /// The `retained_until` column is computed as `logged_at + 30 days`.
    /// This method deletes all rows where `retained_until <= NOW()`, which
    /// is equivalent to rows older than 30 days.
    ///
    /// For custom retention windows, pass a negative `extra_grace` to delete
    /// sooner, or a positive value to keep entries longer.
    ///
    /// See `docs/audit-log-retention.md` for storage cost estimates.
    pub async fn prune_expired(&self) -> Result<u64> {
        let result = sqlx::query(r#"DELETE FROM route_audit_log WHERE retained_until <= NOW()"#)
            .execute(&self.db)
            .await
            .map_err(|e| {
                ApiError::Internal(Arc::new(anyhow::anyhow!(
                    "Failed to prune audit log: {}",
                    e
                )))
            })?;

        Ok(result.rows_affected())
    }

    /// Delete entries older than `retention` duration (for testing / custom windows).
    pub async fn prune_older_than(&self, retention: Duration) -> Result<u64> {
        let cutoff = chrono::Utc::now() - retention;
        let result = sqlx::query(r#"DELETE FROM route_audit_log WHERE logged_at < $1"#)
            .bind(cutoff)
            .execute(&self.db)
            .await
            .map_err(|e| {
                ApiError::Internal(Arc::new(anyhow::anyhow!(
                    "Failed to prune audit log: {}",
                    e
                )))
            })?;

        Ok(result.rows_affected())
    }

    /// Return the total number of entries in the audit log.
    pub async fn count(&self) -> Result<i64> {
        let row = sqlx::query(r#"SELECT COUNT(*) AS n FROM route_audit_log"#)
            .fetch_one(&self.db)
            .await
            .map_err(|e| {
                ApiError::Internal(Arc::new(anyhow::anyhow!(
                    "Failed to count audit entries: {}",
                    e
                )))
            })?;
        Ok(row.get("n"))
    }
}

/// Lightweight summary returned by list queries.
#[derive(Debug, Clone)]
pub struct AuditEntrySummary {
    pub id: i64,
    pub request_id: String,
    pub trace_id: String,
    pub logged_at: chrono::DateTime<chrono::Utc>,
    pub latency_ms: u64,
    pub outcome: String,
    pub cache_hit: bool,
}
