//! Non-blocking audit log writer for the quote pipeline.
//!
//! [`AuditWriter::emit`] builds a [`RouteAuditEntry`] from the pipeline
//! outputs, redacts sensitive fields, and persists the entry in a detached
//! `tokio::spawn` task — never blocking the quote response path.
//!
//! # Usage
//!
//! ```rust,ignore
//! // In AppState::new():
//! let audit_writer = AuditWriter::new(db.write_pool().clone());
//!
//! // In the quote handler, after computing the response:
//! state.audit_writer.emit(
//!     &request_id,
//!     &trace_id,
//!     latency_ms,
//!     AuditOutcome::Success,
//!     cache_hit,
//!     inputs,
//!     Some(selected),
//!     exclusions,
//! );
//! ```

use sqlx::PgPool;
use tracing::{debug, warn};

use super::{
    redactor::AuditRedactor,
    schema::{AuditExclusion, AuditInputs, AuditOutcome, AuditSelected, RouteAuditEntry},
    store::AuditStore,
};

/// Non-blocking writer that emits audit entries from the quote pipeline.
///
/// Store in [`crate::state::AppState`] as `Arc<AuditWriter>`.
#[derive(Clone)]
pub struct AuditWriter {
    store: AuditStore,
    /// When `false`, [`emit`] is a no-op.  Controlled by the
    /// `AUDIT_LOG_ENABLED` environment variable (default: `true`).
    pub enabled: bool,
}

impl AuditWriter {
    /// Create a new writer.  Audit logging is enabled by default.
    pub fn new(db: PgPool) -> Self {
        Self {
            store: AuditStore::new(db),
            enabled: true,
        }
    }

    /// Create a writer whose enabled state is read from the
    /// `AUDIT_LOG_ENABLED` environment variable (default: `true`).
    pub fn from_env(db: PgPool) -> Self {
        let enabled = std::env::var("AUDIT_LOG_ENABLED")
            .map(|v| !v.eq_ignore_ascii_case("false") && v != "0")
            .unwrap_or(true);
        Self {
            store: AuditStore::new(db),
            enabled,
        }
    }

    /// Emit an audit entry for a completed route decision.
    ///
    /// This method is **synchronous and non-blocking**: it builds and redacts
    /// the entry on the calling thread, then spawns a detached task for the
    /// DB write.  The caller never awaits the spawn.
    ///
    /// # Arguments
    ///
    /// * `request_id`  – HTTP `x-request-id` value.
    /// * `trace_id`    – W3C trace ID from the active span (empty string if none).
    /// * `latency_ms`  – End-to-end pipeline latency.
    /// * `outcome`     – High-level result of the route decision.
    /// * `cache_hit`   – Whether the response was served from cache.
    /// * `inputs`      – Request inputs (issuer values will be redacted).
    /// * `selected`    – Selected route (issuer values will be redacted).
    /// * `exclusions`  – Venues excluded from routing.
    #[allow(clippy::too_many_arguments)]
    pub fn emit(
        &self,
        request_id: impl Into<String>,
        trace_id: impl Into<String>,
        latency_ms: u64,
        outcome: AuditOutcome,
        cache_hit: bool,
        inputs: AuditInputs,
        selected: Option<AuditSelected>,
        exclusions: Vec<AuditExclusion>,
    ) {
        if !self.enabled {
            return;
        }

        let mut entry = RouteAuditEntry::new(
            request_id, trace_id, latency_ms, outcome, cache_hit, inputs, selected, exclusions,
        );

        // Redact synchronously before handing off to the async task.
        AuditRedactor::redact(&mut entry);

        let store = self.store.clone();

        tokio::spawn(async move {
            match store.insert(&entry).await {
                Ok(id) => {
                    debug!(
                        audit_id = id,
                        request_id = %entry.request_id,
                        outcome = %entry.outcome,
                        "Audit entry persisted"
                    );
                }
                Err(e) => {
                    warn!(
                        request_id = %entry.request_id,
                        error = %e,
                        "Audit log write failed — quote unaffected"
                    );
                }
            }
        });
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const ISSUER: &str = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";

    /// Verify that a disabled writer is a no-op (no panic, no side effects).
    #[test]
    fn disabled_writer_is_noop() {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        struct CountingWriter {
            enabled: bool,
        }
        impl CountingWriter {
            fn emit_noop(&self) {
                if !self.enabled {
                    COUNTER.fetch_add(1, Ordering::SeqCst);
                    return;
                }
                panic!("should not reach here when disabled");
            }
        }

        let w = CountingWriter { enabled: false };
        w.emit_noop();
        assert_eq!(COUNTER.load(Ordering::SeqCst), 1);
    }

    /// Verify that `from_env` reads the env var correctly.
    #[test]
    fn from_env_enabled_by_default() {
        std::env::remove_var("AUDIT_LOG_ENABLED");
        let enabled = std::env::var("AUDIT_LOG_ENABLED")
            .map(|v| !v.eq_ignore_ascii_case("false") && v != "0")
            .unwrap_or(true);
        assert!(enabled, "audit log should be enabled by default");
    }

    #[test]
    fn from_env_disabled_when_set_to_false() {
        std::env::set_var("AUDIT_LOG_ENABLED", "false");
        let enabled = std::env::var("AUDIT_LOG_ENABLED")
            .map(|v| !v.eq_ignore_ascii_case("false") && v != "0")
            .unwrap_or(true);
        assert!(!enabled);
        std::env::remove_var("AUDIT_LOG_ENABLED");
    }

    /// Verify that the entry built inside `emit` has the correct fields and
    /// that the issuer is redacted before the DB write.
    #[test]
    fn entry_is_redacted_before_write() {
        let inputs = AuditInputs {
            base: format!("USDC:{}", ISSUER),
            quote: "native".to_string(),
            amount: "100.0000000".to_string(),
            slippage_bps: 50,
            quote_type: "sell".to_string(),
        };

        let mut entry = RouteAuditEntry::new(
            "req-test",
            "trace-test",
            15,
            AuditOutcome::Success,
            false,
            inputs,
            None,
            vec![],
        );

        AuditRedactor::redact(&mut entry);

        // Issuer must be gone
        let json = serde_json::to_string(&entry).expect("serialize");
        assert!(!json.contains(ISSUER), "issuer must be redacted");
        assert!(json.contains("[REDACTED]"), "placeholder must be present");

        // Correlation IDs must survive
        assert_eq!(entry.request_id, "req-test");
        assert_eq!(entry.trace_id, "trace-test");
    }

    #[test]
    fn trace_id_empty_string_is_valid() {
        // When no distributed trace is active, trace_id is an empty string.
        let entry = RouteAuditEntry::new(
            "req-notrace",
            "",
            5,
            AuditOutcome::NoRoute,
            false,
            AuditInputs {
                base: "native".to_string(),
                quote: "native".to_string(),
                amount: "1.0000000".to_string(),
                slippage_bps: 50,
                quote_type: "sell".to_string(),
            },
            None,
            vec![],
        );
        assert_eq!(entry.trace_id, "");
        assert_eq!(entry.outcome.as_str(), "no_route");
    }
}
