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
        PairsResponse, QuoteRequest, QuoteResponse,
    },
};

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

    /// `POST /api/v1/batch/quote` — fetch multiple price quotes in a single request.
    ///
    /// Returns [`SdkError::Api`] with [`ApiErrorCode::ValidationError`] if any
    /// request item is malformed or the batch is too large.
    pub async fn batch_quote(&self, request: BatchQuoteRequest) -> Result<BatchQuoteResponse> {
        let url = self.url("api/v1/batch/quote")?;
        self.execute_with_retry(|| self.http.post(url.clone()).json(&request))
            .await
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
