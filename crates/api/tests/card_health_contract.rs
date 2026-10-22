//! CARD-03 contract test (issue #1463).
//!
//! - Flag unset: `GET /api/v1/card/health` answers 404 (fail-closed).
//! - Flag on with no partner address: 200 with
//!   `{ "enabled": true, "issuer": "partner_unconfigured" }`.
//! - Flag on with a partner address: 200 with the configured address echoed
//!   as the issuer (the address is a public G-address).
//!
//! Runs fully in-process against a lazily-connected Postgres pool - no
//! network or database required. Env mutations are serialized via `ENV_LOCK`
//! and restored on drop.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use std::sync::Mutex;
use stellarroute_api::{state::DatabasePools, Server, ServerConfig};
use tower::ServiceExt;

static ENV_LOCK: Mutex<()> = Mutex::new(());

async fn setup_router() -> axum::Router {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .expect("failed to create lazy pool");

    Server::new(ServerConfig::default(), DatabasePools::new(pool, None))
        .await
        .into_router()
}

fn restore_var(key: &str, prev: Option<String>) {
    match prev {
        Some(v) => std::env::set_var(key, v),
        None => std::env::remove_var(key),
    }
}

/// Clears the card env vars for the duration of a test and restores them
/// afterwards so sibling tests never observe leftover state.
struct CardEnvGuard {
    card_enabled: Option<String>,
    partner_address: Option<String>,
}

impl CardEnvGuard {
    fn take() -> Self {
        let guard = Self {
            card_enabled: std::env::var("CARD_ENABLED").ok(),
            partner_address: std::env::var("CARD_PARTNER_STELLAR_ADDRESS").ok(),
        };
        std::env::remove_var("CARD_ENABLED");
        std::env::remove_var("CARD_PARTNER_STELLAR_ADDRESS");
        guard
    }
}

impl Drop for CardEnvGuard {
    fn drop(&mut self) {
        restore_var("CARD_ENABLED", self.card_enabled.take());
        restore_var(
            "CARD_PARTNER_STELLAR_ADDRESS",
            self.partner_address.take(),
        );
    }
}

async fn get_card_health(router: &axum::Router) -> (StatusCode, Value) {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/card/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

#[tokio::test]
async fn card_health_is_404_when_flag_unset() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = CardEnvGuard::take();

    let router = setup_router().await;
    let (status, _) = get_card_health(&router).await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "GET /api/v1/card/health must return 404 when CARD_ENABLED is unset"
    );
}

#[tokio::test]
async fn card_health_enabled_without_partner_reports_partner_unconfigured() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = CardEnvGuard::take();
    std::env::set_var("CARD_ENABLED", "true");

    let router = setup_router().await;
    let (status, json) = get_card_health(&router).await;
    assert_eq!(status, StatusCode::OK, "flag-on health must return 200");
    assert_eq!(
        json["data"]["enabled"],
        Value::Bool(true),
        "health data.enabled must be true, got {json}"
    );
    assert_eq!(
        json["data"]["issuer"],
        Value::String("partner_unconfigured".to_string()),
        "health data.issuer must be partner_unconfigured while the partner \
         address is unset, got {json}"
    );
}

#[tokio::test]
async fn card_health_echoes_configured_partner_address() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = CardEnvGuard::take();
    std::env::set_var("CARD_ENABLED", "true");
    std::env::set_var("CARD_PARTNER_STELLAR_ADDRESS", "GTESTPARTNERADDRESS");

    let router = setup_router().await;
    let (status, json) = get_card_health(&router).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        json["data"]["issuer"],
        Value::String("GTESTPARTNERADDRESS".to_string()),
        "health data.issuer must echo the configured partner address, got {json}"
    );
}
