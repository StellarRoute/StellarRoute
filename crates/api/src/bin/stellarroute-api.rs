//! StellarRoute API Server Binary

use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use stellarroute_api::{state::DatabasePools, telemetry, Server, ServerConfig};
use tracing::{error, info};

fn parse_bool_env(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| {
            let v = value.trim().to_ascii_lowercase();
            matches!(v.as_str(), "1" | "true" | "yes" | "on")
        })
        .unwrap_or(false)
}

fn validate_required_env() -> Result<(), String> {
    let required = ["DATABASE_URL"];
    let mut missing = Vec::new();

    for key in required {
        match std::env::var(key) {
            Ok(value) if !value.trim().is_empty() => {}
            _ => missing.push(key),
        }
    }

    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Missing required environment variable(s): {}",
            missing.join(", ")
        ))
    }
}

async fn run_startup_credential_checks(
    database_url: &str,
    redis_url: Option<&str>,
) -> Result<(), String> {
    if sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
        .is_err()
    {
        return Err("Startup credential check failed: DATABASE_URL is not reachable".to_string());
    }

    if let Some(redis_url) = redis_url {
        let client = redis::Client::open(redis_url)
            .map_err(|_| "Startup credential check failed: REDIS_URL is invalid".to_string())?;
        let mut conn = client.get_connection().map_err(|_| {
            "Startup credential check failed: REDIS_URL is not reachable".to_string()
        })?;
        let pong: String = redis::cmd("PING")
            .query(&mut conn)
            .map_err(|_| "Startup credential check failed: REDIS_URL ping failed".to_string())?;
        if pong != "PONG" {
            return Err(
                "Startup credential check failed: REDIS_URL ping returned unexpected response"
                    .to_string(),
            );
        }
    }

    if let Ok(soroban_url) = std::env::var("SOROBAN_RPC_URL") {
        if !soroban_url.trim().is_empty() {
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .map_err(|_| {
                    "Startup credential check failed: unable to create HTTP client".to_string()
                })?;
            let body = serde_json::json!({
                "jsonrpc": "2.0",
                "id": "startup-check",
                "method": "getHealth",
                "params": {}
            });
            let response = client
                .post(&soroban_url)
                .json(&body)
                .send()
                .await
                .map_err(|_| {
                    "Startup credential check failed: SOROBAN_RPC_URL is not reachable".to_string()
                })?;
            if !response.status().is_success() {
                return Err(
                    "Startup credential check failed: SOROBAN_RPC_URL returned non-success status"
                        .to_string(),
                );
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    // Initialize structured logging (reads RUST_LOG and LOG_FORMAT env vars)
    telemetry::init();

    info!("Starting StellarRoute API Server");

    if let Err(message) = validate_required_env() {
        error!("{}", message);
        std::process::exit(1);
    }

    // Get database URL from environment
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost/stellarroute".to_string());

    let redis_url = std::env::var("REDIS_URL").ok();
    let startup_credential_checks = parse_bool_env("STARTUP_CREDENTIAL_CHECK");
    if startup_credential_checks {
        info!("Running startup credential reachability checks");
        if let Err(message) =
            run_startup_credential_checks(&database_url, redis_url.as_deref()).await
        {
            error!("{}", message);
            std::process::exit(1);
        }
    }

    // Read pool configuration from environment variables
    let max_connections: u32 = std::env::var("DB_MAX_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);
    let min_connections: u32 = std::env::var("DB_MIN_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2);
    let connection_timeout_secs: u64 = std::env::var("DB_CONNECTION_TIMEOUT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);
    let idle_timeout_secs: u64 = std::env::var("DB_IDLE_TIMEOUT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(600);
    let max_lifetime_secs: u64 = std::env::var("DB_MAX_LIFETIME")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1800);
    let statement_timeout_ms: u64 = std::env::var("DB_STATEMENT_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5000);
    let lock_timeout_ms: u64 = std::env::var("DB_LOCK_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2000);
    let idle_in_txn_timeout_ms: u64 = std::env::var("DB_IDLE_IN_TXN_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5000);

    info!(
        "Connecting to database (pool: min={}, max={}, timeout={}s)...",
        min_connections, max_connections, connection_timeout_secs
    );
    info!(
        "DB guardrails: statement_timeout={}ms lock_timeout={}ms idle_in_txn_timeout={}ms",
        statement_timeout_ms, lock_timeout_ms, idle_in_txn_timeout_ms
    );
    let pool = match PgPoolOptions::new()
        .max_connections(max_connections)
        .min_connections(min_connections)
        .acquire_timeout(Duration::from_secs(connection_timeout_secs))
        .idle_timeout(Duration::from_secs(idle_timeout_secs))
        .max_lifetime(Duration::from_secs(max_lifetime_secs))
        .after_connect(move |conn, _meta| {
            Box::pin(async move {
                sqlx::query(&format!(
                    "SET statement_timeout = '{}ms'",
                    statement_timeout_ms
                ))
                .execute(&mut *conn)
                .await?;
                sqlx::query(&format!("SET lock_timeout = '{}ms'", lock_timeout_ms))
                    .execute(&mut *conn)
                    .await?;
                sqlx::query(&format!(
                    "SET idle_in_transaction_session_timeout = '{}ms'",
                    idle_in_txn_timeout_ms
                ))
                .execute(&mut *conn)
                .await?;
                Ok(())
            })
        })
        .connect(&database_url)
        .await
    {
        Ok(pool) => {
            info!(
                "✅ Database connection pool established (max_connections={})",
                max_connections
            );
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
        redis_url,
        quote_cache_ttl_seconds: std::env::var("QUOTE_CACHE_TTL_SECONDS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2),
    };

    // Create and start server
    let server = Server::new(config, DatabasePools::new(pool, None)).await;

    if let Err(e) = server.start().await {
        error!("Server error: {}", e);
        std::process::exit(1);
    }
}
