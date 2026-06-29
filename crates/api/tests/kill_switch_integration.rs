//! Smoke tests for the admin kill switch handlers.
//!
//! These drive the handlers through the real router. No live Postgres is
//! required: the kill switch state is held in-memory, so a lazy pool that
//! never connects is enough to exercise GET/POST and the error mapper.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use stellarroute_api::{state::DatabasePools, Server, ServerConfig};
use tower::ServiceExt;

const KILL_SWITCH_PATH: &str = "/api/v1/admin/kill-switch";

async fn setup_test_router() -> axum::Router {
    // Lazy pool: it only connects when a query runs, and the kill switch
    // handlers never touch the database.
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .expect("Failed to create lazy pool");

    Server::new(ServerConfig::default(), DatabasePools::new(pool, None))
        .await
        .into_router()
}

#[tokio::test]
async fn kill_switch_get_returns_state_shape() {
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

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Fields documented in the OpenAPI `KillSwitchState` schema.
    assert!(json["sources"].is_object());
    assert!(json["venues"].is_object());
}

#[tokio::test]
async fn kill_switch_post_updates_in_memory_state() {
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
    let router = setup_test_router().await;

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(KILL_SWITCH_PATH)
                .header("content-type", "application/json")
                .body(Body::from("{ not valid json"))
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
