//! API server setup and configuration

use axum::{http::Request, Router};
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
    middleware::{
        api_versioning_layer, request_id_layer, EndpointConfig, RateLimitLayer, RequestId,
        REQUEST_ID_HEADER,
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
            quote_cache_ttl_seconds: 2,
        }
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

                    (
                        Arc::new(AppState::with_cache_and_policy(
                            db,
                            cache,
                            cache_policy.clone(),
                        )),
                        rate_limit,
                    )
                }
                Err(e) => {
                    warn!("⚠️  Redis connection failed, running without cache: {}", e);
                    (
                        Arc::new(AppState::new_with_policy(db, cache_policy.clone())),
                        RateLimitLayer::in_memory(EndpointConfig::default()),
                    )
                }
            }
        } else {
            info!("ℹ️  Running without Redis cache");
            (
                Arc::new(AppState::new_with_policy(db, cache_policy)),
                RateLimitLayer::in_memory(EndpointConfig::default()),
            )
        };

        let app = Self::build_app(state, &config, rate_limit_layer);

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
            let cors = CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any);
            app = app.layer(cors);
        }

        // Add rate limiting (innermost — runs before CORS/compression in the response path)
        app = app.layer(rate_limit);

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
        axum::serve(listener, self.app)
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

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 3000);
        assert!(config.enable_cors);
    }
}
