//! API server setup and configuration

use axum::Router;
use sqlx::PgPool;
use std::{net::SocketAddr, sync::Arc};
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
};
use tracing::{info, warn};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    cache::CacheManager, docs::ApiDoc, error::Result, middleware::RateLimitLayer, routes,
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
        let state = if let Some(redis_url) = &config.redis_url {
            match CacheManager::new(redis_url).await {
                Ok(cache) => {
                    info!("✅ Redis cache connected");
                    Arc::new(AppState::with_cache(db, cache))
                }
                Err(e) => {
                    warn!("⚠️  Redis connection failed, running without cache: {}", e);
                    Arc::new(AppState::new(db))
                }
            }
        } else {
            info!("ℹ️  Running without Redis cache");
            Arc::new(AppState::new(db))
        };

        let app = Self::build_app(state, &config);

        Self { config, app }
    }

    /// Build the application router
    fn build_app(state: Arc<AppState>, config: &ServerConfig) -> Router {
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

        // Add rate limiting
        let rate_limit = RateLimitLayer::default();
        app = app.layer(rate_limit);

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

    /// Get router for testing
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
