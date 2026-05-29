use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use stellarroute_routing::health::circuit_breaker::{
    BreakerConfig, BreakerState, CircuitBreakerRegistry,
};

const HORIZON_KEY: &str = "horizon";
const SOROBAN_KEY: &str = "soroban_rpc";

#[derive(Clone)]
pub struct ExternalDependencyHealth {
    client: Client,
    horizon_url: Option<String>,
    soroban_rpc_url: Option<String>,
    horizon_breaker: Arc<CircuitBreakerRegistry>,
    soroban_breaker: Arc<CircuitBreakerRegistry>,
}

impl ExternalDependencyHealth {
    pub fn from_env() -> Self {
        let horizon_url = std::env::var("STELLAR_HORIZON_URL")
            .ok()
            .map(|v| v.trim().trim_end_matches('/').to_string())
            .filter(|v| !v.is_empty());

        let soroban_rpc_url = std::env::var("SOROBAN_RPC_URL")
            .ok()
            .map(|v| v.trim().trim_end_matches('/').to_string())
            .filter(|v| !v.is_empty());

        Self::new(horizon_url, soroban_rpc_url)
    }

    pub fn new(horizon_url: Option<String>, soroban_rpc_url: Option<String>) -> Self {
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
            horizon_url,
            soroban_rpc_url,
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
        let Some(base_url) = &self.horizon_url else {
            return "not_configured".to_string();
        };

        if self.horizon_breaker.is_venue_excluded(HORIZON_KEY) {
            return "degraded (circuit_open)".to_string();
        }

        let url = format!("{}/health", base_url);
        let success = client
            .get(&url)
            .send()
            .await
            .map(|resp| resp.status().is_success())
            .unwrap_or(false);

        self.horizon_breaker.record_result(HORIZON_KEY, success);

        if success {
            "healthy".to_string()
        } else {
            "degraded".to_string()
        }
    }

    async fn probe_soroban_with_client(&self, client: &Client) -> String {
        let Some(url) = &self.soroban_rpc_url else {
            return "not_configured".to_string();
        };

        if self.soroban_breaker.is_venue_excluded(SOROBAN_KEY) {
            return "degraded (circuit_open)".to_string();
        }

        // Half-open recovery is naturally driven by this lightweight getHealth probe:
        // once recovery_timeout elapses the breaker transitions to half-open and this
        // endpoint is tried again; enough consecutive successes closes the breaker.
        let req = json!({
            "jsonrpc": "2.0",
            "id": "dep-health-probe",
            "method": "getHealth",
            "params": {}
        });

        let success = client
            .post(url)
            .json(&req)
            .send()
            .await
            .and_then(|resp| resp.error_for_status())
            .is_ok();

        self.soroban_breaker.record_result(SOROBAN_KEY, success);

        if success {
            "healthy".to_string()
        } else {
            "degraded".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stellarroute_routing::health::circuit_breaker::BreakerState;

    #[test]
    fn soroban_and_horizon_breakers_are_independent() {
        let health = ExternalDependencyHealth::new(None, None);

        for _ in 0..3 {
            health.record_soroban_result(false);
        }

        assert_eq!(health.soroban_breaker_state(), Some(BreakerState::Open));
        assert!(!health.horizon_breaker_is_open());
        assert_ne!(health.horizon_breaker_state(), Some(BreakerState::Open));
    }

    #[test]
    fn soroban_open_does_not_require_horizon_degradation() {
        let health = ExternalDependencyHealth::new(None, None);

        for _ in 0..3 {
            health.record_soroban_result(false);
        }
        for _ in 0..2 {
            health.record_horizon_result(true);
        }

        assert!(health.soroban_breaker_is_open());
        assert!(!health.horizon_breaker_is_open());
    }
}
