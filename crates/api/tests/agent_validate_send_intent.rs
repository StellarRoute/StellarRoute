//! AI-50 capstone: parser, validate, and flag-off 404 together.
//!
//! One API test proving the sample send intent is accepted by validate when
//! the flag is on and 404s when the flag is off. Runs fully in-process with
//! a lazy pool — no network or database required. Does not touch quote
//! ranking or quote route tests.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use stellarroute_api::{state::DatabasePools, Server, ServerConfig};
use tower::ServiceExt;

async fn setup_router() -> axum::Router {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .expect("failed to create lazy pool");

    Server::new(ServerConfig::default(), DatabasePools::new(pool, None))
        .await
        .into_router()
}

fn sample_send_intent() -> Value {
    serde_json::json!({
        "type": "send",
        "amount": "50",
        "asset": "XLM",
        "recipient": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
    })
}

#[tokio::test]
async fn send_intent_validates_when_flag_on_and_404s_when_flag_off() {
    let body = sample_send_intent();

    // Flag off → 404, same request the server accepts when on.
    std::env::remove_var("AI_AGENT_ENABLED");
    let router = setup_router().await;
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agent/intents/validate")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .expect("request failed");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // Flag on → 200 with normalized amount/type.
    std::env::set_var("AI_AGENT_ENABLED", "true");
    let router = setup_router().await;
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agent/intents/validate")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .expect("request failed");
    assert_eq!(response.status(), StatusCode::OK);

    let raw = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(parsed["amount"], "50");
    assert_eq!(parsed["type"], "send");

    std::env::remove_var("AI_AGENT_ENABLED");
}
