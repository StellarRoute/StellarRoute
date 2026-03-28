//! WebSocket message types for the quote stream protocol.
//!
//! # Client → Server
//! Clients send [`ClientMessage`] frames encoded as UTF-8 JSON text.
//!
//! # Server → Client
//! The server sends [`ServerMessage`] frames encoded as UTF-8 JSON text.
//! Every server message includes a `v` (version) field set to `1` and a
//! `timestamp` field (Unix milliseconds).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::response::QuoteResponse;

/// Opaque subscription identifier (UUID v4).
pub type SubscriptionId = Uuid;

// ---------------------------------------------------------------------------
// Client → Server
// ---------------------------------------------------------------------------

/// A message sent from a connected client to the server.
///
/// Serialized with a discriminant `action` field:
/// ```json
/// { "action": "subscribe", "subscription": { ... } }
/// { "action": "unsubscribe", "subscription_id": "uuid" }
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Subscribe to a trading-pair quote stream.
    Subscribe { subscription: SubscriptionRequest },
    /// Cancel an existing subscription by its ID.
    Unsubscribe { subscription_id: SubscriptionId },
}

/// Parameters for a new subscription.
#[derive(Debug, Clone, Deserialize)]
pub struct SubscriptionRequest {
    /// Base asset identifier (e.g. `"native"` or `"CODE:ISSUER"`).
    pub base: String,
    /// Quote asset identifier (e.g. `"USDC:ISSUER"`).
    pub quote: String,
    /// Optional amount filter (positive decimal string).
    /// When present, updates are only emitted when the quoted amount changes
    /// by more than 0.01 % relative to the previous emission.
    pub amount: Option<String>,
}

// ---------------------------------------------------------------------------
// Server → Client
// ---------------------------------------------------------------------------

/// Envelope for every message sent from the server to a client.
///
/// ```json
/// { "v": 1, "timestamp": 1700000000000, "type": "...", ...payload }
/// ```
///
/// The `payload` is flattened into the envelope so that `type` and the
/// payload fields appear at the top level.
#[derive(Debug, Clone, Serialize)]
pub struct ServerMessage {
    /// Schema version — always `1`.
    pub v: u8,
    /// Unix timestamp in milliseconds when this message was produced.
    pub timestamp: i64,
    /// The actual payload, flattened into the envelope.
    #[serde(flatten)]
    pub payload: ServerPayload,
}

impl ServerMessage {
    /// Construct a new [`ServerMessage`] with `v = 1` and the current time.
    pub fn now(payload: ServerPayload) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        Self { v: 1, timestamp, payload }
    }
}

/// The payload variants carried inside a [`ServerMessage`].
///
/// Serialized with a discriminant `type` field:
/// ```json
/// { "type": "subscription_confirmed", "subscription_id": "uuid" }
/// { "type": "quote_update", "subscription_id": "uuid", "quote": { ... } }
/// { "type": "error", "code": "...", "message": "..." }
/// { "type": "ping" }
/// ```
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerPayload {
    /// Sent after a successful subscription.
    SubscriptionConfirmed {
        subscription_id: SubscriptionId,
    },
    /// Sent when a new quote is available for a subscription.
    QuoteUpdate {
        subscription_id: SubscriptionId,
        quote: QuoteResponse,
    },
    /// Sent when an error occurs (connection remains open unless noted).
    Error {
        code: String,
        message: String,
    },
    /// Keepalive ping sent every 30 seconds.
    Ping,
}
