use crate::env_profile::parse_bool_env;

/// Card configuration loaded from environment variables.
#[derive(Clone, Debug)]
pub struct CardConfig {
    /// Whether the card feature is enabled.
    pub enabled: bool,
    /// Stellar address of the card partner (issuer).
    pub partner_stellar_address: Option<String>,
}

impl Default for CardConfig {
    fn default() -> Self {
        Self {
            enabled: parse_bool_env("CARD_ENABLED"),
            partner_stellar_address: std::env::var("CARD_PARTNER_STELLAR_ADDRESS").ok(),
        }
    }
}

impl CardConfig {
    /// Returns the issuer string for health endpoint.
    pub fn issuer(&self) -> &str {
        self.partner_stellar_address
            .as_deref()
            .unwrap_or("partner_unconfigured")
    }
}