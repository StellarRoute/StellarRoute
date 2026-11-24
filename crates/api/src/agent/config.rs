use crate::env_profile::parse_bool_env;

/// Agent configuration loaded from environment variables.
#[derive(Clone, Debug, Default)]
pub struct AgentConfig {
    /// Whether the agent feature is enabled.
    pub enabled: bool,
}

impl AgentConfig {
    pub fn from_env() -> Self {
        Self {
            enabled: parse_bool_env("AI_AGENT_ENABLED"),
        }
    }
}