//! API server setup and configuration

use axum::{
    http::{HeaderValue, Request},
    Router,
};
use std::{net::SocketAddr, sync::Arc};
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::{DefaultOnResponse, TraceLayer},
};
use tracing::{info, warn, Level};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    cache::CacheManager,
    docs::ApiDoc,
    error::Result,
    health_scheduler::{HealthScheduler, HealthSchedulerConfig},
    middleware::{
        api_versioning_layer, request_id_layer, AuthLayer, EndpointConfig, RateLimitLayer,
        RequestId, REQUEST_ID_HEADER,
    },
    routes,
    state::{AppState, CachePolicy, DatabasePools},
};

/// API server configuration
#[derive(Clone)]
pub struct ServerConfig {
    /// Server host address
    pub host: String,
    /// Server port
    pub port: u16,
    /// Enable CORS
    pub enable_cors: bool,
    /// Enable response compression
    pub enable_compression: bool,
    /// Redis URL (optional)
    pub redis_url: Option<String>,
    /// Admin bearer token for operator-only endpoints
    pub admin_auth_token: Option<String>,
    /// Quote cache TTL in seconds
    pub quote_cache_ttl_seconds: u64,
}

impl std::fmt::Debug for ServerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("enable_cors", &self.enable_cors)
            .field("enable_compression", &self.enable_compression)
            .field("redis_url", &self.redis_url.as_ref().map(|_| "[REDACTED]"))
            .field("quote_cache_ttl_seconds", &self.quote_cache_ttl_seconds)
            .finish()
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            enable_cors: true,
            enable_compression: true,
            redis_url: None,
            admin_auth_token: std::env::var("ADMIN_AUTH_TOKEN").ok(),
            quote_cache_ttl_seconds: 2,
        }
    }
}

/// Parse `CORS_ALLOWED_ORIGINS` as a CSV allowlist of exact origin values
/// (e.g. `https://app.example.com,https://staging.example.com`).
pub fn cors_allowed_origins_from_env() -> Vec<String> {
    std::env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Validate the CORS configuration at startup.
///
/// When a strict CORS policy is required (production, or
/// `REQUIRE_STRICT_CORS=1`), `CORS_ALLOWED_ORIGINS` must resolve to a
/// non-empty allowlist of valid origin values. Returns `Err` describing the
/// problem so the caller can refuse to boot.
pub fn validate_cors_config() -> std::result::Result<(), String> {
    if !crate::env_profile::require_strict_cors() {
        return Ok(());
    }

    let origins = cors_allowed_origins_from_env();
    if origins.is_empty() {
        return Err(
            "CORS_ALLOWED_ORIGINS must be set to a non-empty comma-separated allowlist of \
             origins when STELLARROUTE_ENV=production (or REQUIRE_STRICT_CORS=1). Wildcard \
             CORS is not permitted in production."
                .to_string(),
        );
    }

    let invalid: Vec<&String> = origins
        .iter()
        .filter(|o| o.parse::<HeaderValue>().is_err())
        .collect();
    if !invalid.is_empty() {
        return Err(format!(
            "CORS_ALLOWED_ORIGINS contains invalid origin value(s): {:?}",
            invalid
        ));
    }

    Ok(())
}

/// Build the CORS layer for the API.
///
/// In production (or when `REQUIRE_STRICT_CORS=1`), origins are restricted to
/// the explicit `CORS_ALLOWED_ORIGINS` allowlist. Outside of production,
/// CORS remains permissive (`Any`) to preserve local developer experience.
fn build_cors_layer() -> CorsLayer {
    if crate::env_profile::require_strict_cors() {
        let origins: Vec<HeaderValue> = cors_allowed_origins_from_env()
            .iter()
            .filter_map(|o| o.parse::<HeaderValue>().ok())
            .collect();
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    }
}

/// API Server
pub struct Server {
    config: ServerConfig,
    app: Router,
}

impl Server {
    /// Create a new API server
    pub async fn new(config: ServerConfig, db: DatabasePools) -> Self {
        let cache_policy = CachePolicy {
            quote_ttl: std::time::Duration::from_secs(config.quote_cache_ttl_seconds),
        };

        // Clone the write pool so the scheduler can use it independently.
        let scheduler_pool = db.write_pool().clone();

        // Try to connect to Redis if URL is provided
        let (state, rate_limit_layer) = if let Some(redis_url) = &config.redis_url {
            match CacheManager::new(redis_url).await {
                Ok(cache) => {
                    info!("✅ Redis cache connected");

                    // Build rate limit layer backed by the same Redis connection
                    let rate_limit = match redis::Client::open(redis_url.as_str()) {
                        Ok(client) => match redis::aio::ConnectionManager::new(client).await {
                            Ok(conn) => {
                                info!("✅ Rate limiter using Redis backend");
                                RateLimitLayer::with_redis(conn, EndpointConfig::default())
                            }
                            Err(e) => {
                                warn!("⚠️  Redis rate limiter connection failed ({}), using in-memory fallback", e);
                                RateLimitLayer::in_memory(EndpointConfig::default())
                            }
                        },
                        Err(e) => {
                            warn!("⚠️  Redis client error ({}), using in-memory fallback", e);
                            RateLimitLayer::in_memory(EndpointConfig::default())
                        }
                    };

                    {
                        let mut state =
                            AppState::with_cache_and_policy(db, cache, cache_policy.clone());
                        state.admin_auth_token = config.admin_auth_token.clone();
                        // Initialize WS subsystem on the AppState so handlers/broadcaster can start
                        state.ws = Some(crate::routes::ws::WsState::from_env());
                        (Arc::new(state), rate_limit)
                    }
                }
                Err(e) => {
                    warn!("⚠️  Redis connection failed, running without cache: {}", e);
                    {
                        let mut state = AppState::new_with_policy(db, cache_policy.clone());
                        state.admin_auth_token = config.admin_auth_token.clone();
                        state.ws = Some(crate::routes::ws::WsState::from_env());
                        (
                            Arc::new(state),
                            RateLimitLayer::in_memory(EndpointConfig::default()),
                        )
                    }
                }
            }
        } else {
            info!("ℹ️  Running without Redis cache");
            {
                let mut state = AppState::new_with_policy(db, cache_policy);
                state.admin_auth_token = config.admin_auth_token.clone();
                state.ws = Some(crate::routes::ws::WsState::from_env());
                (
                    Arc::new(state),
                    RateLimitLayer::in_memory(EndpointConfig::default()),
                )
            }
        };

        // Start the background health score recomputation scheduler.
        HealthScheduler::start(scheduler_pool, HealthSchedulerConfig::from_env());

        let app = Self::build_app(state.clone(), &config, rate_limit_layer);

        // If WS is enabled on the AppState, spawn the long-lived broadcaster
        // task so real-time quote updates are emitted even before the first
        // client connects.
        if let Some(ws_state) = state.ws.as_ref() {
            let state_for_broadcaster = state.clone();
            let registry = ws_state.registry.clone();
            let poll_interval_ms = ws_state.poll_interval_ms;
            tokio::spawn(async move {
                crate::routes::ws::broadcaster::run_broadcaster(
                    state_for_broadcaster,
                    registry,
                    poll_interval_ms,
                )
                .await;
            });
            info!("✅ WebSocket broadcaster started");
        }

        Self { config, app }
    }

    /// Build the application router
    fn build_app(
        state: Arc<AppState>,
        config: &ServerConfig,
        rate_limit: RateLimitLayer,
    ) -> Router {
        let mut app = routes::create_router(state);

        // Add Swagger UI for API documentation
        let swagger =
            SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi());
        app = app.merge(swagger);

        // Add compression if enabled (gzip for responses > 1KB)
        if config.enable_compression {
            app = app.layer(CompressionLayer::new());
            info!("✅ Response compression enabled");
        }

        // Add CORS if enabled
        if config.enable_cors {
            app = app.layer(build_cors_layer());
        }

        // Add rate limiting (innermost — runs before CORS/compression in the response path)
        app = app.layer(rate_limit);

        // Add API key authentication
        app = app.layer(AuthLayer::default());

        // Add request logging — each request gets a unique span with method, URI, status, and latency.
        app = app.layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    let request_id = request
                        .extensions()
                        .get::<RequestId>()
                        .map(RequestId::as_str)
                        .or_else(|| {
                            request
                                .headers()
                                .get(REQUEST_ID_HEADER)
                                .and_then(|value| value.to_str().ok())
                        })
                        .unwrap_or("missing");

                    tracing::info_span!(
                        "http.request",
                        request_id = %request_id,
                        http.method = %request.method(),
                        http.target = %request.uri(),
                        http.status_code = tracing::field::Empty,
                        otel.kind = "server",
                    )
                })
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        );

        // Add request ID propagation as the outermost wrapper so downstream layers reuse the
        // same correlation ID in logs, spans, and responses.
        app = app.layer(axum::middleware::from_fn(request_id_layer));

        // Add API lifecycle headers (Deprecation/Sunset/Link) for /api/v1 routes.
        app = app.layer(axum::middleware::from_fn(api_versioning_layer));

        app
    }

    /// Start the server with graceful shutdown support.
    ///
    /// The server listens for `SIGTERM` / `SIGINT` and enters a drain window
    /// before exiting.  New requests are rejected with `503` during the drain
    /// window; in-flight requests are allowed to complete up to
    /// `SHUTDOWN_DRAIN_TIMEOUT_S` seconds (default: 30).
    pub async fn start(self) -> Result<()> {
        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .expect("Invalid socket address");

        info!("🚀 StellarRoute API server starting on http://{}", addr);
        info!("📊 Health check: http://{}/health", addr);
        info!("📈 Trading pairs: http://{}/api/v1/pairs", addr);
        info!("📉 Prometheus metrics: http://{}/metrics", addr);
        info!("📚 API Documentation: http://{}/swagger-ui", addr);

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("Failed to bind address");

        let shutdown = crate::shutdown::ShutdownSignal::new();
        info!(
            drain_timeout_secs = shutdown.drain_timeout.as_secs(),
            "Graceful shutdown configured"
        );

        let shutdown_clone = shutdown.clone();

        // Use `into_make_service_with_connect_info::<SocketAddr>()` so handlers
        // that require the client's `SocketAddr` (via `ConnectInfo<SocketAddr>`) work.
        axum::serve(
            listener,
            self.app
                .into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async move {
            shutdown_clone.wait_for_signal().await;
        })
        .await
        .expect("Server error");

        Ok(())
    }

    /// Consume the server and return the router (for integration testing)
    pub fn into_router(self) -> Router {
        self.app
    }

    /// Get router for testing (crate-internal)
    #[cfg(test)]
    pub fn router(self) -> Router {
        self.app
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::DatabasePools;
    use axum::{
        body::Body,
        http::{Method, Request as HttpRequest},
    };
    use sqlx::postgres::PgPoolOptions;
    use std::sync::Mutex;
    use tower::ServiceExt;

    // CORS behavior is driven by process-global env vars; serialize access
    // across tests in this module so they don't race each other.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn reset_cors_env() {
        std::env::remove_var("STELLARROUTE_ENV");
        std::env::remove_var("REQUIRE_STRICT_CORS");
        std::env::remove_var("CORS_ALLOWED_ORIGINS");
    }

    fn lazy_db_pools() -> DatabasePools {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://localhost/unused")
            .expect("failed to create lazy pool");
        DatabasePools::new(pool, None)
    }

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 3000);
        assert!(config.enable_cors);
    }

    #[test]
    fn cors_validate_fails_in_production_without_allowlist() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_cors_env();
        std::env::set_var("STELLARROUTE_ENV", "production");

        let result = validate_cors_config();

        reset_cors_env();
        assert!(result.is_err(), "expected production without an allowlist to fail startup validation");
    }

    #[test]
    fn cors_validate_passes_in_production_with_allowlist() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_cors_env();
        std::env::set_var("STELLARROUTE_ENV", "production");
        std::env::set_var("CORS_ALLOWED_ORIGINS", "https://app.example.com");

        let result = validate_cors_config();

        reset_cors_env();
        assert!(result.is_ok());
    }

    #[test]
    fn cors_validate_passes_outside_production_without_allowlist() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_cors_env();

        let result = validate_cors_config();

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn cors_allows_allowlisted_origin_preflight_in_production() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_cors_env();
        std::env::set_var("STELLARROUTE_ENV", "production");
        std::env::set_var("CORS_ALLOWED_ORIGINS", "https://app.example.com");

        let server = Server::new(ServerConfig::default(), lazy_db_pools()).await;
        let router = server.router();

        let response = router
            .oneshot(
                HttpRequest::builder()
                    .method(Method::OPTIONS)
                    .uri("/health")
                    .header("origin", "https://app.example.com")
                    .header("access-control-request-method", "GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        reset_cors_env();

        assert_eq!(
            response
                .headers()
                .get("access-control-allow-origin")
                .and_then(|v| v.to_str().ok()),
            Some("https://app.example.com"),
            "allowlisted origin must be echoed back on preflight"
        );
    }

    #[tokio::test]
    async fn cors_denies_disallowed_origin_preflight_in_production() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_cors_env();
        std::env::set_var("STELLARROUTE_ENV", "production");
        std::env::set_var("CORS_ALLOWED_ORIGINS", "https://app.example.com");

        let server = Server::new(ServerConfig::default(), lazy_db_pools()).await;
        let router = server.router();

        let response = router
            .oneshot(
                HttpRequest::builder()
                    .method(Method::OPTIONS)
                    .uri("/health")
                    .header("origin", "https://evil.example.com")
                    .header("access-control-request-method", "GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        reset_cors_env();

        assert!(
            response
                .headers()
                .get("access-control-allow-origin")
                .is_none(),
            "disallowed origin must not receive an Access-Control-Allow-Origin header"
        );
    }
}
