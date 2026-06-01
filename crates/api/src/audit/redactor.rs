//! Privacy-safe field redaction for route audit log entries.
//!
//! # What is redacted
//!
//! | Field                          | Before                                    | After                        |
//! |--------------------------------|-------------------------------------------|------------------------------|
//! | `inputs.base` / `inputs.quote` | `"USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5"` | `"USDC:[REDACTED]"` |
//! | `selected.path[*].from`        | `"USDC:GBBD47…"`                          | `"USDC:[REDACTED]"`          |
//! | `selected.path[*].to`          | `"USDC:GBBD47…"`                          | `"USDC:[REDACTED]"`          |
//!
//! # What is NOT redacted
//!
//! - `venue_ref` — offer IDs and pool addresses are public on-chain data.
//! - `price`, `amount`, `slippage_bps` — non-identifying numeric values.
//! - `request_id`, `trace_id` — correlation IDs that must remain intact.
//! - `strategy`, `source` — internal labels with no PII.
//!
//! # Relationship to `replay::Redactor`
//!
//! [`crate::replay::redactor::Redactor`] operates on `ReplayArtifact` JSON
//! blobs.  This module operates on the typed [`RouteAuditEntry`] struct,
//! which is more efficient and avoids the need for recursive JSON traversal.

use super::schema::{AuditInputs, AuditPathStep, AuditSelected, RouteAuditEntry};

/// Placeholder used to replace sensitive field values.
pub const REDACTED: &str = "[REDACTED]";

/// Redacts sensitive fields in a [`RouteAuditEntry`] in-place.
pub struct AuditRedactor;

impl AuditRedactor {
    /// Redact all sensitive fields in `entry`.
    ///
    /// This method is idempotent: calling it twice produces the same result.
    pub fn redact(entry: &mut RouteAuditEntry) {
        entry.inputs = redact_inputs(&entry.inputs);

        if let Some(ref mut selected) = entry.selected {
            redact_selected(selected);
        }
    }
}

/// Redact the issuer portion of a canonical asset string.
///
/// - `"native"` → `"native"` (unchanged)
/// - `"USDC"` → `"USDC"` (no issuer — unchanged)
/// - `"USDC:GBBD47…"` → `"USDC:[REDACTED]"`
pub fn redact_canonical_asset(s: &str) -> String {
    if s == "native" {
        return s.to_string();
    }
    match s.splitn(2, ':').collect::<Vec<_>>().as_slice() {
        [code, _issuer] => format!("{}:{}", code, REDACTED),
        _ => s.to_string(),
    }
}

fn redact_inputs(inputs: &AuditInputs) -> AuditInputs {
    AuditInputs {
        base: redact_canonical_asset(&inputs.base),
        quote: redact_canonical_asset(&inputs.quote),
        amount: inputs.amount.clone(),
        slippage_bps: inputs.slippage_bps,
        quote_type: inputs.quote_type.clone(),
    }
}

fn redact_selected(selected: &mut AuditSelected) {
    for step in &mut selected.path {
        redact_path_step(step);
    }
}

fn redact_path_step(step: &mut AuditPathStep) {
    step.from = redact_canonical_asset(&step.from);
    step.to = redact_canonical_asset(&step.to);
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::schema::{
        AuditExclusion, AuditInputs, AuditOutcome, AuditPathStep, AuditSelected, RouteAuditEntry,
    };
    use proptest::prelude::*;

    const ISSUER: &str = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";

    fn make_entry_with_issuer(issuer: &str) -> RouteAuditEntry {
        RouteAuditEntry::new(
            "req-001",
            "trace-001",
            10,
            AuditOutcome::Success,
            false,
            AuditInputs {
                base: format!("USDC:{}", issuer),
                quote: format!("BTC:{}", issuer),
                amount: "100.0000000".to_string(),
                slippage_bps: 50,
                quote_type: "sell".to_string(),
            },
            Some(AuditSelected {
                venue_type: "sdex".to_string(),
                venue_ref: "offer1".to_string(),
                price: "1.0000000".to_string(),
                path: vec![AuditPathStep {
                    from: format!("USDC:{}", issuer),
                    to: format!("BTC:{}", issuer),
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

    // ── Unit tests ────────────────────────────────────────────────────────────

    #[test]
    fn native_asset_is_unchanged() {
        assert_eq!(redact_canonical_asset("native"), "native");
    }

    #[test]
    fn asset_without_issuer_is_unchanged() {
        assert_eq!(redact_canonical_asset("USDC"), "USDC");
        assert_eq!(redact_canonical_asset("XLM"), "XLM");
    }

    #[test]
    fn issued_asset_issuer_is_redacted() {
        let result = redact_canonical_asset(&format!("USDC:{}", ISSUER));
        assert_eq!(result, format!("USDC:{}", REDACTED));
        assert!(!result.contains(ISSUER));
    }

    #[test]
    fn inputs_base_and_quote_are_redacted() {
        let mut entry = make_entry_with_issuer(ISSUER);
        AuditRedactor::redact(&mut entry);
        assert_eq!(entry.inputs.base, format!("USDC:{}", REDACTED));
        assert_eq!(entry.inputs.quote, format!("BTC:{}", REDACTED));
    }

    #[test]
    fn path_steps_are_redacted() {
        let mut entry = make_entry_with_issuer(ISSUER);
        AuditRedactor::redact(&mut entry);
        let step = &entry.selected.as_ref().unwrap().path[0];
        assert_eq!(step.from, format!("USDC:{}", REDACTED));
        assert_eq!(step.to, format!("BTC:{}", REDACTED));
    }

    #[test]
    fn venue_ref_is_not_redacted() {
        let mut entry = make_entry_with_issuer(ISSUER);
        AuditRedactor::redact(&mut entry);
        // venue_ref is public on-chain data — must not be redacted
        assert_eq!(entry.selected.as_ref().unwrap().venue_ref, "offer1");
        assert_eq!(entry.exclusions[0].venue_ref, "pool1");
    }

    #[test]
    fn numeric_fields_are_preserved() {
        let mut entry = make_entry_with_issuer(ISSUER);
        AuditRedactor::redact(&mut entry);
        assert_eq!(entry.inputs.amount, "100.0000000");
        assert_eq!(entry.inputs.slippage_bps, 50);
        assert_eq!(entry.selected.as_ref().unwrap().price, "1.0000000");
    }

    #[test]
    fn correlation_ids_are_preserved() {
        let mut entry = make_entry_with_issuer(ISSUER);
        AuditRedactor::redact(&mut entry);
        assert_eq!(entry.request_id, "req-001");
        assert_eq!(entry.trace_id, "trace-001");
    }

    #[test]
    fn redaction_is_idempotent() {
        let mut entry = make_entry_with_issuer(ISSUER);
        AuditRedactor::redact(&mut entry);
        let after_first = serde_json::to_string(&entry).unwrap();
        AuditRedactor::redact(&mut entry);
        let after_second = serde_json::to_string(&entry).unwrap();
        assert_eq!(after_first, after_second, "redaction must be idempotent");
    }

    #[test]
    fn no_route_entry_has_no_selected_to_redact() {
        let mut entry = RouteAuditEntry::new(
            "req-002",
            "",
            5,
            AuditOutcome::NoRoute,
            false,
            AuditInputs {
                base: format!("USDC:{}", ISSUER),
                quote: "native".to_string(),
                amount: "1.0000000".to_string(),
                slippage_bps: 50,
                quote_type: "sell".to_string(),
            },
            None,
            vec![],
        );
        // Must not panic
        AuditRedactor::redact(&mut entry);
        assert_eq!(entry.inputs.base, format!("USDC:{}", REDACTED));
        assert!(entry.selected.is_none());
    }

    #[test]
    fn issuer_does_not_appear_in_serialized_entry() {
        let mut entry = make_entry_with_issuer(ISSUER);
        AuditRedactor::redact(&mut entry);
        let json = serde_json::to_string(&entry).expect("serialize");
        assert!(
            !json.contains(ISSUER),
            "issuer '{}' must not appear in serialized entry",
            ISSUER
        );
    }

    // ── Property-based tests ──────────────────────────────────────────────────

    prop_compose! {
        /// Arbitrary Stellar-like issuer address (56 chars, starts with G).
        fn arb_issuer()(suffix in "[A-Z2-7]{55}") -> String {
            format!("G{}", suffix)
        }
    }

    proptest! {
        /// Any issuer value is eliminated from the serialized entry after redaction.
        #[test]
        fn prop_issuer_eliminated_after_redaction(issuer in arb_issuer()) {
            let mut entry = make_entry_with_issuer(&issuer);
            AuditRedactor::redact(&mut entry);
            let json = serde_json::to_string(&entry).expect("serialize");
            prop_assert!(
                !json.contains(issuer.as_str()),
                "issuer '{}' still present after redaction",
                issuer
            );
        }

        /// Native assets are never modified by the redactor.
        #[test]
        fn prop_native_assets_unchanged(_seed in 0u32..1000u32) {
            let result = redact_canonical_asset("native");
            prop_assert_eq!(result, "native");
        }

        /// Redaction is idempotent for any issuer.
        #[test]
        fn prop_redaction_idempotent(issuer in arb_issuer()) {
            let mut entry = make_entry_with_issuer(&issuer);
            AuditRedactor::redact(&mut entry);
            let after_first = serde_json::to_string(&entry).expect("first");
            AuditRedactor::redact(&mut entry);
            let after_second = serde_json::to_string(&entry).expect("second");
            prop_assert_eq!(after_first, after_second);
        }
    }
}
