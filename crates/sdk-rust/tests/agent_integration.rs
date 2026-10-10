//! AI-36 integration tests for agent preview endpoints.
//!
//! Uses `wiremock` so no live API is required. Verifies:
//! - disabled health (404) does not panic and maps to `enabled: false`,
//! - validate posts the intent JSON,
//! - existing tests still pass (run full suite).

use stellarroute_sdk::{ClientBuilder, ValidateIntentRequest};
use wiremock::{
    matchers::{body_json, method, path},
    Mock, MockServer, ResponseTemplate,
};

async fn mock_server() -> MockServer {
    MockServer::start().await
}

fn client(server: &MockServer) -> stellarroute_sdk::StellarRouteClient {
    ClientBuilder::new(server.uri()).build().unwrap()
}

#[tokio::test]
async fn disabled_health_does_not_panic() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/agent/health"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": "not_found",
            "message": "Agent feature is disabled"
        })))
        .mount(&server)
        .await;

    let health = client(&server).agent_health().await.unwrap();
    assert!(!health.is_enabled());
}

#[tokio::test]
async fn health_returns_enabled() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/agent/health"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "enabled": true
        })))
        .mount(&server)
        .await;

    let health = client(&server).agent_health().await.unwrap();
    assert!(health.is_enabled());
}

#[tokio::test]
async fn health_returns_disabled() {
    let server = mock_server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/agent/health"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "enabled": false
        })))
        .mount(&server)
        .await;

    let health = client(&server).agent_health().await.unwrap();
    assert!(!health.is_enabled());
}

#[tokio::test]
async fn validate_sends_intent_json() {
    let server = mock_server().await;
    let expected_request = ValidateIntentRequest {
        r#type: "send".to_string(),
        amount: "100".to_string(),
        asset: "XLM".to_string(),
        recipient: Some("GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN".to_string()),
        destination: None,
        source: None,
        currency: None,
        recurrence: None,
    };

    Mock::given(method("POST"))
        .and(path("/api/v1/agent/intents/validate"))
        .and(body_json(serde_json::json!({
            "type": "send",
            "amount": "100",
            "asset": "XLM",
            "recipient": "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "amount": "100",
            "type": "send"
        })))
        .mount(&server)
        .await;

    let response = client(&server)
        .validate_intent(expected_request)
        .await
        .unwrap();
    assert_eq!(response.amount, "100");
    assert_eq!(response.r#type, "send");
}

#[tokio::test]
async fn validate_disabled_returns_404() {
    let server = mock_server().await;
    let request = ValidateIntentRequest {
        r#type: "send".to_string(),
        amount: "50".to_string(),
        asset: "USDC".to_string(),
        recipient: None,
        destination: None,
        source: None,
        currency: None,
        recurrence: None,
    };

    Mock::given(method("POST"))
        .and(path("/api/v1/agent/intents/validate"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": "not_found",
            "message": "Agent feature is disabled"
        })))
        .mount(&server)
        .await;

    let err = client(&server)
        .validate_intent(request)
        .await
        .unwrap_err();
    assert!(err.is_not_found());
    assert_eq!(err.status_code(), Some(404));
}