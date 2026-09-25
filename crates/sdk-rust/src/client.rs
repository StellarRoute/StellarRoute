//! StellarRoute API client.
//!
//! # Quick start
//!
//! ```no_run
//! use stellarroute_sdk::{ClientBuilder, QuoteRequest, QuoteType};
//!
//! #[tokio::main]
//! async fn main() -> stellarroute_sdk::Result<()> {
//!     let client = ClientBuilder::new("http://localhost:3000").build()?;
//!
//!     let health = client.health().await?;
//!     println!("status: {}", health.status);
//!
//!     let quote = client.quote(QuoteRequest::sell("native", "USDC")).await?;
//!     println!("price: {}", quote.price);
//!
//!     Ok(())
//! }
//! ```

use std::time::Duration;

use reqwest::{header, Url};

use crate::{
    error::{ApiErrorCode, RateLimitInfo, Result, SdkError},
    types::{
        BatchQuoteRequest, BatchQuoteResponse, ErrorResponse, HealthResponse, OrderbookResponse,
        PairsResponse, PriceHistoryResponse, QuoteRequest, QuoteResponse, RoutesRequest,
        RoutesResponse, SimulateRouteRequest, SimulateRouteResponse, SwapPrepareRequest,
        SwapPrepareResponse, SwapSubmitRequest, SwapSubmitResponse,
    },
};

// ── API response envelope ─────────────────────────────────────────────────────

/// Private deserializer for the `{ v, timestamp, request_id, data }` envelope
/// returned by endpoints that wrap their response (e.g. `POST /api/v1/simulate/route`).
///
/// Only `data` is extracted; the envelope metadata is discarded.
#[derive(serde::Deserialize)]
struct ApiEnvelope<T> {
    pub data: T,
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Fluent builder for [`StellarRouteClient`].
///
/// ```no_run
/// use stellarroute_sdk::ClientBuilder;
/// use std::time::Duration;
///
/// let client = ClientBuilder::new("https://api.stellarroute.io")
///     .timeout(Duration::from_secs(10))
///     .user_agent("my-app/1.0")
///     .max_retries(3)
///     .build()
///     .unwrap();
/// ```
pub struct ClientBuilder {
    api_url: String,
    timeout: Duration,
    user_agent: String,
    max_retries: u32,
    base_backoff: Duration,
}

impl ClientBuilder {
    /// Create a new builder targeting `api_url`.
    pub fn new(api_url: impl Into<String>) -> Self {
        Self {
            api_url: api_url.into(),
            timeout: Duration::from_secs(30),
            user_agent: format!("stellarroute-sdk-rust/{}", env!("CARGO_PKG_VERSION")),
            max_retries: 0,
            base_backoff: Duration::from_millis(500),
        }
    }

    /// Override the request timeout (default: 30 s).
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Override the `User-Agent` header.
    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = ua.into();
        self
    }

    /// Maximum number of automatic retries on 429 / 5xx / network errors (default: 0).
    ///
    /// Retries use exponential backoff starting at `base_backoff` (default 500 ms),
    /// doubling each attempt. When the server returns a `Retry-After` header, that
    /// duration is used instead of the computed backoff.
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Base backoff duration for retries (default: 500 ms).
    ///
    /// The actual delay is `base_backoff * 2^attempt`, capped at 30 seconds.
    pub fn base_backoff(mut self, backoff: Duration) -> Self {
        self.base_backoff = backoff;
        self
    }

    /// Build the client. Returns [`SdkError::InvalidConfig`] if the URL is malformed.
    pub fn build(self) -> Result<StellarRouteClient> {
        let mut base_url = Url::parse(&self.api_url).map_err(|e| {
            SdkError::InvalidConfig(format!("Invalid API URL '{}': {e}", self.api_url))
        })?;

        // Ensure the base URL always ends with `/` so `Url::join` works correctly.
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }

        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_str(&self.user_agent)
                .map_err(|e| SdkError::InvalidConfig(format!("Invalid User-Agent header: {e}")))?,
        );

        let http = reqwest::Client::builder()
            .timeout(self.timeout)
            .default_headers(headers)
            .build()
            .map_err(|e| SdkError::InvalidConfig(format!("Failed to build HTTP client: {e}")))?;

        Ok(StellarRouteClient {
            base_url,
            http,
            max_retries: self.max_retries,
            base_backoff: self.base_backoff,
        })
    }
}

// ── Client ────────────────────────────────────────────────────────────────────

/// Async HTTP client for the StellarRoute REST API.
///
/// Construct via [`ClientBuilder`] or the convenience [`StellarRouteClient::new`].
pub type Client = StellarRouteClient;

#[derive(Debug)]
pub struct StellarRouteClient {
    base_url: Url,
    http: reqwest::Client,
    max_retries: u32,
    base_backoff: Duration,
}

impl StellarRouteClient {
    /// Convenience constructor with default settings.
    ///
    /// Equivalent to `ClientBuilder::new(api_url).build()`.
    pub fn new(api_url: &str) -> Result<Self> {
        ClientBuilder::new(api_url).build()
    }

    // ── Public API methods ────────────────────────────────────────────────────

    /// `GET /health` — probe service and dependency health.
    ///
    /// Returns [`SdkError::Api`] with status 503 when any dependency is down.
    pub async fn health(&self) -> Result<HealthResponse> {
        self.get("health").await
    }

    /// `GET /api/v1/pairs` — list active trading pairs.
    pub async fn pairs(&self) -> Result<PairsResponse> {
        self.get("api/v1/pairs").await
    }

    /// `GET /api/v1/orderbook/{base}/{quote}` — fetch orderbook snapshot.
    ///
    /// Returns [`SdkError::Api`] with [`ApiErrorCode::NotFound`] when the pair
    /// has no active offers.
    pub async fn orderbook(&self, base: &str, quote: &str) -> Result<OrderbookResponse> {
        self.get(&format!("api/v1/orderbook/{base}/{quote}")).await
    }

    /// `GET /api/v1/price-history/{base}/{quote}` — fetch 24-hour hourly price series.
    ///
    /// Returns [`SdkError::Api`] with [`ApiErrorCode::ValidationError`] for HTTP 400,
    /// [`ApiErrorCode::NoRoute`] for HTTP 404 (pair not found or no data), and
    /// [`SdkError::RateLimited`] after exhausted retries on HTTP 429.
    pub async fn price_history(&self, base: &str, quote: &str) -> Result<PriceHistoryResponse> {
        self.get(&format!("api/v1/price-history/{base}/{quote}"))
            .await
    }

    /// `GET /api/v1/quote/{base}/{quote}` — get best price quote.
    ///
    /// Returns [`SdkError::Api`] with [`ApiErrorCode::NotFound`] when no route
    /// exists for the pair, or [`ApiErrorCode::ValidationError`] for bad params.
    pub async fn quote(&self, request: QuoteRequest<'_>) -> Result<QuoteResponse> {
        let path = format!("api/v1/quote/{}/{}", request.base, request.quote);
        let base_url = self.url(&path)?;
        let amount = request.amount.map(String::from);
        let quote_type = request.quote_type;

        self.execute_with_retry(|| {
            let mut req = self.http.get(base_url.clone());
            if let Some(ref amount) = amount {
                req = req.query(&[("amount", amount.as_str())]);
            }
            req.query(&[("quote_type", quote_type.as_str())])
        })
        .await
    }

    /// Fetch available routes for a currency pair.
    ///
    /// Calls `GET /api/v1/routes/{base}/{quote}` and returns ranked route
    /// candidates for the requested pair.
    ///
    /// Returns [`ApiError`] with code [`ApiErrorCode::NoRoute`] when no route
    /// exists for the requested pair and amount.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use stellarroute_sdk::{Client, RoutesRequest};
    ///
    /// # async fn example() -> stellarroute_sdk::Result<()> {
    /// let client = Client::new("https://api.stellarroute.io")?;
    /// let response = client.routes(RoutesRequest {
    ///     base: "native",
    ///     quote: "USDC",
    ///     amount: 1_000_000,
    ///     slippage_bps: Some(50),
    ///     quote_type: None,
    /// }).await?;
    /// println!("Best route: {:?}", response.routes.first());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn routes(&self, request: RoutesRequest<'_>) -> Result<RoutesResponse> {
        let path = format!(
            "api/v1/routes/{}/{}",
            encode_path_segment(request.base),
            encode_path_segment(request.quote)
        );
        let base_url = self.url(&path)?;
        let amount = request.amount.to_string();
        let slippage_bps = request.slippage_bps.map(|value| value.to_string());
        let quote_type = request.quote_type.map(|value| value.to_string());

        self.execute_with_retry(|| {
            let mut req = self
                .http
                .get(base_url.clone())
                .query(&[("amount", amount.as_str())]);
            if let Some(ref slippage_bps) = slippage_bps {
                req = req.query(&[("slippage_bps", slippage_bps.as_str())]);
            }
            if let Some(ref quote_type) = quote_type {
                req = req.query(&[("quote_type", quote_type.as_str())]);
            }
            req
        })
        .await
    }

    /// `POST /api/v1/batch/quote` — fetch multiple price quotes in a single request.
    ///
    /// Returns [`SdkError::Api`] with [`ApiErrorCode::ValidationError`] if any
    /// request item is malformed or the batch is too large.
    pub async fn batch_quote(&self, request: BatchQuoteRequest) -> Result<BatchQuoteResponse> {
        let url = self.url("api/v1/batch/quote")?;
        self.execute_with_retry(|| self.http.post(url.clone()).json(&request))
            .await
    }

    /// `POST /api/v1/swap/prepare` — build an unsigned swap transaction.
    ///
    /// The server validates the route and returns a base64 XDR envelope. Sign it
    /// with your own keys (the SDK never sees them), then hand it to
    /// [`submit_swap`](Self::submit_swap).
    ///
    /// Returns [`SdkError::Api`] with [`ApiErrorCode::NoRoute`] when the path is no
    /// longer executable, or [`ApiErrorCode::ValidationError`] for a malformed request.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use stellarroute_sdk::{Client, RoutesRequest, SwapPrepareRequest};
    ///
    /// # async fn example() -> stellarroute_sdk::Result<()> {
    /// let client = Client::new("https://api.stellarroute.io")?;
    ///
    /// let routes = client.routes(RoutesRequest {
    ///     base: "native",
    ///     quote: "USDC",
    ///     amount: 1_000_000,
    ///     slippage_bps: Some(50),
    ///     quote_type: None,
    /// }).await?;
    ///
    /// let best = routes.routes.first().expect("at least one route");
    /// let prepared = client
    ///     .prepare_swap(SwapPrepareRequest::from_route(best, "100", "GABC...").slippage_bps(50))
    ///     .await?;
    ///
    /// println!("sign this envelope: {}", prepared.xdr_envelope);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn prepare_swap(&self, request: SwapPrepareRequest) -> Result<SwapPrepareResponse> {
        let url = self.url("api/v1/swap/prepare")?;
        self.execute_with_retry(|| self.http.post(url.clone()).json(&request))
            .await
    }

    /// `POST /api/v1/swap/submit` — broadcast a signed swap transaction.
    ///
    /// Takes the signed envelope produced from
    /// [`prepare_swap`](Self::prepare_swap) and returns the transaction hash plus
    /// submission status.
    ///
    /// Returns [`SdkError::Api`] with [`ApiErrorCode::ValidationError`] when the
    /// envelope is malformed, unsigned, or has expired.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use stellarroute_sdk::{Client, SwapSubmitRequest};
    ///
    /// # async fn example(signed_xdr: String) -> stellarroute_sdk::Result<()> {
    /// let client = Client::new("https://api.stellarroute.io")?;
    ///
    /// let receipt = client.submit_swap(SwapSubmitRequest::new(signed_xdr)).await?;
    /// if receipt.is_success() {
    ///     println!("swap confirmed in tx {}", receipt.tx_hash);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn submit_swap(&self, request: SwapSubmitRequest) -> Result<SwapSubmitResponse> {
        let url = self.url("api/v1/swap/submit")?;
        self.execute_with_retry(|| self.http.post(url.clone()).json(&request))
            .await
    }

    /// `POST /api/v1/simulate/route` — dry-run a pre-selected multi-hop route.
    ///
    /// Performs a side-effect-free simulation of the supplied route, returning
    /// a full quote with diagnostics and a swap-path breakdown.  No wallet
    /// signing or on-chain execution occurs.
    ///
    /// The server response is wrapped in an `ApiResponse` envelope
    /// (`{ v, timestamp, request_id, data }`); the SDK unwraps the envelope
    /// and returns only the `data` field as [`SimulateRouteResponse`].
    ///
    /// Returns [`SdkError::Api`] with [`ApiErrorCode::ValidationError`] on HTTP
    /// 400, and [`ApiErrorCode::NoRoute`] on HTTP 404.  Retry-and-backoff
    /// behaviour on 429 and 5xx matches all other SDK methods.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use stellarroute_sdk::{Client, DryRunHop, SimulateRouteRequest};
    ///
    /// # async fn example() -> stellarroute_sdk::Result<()> {
    /// let client = Client::new("https://api.stellarroute.io")?;
    ///
    /// let response = client.simulate_route(SimulateRouteRequest {
    ///     hops: vec![DryRunHop {
    ///         from_asset: "native".into(),
    ///         to_asset: "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".into(),
    ///         source: "sdex".into(),
    ///         fee_bps: Some(30),
    ///         price: Some("0.12".into()),
    ///         venue_ref: Some("sdex".into()),
    ///     }],
    ///     amount: "100.0".into(),
    ///     slippage_bps: Some(50),
    ///     slippage_bps_overrides: vec![],
    /// }).await?;
    ///
    /// println!("simulated price: {}", response.quote.price);
    /// println!("estimated output: {}", response.swap_path.estimated_output);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn simulate_route(
        &self,
        request: SimulateRouteRequest,
    ) -> Result<SimulateRouteResponse> {
        let url = self.url("api/v1/simulate/route")?;
        let envelope: ApiEnvelope<SimulateRouteResponse> = self
            .execute_with_retry(|| self.http.post(url.clone()).json(&request))
            .await?;
        Ok(envelope.data)
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn url(&self, path: &str) -> Result<Url> {
        self.base_url
            .join(path)
            .map_err(|e| SdkError::InvalidConfig(format!("Invalid request path '{path}': {e}")))
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = self.url(path)?;
        self.execute_with_retry(|| self.http.get(url.clone())).await
    }

    async fn execute_with_retry<T, F>(&self, build_request: F) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
        F: Fn() -> reqwest::RequestBuilder,
    {
        let mut attempts = 0u32;
        loop {
            let response = build_request()
                .send()
                .await
                .map_err(|e| SdkError::Http(e.to_string()))?;

            let status = response.status();

            // Handle rate limiting before reading the body.
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                let info = extract_rate_limit_info(response.headers());

                if attempts < self.max_retries {
                    let delay = retry_delay(&info, self.base_backoff, attempts);
                    tokio::time::sleep(delay).await;
                    attempts += 1;
                    continue;
                }
                return Err(SdkError::RateLimited { info });
            }

            let body = response
                .text()
                .await
                .map_err(|e| SdkError::Http(format!("Failed to read response body: {e}")))?;

            if !status.is_success() {
                // SURFACE: ApiErrorCode::NoRoute — documented in OpenAPI as error_code "NO_ROUTE" on empty candidate set
                let parsed_body = serde_json::from_str::<serde_json::Value>(&body).ok();
                let body_error_code = parsed_body.as_ref().and_then(|value| {
                    value
                        .get("error_code")
                        .or_else(|| value.get("error"))
                        .and_then(serde_json::Value::as_str)
                });

                // A body that names its own error code wins; a bare 404 with no
                // usable body is treated as "no route" for the routing endpoints.
                let is_no_route = match body_error_code {
                    Some(code) => code.eq_ignore_ascii_case("no_route"),
                    None => status == reqwest::StatusCode::NOT_FOUND,
                };

                if is_no_route {
                    let message = serde_json::from_str::<ErrorResponse>(&body)
                        .map(|err| err.message)
                        .unwrap_or_else(|_| "No route found".to_string());
                    return Err(SdkError::Api {
                        code: ApiErrorCode::NoRoute,
                        message,
                        status: status.as_u16(),
                    });
                }

                // Retry on 5xx errors.
                if status.is_server_error() && attempts < self.max_retries {
                    let delay = self.base_backoff.saturating_mul(1u32.pow(attempts));
                    let delay = delay.min(Duration::from_secs(30));
                    tokio::time::sleep(delay).await;
                    attempts += 1;
                    continue;
                }

                let (code, message) = match serde_json::from_str::<ErrorResponse>(&body) {
                    Ok(err) => (
                        err.error.parse::<ApiErrorCode>().expect("infallible parse"),
                        err.message,
                    ),
                    Err(_) => (
                        ApiErrorCode::InternalError,
                        format!("API request failed with status {status}"),
                    ),
                };
                return Err(SdkError::Api {
                    code,
                    message,
                    status: status.as_u16(),
                });
            }

            return serde_json::from_str(&body).map_err(Into::into);
        }
    }
}

// ── Rate-limit header extraction ──────────────────────────────────────────────

fn encode_path_segment(segment: &str) -> String {
    segment
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
                (byte as char).to_string()
            } else {
                format!("%{:02X}", byte)
            }
        })
        .collect()
}

fn extract_rate_limit_info(headers: &reqwest::header::HeaderMap) -> RateLimitInfo {
    let parse_u32 = |name: &str| -> Option<u32> {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
    };
    let parse_u64 = |name: &str| -> Option<u64> {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
    };

    RateLimitInfo {
        limit: parse_u32("x-ratelimit-limit"),
        remaining: parse_u32("x-ratelimit-remaining"),
        reset: parse_u64("x-ratelimit-reset"),
        retry_after: parse_u64("retry-after"),
    }
}

/// Compute the delay before a retry attempt.
///
/// Honors the `Retry-After` header from rate-limit responses, falling back to
/// exponential backoff: `base_backoff * 2^attempt`, capped at 30 seconds.
fn retry_delay(info: &RateLimitInfo, base_backoff: Duration, attempt: u32) -> Duration {
    if let Some(seconds) = info.retry_after {
        return Duration::from_secs(seconds);
    }
    let delay = base_backoff.saturating_mul(1u32.pow(attempt));
    delay.min(Duration::from_secs(30))
}
