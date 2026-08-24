//! Integration tests for the route decision audit log.
//!
//! Tests that require a live PostgreSQL instance are marked `#[ignore]`
//! and must be run with `DATABASE_URL` set.
//!
//! Unit tests (no DB) run unconditionally and cover:
//! - Schema correctness (all required fields present)
//! - Redaction (sensitive fields removed, correlation IDs preserved)
//! - Correlation (request_id and trace_id round-trip)
//! - Retention (prune_older_than logic)

use stellarroute_api::audit::{
    AuditExclusion, AuditInputs, AuditOutcome, AuditPathStep, AuditRedactor, AuditSelected,
    RouteAuditEntry,
};

const ISSUER: &str = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";
const REQUEST_ID: &str = "req-audit-test-001";
const TRACE_ID: &str = "0af7651916cd43dd8448eb211c80319c";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn make_success_entry() -> RouteAuditEntry {
    RouteAuditEntry::new(
        REQUEST_ID,
        TRACE_ID,
        42,
        AuditOutcome::Success,
        false,
        AuditInputs {
            base: format!("USDC:{}", ISSUER),
            quote: "native".to_string(),
            amount: "100.0000000".to_string(),
            slippage_bps: 50,
            quote_type: "sell".to_string(),
        },
        Some(AuditSelected {
            venue_type: "sdex".to_string(),
            venue_ref: "offer1".to_string(),
            price: "1.0000000".to_string(),
            path: vec![AuditPathStep {
                from: format!("USDC:{}", ISSUER),
                to: "native".to_string(),
                price: "1.0000000".to_string(),
                source: "sdex".to_string(),
            }],
            strategy: "single_hop_direct_venue_comparison".to_string(),
        }),
        vec![AuditExclusion {
            venue_ref: "pool1".to_string(),
            reason: "stale_data".to_string(),
        }],
    )
}

fn make_no_route_entry() -> RouteAuditEntry {
    RouteAuditEntry::new(
        "req-no-route",
        TRACE_ID,
        8,
        AuditOutcome::NoRoute,
        false,
        AuditInputs {
            base: format!("BTC:{}", ISSUER),
            quote: "native".to_string(),
            amount: "1.0000000".to_string(),
            slippage_bps: 50,
            quote_type: "sell".to_string(),
        },
        None,
        vec![],
    )
}

// ─── AC #1: Schema captures inputs, selected route, and exclusion reasons ────

#[test]
fn schema_captures_all_required_fields() {
    let entry = make_success_entry();

    // Inputs
    assert!(!entry.inputs.base.is_empty(), "base must be present");
    assert!(!entry.inputs.quote.is_empty(), "quote must be present");
    assert!(!entry.inputs.amount.is_empty(), "amount must be present");
    assert!(
        entry.inputs.slippage_bps > 0,
        "slippage_bps must be present"
    );
    assert!(
        !entry.inputs.quote_type.is_empty(),
        "quote_type must be present"
    );

    // Selected route
    let selected = entry
        .selected
        .as_ref()
        .expect("selected must be present on success");
    assert!(
        !selected.venue_type.is_empty(),
        "venue_type must be present"
    );
    assert!(!selected.venue_ref.is_empty(), "venue_ref must be present");
    assert!(!selected.price.is_empty(), "price must be present");
    assert!(!selected.path.is_empty(), "path must be non-empty");
    assert!(!selected.strategy.is_empty(), "strategy must be present");

    // Exclusion reasons
    assert_eq!(entry.exclusions.len(), 1, "exclusions must be captured");
    assert_eq!(entry.exclusions[0].venue_ref, "pool1");
    assert_eq!(entry.exclusions[0].reason, "stale_data");
}

#[test]
fn no_route_entry_has_no_selected_and_empty_exclusions() {
    let entry = make_no_route_entry();
    assert!(
        entry.selected.is_none(),
        "no_route must have no selected route"
    );
    assert!(
        entry.exclusions.is_empty(),
        "no_route may have empty exclusions"
    );
    assert_eq!(entry.outcome.as_str(), "no_route");
}

#[test]
fn all_outcome_variants_are_representable() {
    for (outcome, expected) in [
        (AuditOutcome::Success, "success"),
        (AuditOutcome::NoRoute, "no_route"),
        (AuditOutcome::StaleData, "stale_data"),
        (AuditOutcome::Error, "error"),
    ] {
        assert_eq!(outcome.as_str(), expected);
    }
}

// ─── AC #2: Sensitive fields are redacted before persistence/export ───────────

#[test]
fn issuer_is_redacted_in_inputs() {
    let mut entry = make_success_entry();
    AuditRedactor::redact(&mut entry);
    assert!(
        !entry.inputs.base.contains(ISSUER),
        "issuer must not appear in inputs.base"
    );
    assert!(
        entry.inputs.base.contains("[REDACTED]"),
        "placeholder must appear in inputs.base"
    );
}

#[test]
fn issuer_is_redacted_in_path_steps() {
    let mut entry = make_success_entry();
    AuditRedactor::redact(&mut entry);
    let step = &entry.selected.as_ref().unwrap().path[0];
    assert!(
        !step.from.contains(ISSUER),
        "issuer must not appear in path.from"
    );
    assert!(
        !step.to.contains(ISSUER),
        "issuer must not appear in path.to"
    );
}

#[test]
fn venue_ref_is_not_redacted() {
    let mut entry = make_success_entry();
    AuditRedactor::redact(&mut entry);
    assert_eq!(
        entry.selected.as_ref().unwrap().venue_ref,
        "offer1",
        "venue_ref is public on-chain data and must not be redacted"
    );
    assert_eq!(
        entry.exclusions[0].venue_ref, "pool1",
        "exclusion venue_ref must not be redacted"
    );
}

#[test]
fn issuer_absent_from_serialized_json() {
    let mut entry = make_success_entry();
    AuditRedactor::redact(&mut entry);
    let json = serde_json::to_string(&entry).expect("serialize");
    assert!(
        !json.contains(ISSUER),
        "issuer '{}' must not appear anywhere in the serialized entry",
        ISSUER
    );
}

#[test]
fn redaction_is_idempotent() {
    let mut entry = make_success_entry();
    AuditRedactor::redact(&mut entry);
    let first = serde_json::to_string(&entry).expect("first");
    AuditRedactor::redact(&mut entry);
    let second = serde_json::to_string(&entry).expect("second");
    assert_eq!(first, second, "redaction must be idempotent");
}

// ─── AC #3: Logs correlate with request IDs and trace IDs ────────────────────

#[test]
fn request_id_is_preserved_through_redaction() {
    let mut entry = make_success_entry();
    AuditRedactor::redact(&mut entry);
    assert_eq!(
        entry.request_id, REQUEST_ID,
        "request_id must survive redaction"
    );
}

#[test]
fn trace_id_is_preserved_through_redaction() {
    let mut entry = make_success_entry();
    AuditRedactor::redact(&mut entry);
    assert_eq!(entry.trace_id, TRACE_ID, "trace_id must survive redaction");
}

#[test]
fn empty_trace_id_is_valid_when_no_trace_active() {
    let entry = RouteAuditEntry::new(
        "req-no-trace",
        "",
        5,
        AuditOutcome::Success,
        true,
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
    assert_eq!(entry.trace_id, "", "empty trace_id must be accepted");
}

#[test]
fn serde_round_trip_preserves_correlation_ids() {
    let mut entry = make_success_entry();
    AuditRedactor::redact(&mut entry);
    let json = serde_json::to_string(&entry).expect("serialize");
    let back: RouteAuditEntry = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.request_id, REQUEST_ID);
    assert_eq!(back.trace_id, TRACE_ID);
}

// ─── AC #4: Retention policy is documented and enforced ──────────────────────

#[test]
fn schema_version_is_current() {
    use stellarroute_api::audit::schema::AUDIT_SCHEMA_VERSION;
    let entry = make_success_entry();
    assert_eq!(
        entry.schema_version, AUDIT_SCHEMA_VERSION,
        "every entry must carry the current schema version"
    );
}

#[test]
fn retention_doc_exists() {
    // The test binary runs from the workspace root.  Try both the workspace
    // root and the crate root to handle different invocation contexts.
    let candidates = [
        std::path::PathBuf::from("docs/audit-log-retention.md"),
        std::path::PathBuf::from("../../docs/audit-log-retention.md"),
    ];
    let exists = candidates.iter().any(|p| p.exists());
    assert!(
        exists,
        "docs/audit-log-retention.md must exist (AC #4: retention policy documented)"
    );
}

// ─── DB integration tests ─────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn insert_and_fetch_round_trip() {
    use stellarroute_api::audit::AuditStore;

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
    });
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("connect");

    let store = AuditStore::new(pool);

    let mut entry = make_success_entry();
    AuditRedactor::redact(&mut entry);

    let id = store.insert(&entry).await.expect("insert");
    assert!(id > 0, "inserted id must be positive");

    let fetched = store.fetch(id).await.expect("fetch");
    assert_eq!(fetched.request_id, entry.request_id);
    assert_eq!(fetched.trace_id, entry.trace_id);
    assert_eq!(fetched.outcome.as_str(), entry.outcome.as_str());
    assert_eq!(fetched.latency_ms, entry.latency_ms);
    assert_eq!(fetched.inputs.base, entry.inputs.base);
    assert_eq!(fetched.inputs.quote, entry.inputs.quote);
    assert!(fetched.selected.is_some());
    assert_eq!(fetched.exclusions.len(), 1);
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn list_by_request_id_returns_matching_entries() {
    use stellarroute_api::audit::AuditStore;

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
    });
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("connect");

    let store = AuditStore::new(pool);
    let unique_req_id = format!("req-list-test-{}", uuid::Uuid::new_v4());

    // Insert two entries with the same request_id
    for _ in 0..2 {
        let mut entry = RouteAuditEntry::new(
            &unique_req_id,
            TRACE_ID,
            10,
            AuditOutcome::Success,
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
        AuditRedactor::redact(&mut entry);
        store.insert(&entry).await.expect("insert");
    }

    let summaries = store
        .list_by_request_id(&unique_req_id)
        .await
        .expect("list");
    assert_eq!(summaries.len(), 2, "both entries must be returned");
    for s in &summaries {
        assert_eq!(s.request_id, unique_req_id);
    }
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
async fn prune_older_than_removes_old_entries() {
    use stellarroute_api::audit::AuditStore;

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
    });
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("connect");

    let store = AuditStore::new(pool.clone());

    // Insert a fresh entry
    let mut entry = make_success_entry();
    AuditRedactor::redact(&mut entry);
    let id = store.insert(&entry).await.expect("insert");

    // Pruning with a 31-day window should NOT delete the fresh entry
    let deleted = store
        .prune_older_than(chrono::Duration::days(31))
        .await
        .expect("prune");
    assert_eq!(deleted, 0, "fresh entry must not be pruned");

    // Pruning with a 0-second window should delete it
    let deleted = store
        .prune_older_than(chrono::Duration::seconds(0))
        .await
        .expect("prune");
    assert!(deleted >= 1, "entry must be pruned with zero retention");

    // Verify it's gone
    let result = store.fetch(id).await;
    assert!(result.is_err(), "pruned entry must not be fetchable");
}
