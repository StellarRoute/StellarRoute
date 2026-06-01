//! WebSocket quote stream handler

pub mod messages;

pub mod registry;

pub mod connection;

pub mod broadcaster;
pub mod rate_limit;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::{ws::WebSocketUpgrade, ConnectInfo, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::state::AppState;
use crate::{
    models::{ApiErrorCode, ErrorResponse},
    routes::ws::registry::SubscriptionRegistry,
};
use connection::run_connection;

/// Configuration and shared state for the WebSocket endpoint.
pub struct WsState {
    /// Shared subscription registry (connections + subscriptions).
    pub registry: Arc<RwLock<SubscriptionRegistry>>,
    /// Atomic counter of currently active WebSocket connections.
    pub connection_counter: Arc<AtomicUsize>,
    /// Maximum number of concurrent WebSocket connections (from WS_MAX_CONNECTIONS, default 500).
    pub max_connections: usize,
    /// Per-IP connection rate limiter: tracks new connection timestamps per IP.
    pub ip_rate_limiter: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    /// Broadcaster poll interval in milliseconds (from WS_POLL_INTERVAL_MS, default 1000).
    pub poll_interval_ms: u64,
    /// Keepalive ping interval in seconds (from WS_PING_INTERVAL_SECS, default 30).
    pub ping_interval_secs: u64,
    /// Pong response timeout in seconds (from WS_PONG_TIMEOUT_SECS, default 10).
    pub pong_timeout_secs: u64,
    /// Backpressure timeout in seconds (from WS_BACKPRESSURE_TIMEOUT_SECS, default 10).
    pub backpressure_timeout_secs: u64,
}

impl WsState {
    /// Create a new `WsState` reading configuration from environment variables.
    pub fn from_env() -> Arc<Self> {
        let max_connections = std::env::var("WS_MAX_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(500usize);
        let poll_interval_ms = std::env::var("WS_POLL_INTERVAL_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1000u64);
        let ping_interval_secs = std::env::var("WS_PING_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30u64);
        let pong_timeout_secs = std::env::var("WS_PONG_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10u64);
        let backpressure_timeout_secs = std::env::var("WS_BACKPRESSURE_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10u64);

        Arc::new(Self {
            registry: SubscriptionRegistry::shared(),
            connection_counter: Arc::new(AtomicUsize::new(0)),
            max_connections,
            ip_rate_limiter: Arc::new(Mutex::new(HashMap::new())),
            poll_interval_ms,
            ping_interval_secs,
            pong_timeout_secs,
            backpressure_timeout_secs,
        })
    }
}

/// Per-IP rate limit: maximum new connections per minute.
const IP_RATE_LIMIT_PER_MINUTE: usize = 10;

/// WebSocket upgrade handler.
///
/// Checks the connection cap and per-IP rate limit before accepting the
/// upgrade. On success, spawns the broadcaster (if not already running) and
/// the per-connection task.
///
/// NOTE: `ConnectInfo` requires the server to be served via
/// `into_make_service_with_connect_info::<SocketAddr>()` in server.rs.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Response {
    // Retrieve WsState — return 503 if WS feature is disabled.
    let ws_state = match &state.ws {
        Some(s) => s.clone(),
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(crate::models::ApiResponse::new(
                    ErrorResponse::new(
                        ApiErrorCode::Overloaded,
                        "WebSocket endpoint is not enabled.",
                    ),
                    "system",
                )),
            )
                .into_response();
        }
    };

    // Check connection cap.
    let current = ws_state.connection_counter.load(Ordering::Relaxed);
    if current >= ws_state.max_connections {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(crate::models::ApiResponse::new(
                ErrorResponse::new(
                    ApiErrorCode::Overloaded,
                    format!(
                        "Server has reached the maximum of {} concurrent connections.",
                        ws_state.max_connections
                    ),
                ),
                "system",
            )),
        )
            .into_response();
    }

    // Check per-IP rate limit (10 new connections per minute).
    let ip_key = addr.ip().to_string();
    {
        let mut limiter = ws_state.ip_rate_limiter.lock().await;
        let now = Instant::now();
        let window = Duration::from_secs(60);
        let timestamps = limiter.entry(ip_key.clone()).or_default();

        // Evict entries older than 60 seconds.
        timestamps.retain(|t| now.duration_since(*t) < window);

        if timestamps.len() >= IP_RATE_LIMIT_PER_MINUTE {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                Json(crate::models::ApiResponse::new(
                    ErrorResponse::new(
                        ApiErrorCode::RateLimitExceeded,
                        "Too many new WebSocket connections from this IP. Try again later.",
                    ),
                    "system",
                )),
            )
                .into_response();
        }

        timestamps.push(now);
    }

    // Increment connection counter before handing off to the upgrade callback.
    ws_state.connection_counter.fetch_add(1, Ordering::Relaxed);

    let conn_id = Uuid::new_v4();
    let registry = ws_state.registry.clone();
    let connection_counter = ws_state.connection_counter.clone();
    let poll_interval_ms = ws_state.poll_interval_ms;
    let state_for_broadcaster = state.clone();
    let registry_for_broadcaster = registry.clone();

    ws.on_upgrade(move |socket| async move {
        // Spawn the broadcaster task. It is idempotent — if it is already
        // running the extra spawn will simply race to acquire the same
        // registry and poll alongside the existing one (harmless for now;
        // a future task can add a once-flag).
        tokio::spawn(broadcaster::run_broadcaster(
            state_for_broadcaster,
            registry_for_broadcaster,
            poll_interval_ms,
        ));

        run_connection(socket, conn_id, registry, connection_counter).await;
    })
}
