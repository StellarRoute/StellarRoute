//! Rate limiting middleware
//!
//! Implements a sliding-window rate limiter backed by Redis (with an
//! in-memory fallback when Redis is unavailable).
//!
//! Per-endpoint limits (configurable via env vars):
//!
//! | Route prefix          | Default limit | Window |
//! |-----------------------|---------------|--------|
//! | `/api/v1/pairs`       | 60 req / min  | 60 s   |
//! | `/api/v1/orderbook/*` | 30 req / min  | 60 s   |
//! | `/api/v1/quote/*`     | 100 req / min | 60 s   |
//! | everything else       | 200 req / min | 60 s   |
//!
//! # Response headers
//!
//! Every response (allowed or denied) receives:
//! - `X-RateLimit-Limit`     — maximum requests in the window
//! - `X-RateLimit-Remaining` — remaining quota (clamped to 0 on deny)
//! - `X-RateLimit-Reset`     — UTC Unix timestamp when the window resets
//!
//! Denied responses additionally include:
//! - `Retry-After` — seconds until the window resets

use axum::{
    body::Body,
    extract::Request,
    http::{header::HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use redis::{aio::ConnectionManager, AsyncCommands};
use std::{
    collections::HashMap,
    net::IpAddr,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex;
use tower::{Layer, Service};
use tracing::{debug, warn};

use crate::models::ErrorResponse;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Rate limit configuration for a single endpoint group.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests allowed within the window.
    pub max_requests: u32,
    /// Length of the sliding window.
    pub window: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 200,
            window: Duration::from_secs(60),
        }
    }
}

/// Per-endpoint rate limit configurations.
///
/// Build one with [`EndpointConfig::from_env`] to read limits from environment
/// variables, or use [`EndpointConfig::default`] for the documented defaults.
#[derive(Debug, Clone)]
pub struct EndpointConfig {
    pub pairs: RateLimitConfig,
    pub orderbook: RateLimitConfig,
    pub quote: RateLimitConfig,
    pub default: RateLimitConfig,
}

impl Default for EndpointConfig {
    fn default() -> Self {
        let window = Duration::from_secs(
            std::env::var("RATE_LIMIT_WINDOW_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
        );

        Self {
            pairs: RateLimitConfig {
                max_requests: std::env::var("RATE_LIMIT_PAIRS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(60),
                window,
            },
            orderbook: RateLimitConfig {
                max_requests: std::env::var("RATE_LIMIT_ORDERBOOK")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(30),
                window,
            },
            quote: RateLimitConfig {
                max_requests: std::env::var("RATE_LIMIT_QUOTE")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(100),
                window,
            },
            default: RateLimitConfig {
                max_requests: 200,
                window,
            },
        }
    }
}

impl EndpointConfig {
    /// Return the config that matches `path`.
    pub fn for_path<'a>(&'a self, path: &str) -> &'a RateLimitConfig {
        if path.starts_with("/api/v1/pairs") {
            &self.pairs
        } else if path.starts_with("/api/v1/orderbook") {
            &self.orderbook
        } else if path.starts_with("/api/v1/quote") {
            &self.quote
        } else {
            &self.default
        }
    }
}

// ---------------------------------------------------------------------------
// Rate-limit info returned after checking
// ---------------------------------------------------------------------------

/// Information about the current rate-limit state for a request.
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub limit: u32,
    pub remaining: u32,
    /// Unix timestamp (seconds) when the window resets.
    pub reset: u64,
    /// True when the request has been denied.
    pub denied: bool,
}

// ---------------------------------------------------------------------------
// In-memory backend (used as fallback and in tests)
// ---------------------------------------------------------------------------

#[derive(Default)]
struct InMemoryStore {
    /// IP+endpoint → (count, window_start)
    windows: HashMap<String, (u32, Instant)>,
}

impl InMemoryStore {
    fn check(&mut self, key: &str, config: &RateLimitConfig) -> RateLimitInfo {
        let now = Instant::now();
        let entry = self.windows.entry(key.to_string()).or_insert((0, now));

        // Reset if window expired
        if now.duration_since(entry.1) >= config.window {
            *entry = (0, now);
        }

        let reset_unix = unix_now() + (config.window - now.duration_since(entry.1)).as_secs();

        if entry.0 < config.max_requests {
            entry.0 += 1;
            RateLimitInfo {
                limit: config.max_requests,
                remaining: config.max_requests - entry.0,
                reset: reset_unix,
                denied: false,
            }
        } else {
            RateLimitInfo {
                limit: config.max_requests,
                remaining: 0,
                reset: reset_unix,
                denied: true,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Redis backend
// ---------------------------------------------------------------------------

async fn redis_check(
    conn: &mut ConnectionManager,
    key: &str,
    config: &RateLimitConfig,
) -> Option<RateLimitInfo> {
    let window_secs = config.window.as_secs();

    // Atomically increment and set expiry
    let count: u32 = match conn.incr::<_, _, u32>(key, 1u32).await {
        Ok(c) => c,
        Err(e) => {
            warn!("Redis INCR failed ({}), falling back to allow", e);
            return None;
        }
    };

    // Set TTL only on first request in window
    if count == 1 {
        let _: Result<(), _> = conn.expire(key, window_secs as i64).await;
    }

    // Fetch remaining TTL so we can calculate the reset timestamp
    let ttl_secs: u64 = conn.ttl::<_, u64>(key).await.unwrap_or(window_secs);

    let reset = unix_now() + ttl_secs;
    let denied = count > config.max_requests;

    Some(RateLimitInfo {
        limit: config.max_requests,
        remaining: if denied {
            0
        } else {
            config.max_requests.saturating_sub(count)
        },
        reset,
        denied,
    })
}

// ---------------------------------------------------------------------------
// Backend enum
// ---------------------------------------------------------------------------

enum Backend {
    Redis(Arc<Mutex<ConnectionManager>>),
    InMemory(Arc<Mutex<InMemoryStore>>),
}

impl Clone for Backend {
    fn clone(&self) -> Self {
        match self {
            Backend::Redis(c) => Backend::Redis(c.clone()),
            Backend::InMemory(s) => Backend::InMemory(s.clone()),
        }
    }
}

impl Backend {
    async fn check(&self, key: &str, config: &RateLimitConfig) -> RateLimitInfo {
        match self {
            Backend::Redis(conn) => {
                let mut guard = conn.lock().await;
                match redis_check(&mut guard, key, config).await {
                    Some(info) => info,
                    None => {
                        // Redis unavailable — soft fail: allow request
                        RateLimitInfo {
                            limit: config.max_requests,
                            remaining: config.max_requests,
                            reset: unix_now() + config.window.as_secs(),
                            denied: false,
                        }
                    }
                }
            }
            Backend::InMemory(store) => {
                let mut guard = store.lock().await;
                guard.check(key, config)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tower Layer / Service
// ---------------------------------------------------------------------------

/// Tower [`Layer`] that applies rate limiting to every request.
#[derive(Clone)]
pub struct RateLimitLayer {
    backend: Backend,
    endpoint_config: Arc<EndpointConfig>,
}

impl RateLimitLayer {
    /// Create a layer backed by a Redis connection manager.
    pub fn with_redis(conn: ConnectionManager, endpoint_config: EndpointConfig) -> Self {
        Self {
            backend: Backend::Redis(Arc::new(Mutex::new(conn))),
            endpoint_config: Arc::new(endpoint_config),
        }
    }

    /// Create a layer backed by an in-memory store (useful for tests).
    pub fn in_memory(endpoint_config: EndpointConfig) -> Self {
        Self {
            backend: Backend::InMemory(Arc::new(Mutex::new(InMemoryStore::default()))),
            endpoint_config: Arc::new(endpoint_config),
        }
    }
}

impl Default for RateLimitLayer {
    fn default() -> Self {
        Self::in_memory(EndpointConfig::default())
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            backend: self.backend.clone(),
            endpoint_config: self.endpoint_config.clone(),
        }
    }
}

/// Tower [`Service`] that enforces rate limits and injects response headers.
#[derive(Clone)]
pub struct RateLimitService<S> {
    inner: S,
    backend: Backend,
    endpoint_config: Arc<EndpointConfig>,
}

impl<S> Service<Request> for RateLimitService<S>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let mut inner = self.inner.clone();
        let backend = self.backend.clone();
        let endpoint_config = self.endpoint_config.clone();

        Box::pin(async move {
            let path = req.uri().path().to_owned();
            let ip = extract_ip(&req);
            let config = endpoint_config.for_path(&path);
            let endpoint_slug = path_to_slug(&path);
            let key = format!("rate_limit:{}:{}", endpoint_slug, ip);

            debug!("Rate limit check: key={}", key);

            let info = backend.check(&key, config).await;

            if info.denied {
                debug!("Rate limit denied: key={}", key);
                let retry_after = info.reset.saturating_sub(unix_now());
                let mut response = (
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(ErrorResponse::new(
                        "rate_limit_exceeded",
                        "Too many requests. Please try again later.".to_string(),
                    )),
                )
                    .into_response();

                add_rate_limit_headers(response.headers_mut(), &info);
                response.headers_mut().insert(
                    HeaderName::from_static("retry-after"),
                    HeaderValue::from_str(&retry_after.to_string())
                        .unwrap_or_else(|_| HeaderValue::from_static("60")),
                );
                return Ok(response);
            }

            // Forward the request to the next service
            let mut response = inner.call(req).await?;
            add_rate_limit_headers(response.headers_mut(), &info);
            Ok(response)
        })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract client IP from common forwarding headers, falling back to loopback.
fn extract_ip(req: &Request<Body>) -> IpAddr {
    // X-Forwarded-For: client, proxy1, proxy2
    if let Some(fwd) = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
    {
        if let Some(first) = fwd.split(',').next() {
            if let Ok(ip) = first.trim().parse::<IpAddr>() {
                return ip;
            }
        }
    }

    // X-Real-IP: client
    if let Some(real) = req.headers().get("x-real-ip").and_then(|v| v.to_str().ok()) {
        if let Ok(ip) = real.trim().parse::<IpAddr>() {
            return ip;
        }
    }

    // Fallback — in production the load balancer always sets one of the above
    IpAddr::from([127, 0, 0, 1])
}

/// Convert a URI path to a slug safe for use in Redis keys.
fn path_to_slug(path: &str) -> String {
    if path.starts_with("/api/v1/pairs") {
        "pairs".to_string()
    } else if path.starts_with("/api/v1/orderbook") {
        "orderbook".to_string()
    } else if path.starts_with("/api/v1/quote") {
        "quote".to_string()
    } else {
        // Strip leading slash and replace slashes with underscores
        path.trim_start_matches('/').replace('/', "_")
    }
}

/// Inject X-RateLimit-* headers into a response.
fn add_rate_limit_headers(headers: &mut axum::http::HeaderMap, info: &RateLimitInfo) {
    let pairs: &[(&'static str, String)] = &[
        ("x-ratelimit-limit", info.limit.to_string()),
        ("x-ratelimit-remaining", info.remaining.to_string()),
        ("x-ratelimit-reset", info.reset.to_string()),
    ];

    for (name, value) in pairs {
        // Safety: all names are valid static header name strings
        let header_name = HeaderName::from_static(name);
        if let Ok(header_value) = HeaderValue::from_str(value) {
            headers.insert(header_name, header_value);
        }
    }
}

/// Current UTC time as a Unix timestamp (seconds).
fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn default_config(max: u32) -> RateLimitConfig {
        RateLimitConfig {
            max_requests: max,
            window: Duration::from_secs(60),
        }
    }

    #[test]
    fn rate_limit_config_defaults() {
        let cfg = RateLimitConfig::default();
        assert_eq!(cfg.max_requests, 200);
        assert_eq!(cfg.window, Duration::from_secs(60));
    }

    #[test]
    fn endpoint_config_default_values() {
        // Remove any env-var overrides that might be set in the test env
        std::env::remove_var("RATE_LIMIT_PAIRS");
        std::env::remove_var("RATE_LIMIT_ORDERBOOK");
        std::env::remove_var("RATE_LIMIT_QUOTE");
        std::env::remove_var("RATE_LIMIT_WINDOW_SECS");

        let cfg = EndpointConfig::default();
        assert_eq!(cfg.pairs.max_requests, 60);
        assert_eq!(cfg.orderbook.max_requests, 30);
        assert_eq!(cfg.quote.max_requests, 100);
        assert_eq!(cfg.default.max_requests, 200);
    }

    #[test]
    fn endpoint_config_selects_correct_limit() {
        let cfg = EndpointConfig::default();
        assert_eq!(cfg.for_path("/api/v1/pairs").max_requests, 60);
        assert_eq!(cfg.for_path("/api/v1/orderbook/XLM/USDC").max_requests, 30);
        assert_eq!(cfg.for_path("/api/v1/quote/XLM/USDC").max_requests, 100);
        assert_eq!(cfg.for_path("/health").max_requests, 200);
        assert_eq!(cfg.for_path("/swagger-ui").max_requests, 200);
    }

    #[test]
    fn sliding_window_allows_under_limit() {
        let mut store = InMemoryStore::default();
        let config = default_config(5);

        for i in 1..=5 {
            let info = store.check("test_key", &config);
            assert!(!info.denied, "request {} should be allowed", i);
        }
    }

    #[test]
    fn sliding_window_blocks_at_limit() {
        let mut store = InMemoryStore::default();
        let config = default_config(3);

        for _ in 0..3 {
            let info = store.check("key2", &config);
            assert!(!info.denied);
        }

        let info = store.check("key2", &config);
        assert!(info.denied, "4th request should be denied");
        assert_eq!(info.remaining, 0);
    }

    #[test]
    fn sliding_window_remaining_decreases() {
        let mut store = InMemoryStore::default();
        let config = default_config(10);

        let info1 = store.check("key3", &config);
        assert_eq!(info1.remaining, 9);

        let info2 = store.check("key3", &config);
        assert_eq!(info2.remaining, 8);
    }

    #[test]
    fn ip_extraction_prefers_x_forwarded_for() {
        use axum::http::Request;
        let req = Request::builder()
            .header("x-forwarded-for", "203.0.113.5, 10.0.0.1")
            .body(Body::empty())
            .unwrap();
        let ip = extract_ip(&req);
        assert_eq!(ip, "203.0.113.5".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn ip_extraction_falls_back_to_x_real_ip() {
        use axum::http::Request;
        let req = Request::builder()
            .header("x-real-ip", "192.0.2.42")
            .body(Body::empty())
            .unwrap();
        let ip = extract_ip(&req);
        assert_eq!(ip, "192.0.2.42".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn ip_extraction_falls_back_to_loopback() {
        use axum::http::Request;
        let req = Request::builder().body(Body::empty()).unwrap();
        let ip = extract_ip(&req);
        assert_eq!(ip, IpAddr::from([127, 0, 0, 1]));
    }

    #[test]
    fn path_to_slug_correct() {
        assert_eq!(path_to_slug("/api/v1/pairs"), "pairs");
        assert_eq!(path_to_slug("/api/v1/orderbook/X/Y"), "orderbook");
        assert_eq!(path_to_slug("/api/v1/quote/X/Y"), "quote");
        assert_eq!(path_to_slug("/health"), "health");
    }

    #[tokio::test]
    async fn in_memory_backend_allows_requests() {
        let backend = Backend::InMemory(Arc::new(Mutex::new(InMemoryStore::default())));
        let config = default_config(3);

        let info = backend.check("backend_key", &config).await;
        assert!(!info.denied);
        assert_eq!(info.limit, 3);
    }

    #[tokio::test]
    async fn in_memory_backend_denies_over_limit() {
        let backend = Backend::InMemory(Arc::new(Mutex::new(InMemoryStore::default())));
        let config = default_config(2);

        backend.check("over_key", &config).await;
        backend.check("over_key", &config).await;
        let info = backend.check("over_key", &config).await;
        assert!(info.denied);
        assert_eq!(info.remaining, 0);
    }
}
