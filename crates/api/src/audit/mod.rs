//! Route decision audit log with privacy-safe field redaction.
//!
//! # Overview
//!
//! Every quote pipeline execution emits a structured [`RouteAuditEntry`] that
//! captures the full decision context — inputs, selected route, exclusion
//! reasons, latency, and outcome — while stripping all sensitive fields before
//! persistence.
//!
//! # Components
//!
//! - [`schema`]   – [`RouteAuditEntry`] data types and PostgreSQL persistence.
//! - [`redactor`] – Privacy-safe field redaction (extends the replay redactor).
//! - [`writer`]   – Non-blocking fire-and-forget writer for the quote pipeline.
//!
//! # Correlation
//!
//! Each entry carries:
//! - `request_id` — the HTTP `x-request-id` header value (or a generated UUID).
//! - `trace_id`   — the W3C traceparent trace ID extracted from the active span.
//!
//! # Retention
//!
//! Default retention is **30 days**, enforced by the `retained_until` generated
//! column in the `route_audit_log` table.  See
//! [`store::AuditStore::prune_older_than`] and `docs/audit-log-retention.md`
//! for tuning guidance.

pub mod redactor;
pub mod schema;
pub mod store;
pub mod writer;

pub use redactor::AuditRedactor;
pub use schema::{
    AuditExclusion, AuditInputs, AuditOutcome, AuditPathStep, AuditSelected, RouteAuditEntry,
};
pub use store::AuditStore;
pub use writer::AuditWriter;
