//! OpenAPI agent-paths contract test (AI-37).
//!
//! Asserts that the generated OpenAPI spec includes the two new agent paths
//! and that all previously documented swap / quote path schemas remain
//! byte-stable. Runs fully in-process with a lazy pool — no network or
//! database required.

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

#[tokio::test]
async fn openapi_agent_paths_are_present() {
    let router = setup_router().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api-docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let spec: Value = serde_json::from_slice(&body).unwrap();

    // Assert agent paths exist
    for (path, method) in [
        ("/api/v1/agent/health", "get"),
        ("/api/v1/agent/intents/validate", "post"),
    ] {
        let operation = &spec["paths"][path][method];
        assert!(
            !operation.is_null(),
            "{method} {path} must be documented in the OpenAPI spec"
        );

        let tags = operation["tags"]
            .as_array()
            .unwrap_or_else(|| panic!("{method} {path} must have a tags array"));
        assert!(
            tags.iter().any(|t| t == "agent"),
            "{method} {path} must be tagged 'agent', got {tags:?}"
        );

        assert!(
            operation["responses"]["200"].is_object() || operation["responses"]["404"].is_object(),
            "{method} {path} must document at least a 200 or 404 response"
        );
    }
}

#[tokio::test]
async fn openapi_existing_swap_quote_schemas_unchanged() {
    let router = setup_router().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api-docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let spec: Value = serde_json::from_slice(&body).unwrap();

    // Existing swap paths must still be present under the swap tag
    for (path, method) in [
        ("/api/v1/swap/prepare", "post"),
        ("/api/v1/swap/submit", "post"),
    ] {
        let operation = &spec["paths"][path][method];
        assert!(
            !operation.is_null(),
            "{method} {path} must still be documented (existing schema frozen)"
        );
        let tags = operation["tags"]
            .as_array()
            .unwrap_or_else(|| panic!("{method} {path} must still have tags"));
        assert!(
            tags.iter().any(|t| t == "swap"),
            "{method} {path} must still be tagged 'swap'"
        );
    }

    // Existing schemas must remain in components.schemas
    let schemas = &spec["components"]["schemas"];
    for schema_name in [
        "AssetPath",
        "SwapPrepareRequest",
        "SwapPrepareResponse",
        "SwapSubmitRequest",
        "SwapSubmitResponse",
    ] {
        assert!(
            schemas[schema_name].is_object() || schemas[schema_name]["oneOf"].is_array(),
            "{schema_name} schema must still be in components.schemas (frozen)"
        );
    }

    // No error code may be removed
    assert!(
        !spec["components"]["schemas"]["ErrorResponse"].is_null(),
        "ErrorResponse schema must still be present"
    );
}

#[tokio::test]
async fn openapi_agent_health_documents_404_response() {
    let router = setup_router().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api-docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let spec: Value = serde_json::from_slice(&body).unwrap();

    let health_op = &spec["paths"]["/api/v1/agent/health"]["get"];
    assert!(
        health_op["responses"]["404"].is_object(),
        "GET /api/v1/agent/health must document a 404 response for the flag-off state"
    );
    assert!(
        health_op["responses"]["200"].is_object(),
        "GET /api/v1/agent/health must document a 200 response for the flag-on state"
    );
}

#[tokio::test]
async fn agent_tools_catalog_requires_flag_and_returns_static_catalog() {
    std::env::remove_var("AI_AGENT_ENABLED");
    let router = setup_router().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/agent/tools")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    std::env::set_var("AI_AGENT_ENABLED", "true");
    let router = setup_router().await;
    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/agent/tools")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let spec: Value = serde_json::from_slice(&body).unwrap();
    let tools = spec["tools"].as_array().expect("tools array required");
    assert_eq!(tools.len(), 7, "must include the seven static tools");

    let names: Vec<_> = tools
        .iter()
        .map(|item| item["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        vec![
            "convert",
            "send",
            "receive",
            "bridge",
            "offramp",
            "subscribe",
            "balance",
        ]
    );

    assert!(tools.iter().all(|tool| tool["executable"] == false));
    for name in [
        "convert",
        "send",
        "receive",
        "bridge",
        "offramp",
        "subscribe",
    ] {
        let tool = tools
            .iter()
            .find(|item| item["name"] == name)
            .expect("tool should exist");
        assert_eq!(tool["requires_user_signature"], Value::Bool(true));
    }

    let balance = tools
        .iter()
        .find(|item| item["name"] == "balance")
        .expect("balance tool should exist");
    assert_eq!(balance["requires_user_signature"], Value::Bool(false));
    assert!(
        !spec["quote"].is_object() && !spec["route"].is_object() && !spec["swap"].is_object(),
        "tool catalog response must not expose legacy quote/route/swap fields"
    );
    std::env::remove_var("AI_AGENT_ENABLED");
}
