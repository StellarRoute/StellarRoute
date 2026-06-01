//! Audit log schema — data types for route decision audit entries.
//!
//! # Privacy guarantees
//!
//! All fields that could identify a counterparty are redacted before an entry
//! is written to storage:
//!
//! - `asset_issuer` values in `inputs.base` / `inputs.quote` are replaced with
//!   `"[REDACTED]"` (e.g. `"USDC:GBBD47…"` → `"USDC:[REDACTED]"`).
//! - `venue_ref` values in `selected` and `exclusions` are **not** redacted
//!   because they are pool addresses / offer IDs that are already public on the
//!   Stellar network.
//!
//! # Schema version
//!
//! Bump [`AUDIT_SCHEMA_VERSION`] whenever the shape of `inputs`, `selected`, or
//! `exclusions` changes in a breaking way.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Current schema version.  Stored in every entry for forward-compatibility.
pub const AUDIT_SCHEMA_VERSION: u32 = 1;

// ─── Outcome ─────────────────────────────────────────────────────────────────

/// High-level outcome of a route decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    /// A valid route was found and a price was returned.
    Success,
    /// No executable route exists for this pair/amount.
    NoRoute,
    /// All market data inputs were too stale to compute a quote.
    StaleData,
    /// An unexpected internal error occurred.
    Error,
}

impl AuditOutcome {
    /// Database-safe string representation (matches the CHECK constraint).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::NoRoute => "no_route",
            Self::StaleData => "stale_data",
            Self::Error => "error",
        }
    }
}

impl std::fmt::Display for AuditOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ─── Sub-structures ───────────────────────────────────────────────────────────

/// Redacted request inputs captured at the start of the pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditInputs {
    /// Canonical base asset string with issuer redacted, e.g. `"USDC:[REDACTED]"`.
    pub base: String,
    /// Canonical quote asset string with issuer redacted.
    pub quote: String,
    /// Trade amount as a 7-decimal string.
    pub amount: String,
    /// Slippage tolerance in basis points.
    pub slippage_bps: u32,
    /// `"sell"` or `"buy"`.
    pub quote_type: String,
}

/// Redacted description of the selected route.
///
/// Present only when `outcome == AuditOutcome::Success`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSelected {
    /// Venue type: `"sdex"` or `"amm"`.
    pub venue_type: String,
    /// Venue reference (offer ID or pool address — public on-chain data).
    pub venue_ref: String,
    /// Best price as a 7-decimal string.
    pub price: String,
    /// Execution path: list of `{from, to, price, source}` hops.
    pub path: Vec<AuditPathStep>,
    /// Strategy label, e.g. `"single_hop_direct_venue_comparison"`.
    pub strategy: String,
}

/// A single hop in the execution path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditPathStep {
    /// Canonical from-asset string (issuer redacted).
    pub from: String,
    /// Canonical to-asset string (issuer redacted).
    pub to: String,
    /// Price at this hop as a 7-decimal string.
    pub price: String,
    /// Source label, e.g. `"sdex"` or `"amm:POOL_ADDRESS"`.
    pub source: String,
}

/// A venue that was excluded from routing and the reason why.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditExclusion {
    /// Venue reference (public on-chain data — not redacted).
    pub venue_ref: String,
    /// Human-readable exclusion reason.
    pub reason: String,
}

// ─── Top-level entry ─────────────────────────────────────────────────────────

/// A single structured audit log entry for one route decision.
///
/// Built by [`crate::audit::writer::AuditWriter`] and persisted via
/// [`crate::audit::store::AuditStore`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteAuditEntry {
    /// Schema version for forward-compatibility.
    pub schema_version: u32,

    // ── Correlation ──────────────────────────────────────────────────────────
    /// HTTP `x-request-id` header value (or a generated UUID).
    pub request_id: String,
    /// W3C traceparent trace ID (32-char hex).  Empty string when no trace is
    /// active (e.g. in unit tests or when OTLP is disabled).
    pub trace_id: String,

    // ── Timing ───────────────────────────────────────────────────────────────
    /// Wall-clock time when the entry was created.
    pub logged_at: DateTime<Utc>,
    /// End-to-end quote pipeline latency in milliseconds.
    pub latency_ms: u64,

    // ── Outcome ──────────────────────────────────────────────────────────────
    /// High-level result of the route decision.
    pub outcome: AuditOutcome,
    /// `true` when the response was served from the Redis cache.
    pub cache_hit: bool,

    // ── Decision details ─────────────────────────────────────────────────────
    /// Redacted request inputs.
    pub inputs: AuditInputs,
    /// Redacted selected route.  `None` when `outcome != Success`.
    pub selected: Option<AuditSelected>,
    /// Venues excluded from routing (may be empty).
    pub exclusions: Vec<AuditExclusion>,
}

impl RouteAuditEntry {
    /// Create a new entry with the current timestamp and schema version.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        request_id: impl Into<String>,
        trace_id: impl Into<String>,
        latency_ms: u64,
        outcome: AuditOutcome,
        cache_hit: bool,
        inputs: AuditInputs,
        selected: Option<AuditSelected>,
        exclusions: Vec<AuditExclusion>,
    ) -> Self {
        Self {
            schema_version: AUDIT_SCHEMA_VERSION,
            request_id: request_id.into(),
            trace_id: trace_id.into(),
            logged_at: Utc::now(),
            latency_ms,
            outcome,
            cache_hit,
            inputs,
            selected,
            exclusions,
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(outcome: AuditOutcome) -> RouteAuditEntry {
        RouteAuditEntry::new(
            "req-001",
            "0af7651916cd43dd8448eb211c80319c",
            42,
            outcome,
            false,
            AuditInputs {
                base: "native".to_string(),
                quote: "USDC:[REDACTED]".to_string(),
                amount: "100.0000000".to_string(),
                slippage_bps: 50,
                quote_type: "sell".to_string(),
            },
            if outcome == AuditOutcome::Success {
                Some(AuditSelected {
                    venue_type: "sdex".to_string(),
                    venue_ref: "offer1".to_string(),
                    price: "1.0000000".to_string(),
                    path: vec![AuditPathStep {
                        from: "native".to_string(),
                        to: "USDC:[REDACTED]".to_string(),
                        price: "1.0000000".to_string(),
                        source: "sdex".to_string(),
                    }],
                    strategy: "single_hop_direct_venue_comparison".to_string(),
                })
            } else {
                None
            },
            vec![],
        )
    }

    #[test]
    fn schema_version_is_current() {
        let entry = make_entry(AuditOutcome::Success);
        assert_eq!(entry.schema_version, AUDIT_SCHEMA_VERSION);
    }

    #[test]
    fn outcome_as_str_matches_db_constraint() {
        assert_eq!(AuditOutcome::Success.as_str(), "success");
        assert_eq!(AuditOutcome::NoRoute.as_str(), "no_route");
        assert_eq!(AuditOutcome::StaleData.as_str(), "stale_data");
        assert_eq!(AuditOutcome::Error.as_str(), "error");
    }

    #[test]
    fn success_entry_has_selected() {
        let entry = make_entry(AuditOutcome::Success);
        assert!(entry.selected.is_some());
    }

    #[test]
    fn non_success_entry_has_no_selected() {
        let entry = make_entry(AuditOutcome::NoRoute);
        assert!(entry.selected.is_none());
    }

    #[test]
    fn serde_round_trip() {
        let entry = make_entry(AuditOutcome::Success);
        let json = serde_json::to_string(&entry).expect("serialize");
        let back: RouteAuditEntry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(entry.request_id, back.request_id);
        assert_eq!(entry.trace_id, back.trace_id);
        assert_eq!(entry.outcome.as_str(), back.outcome.as_str());
        assert_eq!(entry.latency_ms, back.latency_ms);
        assert_eq!(entry.inputs.base, back.inputs.base);
        assert_eq!(entry.inputs.quote, back.inputs.quote);
    }

    #[test]
    fn serde_round_trip_no_route() {
        let entry = make_entry(AuditOutcome::NoRoute);
        let json = serde_json::to_string(&entry).expect("serialize");
        let back: RouteAuditEntry = serde_json::from_str(&json).expect("deserialize");
        assert!(back.selected.is_none());
        assert_eq!(back.outcome.as_str(), "no_route");
    }

    #[test]
    fn exclusions_are_serialized() {
        let mut entry = make_entry(AuditOutcome::Success);
        entry.exclusions = vec![
            AuditExclusion {
                venue_ref: "pool1".to_string(),
                reason: "stale_data".to_string(),
            },
            AuditExclusion {
                venue_ref: "offer2".to_string(),
                reason: "circuit_breaker_open".to_string(),
            },
        ];
        let json = serde_json::to_value(&entry).expect("serialize");
        let excl = json["exclusions"].as_array().expect("array");
        assert_eq!(excl.len(), 2);
        assert_eq!(excl[0]["venue_ref"], "pool1");
        assert_eq!(excl[1]["reason"], "circuit_breaker_open");
    }

    #[test]
    fn correlation_ids_are_preserved() {
        let entry = make_entry(AuditOutcome::Success);
        assert_eq!(entry.request_id, "req-001");
        assert_eq!(entry.trace_id, "0af7651916cd43dd8448eb211c80319c");
    }
}
