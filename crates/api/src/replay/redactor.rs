//! Sensitive field redaction for replay artifacts.
//!
//! Replaces all `asset_issuer` string values with `"[REDACTED]"` before
//! any artifact is written to storage. Native assets (null or absent
//! `asset_issuer`) are left unchanged.

use super::artifact::ReplayArtifact;

/// Placeholder string used to replace sensitive field values.
pub const REDACTED: &str = "[REDACTED]";

/// Redacts sensitive fields in replay artifacts before storage.
pub struct Redactor;

impl Redactor {
    /// Redact all `asset_issuer` fields in a `ReplayArtifact` in-place.
    ///
    /// Applies redaction to:
    /// - `artifact.base` and `artifact.quote` canonical strings (strips issuer suffix)
    /// - `artifact.original_output` JSON tree (all nested `asset_issuer` keys)
    pub fn redact(artifact: &mut ReplayArtifact) {
        // Redact canonical asset strings: "CODE:ISSUER" → "CODE:[REDACTED]"
        artifact.base = redact_canonical_asset(&artifact.base);
        artifact.quote = redact_canonical_asset(&artifact.quote);

        Self::redact_value(&mut artifact.original_output);
    }

    /// Recursively replace all `"asset_issuer"` string values in a JSON tree
    /// with `"[REDACTED]"`. Null values and absent keys are left unchanged.
    pub fn redact_value(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                for (key, val) in map.iter_mut() {
                    if key == "asset_issuer" {
                        if val.is_string() {
                            *val = serde_json::Value::String(REDACTED.to_string());
                        }
                        // null / absent → leave unchanged
                    } else {
                        Self::redact_value(val);
                    }
                }
            }
            serde_json::Value::Array(arr) => {
                for item in arr.iter_mut() {
                    Self::redact_value(item);
                }
            }
            _ => {}
        }
    }
}

/// Redact the issuer portion of a canonical asset string.
/// "native" → "native"
/// "USDC" → "USDC"
/// "USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5" → "USDC:[REDACTED]"
fn redact_canonical_asset(s: &str) -> String {
    if s == "native" {
        return s.to_string();
    }
    let parts: Vec<&str> = s.splitn(2, ':').collect();
    match parts.as_slice() {
        [code, _issuer] => format!("{}:{}", code, REDACTED),
        _ => s.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::artifact::{
        HealthConfigSnapshot, LiquidityCandidate, ReplayArtifact, CURRENT_SCHEMA_VERSION,
    };
    use chrono::Utc;
    use proptest::prelude::*;
    use uuid::Uuid;

    fn make_artifact_with_issuer(issuer: &str) -> ReplayArtifact {
        ReplayArtifact {
            id: Uuid::new_v4(),
            schema_version: CURRENT_SCHEMA_VERSION,
            incident_id: None,
            captured_at: Utc::now(),
            base: format!("USDC:{}", issuer),
            quote: "native".to_string(),
            amount: "100.0000000".to_string(),
            slippage_bps: 50,
            quote_type: "sell".to_string(),
            liquidity_snapshot: vec![LiquidityCandidate {
                venue_type: "sdex".to_string(),
                venue_ref: "offer1".to_string(),
                price: "1.0000000".to_string(),
                available_amount: "100.0000000".to_string(),
            }],
            health_config_snapshot: HealthConfigSnapshot {
                freshness_threshold_secs_sdex: 30,
                freshness_threshold_secs_amm: 60,
                staleness_threshold_secs: 30,
                min_tvl_threshold_e7: 1_000_000_000,
            },
            original_output: serde_json::json!({
                "base_asset": {
                    "asset_type": "credit_alphanum4",
                    "asset_code": "USDC",
                    "asset_issuer": issuer
                },
                "quote_asset": {
                    "asset_type": "native"
                },
                "price": "1.0000000",
                "path": [
                    {
                        "from_asset": {
                            "asset_type": "credit_alphanum4",
                            "asset_code": "USDC",
                            "asset_issuer": issuer
                        },
                        "to_asset": { "asset_type": "native" }
                    }
                ]
            }),
        }
    }

    fn make_native_artifact() -> ReplayArtifact {
        ReplayArtifact {
            id: Uuid::new_v4(),
            schema_version: CURRENT_SCHEMA_VERSION,
            incident_id: None,
            captured_at: Utc::now(),
            base: "native".to_string(),
            quote: "native".to_string(),
            amount: "1.0000000".to_string(),
            slippage_bps: 50,
            quote_type: "sell".to_string(),
            liquidity_snapshot: vec![],
            health_config_snapshot: HealthConfigSnapshot {
                freshness_threshold_secs_sdex: 30,
                freshness_threshold_secs_amm: 60,
                staleness_threshold_secs: 30,
                min_tvl_threshold_e7: 1_000_000_000,
            },
            original_output: serde_json::json!({
                "base_asset": { "asset_type": "native", "asset_issuer": null },
                "quote_asset": { "asset_type": "native" },
                "price": "1.0000000"
            }),
        }
    }

    // ── Unit tests ──────────────────────────────────────────────────────────

    #[test]
    fn native_only_artifact_is_unchanged() {
        let mut artifact = make_native_artifact();
        Redactor::redact(&mut artifact);
        assert_eq!(artifact.base, "native");
        assert_eq!(artifact.quote, "native");
        // null asset_issuer must remain null
        assert!(artifact.original_output["base_asset"]["asset_issuer"].is_null());
    }

    #[test]
    fn issued_asset_issuer_is_redacted_in_all_locations() {
        let issuer = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";
        let mut artifact = make_artifact_with_issuer(issuer);
        Redactor::redact(&mut artifact);

        // Canonical base string
        assert_eq!(artifact.base, format!("USDC:{}", REDACTED));

        // original_output top-level base_asset
        assert_eq!(
            artifact.original_output["base_asset"]["asset_issuer"],
            serde_json::Value::String(REDACTED.to_string())
        );

        // original_output nested in path
        assert_eq!(
            artifact.original_output["path"][0]["from_asset"]["asset_issuer"],
            serde_json::Value::String(REDACTED.to_string())
        );

        // Original issuer string must not appear anywhere
        let json = serde_json::to_string(&artifact).unwrap();
        assert!(!json.contains(issuer));
    }

    #[test]
    fn redact_value_leaves_non_issuer_fields_unchanged() {
        let mut val = serde_json::json!({
            "price": "1.0000000",
            "venue_ref": "offer1",
            "asset_code": "USDC"
        });
        Redactor::redact_value(&mut val);
        assert_eq!(val["price"], "1.0000000");
        assert_eq!(val["venue_ref"], "offer1");
        assert_eq!(val["asset_code"], "USDC");
    }

    // ── Property-based tests ────────────────────────────────────────────────

    prop_compose! {
        /// Arbitrary Stellar-like issuer address (56 chars, starts with G).
        fn arb_issuer()(
            suffix in "[A-Z2-7]{55}"
        ) -> String {
            format!("G{}", suffix)
        }
    }

    proptest! {
        /// Property 3: Redactor eliminates all issuer values.
        ///
        /// Feature: quote-replay-system, Property 3: redactor eliminates all issuer values
        #[test]
        fn prop_redactor_removes_all_issuers(issuer in arb_issuer()) {
            let mut artifact = make_artifact_with_issuer(&issuer);
            Redactor::redact(&mut artifact);
            let json = serde_json::to_string(&artifact).expect("serialize");
            prop_assert!(!json.contains(issuer.as_str()),
                "issuer '{}' still present after redaction", issuer);
        }

        /// Native assets are never modified by the redactor.
        #[test]
        fn prop_native_assets_unchanged(_seed in 0u32..1000u32) {
            let mut artifact = make_native_artifact();
            Redactor::redact(&mut artifact);
            prop_assert_eq!(&artifact.base, "native");
            prop_assert_eq!(&artifact.quote, "native");
        }
    }
}
