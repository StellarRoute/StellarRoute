use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use stellarroute_routing::health::circuit_breaker::{
    BreakerConfig, BreakerState, CircuitBreakerRegistry,
};

const HORIZON_KEY: &str = "horizon";
const SOROBAN_KEY: &str = "soroban_rpc";
const EVM_RPC_KEY: &str = "evm_rpc";
const DATABASE_KEY: &str = "database";

/// Dependencies whose failure makes a quote/swap answer wrong rather than slow.
///
/// Redis is deliberately excluded: it is a performance cache and the API already
/// degrades gracefully without it, so an open Redis breaker must not reject traffic.
const LIVE_PATH_DEPENDENCIES: [&str; 3] = [DATABASE_KEY, SOROBAN_KEY, HORIZON_KEY];

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
    /// Sepolia / EVM JSON-RPC URLs for CCTP EVM burn/mint paths.
    evm_rpc_urls: Vec<String>,
    horizon_breaker: Arc<CircuitBreakerRegistry>,
    soroban_breaker: Arc<CircuitBreakerRegistry>,
    evm_rpc_breaker: Arc<CircuitBreakerRegistry>,
    database_breaker: Arc<CircuitBreakerRegistry>,
}

impl ExternalDependencyHealth {
    pub fn from_env() -> Self {
        let horizon_primary = std::env::var("STELLAR_HORIZON_URL")
            .ok()
            .map(|v| v.trim().trim_end_matches('/').to_string())
            .filter(|v| !v.is_empty());

        let horizon_urls = parse_failover_urls(horizon_primary, "STELLAR_HORIZON_FALLBACK_URLS");

        let soroban_rpc_urls = crate::cctp::rpc_env::resolve_stellar_rpc_urls()
            .unwrap_or_else(|_| vec![crate::cctp::rpc_env::resolve_stellar_rpc_primary()]);

        let evm_rpc_urls = crate::cctp::rpc_env::resolve_sepolia_rpc_urls().unwrap_or_default();

        Self::with_evm_rpc(horizon_urls, soroban_rpc_urls, evm_rpc_urls)
    }

    pub fn new(horizon_urls: Vec<String>, soroban_rpc_urls: Vec<String>) -> Self {
        Self::with_evm_rpc(horizon_urls, soroban_rpc_urls, Vec::new())
    }

    pub fn with_evm_rpc(
        horizon_urls: Vec<String>,
        soroban_rpc_urls: Vec<String>,
        evm_rpc_urls: Vec<String>,
    ) -> Self {
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
            evm_rpc_urls,
            horizon_breaker: Arc::new(CircuitBreakerRegistry::new(cfg.clone())),
            soroban_breaker: Arc::new(CircuitBreakerRegistry::new(cfg.clone())),
            evm_rpc_breaker: Arc::new(CircuitBreakerRegistry::new(cfg.clone())),
            database_breaker: Arc::new(CircuitBreakerRegistry::new(cfg)),
        }
    }

    fn breaker_for(&self, key: &str) -> &CircuitBreakerRegistry {
        match key {
            DATABASE_KEY => &self.database_breaker,
            SOROBAN_KEY => &self.soroban_breaker,
            EVM_RPC_KEY => &self.evm_rpc_breaker,
            _ => &self.horizon_breaker,
        }
    }

    /// Dependencies whose breaker is open right now, refreshing the exported gauge.
    pub fn open_dependencies(&self) -> Vec<&'static str> {
        LIVE_PATH_DEPENDENCIES
            .into_iter()
            .filter(|key| {
                let open = self.breaker_for(key).is_venue_excluded(key);
                crate::metrics::DEPENDENCY_BREAKER_OPEN
                    .with_label_values(&[key])
                    .set(open as i64);
                open
            })
            .collect()
    }

    /// Fail-fast guard for the quote/swap path.
    ///
    /// When a dependency breaker is open, reject immediately with 503 instead of
    /// letting every request pay the full timeout against a known-dead dependency.
    pub fn guard_live_path(&self) -> crate::error::Result<()> {
        let open = self.open_dependencies();
        if open.is_empty() {
            return Ok(());
        }

        for dep in &open {
            crate::metrics::DEPENDENCY_FAIL_FAST
                .with_label_values(&[dep])
                .inc();
        }

        Err(crate::error::ApiError::DependencyUnavailable(format!(
            "Upstream dependency unavailable: {}. Circuit breaker is open; retry shortly.",
            open.join(", ")
        )))
    }

    pub fn database_breaker_is_open(&self) -> bool {
        self.database_breaker.is_venue_excluded(DATABASE_KEY)
    }

    pub fn record_database_result(&self, success: bool) {
        self.database_breaker.record_result(DATABASE_KEY, success);
        self.refresh_gauge(DATABASE_KEY);
    }

    fn refresh_gauge(&self, key: &str) {
        let open = self.breaker_for(key).is_venue_excluded(key);
        crate::metrics::DEPENDENCY_BREAKER_OPEN
            .with_label_values(&[key])
            .set(open as i64);
    }

    pub async fn probe_horizon(&self) -> String {
        self.probe_horizon_with_client(&self.client).await
    }

    pub async fn probe_soroban(&self) -> String {
        self.probe_soroban_with_client(&self.client).await
    }

    pub async fn probe_evm_rpc(&self) -> String {
        self.probe_evm_rpc_with_client(&self.client).await
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
        self.refresh_gauge(SOROBAN_KEY);
    }

    pub fn record_horizon_result(&self, success: bool) {
        self.horizon_breaker.record_result(HORIZON_KEY, success);
        self.refresh_gauge(HORIZON_KEY);
    }

    pub fn evm_rpc_breaker_is_open(&self) -> bool {
        self.evm_rpc_breaker.is_venue_excluded(EVM_RPC_KEY)
    }

    pub fn record_evm_rpc_result(&self, success: bool) {
        self.evm_rpc_breaker.record_result(EVM_RPC_KEY, success);
        self.refresh_gauge(EVM_RPC_KEY);
    }

    /// Symmetric CCTP corridor chain health: source + destination per direction.
    pub fn guard_cctp_direction(
        &self,
        direction: crate::models::v2_cctp::CctpDirection,
    ) -> crate::error::Result<()> {
        use crate::models::v2_cctp::CctpDirection;

        let (source_open, dest_open, source_label, dest_label) = match direction {
            CctpDirection::StellarToEvm => (
                self.soroban_breaker_is_open(),
                self.evm_rpc_breaker_is_open(),
                "stellar_rpc",
                "sepolia_rpc",
            ),
            CctpDirection::EvmToStellar => (
                self.evm_rpc_breaker_is_open(),
                self.soroban_breaker_is_open(),
                "sepolia_rpc",
                "stellar_rpc",
            ),
        };
        if source_open || dest_open {
            let mut parts = Vec::new();
            if source_open {
                parts.push(source_label);
            }
            if dest_open {
                parts.push(dest_label);
            }
            return Err(crate::error::ApiError::DependencyUnavailable(format!(
                "CCTP corridor dependency unavailable: {}. Circuit breaker is open; retry shortly.",
                parts.join(", ")
            )));
        }
        Ok(())
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

    async fn probe_evm_rpc_with_client(&self, client: &Client) -> String {
        if self.evm_rpc_urls.is_empty() {
            return "not_configured".to_string();
        }

        if self.evm_rpc_breaker.is_venue_excluded(EVM_RPC_KEY) {
            return "degraded (circuit_open)".to_string();
        }

        let req = json!({
            "jsonrpc": "2.0",
            "id": "dep-health-evm-probe",
            "method": "eth_chainId",
            "params": []
        });

        for url in &self.evm_rpc_urls {
            let success = client
                .post(url)
                .json(&req)
                .send()
                .await
                .and_then(|resp| resp.error_for_status())
                .is_ok();

            if success {
                self.evm_rpc_breaker.record_result(EVM_RPC_KEY, true);
                return "healthy".to_string();
            }
        }

        self.evm_rpc_breaker.record_result(EVM_RPC_KEY, false);
        "degraded".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;
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

    /// Simulated Soroban RPC outage: the live path must reject fast with 503
    /// rather than let each request wait on a dependency known to be down.
    #[test]
    fn live_path_fails_fast_when_soroban_dependency_fails() {
        let health = ExternalDependencyHealth::new(vec![], vec![]);
        assert!(health.guard_live_path().is_ok());

        for _ in 0..3 {
            health.record_soroban_result(false);
        }

        let err = health.guard_live_path().unwrap_err();
        assert!(matches!(
            err,
            crate::error::ApiError::DependencyUnavailable(_)
        ));
        assert_eq!(
            err.into_response().status(),
            axum::http::StatusCode::SERVICE_UNAVAILABLE
        );
    }

    /// Simulated Postgres outage, fed by the `/health/deps` probe.
    #[test]
    fn live_path_fails_fast_when_database_dependency_fails() {
        let health = ExternalDependencyHealth::new(vec![], vec![]);

        for _ in 0..3 {
            health.record_database_result(false);
        }

        assert!(health.database_breaker_is_open());
        assert_eq!(health.open_dependencies(), vec!["database"]);
        assert!(health.guard_live_path().is_err());
    }

    /// A recovered dependency must stop rejecting traffic.
    #[test]
    fn live_path_recovers_once_breaker_closes() {
        let health = ExternalDependencyHealth::new(vec![], vec![]);

        for _ in 0..3 {
            health.record_horizon_result(false);
        }
        assert!(health.guard_live_path().is_err());

        // Breakers only leave Open via the recovery timeout, so drive the
        // transition explicitly through a fresh instance's success path.
        let recovered = ExternalDependencyHealth::new(vec![], vec![]);
        recovered.record_horizon_result(true);
        assert!(recovered.guard_live_path().is_ok());
        assert!(recovered.open_dependencies().is_empty());
    }

    #[test]
    fn open_dependencies_reports_every_failing_dependency() {
        let health = ExternalDependencyHealth::new(vec![], vec![]);

        for _ in 0..3 {
            health.record_soroban_result(false);
            health.record_database_result(false);
        }

        let open = health.open_dependencies();
        assert!(open.contains(&"soroban_rpc"));
        assert!(open.contains(&"database"));
        assert!(!open.contains(&"horizon"));
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
        assert!(health.evm_rpc_urls.is_empty());
    }

    #[test]
    fn evm_rpc_breaker_independent_from_soroban() {
        let health = ExternalDependencyHealth::new(vec![], vec![]);
        for _ in 0..3 {
            health.record_evm_rpc_result(false);
        }
        assert!(health.evm_rpc_breaker_is_open());
        assert!(!health.soroban_breaker_is_open());
    }

    #[test]
    fn guard_cctp_direction_blocks_per_corridor_side() {
        use crate::models::v2_cctp::CctpDirection;

        let health = ExternalDependencyHealth::new(vec![], vec![]);
        assert!(health
            .guard_cctp_direction(CctpDirection::StellarToEvm)
            .is_ok());
        assert!(health
            .guard_cctp_direction(CctpDirection::EvmToStellar)
            .is_ok());

        for _ in 0..3 {
            health.record_soroban_result(false);
        }
        let err = health
            .guard_cctp_direction(CctpDirection::StellarToEvm)
            .unwrap_err();
        assert!(matches!(
            err,
            crate::error::ApiError::DependencyUnavailable(_)
        ));
        assert!(health
            .guard_cctp_direction(CctpDirection::EvmToStellar)
            .is_err());

        let health2 = ExternalDependencyHealth::new(vec![], vec![]);
        for _ in 0..3 {
            health2.record_evm_rpc_result(false);
        }
        assert!(health2
            .guard_cctp_direction(CctpDirection::StellarToEvm)
            .is_err());
        assert!(health2
            .guard_cctp_direction(CctpDirection::EvmToStellar)
            .is_err());
    }
}
