//! Subscription registry for the WebSocket quote stream.
//!
//! [`SubscriptionRegistry`] tracks all active subscriptions across all
//! connected clients. It is shared between the connection tasks and the
//! quote broadcaster via `Arc<RwLock<SubscriptionRegistry>>`.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

use super::messages::{ServerMessage, SubscriptionId};

/// Opaque connection identifier (UUID v4).
pub type ConnId = Uuid;

/// Tracks all active subscriptions and outbound senders for every connection.
///
/// Wrap in `Arc<RwLock<SubscriptionRegistry>>` for shared access across tasks.
#[derive(Debug, Default)]
pub struct SubscriptionRegistry {
    connections: HashMap<ConnId, ConnectionEntry>,
}

/// Per-connection state stored in the registry.
#[derive(Debug)]
pub struct ConnectionEntry {
    /// Active subscriptions for this connection.
    pub subscriptions: Vec<Subscription>,
    /// Sender half of the bounded outbound channel for this connection.
    pub tx: mpsc::Sender<ServerMessage>,
}

/// A single subscription registered by a client.
#[derive(Debug, Clone)]
pub struct Subscription {
    /// Unique identifier for this subscription (client- or server-generated).
    pub id: SubscriptionId,
    /// Base asset identifier (e.g. `"native"` or `"CODE:ISSUER"`).
    pub base: String,
    /// Quote asset identifier (e.g. `"USDC:ISSUER"`).
    pub quote: String,
    /// Optional amount filter (positive decimal string).
    pub amount: Option<String>,
    /// Last price emitted for this subscription, used for dedup / 0.01% threshold.
    pub last_emitted_price: Option<f64>,
}

/// Maximum number of concurrent subscriptions allowed per connection.
const MAX_SUBSCRIPTIONS_PER_CONNECTION: usize = 20;

impl SubscriptionRegistry {
    /// Create a new, empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Wrap a new registry in `Arc<RwLock<...>>` ready for shared use.
    pub fn shared() -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Self::new()))
    }

    /// Add a subscription for a connection.
    ///
    /// If the connection has no entry yet, one is created using `entry_tx`.
    /// If the connection already has 20 subscriptions, returns
    /// `Err("subscription_limit_exceeded")`.
    pub fn add_subscription(
        &mut self,
        conn_id: ConnId,
        entry_tx: mpsc::Sender<ServerMessage>,
        sub: Subscription,
    ) -> Result<(), &'static str> {
        let entry = self.connections.entry(conn_id).or_insert_with(|| ConnectionEntry {
            subscriptions: Vec::new(),
            tx: entry_tx,
        });

        if entry.subscriptions.len() >= MAX_SUBSCRIPTIONS_PER_CONNECTION {
            return Err("subscription_limit_exceeded");
        }

        entry.subscriptions.push(sub);
        Ok(())
    }

    /// Remove a specific subscription from a connection.
    ///
    /// If the connection has no remaining subscriptions after removal, the
    /// connection entry is kept (the sender may still be needed for other
    /// messages). Use [`remove_connection`] to fully clean up.
    pub fn remove_subscription(&mut self, conn_id: ConnId, sub_id: SubscriptionId) {
        if let Some(entry) = self.connections.get_mut(&conn_id) {
            entry.subscriptions.retain(|s| s.id != sub_id);
        }
    }

    /// Remove all subscriptions and the connection entry for `conn_id`.
    ///
    /// Called when a WebSocket connection is closed.
    pub fn remove_connection(&mut self, conn_id: ConnId) {
        self.connections.remove(&conn_id);
    }

    /// Return all (conn_id, sender, subscription) triples that match the
    /// given `base`/`quote` pair.
    ///
    /// Used by the broadcaster to fan out quote updates.
    pub fn get_connections_for_pair(
        &self,
        base: &str,
        quote: &str,
    ) -> Vec<(ConnId, mpsc::Sender<ServerMessage>, Subscription)> {
        let mut result = Vec::new();
        for (&conn_id, entry) in &self.connections {
            for sub in &entry.subscriptions {
                if sub.base == base && sub.quote == quote {
                    result.push((conn_id, entry.tx.clone(), sub.clone()));
                }
            }
        }
        result
    }

    /// Return the set of unique `(base, quote)` pairs across all active subscriptions.
    ///
    /// Used by the broadcaster to determine which pairs to poll.
    pub fn all_pairs(&self) -> std::collections::HashSet<(String, String)> {
        let mut pairs = std::collections::HashSet::new();
        for entry in self.connections.values() {
            for sub in &entry.subscriptions {
                pairs.insert((sub.base.clone(), sub.quote.clone()));
            }
        }
        pairs
    }

    /// Update the `last_emitted_price` for a specific subscription.
    ///
    /// Called by the broadcaster after successfully sending a `QuoteUpdate`.
    pub fn update_last_emitted_price(
        &mut self,
        conn_id: ConnId,
        sub_id: SubscriptionId,
        price: f64,
    ) {
        if let Some(entry) = self.connections.get_mut(&conn_id) {
            if let Some(sub) = entry.subscriptions.iter_mut().find(|s| s.id == sub_id) {
                sub.last_emitted_price = Some(price);
            }
        }
    }
}
