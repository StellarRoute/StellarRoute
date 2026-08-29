//! Integration & unit tests for the audit log export CLI (`audit-export`).

use stellarroute_api::audit::{
    AuditExclusion, AuditInputs, AuditOutcome, AuditPathStep, AuditRedactor, AuditSelected,
    RouteAuditEntry, SwapSubmitAuditEntry, SwapSubmitOutcome,
};

const ISSUER: &str = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";
const ACCOUNT: &str = "GABCD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";

#[test]
fn export_redaction_strips_asset_issuers_and_secret_keys() {
    let mut entry = RouteAuditEntry::new(
        "req-export-01",
        "trace-export-01",
        35,
        AuditOutcome::Success,
        false,
        AuditInputs {
            base: format!("USDC:{}", ISSUER),
            quote: format!("BTC:{}", ISSUER),
            amount: "500.0000000".to_string(),
            slippage_bps: 100,
            quote_type: "sell".to_string(),
        },
        Some(AuditSelected {
            venue_type: "sdex".to_string(),
            venue_ref: "offer-99".to_string(),
            price: "2.5000000".to_string(),
            path: vec![AuditPathStep {
                from: format!("USDC:{}", ISSUER),
                to: format!("BTC:{}", ISSUER),
                price: "2.5000000".to_string(),
                source: "sdex".to_string(),
            }],
            strategy: "direct_comparison".to_string(),
        }),
        vec![AuditExclusion {
            venue_ref: "pool-77".to_string(),
            reason: "depth".to_string(),
        }],
    );

    AuditRedactor::redact(&mut entry);

    let json = serde_json::to_string(&entry).expect("serialize");

    assert!(!json.contains(ISSUER), "raw issuer must be stripped");
    assert!(json.contains("[REDACTED]"), "redaction placeholder present");
    assert!(json.contains("req-export-01"), "request_id preserved");
}

#[test]
fn export_swap_submit_redacts_account_public_key() {
    let mut entry = SwapSubmitAuditEntry::new(
        "quote-export-01",
        Some("txhash999".to_string()),
        ACCOUNT,
        "req-swap-export",
        "trace-swap-export",
        88,
        SwapSubmitOutcome::Submitted,
        "",
        serde_json::json!({"amount": "100.0000000"}),
    );

    if !entry.account.contains('#') && entry.account != "native" {
        entry.account = AuditRedactor::redact_account(&entry.account);
    }

    let json = serde_json::to_string(&entry).expect("serialize");

    assert!(!json.contains(ACCOUNT), "raw account key must not be present");
    assert!(json.contains("GABC"), "account prefix preserved");
    assert!(json.contains('#'), "fingerprint hash separator present");
}

#[test]
fn export_ndjson_line_serialization_is_valid() {
    let mut route_entry = RouteAuditEntry::new(
        "req-01",
        "trace-01",
        12,
        AuditOutcome::Success,
        true,
        AuditInputs {
            base: "native".to_string(),
            quote: format!("USDC:{}", ISSUER),
            amount: "10.0000000".to_string(),
            slippage_bps: 50,
            quote_type: "sell".to_string(),
        },
        None,
        vec![],
    );
    AuditRedactor::redact(&mut route_entry);

    let line = serde_json::to_string(&route_entry).expect("serialize line");
    assert!(!line.contains('\n'), "NDJSON lines must not contain raw newlines");

    let parsed: serde_json::Value = serde_json::from_str(&line).expect("parse line");
    assert_eq!(parsed["request_id"], "req-01");
}
