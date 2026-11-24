//! Unit tests for audit log export pipeline (no database required).

use stellarroute_api::audit::export::storage::{build_manifest_key, build_object_key};
use stellarroute_api::audit::redactor::{AuditRedactor, REDACTED};
use stellarroute_api::audit::schema::{
    AuditExclusion, AuditInputs, AuditOutcome, AuditPathStep, AuditSelected, RouteAuditEntry,
};
use stellarroute_api::audit::AuditExportConfig;

const ISSUER: &str = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";

fn sample_entry() -> RouteAuditEntry {
    RouteAuditEntry::new(
        "req-export-001",
        "trace-export-001",
        15,
        AuditOutcome::Success,
        false,
        AuditInputs {
            base: format!("USDC:{}", ISSUER),
            quote: "native".to_string(),
            amount: "25.0000000".to_string(),
            slippage_bps: 50,
            quote_type: "sell".to_string(),
        },
        Some(AuditSelected {
            venue_type: "sdex".to_string(),
            venue_ref: "offer-export".to_string(),
            price: "1.0000000".to_string(),
            path: vec![AuditPathStep {
                from: format!("USDC:{}", ISSUER),
                to: "native".to_string(),
                price: "1.0000000".to_string(),
                source: "sdex".to_string(),
            }],
            strategy: "single_hop".to_string(),
        }),
        vec![AuditExclusion {
            venue_ref: "pool-export".to_string(),
            reason: "stale_data".to_string(),
        }],
    )
}

#[test]
fn export_config_defaults_to_disabled() {
    let cfg = AuditExportConfig::default();
    assert!(!cfg.enabled);
    assert_eq!(cfg.batch_size, 5_000);
}

#[test]
fn object_key_supports_incident_replay_partitioning() {
    let key = build_object_key("stellarroute/audit-logs", 1000, 1500, 1_700_000_000);
    assert!(key.contains("/2023/"));
    assert!(key.contains("batch-1000-1500-"));
    assert!(key.ends_with(".jsonl.gz"));

    let manifest = build_manifest_key(&key);
    assert!(manifest.ends_with(".manifest.json"));
}

#[test]
fn redaction_rules_remove_issuers_before_export() {
    let mut entry = sample_entry();
    AuditRedactor::redact(&mut entry);
    let json = serde_json::to_string(&entry).unwrap();

    assert!(!json.contains(ISSUER));
    assert!(json.contains(REDACTED));
    assert!(json.contains("req-export-001"));
    assert!(json.contains("trace-export-001"));
    assert!(json.contains("offer-export"));
}

#[test]
fn redaction_is_idempotent_for_export_safety() {
    let mut entry = sample_entry();
    AuditRedactor::redact(&mut entry);
    let once = serde_json::to_string(&entry).unwrap();
    AuditRedactor::redact(&mut entry);
    let twice = serde_json::to_string(&entry).unwrap();
    assert_eq!(once, twice);
}
