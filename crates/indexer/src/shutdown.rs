//! Graceful shutdown for the StellarRoute indexer.
//!
//! # Behaviour
//!
//! 1. On `SIGTERM` or `SIGINT` the indexer stops accepting new work.
//! 2. The current in-progress poll cycle is allowed to finish.
//! 3. The cursor is checkpointed before exit so no progress is lost.
//! 4. The process exits cleanly after the drain window.
//!
//! # Configuration
//!
//! | Env var                    | Default | Description                          |
//! |----------------------------|---------|--------------------------------------|
//! | `SHUTDOWN_DRAIN_TIMEOUT_S` | `30`    | Seconds to wait for in-flight work   |

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tracing::info;

/// Shared shutdown token for the indexer.
#[derive(Clone)]
pub struct IndexerShutdown {
    sender: Arc<watch::Sender<bool>>,
    receiver: watch::Receiver<bool>,
    stopping: Arc<AtomicBool>,
    pub drain_timeout: Duration,
}

impl IndexerShutdown {
    pub fn new() -> Self {
        let drain_secs: u64 = std::env::var("SHUTDOWN_DRAIN_TIMEOUT_S")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);

        let (tx, rx) = watch::channel(false);
        Self {
            sender: Arc::new(tx),
            receiver: rx,
            stopping: Arc::new(AtomicBool::new(false)),
            drain_timeout: Duration::from_secs(drain_secs),
        }
    }

    /// Returns `true` once a shutdown signal has been received.
    pub fn is_stopping(&self) -> bool {
        self.stopping.load(Ordering::Relaxed)
    }

    /// Trigger shutdown programmatically (useful in tests).
    pub fn trigger(&self) {
        self.stopping.store(true, Ordering::Relaxed);
        let _ = self.sender.send(true);
    }

    /// Subscribe to shutdown notifications.
    pub fn subscribe(&self) -> watch::Receiver<bool> {
        self.receiver.clone()
    }

    /// Wait for a shutdown signal (SIGTERM / SIGINT / programmatic trigger).
    pub async fn wait_for_signal(&self) {
        let mut rx = self.receiver.clone();

        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm =
                signal(SignalKind::terminate()).expect("Failed to register SIGTERM handler");
            let mut sigint =
                signal(SignalKind::interrupt()).expect("Failed to register SIGINT handler");

            tokio::select! {
                _ = sigterm.recv() => {
                    info!("Indexer received SIGTERM — stopping after current cycle");
                }
                _ = sigint.recv() => {
                    info!("Indexer received SIGINT — stopping after current cycle");
                }
                _ = rx.changed() => {
                    info!("Indexer programmatic shutdown triggered");
                }
            }
        }

        #[cfg(not(unix))]
        {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    info!("Indexer received Ctrl-C — stopping after current cycle");
                }
                _ = rx.changed() => {
                    info!("Indexer programmatic shutdown triggered");
                }
            }
        }

        self.stopping.store(true, Ordering::Relaxed);
        let _ = self.sender.send(true);

        info!(
            drain_timeout_secs = self.drain_timeout.as_secs(),
            "Indexer drain window started — waiting for in-flight cycle to complete"
        );

        tokio::time::sleep(self.drain_timeout).await;
        info!("Indexer drain window complete");
    }
}

impl Default for IndexerShutdown {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_stopping_initially() {
        let s = IndexerShutdown::new();
        assert!(!s.is_stopping());
    }

    #[test]
    fn test_trigger_sets_stopping() {
        let s = IndexerShutdown::new();
        s.trigger();
        assert!(s.is_stopping());
    }

    #[test]
    fn test_clone_shares_state() {
        let s = IndexerShutdown::new();
        let c = s.clone();
        s.trigger();
        assert!(c.is_stopping());
    }

    #[tokio::test]
    async fn test_subscribe_receives_signal() {
        let s = IndexerShutdown::new();
        let mut rx = s.subscribe();
        s.trigger();
        // The channel should have been updated
        let _ = rx.changed().await;
        assert!(*rx.borrow());
    }
}
