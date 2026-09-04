//! Property tests verifying `docs/runbooks/quote-inspector.md` accuracy.
//!
//! These tests are deterministic set-membership checks: they assert that
//! every field name, error code, and enum variant documented in the runbook
//! actually exists in the live schema files. They run as part of
//! `cargo test -p stellarroute-api --lib`.

#[cfg(test)]
mod tests {
    fn read_file(relative_path: &str) -> String {
        if let Ok(content) = std::fs::read_to_string(relative_path) {
            return content;
        }
        let manifest_relative = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative_path);
        std::fs::read_to_string(&manifest_relative).unwrap_or_else(|_| {
            panic!(
                "could not read {relative_path} directly or at {:?}",
                manifest_relative
            )
        })
    }

    fn runbook() -> String {
        read_file("docs/runbooks/quote-inspector.md")
    }

    fn openapi_yaml() -> String {
        read_file("docs/api/openapi.yaml")
    }

    // Feature: quote-inspector-operator-guide, Property 1: all documented QuoteResponse field names exist in the schema
    #[test]
    fn prop1_documented_quote_response_fields_exist_in_schema() {
        let schema = openapi_yaml();
        let documented_fields = [
            "amount",
            "total",
            "price",
            "quote_type",
            "base_asset",
            "quote_asset",
            "path",
            "expires_at",
            "data_freshness",
            "degraded",
            "price_impact",
            "midpoint",
            "spread_bps",
        ];
        for field in &documented_fields {
            assert!(
                schema.contains(field),
                "Field `{field}` is documented in the quote-inspector runbook \
                 but not found in docs/api/openapi.yaml — runbook may be out of date",
            );
        }
    }

    // Feature: quote-inspector-operator-guide, Property 2: all ExclusionReason variants are documented
    #[test]
    fn prop2_all_exclusion_reason_variants_are_documented() {
        let doc = runbook();
        let variants = [
            "policy_threshold",
            "override",
            "stale_data",
            "circuit_breaker_open",
            "liquidity_anomaly",
        ];
        for variant in &variants {
            assert!(
                doc.contains(variant),
                "ExclusionReason variant `{variant}` is defined in response.rs \
                 but not found in docs/runbooks/quote-inspector.md",
            );
        }
    }

    // Feature: quote-inspector-operator-guide, Property 3: all ApiResponse envelope fields are mentioned
    #[test]
    fn prop3_api_response_envelope_fields_are_mentioned() {
        let doc = runbook();
        // These are the top-level fields of ApiResponse<T>
        let envelope_fields = ["\"v\"", "timestamp", "request_id", "\"data\""];
        for field in &envelope_fields {
            assert!(
                doc.contains(field),
                "ApiResponse envelope field `{field}` is not mentioned \
                 in docs/runbooks/quote-inspector.md",
            );
        }
    }

    // Feature: quote-inspector-operator-guide, Property 4: all key flow constraint identifiers are present
    #[test]
    fn prop4_key_flow_constraint_identifiers_are_present() {
        let doc = runbook();
        let identifiers = [
            "quote_expired",
            "already_submitted",
            "unsupported_execution_mode",
            "network_passphrase",
            "signed_xdr",
            "classic_path_payment",
        ];
        for id in &identifiers {
            assert!(
                doc.contains(id),
                "Flow constraint identifier `{id}` is not found \
                 in docs/runbooks/quote-inspector.md — runbook may be incomplete",
            );
        }
    }
}
