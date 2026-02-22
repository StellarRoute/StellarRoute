//! StellarRoute API Server Binary

use sqlx::PgPool;
use stellarroute_api::{Server, ServerConfig};
use tracing::{error, info};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "stellarroute_api=debug,tower_http=debug".into()),
        )
        .init();

    info!("Starting StellarRoute API Server");

    // Get database URL from environment
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost/stellarroute".to_string());

    info!("Connecting to database...");
    let pool = match PgPool::connect(&database_url).await {
        Ok(pool) => {
            info!("✅ Database connection established");
            pool
        }
        Err(e) => {
            error!("❌ Failed to connect to database: {}", e);
            std::process::exit(1);
        }
    };

    // Create server configuration
    let config = ServerConfig {
        host: std::env::var("API_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
        port: std::env::var("API_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3000),
        enable_cors: true,
        enable_compression: true,
        redis_url: std::env::var("REDIS_URL").ok(),
    };

    // Create and start server
    let server = Server::new(config, pool).await;

    if let Err(e) = server.start().await {
        error!("Server error: {}", e);
        std::process::exit(1);
    }
}
