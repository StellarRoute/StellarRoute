//! AI Agent module and endpoints
//!
//! Provides fail-closed AI agent surfaces gated behind `AI_AGENT_ENABLED`.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

/// Health status response for the AI agent service.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentHealthResponse {
    pub enabled: bool,
    pub execution: String,
}

/// Check whether the AI agent service is enabled via environment variable `AI_AGENT_ENABLED`.
/// Unset or false values return false (fail-closed by default).
pub fn is_agent_enabled() -> bool {
    crate::env_profile::parse_bool_env("AI_AGENT_ENABLED")
}

/// GET /api/v1/agent/health
///
/// Returns 404 Not Found when `AI_AGENT_ENABLED` is unset or false.
/// When enabled, returns 200 OK with `{ "enabled": true, "execution": "preview_only" }`.
pub async fn agent_health() -> Response {
    if !is_agent_enabled() {
        return StatusCode::NOT_FOUND.into_response();
    }

    (
        StatusCode::OK,
        Json(AgentHealthResponse {
            enabled: true,
            execution: "preview_only".to_string(),
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[tokio::test]
    async fn test_agent_health_disabled_when_unset() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("AI_AGENT_ENABLED");

        assert!(!is_agent_enabled());
        let res = agent_health().await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_agent_health_disabled_when_false() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("AI_AGENT_ENABLED", "false");

        assert!(!is_agent_enabled());
        let res = agent_health().await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);

        std::env::set_var("AI_AGENT_ENABLED", "0");
        assert!(!is_agent_enabled());
        let res = agent_health().await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);

        std::env::remove_var("AI_AGENT_ENABLED");
    }

    #[tokio::test]
    async fn test_agent_health_enabled_returns_preview_only() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("AI_AGENT_ENABLED", "true");

        assert!(is_agent_enabled());
        let res = agent_health().await;
        assert_eq!(res.status(), StatusCode::OK);

        let body_bytes = to_bytes(res.into_body(), usize::MAX)
            .await
            .expect("read body");
        let parsed: AgentHealthResponse =
            serde_json::from_slice(&body_bytes).expect("parse json response");

        assert!(parsed.enabled);
        assert_eq!(parsed.execution, "preview_only");

        std::env::remove_var("AI_AGENT_ENABLED");
    }
}
//! AI agent routes and handlers

pub mod intents;
pub mod tools;

pub use intents::validate_intent;
pub use tools::{list_agent_tools, AgentTool, AgentToolsResponse};
