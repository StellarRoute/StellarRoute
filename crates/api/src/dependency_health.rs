use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use stellarroute_routing::health::circuit_breaker::{
    BreakerConfig, BreakerState, CircuitBreakerRegistry,
};

const HORIZON_KEY: &str = "horizon";
const SOROBAN_KEY: &str = "soroban_rpc";

/// Parse a comma-separated list of URLs (primary + optional fallbacks).
/// The first entry is the primary; subsequent entries are tried in order.
fn parse_failover_urls(primary: Option<String>, fallback_env: &str) -> Vec<String> {
    let mut urls: Vec<String> = primary
        .into_iter()
        .map(|u| u.trim().trim_end_matches('/').to_string())
        .filter(|u| !u.is_empty())
        .collect();

    if let Ok(extra) = std::env::var(fallback_env) {
        for u in extra.split(',') {
            let u = u.trim().trim_end_matches('/').to_string();
            if !u.is_empty() {
                urls.push(u);
            }
        }
    }

    urls
}

#[derive(Clone)]
pub struct ExternalDependencyHealth {
    client: Client,
    /// Ordered list of Horizon URLs: primary first, then fallbacks.
    horizon_urls: Vec<String>,
    /// Ordered list of Soroban RPC URLs: primary first, then fallbacks.
    soroban_rpc_urls: Vec<String>,
    horizon_breaker: Arc<CircuitBreakerRegistry>,
    soroban_breaker: Arc<CircuitBreakerRegistry>,
}

impl ExternalDependencyHealth {
    pub fn from_env() -> Self {
        let horizon_primary = std::env::var("STELLAR_HORIZON_URL")
            .ok()
            .map(|v| v.trim().trim_end_matches('/').to_string())
            .filter(|v| !v.is_empty());

        let soroban_primary = std::env::var("SOROBAN_RPC_URL")
            .ok()
            .map(|v| v.trim().trim_end_matches('/').to_string())
            .filter(|v| !v.is_empty());

        let horizon_urls =
            parse_failover_urls(horizon_primary, "STELLAR_HORIZON_FALLBACK_URLS");
        let soroban_rpc_urls =
            parse_failover_urls(soroban_primary, "SOROBAN_RPC_FALLBACK_URLS");

        Self::new(horizon_urls, soroban_rpc_urls)
    }

    pub fn new(horizon_urls: Vec<String>, soroban_rpc_urls: Vec<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap_or_default();

        let cfg = BreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            recovery_timeout_secs: 15,
        };

        Self {
            client,
            horizon_urls,
            soroban_rpc_urls,
            horizon_breaker: Arc::new(CircuitBreakerRegistry::new(cfg.clone())),
            soroban_breaker: Arc::new(CircuitBreakerRegistry::new(cfg)),
        }
    }

    pub async fn probe_horizon(&self) -> String {
        self.probe_horizon_with_client(&self.client).await
    }

    pub async fn probe_soroban(&self) -> String {
        self.probe_soroban_with_client(&self.client).await
    }

    pub fn soroban_breaker_is_open(&self) -> bool {
        self.soroban_breaker.is_venue_excluded(SOROBAN_KEY)
    }

    pub fn horizon_breaker_is_open(&self) -> bool {
        self.horizon_breaker.is_venue_excluded(HORIZON_KEY)
    }

    pub fn soroban_breaker_state(&self) -> Option<BreakerState> {
        self.soroban_breaker.get_state(SOROBAN_KEY)
    }

    pub fn horizon_breaker_state(&self) -> Option<BreakerState> {
        self.horizon_breaker.get_state(HORIZON_KEY)
    }

    pub fn record_soroban_result(&self, success: bool) {
        self.soroban_breaker.record_result(SOROBAN_KEY, success);
    }

    pub fn record_horizon_result(&self, success: bool) {
        self.horizon_breaker.record_result(HORIZON_KEY, success);
    }

    async fn probe_horizon_with_client(&self, client: &Client) -> String {
        if self.horizon_urls.is_empty() {
            return "not_configured".to_string();
        }

        if self.horizon_breaker.is_venue_excluded(HORIZON_KEY) {
            return "degraded (circuit_open)".to_string();
        }

        for base_url in &self.horizon_urls {
            let url = format!("{}/health", base_url);
            let success = client
                .get(&url)
                .send()
                .await
                .map(|resp| resp.status().is_success())
                .unwrap_or(false);

            if success {
                self.horizon_breaker.record_result(HORIZON_KEY, true);
                return "healthy".to_string();
            }
        }

        // All URLs failed
        self.horizon_breaker.record_result(HORIZON_KEY, false);
        "degraded".to_string()
    }

    async fn probe_soroban_with_client(&self, client: &Client) -> String {
        if self.soroban_rpc_urls.is_empty() {
            return "not_configured".to_string();
        }

        if self.soroban_breaker.is_venue_excluded(SOROBAN_KEY) {
            return "degraded (circuit_open)".to_string();
        }

        let req = json!({
            "jsonrpc": "2.0",
            "id": "dep-health-probe",
            "method": "getHealth",
            "params": {}
        });

        for url in &self.soroban_rpc_urls {
            let success = client
                .post(url)
                .json(&req)
                .send()
                .await
                .and_then(|resp| resp.error_for_status())
                .is_ok();

            if success {
                self.soroban_breaker.record_result(SOROBAN_KEY, true);
                return "healthy".to_string();
            }
        }

        // All URLs failed
        self.soroban_breaker.record_result(SOROBAN_KEY, false);
        "degraded".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stellarroute_routing::health::circuit_breaker::BreakerState;

    #[test]
    fn soroban_and_horizon_breakers_are_independent() {
        let health = ExternalDependencyHealth::new(vec![], vec![]);

        for _ in 0..3 {
            health.record_soroban_result(false);
        }

        assert_eq!(health.soroban_breaker_state(), Some(BreakerState::Open));
        assert!(!health.horizon_breaker_is_open());
        assert_ne!(health.horizon_breaker_state(), Some(BreakerState::Open));
    }

    #[test]
    fn soroban_open_does_not_require_horizon_degradation() {
        let health = ExternalDependencyHealth::new(vec![], vec![]);

        for _ in 0..3 {
            health.record_soroban_result(false);
        }
        for _ in 0..2 {
            health.record_horizon_result(true);
        }

        assert!(health.soroban_breaker_is_open());
        assert!(!health.horizon_breaker_is_open());
    }

    #[test]
    fn parse_failover_urls_empty_primary() {
        // When primary is None and env var is absent, result should be empty.
        let urls = parse_failover_urls(None, "DOES_NOT_EXIST_ENV_VAR_XYZ");
        assert!(urls.is_empty());
    }

    #[test]
    fn parse_failover_urls_primary_only() {
        let urls = parse_failover_urls(
            Some("https://horizon.stellar.org".to_string()),
            "DOES_NOT_EXIST_ENV_VAR_XYZ",
        );
        assert_eq!(urls, vec!["https://horizon.stellar.org"]);
    }

    #[test]
    fn horizon_urls_returns_not_configured_when_empty() {
        let health = ExternalDependencyHealth::new(vec![], vec![]);
        // sync check
        assert!(health.horizon_urls.is_empty());
        assert!(health.soroban_rpc_urls.is_empty());
    }
}
