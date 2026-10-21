use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct AgentTool {
    pub name: String,
    pub executable: bool,
    pub requires_user_signature: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AgentToolsResponse {
    pub tools: Vec<AgentTool>,
}

pub fn list_agent_tools() -> Vec<AgentTool> {
    vec![
        AgentTool { name: "convert".into(), executable: false, requires_user_signature: true },
        AgentTool { name: "send".into(), executable: false, requires_user_signature: true },
        AgentTool { name: "receive".into(), executable: false, requires_user_signature: true },
        AgentTool { name: "bridge".into(), executable: false, requires_user_signature: true },
        AgentTool { name: "offramp".into(), executable: false, requires_user_signature: true },
        AgentTool { name: "subscribe".into(), executable: false, requires_user_signature: true },
        AgentTool { name: "balance".into(), executable: false, requires_user_signature: false },
    ]
}