//! Soroban simulation client for dry-run quote validation.
//!
//! Calls `simulateTransaction` on the Soroban RPC to validate route feasibility
//! against on-chain state before returning high-confidence quotes.
//!
//! # Design
//! - Simulation is **optional**: if the RPC URL is not configured, or if the
//!   simulation times out / fails, the caller receives a non-simulated quote
//!   with `simulated: false`.
//! - A token-bucket rate limiter protects RPC usage.
//! - Integration tests use a mock HTTP server (wiremock).

use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{debug, warn};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Result of a Soroban dry-run simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    /// Whether the simulation was actually executed (false = degraded path).
    pub simulated: bool,
    /// Whether the simulated transaction succeeded on-chain.
    pub success: bool,
    /// Estimated resource fee in stroops (present only when `simulated = true`).
    pub fee_stroops: Option<u64>,
    /// Human-readable failure reason when `success = false`.
    pub failure_reason: Option<String>,
}

impl SimulationResult {
    /// Degraded result used when simulation is skipped or fails.
    pub fn degraded(reason: impl Into<String>) -> Self {
        Self {
            simulated: false,
            success: false,
            fee_stroops: None,
            failure_reason: Some(reason.into()),
        }
    }
}

/// Configuration for the simulation client.
#[derive(Debug, Clone)]
pub struct SimulationConfig {
    /// Soroban RPC base URL (e.g. `https://soroban-testnet.stellar.org`).
    pub rpc_url: String,
    /// Hard timeout for a single simulation call.
    pub timeout: Duration,
    /// Maximum simulations per second (token-bucket rate limit).
    pub max_rps: u32,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            rpc_url: String::new(),
            timeout: Duration::from_secs(3),
            max_rps: 10,
        }
    }
}

// ---------------------------------------------------------------------------
// Rate limiter (simple token bucket)
// ---------------------------------------------------------------------------

struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    refill_per_ms: f64,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(max_rps: u32) -> Self {
        let max = max_rps as f64;
        Self {
            tokens: max,
            max_tokens: max,
            refill_per_ms: max / 1_000.0,
            last_refill: Instant::now(),
        }
    }

    /// Returns `true` if a token was consumed (request allowed).
    fn try_consume(&mut self) -> bool {
        let now = Instant::now();
        let elapsed_ms = now.duration_since(self.last_refill).as_millis() as f64;
        self.tokens = (self.tokens + elapsed_ms * self.refill_per_ms).min(self.max_tokens);
        self.last_refill = now;

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

// ---------------------------------------------------------------------------
// Simulation client
// ---------------------------------------------------------------------------

/// Soroban simulation client.
///
/// Wrap in `Arc` and share across handlers.
pub struct SorobanSimulator {
    config: SimulationConfig,
    http: reqwest::Client,
    bucket: Arc<Mutex<TokenBucket>>,
    /// Total simulations attempted.
    pub attempts: AtomicU64,
    /// Simulations that degraded (timeout / rate-limited / RPC error).
    pub degraded: AtomicU64,
}

impl SorobanSimulator {
    /// Create a new simulator.  Returns `None` when `rpc_url` is empty so
    /// callers can skip simulation entirely when it is not configured.
    pub fn new(config: SimulationConfig) -> Option<Arc<Self>> {
        if config.rpc_url.is_empty() {
            return None;
        }
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .ok()?;
        let bucket = Arc::new(Mutex::new(TokenBucket::new(config.max_rps)));
        Some(Arc::new(Self {
            config,
            http,
            bucket,
            attempts: AtomicU64::new(0),
            degraded: AtomicU64::new(0),
        }))
    }

    /// Simulate a transaction XDR.
    ///
    /// Always returns `Ok(SimulationResult)` — failures degrade gracefully.
    pub async fn simulate(&self, transaction_xdr: &str) -> SimulationResult {
        self.attempts.fetch_add(1, Ordering::Relaxed);

        // Rate-limit check
        {
            let mut bucket = self.bucket.lock().await;
            if !bucket.try_consume() {
                self.degraded.fetch_add(1, Ordering::Relaxed);
                warn!("Soroban simulation rate-limited; degrading to non-simulated quote");
                return SimulationResult::degraded("rate_limited");
            }
        }

        match tokio::time::timeout(
            self.config.timeout,
            self.call_simulate_transaction(transaction_xdr),
        )
        .await
        {
            Ok(Ok(result)) => result,
            Ok(Err(e)) => {
                self.degraded.fetch_add(1, Ordering::Relaxed);
                warn!("Soroban simulation RPC error: {}", e);
                SimulationResult::degraded(format!("rpc_error: {e}"))
            }
            Err(_elapsed) => {
                self.degraded.fetch_add(1, Ordering::Relaxed);
                warn!("Soroban simulation timed out");
                SimulationResult::degraded("timeout")
            }
        }
    }

    async fn call_simulate_transaction(
        &self,
        transaction_xdr: &str,
    ) -> Result<SimulationResult, String> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "stellarroute-sim",
            "method": "simulateTransaction",
            "params": { "transaction": transaction_xdr }
        });

        debug!(
            "Calling simulateTransaction on {}",
            self.config.rpc_url
        );

        let resp = self
            .http
            .post(&self.config.rpc_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }

        let payload: SimulateTransactionResponse =
            resp.json().await.map_err(|e| e.to_string())?;

        if let Some(err) = payload.error {
            return Ok(SimulationResult {
                simulated: true,
                success: false,
                fee_stroops: None,
                failure_reason: Some(format!("rpc_error {}: {}", err.code, err.message)),
            });
        }

        let result = payload.result.ok_or("missing result")?;

        // A successful simulation has no `error` field in the result object.
        let success = result.get("error").is_none();
        let failure_reason = if success {
            None
        } else {
            result
                .get("error")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        };

        let fee_stroops = result
            .get("minResourceFee")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<u64>().ok());

        Ok(SimulationResult {
            simulated: true,
            success,
            fee_stroops,
            failure_reason,
        })
    }
}

// ---------------------------------------------------------------------------
// JSON-RPC response shapes
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct SimulateTransactionResponse {
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{body_partial_json, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_simulator(url: &str) -> Arc<SorobanSimulator> {
        SorobanSimulator::new(SimulationConfig {
            rpc_url: url.to_string(),
            timeout: Duration::from_secs(5),
            max_rps: 100,
        })
        .expect("simulator must be created when url is non-empty")
    }

    #[test]
    fn returns_none_when_rpc_url_is_empty() {
        let sim = SorobanSimulator::new(SimulationConfig::default());
        assert!(sim.is_none());
    }

    #[tokio::test]
    async fn successful_simulation_returns_simulated_true() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(body_partial_json(json!({"method":"simulateTransaction"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0",
                "id": "stellarroute-sim",
                "result": {
                    "minResourceFee": "1234",
                    "transactionData": "AAAA"
                }
            })))
            .mount(&server)
            .await;

        let sim = make_simulator(&server.uri());
        let result = sim.simulate("AAAA_XDR").await;

        assert!(result.simulated);
        assert!(result.success);
        assert_eq!(result.fee_stroops, Some(1234));
        assert!(result.failure_reason.is_none());
    }

    #[tokio::test]
    async fn failed_simulation_returns_failure_reason() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0",
                "id": "stellarroute-sim",
                "result": {
                    "error": "HostError: insufficient balance"
                }
            })))
            .mount(&server)
            .await;

        let sim = make_simulator(&server.uri());
        let result = sim.simulate("AAAA_XDR").await;

        assert!(result.simulated);
        assert!(!result.success);
        assert!(result.failure_reason.is_some());
    }

    #[tokio::test]
    async fn rpc_error_degrades_gracefully() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let sim = make_simulator(&server.uri());
        let result = sim.simulate("AAAA_XDR").await;

        assert!(!result.simulated);
        assert!(!result.success);
        assert_eq!(sim.degraded.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn timeout_degrades_gracefully() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(Duration::from_secs(10))
                    .set_body_json(json!({"jsonrpc":"2.0","id":"x","result":{}})),
            )
            .mount(&server)
            .await;

        let sim = SorobanSimulator::new(SimulationConfig {
            rpc_url: server.uri(),
            timeout: Duration::from_millis(50),
            max_rps: 100,
        })
        .unwrap();

        let result = sim.simulate("AAAA_XDR").await;
        assert!(!result.simulated);
        assert_eq!(
            result.failure_reason.as_deref(),
            Some("timeout")
        );
    }

    #[tokio::test]
    async fn rate_limit_degrades_gracefully() {
        let server = MockServer::start().await;
        // No mock needed — rate limiter fires before HTTP call

        let sim = SorobanSimulator::new(SimulationConfig {
            rpc_url: server.uri(),
            timeout: Duration::from_secs(5),
            max_rps: 1, // 1 token
        })
        .unwrap();

        // First call consumes the token
        {
            let mut bucket = sim.bucket.lock().await;
            bucket.try_consume();
        }

        // Second call should be rate-limited
        let result = sim.simulate("AAAA_XDR").await;
        assert!(!result.simulated);
        assert_eq!(result.failure_reason.as_deref(), Some("rate_limited"));
    }

    #[tokio::test]
    async fn json_rpc_error_in_result_degrades_gracefully() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0",
                "id": "stellarroute-sim",
                "error": { "code": -32602, "message": "invalid params" }
            })))
            .mount(&server)
            .await;

        let sim = make_simulator(&server.uri());
        let result = sim.simulate("AAAA_XDR").await;

        assert!(result.simulated);
        assert!(!result.success);
        assert!(result
            .failure_reason
            .as_deref()
            .unwrap_or("")
            .contains("rpc_error"));
    }
}
