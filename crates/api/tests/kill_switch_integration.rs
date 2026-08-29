//! Smoke tests for the admin kill switch handlers.
//!
//! These drive the handlers through the real router. No live Postgres is
//! required: the kill switch state is held in-memory, so a lazy pool that
//! never connects is enough to exercise GET/POST and the error mapper.
//!
//! GET is gated in production (issue #1053): open in dev/test for
//! operational visibility, admin-token-required when
//! `STELLARROUTE_ENV=production`. POST always requires AdminAuth regardless
//! of environment. Tests that touch `STELLARROUTE_ENV` are serialized via
//! `ENV_LOCK` since it's process-global.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use std::collections::HashMap;
use std::sync::Mutex;
use stellarroute_api::kill_switch::{KillSwitchManager, KillSwitchState};
use stellarroute_api::{state::DatabasePools, Server, ServerConfig};
use stellarroute_routing::health::policy::{
    ExclusionPolicy, ExclusionThresholds, OverrideDirective,
};
use stellarroute_routing::health::scorer::{HealthRecord, ScoredVenue, VenueType};
use tower::ServiceExt;

const KILL_SWITCH_PATH: &str = "/api/v1/admin/kill-switch";
const ADMIN_TOKEN: &str = "test-admin-token";

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn reset_env() {
    std::env::remove_var("STELLARROUTE_ENV");
}

async fn setup_test_router() -> axum::Router {
    // Lazy pool: it only connects when a query runs, and the kill switch
    // handlers never touch the database.
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .expect("Failed to create lazy pool");

    let mut config = ServerConfig::default();
    config.admin_auth_token = Some(ADMIN_TOKEN.to_string());

    Server::new(config, DatabasePools::new(pool, None))
        .await
        .into_router()
}

#[tokio::test]
async fn kill_switch_get_returns_state_shape() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_env();
    let router = setup_test_router().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri(KILL_SWITCH_PATH)
                .header("x-admin-token", ADMIN_TOKEN)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Fields documented in the OpenAPI `KillSwitchState` schema.
    assert!(json["sources"].is_object());
    assert!(json["venues"].is_object());
}

#[tokio::test]
async fn kill_switch_get_without_admin_token_returns_401() {
    let router = setup_test_router().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri(KILL_SWITCH_PATH)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn kill_switch_post_updates_in_memory_state() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_env();
    let router = setup_test_router().await;

    let payload = json!({
        "sources": { "amm": "force_exclude" },
        "venues": { "sdex:123": "force_exclude" },
    });

    let post = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(KILL_SWITCH_PATH)
                .header("content-type", "application/json")
                .header("x-admin-token", ADMIN_TOKEN)
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(post.status(), StatusCode::OK);

    // The update lives in the shared in-memory state, so a follow-up GET
    // against the same router must reflect it.
    let get = router
        .oneshot(
            Request::builder()
                .uri(KILL_SWITCH_PATH)
                .header("x-admin-token", ADMIN_TOKEN)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(get.status(), StatusCode::OK);

    let body = axum::body::to_bytes(get.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["sources"]["amm"], "force_exclude");
    assert_eq!(json["venues"]["sdex:123"], "force_exclude");
}

#[tokio::test]
async fn kill_switch_post_invalid_payload_returns_400() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_env();
    let router = setup_test_router().await;

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(KILL_SWITCH_PATH)
                .header("content-type", "application/json")
                .header("x-admin-token", ADMIN_TOKEN)
                .body(Body::from("{ not valid json"))
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn kill_switch_post_without_admin_token_returns_401() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_env();
    let router = setup_test_router().await;

    let payload = json!({
        "sources": { "amm": "force_exclude" },
        "venues": {},
    });

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(KILL_SWITCH_PATH)
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn kill_switch_get_public_outside_production() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_env();
    let router = setup_test_router().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri(KILL_SWITCH_PATH)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn kill_switch_get_requires_admin_auth_in_production() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_env();
    std::env::set_var("STELLARROUTE_ENV", "production");

    let router = setup_test_router().await;

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(KILL_SWITCH_PATH)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let with_token = router
        .oneshot(
            Request::builder()
                .uri(KILL_SWITCH_PATH)
                .header("x-admin-token", ADMIN_TOKEN)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");
    assert_eq!(with_token.status(), StatusCode::OK);

    reset_env();
}

#[tokio::test]
async fn kill_switch_post_requires_admin_auth_in_production_too() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_env();
    std::env::set_var("STELLARROUTE_ENV", "production");

    let router = setup_test_router().await;

    let payload = json!({ "sources": {}, "venues": {} });
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(KILL_SWITCH_PATH)
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .expect("request failed");

    reset_env();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ── Default-off regression guards (issue #1283) ──────────────────────────────
//
// The kill switch ships empty. These lock that a default deployment leaves
// classic SDEX quotes untouched, and that flipping one entry only removes the
// entry that was targeted.

/// A freshly built server must report an empty kill switch state: no source
/// entry for SDEX, no venue entries at all.
#[tokio::test]
async fn kill_switch_default_state_has_no_sdex_entry() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_env();
    let router = setup_test_router().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri(KILL_SWITCH_PATH)
                .header("x-admin-token", ADMIN_TOKEN)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        json["sources"].as_object().expect("sources object").len(),
        0,
        "default config must not disable any quote source"
    );
    assert!(
        json["sources"]["sdex"].is_null(),
        "default config must not carry an SDEX kill switch"
    );
    assert_eq!(
        json["venues"].as_object().expect("venues object").len(),
        0,
        "default config must not disable any venue"
    );
}

/// The routing-side view of the default state: an SDEX venue scored healthy is
/// not excluded, so classic SDEX quotes are still produced.
#[tokio::test]
async fn default_kill_switch_registry_keeps_sdex_quotable() {
    let manager = KillSwitchManager::new(None);
    let registry = manager.get_override_registry().await;

    assert!(registry.source_entries.is_empty());
    assert!(registry.venue_entries.is_empty());

    let policy = ExclusionPolicy {
        thresholds: ExclusionThresholds::default(),
        overrides: registry,
        circuit_breaker: None,
    };
    let (excluded, _diagnostics) = policy.apply(&[scored_venue("sdex:1", VenueType::Sdex, 0.9)]);

    assert!(
        !excluded.contains("sdex:1"),
        "default (empty) kill switch must not exclude SDEX"
    );
}

/// Enabling the switch for one source removes only that source: excluding AMM
/// leaves the SDEX venue selectable.
#[tokio::test]
async fn enabled_kill_switch_excludes_only_the_targeted_source() {
    let manager = KillSwitchManager::new(None);
    let mut sources = HashMap::new();
    sources.insert(VenueType::Amm, OverrideDirective::ForceExclude);
    manager
        .update_state(KillSwitchState {
            sources,
            ..Default::default()
        })
        .await
        .expect("in-memory update");

    let policy = ExclusionPolicy {
        thresholds: ExclusionThresholds::default(),
        overrides: manager.get_override_registry().await,
        circuit_breaker: None,
    };
    let (excluded, _diagnostics) = policy.apply(&[
        scored_venue("sdex:1", VenueType::Sdex, 0.9),
        scored_venue("amm:1", VenueType::Amm, 0.9),
    ]);

    assert!(excluded.contains("amm:1"), "targeted source is excluded");
    assert!(
        !excluded.contains("sdex:1"),
        "untargeted SDEX source stays quotable"
    );
}

/// Likewise for a single venue entry: one SDEX offer can be pulled without
/// taking the rest of SDEX with it.
#[tokio::test]
async fn enabled_kill_switch_excludes_only_the_targeted_venue() {
    let manager = KillSwitchManager::new(None);
    let mut venues = HashMap::new();
    venues.insert("sdex:123".to_string(), OverrideDirective::ForceExclude);
    manager
        .update_state(KillSwitchState {
            venues,
            ..Default::default()
        })
        .await
        .expect("in-memory update");

    let policy = ExclusionPolicy {
        thresholds: ExclusionThresholds::default(),
        overrides: manager.get_override_registry().await,
        circuit_breaker: None,
    };
    let (excluded, _diagnostics) = policy.apply(&[
        scored_venue("sdex:123", VenueType::Sdex, 0.9),
        scored_venue("sdex:456", VenueType::Sdex, 0.9),
    ]);

    assert!(excluded.contains("sdex:123"), "targeted venue is excluded");
    assert!(
        !excluded.contains("sdex:456"),
        "sibling SDEX venue stays quotable"
    );
}

/// Healthy venue fixture — score is above the default 0.5 exclusion threshold,
/// so anything excluded here was excluded by the kill switch and nothing else.
fn scored_venue(venue_ref: &str, venue_type: VenueType, score: f64) -> ScoredVenue {
    ScoredVenue {
        venue_ref: venue_ref.to_string(),
        venue_type: venue_type.clone(),
        record: HealthRecord {
            venue_ref: venue_ref.to_string(),
            venue_type,
            score,
            signals: json!({}),
            computed_at: chrono::Utc::now(),
        },
    }
}
