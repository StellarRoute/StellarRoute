//! Route decision and swap submit audit logs with privacy-safe field redaction.
//!
//! # Overview
//!
//! Every quote pipeline execution emits a structured [`RouteAuditEntry`] that
//! captures the full decision context — inputs, selected route, exclusion
//! reasons, latency, and outcome — while stripping all sensitive fields before
//! persistence.
//!
//! Swap prepare/submit attempts are logged separately via
//! [`SwapSubmitAuditEntry`]; the account field is redacted to a hash-prefix
//! fingerprint and the raw public key is never stored.
//!
//! # Components
//!
//! - [`schema`]   – [`RouteAuditEntry`] and [`SwapSubmitAuditEntry`] data types.
//! - [`redactor`] – Privacy-safe field redaction (extends the replay redactor).
//! - [`store`]    – PostgreSQL persistence for both audit log tables.
//! - [`writer`]   – Non-blocking fire-and-forget writers.
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
//! column in the `route_audit_log` and `swap_submit_audit_log` tables.  See
//! [`store::AuditStore::prune_older_than`] and `docs/audit-log-retention.md`
//! for tuning guidance.

pub mod redactor;
pub mod schema;
pub mod store;
pub mod writer;

pub use redactor::AuditRedactor;
pub use schema::{
    AuditExclusion, AuditInputs, AuditOutcome, AuditPathStep, AuditSelected, RouteAuditEntry,
    SwapSubmitAuditEntry, SwapSubmitOutcome,
};
pub use store::AuditStore;
pub use writer::AuditWriter;
