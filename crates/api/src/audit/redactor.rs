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

    /// Redact a Stellar account public key to a non-reversible fingerprint.
    ///
    /// The raw key is never stored.  The result keeps the first 4 and last 4
    /// characters for operator correlation, separated by `...`, and appends the
    /// first 8 hex characters of a SHA-256 hash so that identical accounts still
    /// group together in queries while remaining computationally infeasible to
    /// reverse.
    ///
    /// Examples:
    /// - `"GABC...LA5#deadbeef"` (typical Stellar address)
    /// - `"native"` is returned unchanged (used for issuing account sentinel).
    /// - Short / malformed inputs are fully replaced with `[REDACTED]`.
    pub fn redact_account(account: &str) -> String {
        if account == "native" {
            return account.to_string();
        }

        // Stellar public keys are 56 characters.  Be defensive with malformed input.
        if account.len() < 12 {
            return REDACTED.to_string();
        }

        let prefix = &account[..4];
        let suffix = &account[account.len() - 4..];
        let hash = sha256_hex_prefix(account, 8);

        format!("{}...{}#{}", prefix, suffix, hash)
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

/// Redact opaque CCTP attestation or message bytes for structured logs.
pub fn redact_cctp_secret_bytes(label: &str, bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return format!("{}:empty", label);
    }
    let prefix = sha256_hex_prefix(&hex::encode(bytes), 8);
    format!("{}:sha256#{}", label, prefix)
}

fn sha256_hex_prefix(input: &str, len: usize) -> String {
    use sha2::{Digest, Sha256};

    let digest = Sha256::digest(input.as_bytes());
    let mut hex = hex::encode(digest);
    hex.truncate(len);
    hex
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

    #[test]
    fn redact_cctp_bytes_never_includes_raw_payload() {
        let redacted = redact_cctp_secret_bytes("attestation", &[0xde, 0xad, 0xbe, 0xef]);
        assert!(!redacted.contains("dead"));
        assert!(redacted.starts_with("attestation:sha256#"));
    }

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

    // ── Account redaction tests ───────────────────────────────────────────────

    const ACCOUNT: &str = "GABCD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";

    #[test]
    fn account_native_is_unchanged() {
        assert_eq!(AuditRedactor::redact_account("native"), "native");
    }

    #[test]
    fn short_account_is_fully_redacted() {
        assert_eq!(AuditRedactor::redact_account("short"), REDACTED);
    }

    #[test]
    fn account_is_redacted_to_fingerprint() {
        let redacted = AuditRedactor::redact_account(ACCOUNT);
        assert!(
            !redacted.contains(ACCOUNT),
            "raw account must not appear in redacted form"
        );
        assert!(redacted.starts_with("GABC"), "prefix preserved");
        // Implementation keeps the last 4 chars before the `#hash` suffix.
        let fingerprint = redacted.split('#').next().expect("hash separator");
        assert!(
            fingerprint.ends_with("FLA5"),
            "suffix preserved: {redacted}"
        );
        assert!(redacted.contains("..."), "middle replaced");
        assert!(redacted.contains('#'), "hash separator present");
    }

    #[test]
    fn account_redaction_is_idempotent() {
        let first = AuditRedactor::redact_account(ACCOUNT);
        let second = AuditRedactor::redact_account(ACCOUNT);
        assert_eq!(first, second);
    }

    // ── Extra issuer / secret fixtures (issue #1305) ─────────────────────────

    /// A second, unrelated issuer — proves redaction is not keyed to one fixture.
    const OTHER_ISSUER: &str = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";

    #[test]
    fn alphanum12_asset_code_issuer_is_redacted() {
        let asset = format!("LONGASSET123:{}", ISSUER);
        let result = redact_canonical_asset(&asset);
        assert_eq!(result, format!("LONGASSET123:{}", REDACTED));
        assert!(!result.contains(ISSUER));
    }

    #[test]
    fn issuer_with_extra_colons_is_fully_redacted() {
        // Only the code survives: everything after the first `:` is the issuer.
        let asset = format!("USDC:{}:memo", ISSUER);
        let result = redact_canonical_asset(&asset);
        assert_eq!(result, format!("USDC:{}", REDACTED));
        assert!(!result.contains(ISSUER));
    }

    #[test]
    fn asset_with_missing_code_still_redacts_issuer() {
        let asset = format!(":{}", ISSUER);
        let result = redact_canonical_asset(&asset);
        assert!(!result.contains(ISSUER), "issuer leaked: {result}");
    }

    #[test]
    fn empty_asset_string_is_unchanged() {
        assert_eq!(redact_canonical_asset(""), "");
    }

    #[test]
    fn distinct_issuers_on_base_and_quote_are_both_redacted() {
        let mut entry = make_entry_with_issuer(ISSUER);
        entry.inputs.quote = format!("BTC:{}", OTHER_ISSUER);
        if let Some(selected) = entry.selected.as_mut() {
            selected.path[0].to = format!("BTC:{}", OTHER_ISSUER);
        }

        AuditRedactor::redact(&mut entry);

        let json = serde_json::to_string(&entry).expect("serialize");
        assert!(!json.contains(ISSUER), "base issuer leaked");
        assert!(!json.contains(OTHER_ISSUER), "quote issuer leaked");
    }

    #[test]
    fn native_quote_survives_alongside_a_redacted_base() {
        let mut entry = make_entry_with_issuer(ISSUER);
        entry.inputs.quote = "native".to_string();
        AuditRedactor::redact(&mut entry);
        assert_eq!(entry.inputs.quote, "native");
        assert_eq!(entry.inputs.base, format!("USDC:{}", REDACTED));
    }

    #[test]
    fn account_boundary_lengths_are_handled() {
        // 11 chars — below the 12-char guard, so fully replaced.
        assert_eq!(AuditRedactor::redact_account("GABCDEFGHIJ"), REDACTED);
        // 12 chars — the shortest input that keeps a prefix/suffix fingerprint.
        let redacted = AuditRedactor::redact_account("GABCDEFGHIJK");
        assert!(redacted.starts_with("GABC"));
        assert!(redacted.contains("..."));
        assert!(!redacted.contains("GABCDEFGHIJK"));
    }

    #[test]
    fn empty_account_is_fully_redacted() {
        assert_eq!(AuditRedactor::redact_account(""), REDACTED);
    }

    #[test]
    fn distinct_accounts_get_distinct_fingerprints() {
        let a = AuditRedactor::redact_account(ACCOUNT);
        let b = AuditRedactor::redact_account(OTHER_ISSUER);
        assert_ne!(a, b, "different accounts must not collide");
    }

    #[test]
    fn issuer_is_redacted_when_used_as_an_account() {
        let redacted = AuditRedactor::redact_account(ISSUER);
        assert!(!redacted.contains(ISSUER));
        assert!(redacted.len() < ISSUER.len(), "fingerprint is shorter");
    }

    #[test]
    fn cctp_secret_bytes_are_stable_and_distinct() {
        let a = redact_cctp_secret_bytes("attestation", &[0x01, 0x02, 0x03]);
        let again = redact_cctp_secret_bytes("attestation", &[0x01, 0x02, 0x03]);
        let b = redact_cctp_secret_bytes("attestation", &[0x01, 0x02, 0x04]);
        assert_eq!(a, again, "same bytes must fingerprint identically");
        assert_ne!(a, b, "different bytes must fingerprint differently");
    }

    #[test]
    fn empty_cctp_secret_bytes_are_labelled_not_hashed() {
        assert_eq!(
            redact_cctp_secret_bytes("message", &[]),
            "message:empty".to_string()
        );
    }

    #[test]
    fn cctp_secret_bytes_never_leak_the_hex_payload() {
        let secret = [0xca, 0xfe, 0xba, 0xbe, 0xde, 0xad, 0xbe, 0xef];
        let redacted = redact_cctp_secret_bytes("attestation", &secret);
        assert!(!redacted.contains(&hex::encode(secret)));
        assert!(!redacted.contains("cafe"));
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
