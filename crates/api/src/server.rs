//! API server setup and configuration

use axum::Router;
use sqlx::PgPool;
use std::{net::SocketAddr, sync::Arc};
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, warn};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    cache::CacheManager,
    docs::ApiDoc,
    error::Result,
    middleware::{EndpointConfig, RateLimitLayer},
    routes,
    state::AppState,
};

/// API server configuration
#[derive(Debug, Clone)]
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
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            enable_cors: true,
            enable_compression: true,
            redis_url: None,
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
    pub async fn new(config: ServerConfig, db: PgPool) -> Self {
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

                    (Arc::new(AppState::with_cache(db, cache)), rate_limit)
                }
                Err(e) => {
                    warn!("⚠️  Redis connection failed, running without cache: {}", e);
                    (
                        Arc::new(AppState::new(db)),
                        RateLimitLayer::in_memory(EndpointConfig::default()),
                    )
                }
            }
        } else {
            info!("ℹ️  Running without Redis cache");
            (
                Arc::new(AppState::new(db)),
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

        // Add request logging (method, URI, status code, latency)
        app = app.layer(TraceLayer::new_for_http());

        app
    }

    /// Start the server
    pub async fn start(self) -> Result<()> {
        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .expect("Invalid socket address");

        info!("🚀 StellarRoute API server starting on http://{}", addr);
        info!("📊 Health check: http://{}/health", addr);
        info!("📈 Trading pairs: http://{}/api/v1/pairs", addr);
        info!("📚 API Documentation: http://{}/swagger-ui", addr);

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("Failed to bind address");

        axum::serve(listener, self.app).await.expect("Server error");

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
