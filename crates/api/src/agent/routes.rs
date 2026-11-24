use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{OpenApi, ToSchema};

use crate::state::AppState;
use crate::models::{ApiErrorCode, ErrorResponse};

/// Agent chat request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentChatRequest {
    /// The message to send to the agent
    pub message: String,
    /// Optional conversation ID for context
    pub conversation_id: Option<String>,
}

/// Agent chat response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentChatResponse {
    /// The agent's response
    pub response: String,
    /// Conversation ID for follow-up messages
    pub conversation_id: String,
}

/// Error response for 404 when agent is disabled
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentNotEnabledResponse {
    pub error: String,
}

/// POST /api/v1/agent/chat
#[utoipa::path(
    post,
    path = "/api/v1/agent/chat",
    tag = "agent",
    request_body = AgentChatRequest,
    responses(
        (status = 200, description = "Agent response", body = AgentChatResponse),
        (status = 404, description = "Agent feature not enabled", body = AgentNotEnabledResponse),
        (status = 429, description = "Rate limit exceeded"),
    )
)]
pub async fn agent_chat(
    State(state): State<crate::state::AppStateArc>,
    Json(request): Json<AgentChatRequest>,
) -> Result<Json<crate::models::ApiResponse<AgentChatResponse>>, (StatusCode, Json<crate::models::ApiResponse<AgentNotEnabledResponse>>)> {
    let agent_config = &state.agent_config;
    
    if !agent_config.enabled {
        return Err((
            StatusCode::NOT_FOUND,
            Json(crate::models::ApiResponse::with_version(
                1,
                AgentNotEnabledResponse {
                    error: "Agent feature not enabled".to_string(),
                },
                "",
            )),
        ));
    }

    // TODO: Implement actual agent logic
    // For now, return a placeholder response
    Ok(Json(crate::models::ApiResponse::with_version(
        1,
        AgentChatResponse {
            response: format!("Agent received: {}", request.message),
            conversation_id: request.conversation_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        },
        "",
    )))
}