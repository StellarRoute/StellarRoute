//! Graceful shutdown for the StellarRoute API server.
//!
//! # Behaviour
//!
//! 1. On `SIGTERM` or `SIGINT` the server enters a **drain window**.
//! 2. During the drain window new requests receive `503 Service Unavailable`.
//! 3. In-flight requests are allowed to complete up to `drain_timeout`.
//! 4. After the drain window the process exits cleanly.
//!
//! # Configuration
//!
//! | Env var                    | Default | Description                          |
//! |----------------------------|---------|--------------------------------------|
//! | `SHUTDOWN_DRAIN_TIMEOUT_S` | `30`    | Seconds to wait for in-flight work   |
//!
//! # Usage
//!
//! ```rust,ignore
//! let shutdown = ShutdownSignal::new();
//! let guard = shutdown.guard();   // pass to axum::serve
//! shutdown.wait().await;          // blocks until signal received
//! ```

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tracing::{info, warn};

/// Shared shutdown state.
#[derive(Clone)]
pub struct ShutdownSignal {
    /// Broadcast channel — receivers block until the sender drops or sends.
    sender: Arc<watch::Sender<bool>>,
    /// Convenience receiver for callers that want to `await` shutdown.
    receiver: watch::Receiver<bool>,
    /// Set to `true` once the drain window has started.
    draining: Arc<AtomicBool>,
    /// How long to wait for in-flight work before forcing exit.
    pub drain_timeout: Duration,
}

impl ShutdownSignal {
    /// Create a new `ShutdownSignal` with the drain timeout read from the
    /// `SHUTDOWN_DRAIN_TIMEOUT_S` environment variable (default: 30 s).
    pub fn new() -> Self {
        let drain_secs: u64 = std::env::var("SHUTDOWN_DRAIN_TIMEOUT_S")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);

        let (tx, rx) = watch::channel(false);
        Self {
            sender: Arc::new(tx),
            receiver: rx,
            draining: Arc::new(AtomicBool::new(false)),
            drain_timeout: Duration::from_secs(drain_secs),
        }
    }

    /// Returns `true` if the server is currently in the drain window.
    pub fn is_draining(&self) -> bool {
        self.draining.load(Ordering::Relaxed)
    }

    /// Trigger the shutdown sequence programmatically (useful in tests).
    pub fn trigger(&self) {
        self.draining.store(true, Ordering::Relaxed);
        let _ = self.sender.send(true);
    }

    /// Returns a future that resolves when a shutdown signal is received.
    ///
    /// This is the value passed to `axum::serve(...).with_graceful_shutdown(...)`.
    pub async fn wait_for_signal(&self) {
        let mut rx = self.receiver.clone();

        // Wait for either SIGTERM or SIGINT.
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm =
                signal(SignalKind::terminate()).expect("Failed to register SIGTERM handler");
            let mut sigint =
                signal(SignalKind::interrupt()).expect("Failed to register SIGINT handler");

            tokio::select! {
                _ = sigterm.recv() => {
                    info!("Received SIGTERM — entering drain window");
                }
                _ = sigint.recv() => {
                    info!("Received SIGINT — entering drain window");
                }
                _ = rx.changed() => {
                    info!("Programmatic shutdown triggered — entering drain window");
                }
            }
        }

        #[cfg(not(unix))]
        {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    info!("Received Ctrl-C — entering drain window");
                }
                _ = rx.changed() => {
                    info!("Programmatic shutdown triggered — entering drain window");
                }
            }
        }

        self.draining.store(true, Ordering::Relaxed);
        let _ = self.sender.send(true);

        info!(
            drain_timeout_secs = self.drain_timeout.as_secs(),
            "Drain window started — rejecting new requests, waiting for in-flight work"
        );

        // Give in-flight requests time to complete.
        tokio::time::sleep(self.drain_timeout).await;

        info!("Drain window complete — shutting down");
    }

    /// Subscribe to shutdown notifications.
    pub fn subscribe(&self) -> watch::Receiver<bool> {
        self.receiver.clone()
    }
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        Self::new()
    }
}

/// Axum middleware layer that rejects new requests with 503 during the drain window.
///
/// # Usage
///
/// ```rust,ignore
/// use axum::middleware;
/// use crate::shutdown::ShutdownSignal;
///
/// let shutdown = ShutdownSignal::new();
/// let app = Router::new()
///     .layer(middleware::from_fn_with_state(
///         shutdown.clone(),
///         drain_guard_middleware,
///     ));
/// ```
pub async fn drain_guard_middleware(
    axum::extract::State(shutdown): axum::extract::State<ShutdownSignal>,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    if shutdown.is_draining() {
        warn!("Rejecting request during drain window");
        return axum::response::Response::builder()
            .status(axum::http::StatusCode::SERVICE_UNAVAILABLE)
            .header("Retry-After", "30")
            .body(axum::body::Body::from(
                r#"{"error":"service_unavailable","message":"Server is shutting down, please retry"}"#,
            ))
            .unwrap_or_default();
    }
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shutdown_signal_not_draining_initially() {
        let signal = ShutdownSignal::new();
        assert!(!signal.is_draining());
    }

    #[test]
    fn test_shutdown_signal_trigger_sets_draining() {
        let signal = ShutdownSignal::new();
        signal.trigger();
        assert!(signal.is_draining());
    }

    #[test]
    fn test_shutdown_signal_clone_shares_state() {
        let signal = ShutdownSignal::new();
        let clone = signal.clone();
        signal.trigger();
        assert!(clone.is_draining());
    }

    #[tokio::test]
    async fn test_drain_guard_rejects_when_draining() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let shutdown = ShutdownSignal::new();
        shutdown.trigger();

        let app = axum::Router::new()
            .route("/", axum::routing::get(|| async { "ok" }))
            .layer(axum::middleware::from_fn_with_state(
                shutdown,
                drain_guard_middleware,
            ));

        let req = Request::builder().uri("/").body(Body::empty()).unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_drain_guard_passes_when_not_draining() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let shutdown = ShutdownSignal::new();

        let app = axum::Router::new()
            .route("/", axum::routing::get(|| async { "ok" }))
            .layer(axum::middleware::from_fn_with_state(
                shutdown,
                drain_guard_middleware,
            ));

        let req = Request::builder().uri("/").body(Body::empty()).unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
