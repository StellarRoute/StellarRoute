//! Per-connection WebSocket state machine.
//!
//! [`run_connection`] drives a single WebSocket connection: it reads inbound
//! client frames, dispatches subscribe/unsubscribe actions, enforces rate
//! limits, drains the outbound [`ServerMessage`] channel, and sends keepalive
//! pings. On exit it cleans up the registry and decrements the connection
//! counter.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::ws::{Message, WebSocket};
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, sleep};
use uuid::Uuid;

use super::messages::{ClientMessage, ServerMessage, ServerPayload};
use super::rate_limit::MessageRateLimiter;
use super::registry::{ConnId, Subscription, SubscriptionRegistry};

/// Ping interval — a WS ping frame is sent every 30 seconds.
const PING_INTERVAL: Duration = Duration::from_secs(30);
/// How long to wait for a pong before closing the connection.
const PONG_TIMEOUT: Duration = Duration::from_secs(10);
/// How long the outbound channel may remain full before the connection is
/// closed with code 1008.
const BACKPRESSURE_TIMEOUT: Duration = Duration::from_secs(10);
/// Outbound channel capacity (matches the registry / broadcaster expectation).
pub const OUTBOUND_CHANNEL_CAPACITY: usize = 32;

/// Shared state passed into the connection task.
pub struct ConnectionTask {
    pub conn_id: ConnId,
    pub registry: Arc<RwLock<SubscriptionRegistry>>,
    pub connection_counter: Arc<AtomicUsize>,
}

/// Drive a single WebSocket connection to completion.
///
/// Creates the outbound `mpsc` channel, registers the sender in the registry,
/// then runs the `tokio::select!` event loop until the connection closes.
/// On exit, removes the connection from the registry and decrements the
/// connection counter.
pub async fn run_connection(
    mut socket: WebSocket,
    conn_id: ConnId,
    registry: Arc<RwLock<SubscriptionRegistry>>,
    connection_counter: Arc<AtomicUsize>,
) {
    // Create the bounded outbound channel.  The sender is stored in the
    // registry so the broadcaster can push messages; the receiver is drained
    // here in the select loop.
    let (tx, mut rx) = mpsc::channel::<ServerMessage>(OUTBOUND_CHANNEL_CAPACITY);

    // Per-connection message rate limiter (60 msg / 60 s).
    let mut rate_limiter = MessageRateLimiter::new();

    // Ping / pong state.
    let mut ping_timer = interval(PING_INTERVAL);
    ping_timer.tick().await; // consume the immediate first tick
    let mut awaiting_pong = false;
    let mut pong_deadline: Option<Instant> = None;

    // Backpressure watchdog: tracks when the outbound channel first became full.
    let mut backpressure_since: Option<Instant> = None;

    // Store the tx in the registry so the broadcaster can find this connection.
    // We do this before entering the loop so no messages are lost.
    // (No subscription yet — the entry will be created on first subscribe.)
    // We keep a clone of tx to use for try_send backpressure checks.
    let tx_for_registry = tx.clone();

    loop {
        // Compute how long until the pong deadline fires (if we're waiting).
        let pong_remaining = pong_deadline.map(|d| {
            d.checked_duration_since(Instant::now())
                .unwrap_or(Duration::ZERO)
        });

        // Compute how long until the backpressure deadline fires (if active).
        let bp_remaining = backpressure_since.map(|s| {
            let elapsed = s.elapsed();
            BACKPRESSURE_TIMEOUT
                .checked_sub(elapsed)
                .unwrap_or(Duration::ZERO)
        });

        tokio::select! {
            // ----------------------------------------------------------------
            // Inbound WS frame from the client
            // ----------------------------------------------------------------
            maybe_msg = socket.recv() => {
                match maybe_msg {
                    None => {
                        // Socket closed by the client.
                        break;
                    }
                    Some(Err(_)) => {
                        // Transport error — treat as close.
                        break;
                    }
                    Some(Ok(Message::Close(_))) => {
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        // Respond to client-initiated pings.
                        let _ = socket.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Pong(_))) => {
                        // Client responded to our keepalive ping.
                        awaiting_pong = false;
                        pong_deadline = None;
                    }
                    Some(Ok(Message::Text(text))) => {
                        // Check rate limit first.
                        if !rate_limiter.check_and_increment() {
                            let err = ServerMessage::now(ServerPayload::Error {
                                code: "rate_limit_exceeded".into(),
                                message: "Message rate limit exceeded (60/min). Connection closing.".into(),
                            });
                            if let Ok(json) = serde_json::to_string(&err) {
                                let _ = socket.send(Message::Text(json)).await;
                            }
                            let _ = socket
                                .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                                    code: 1008,
                                    reason: "rate_limit_exceeded".into(),
                                })))
                                .await;
                            break;
                        }

                        // Parse the client message.
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Err(_) => {
                                // Could be unknown action or malformed JSON.
                                // Try to detect unknown action vs malformed subscription.
                                let reply = classify_parse_error(&text);
                                send_server_message(&mut socket, reply).await;
                            }
                            Ok(ClientMessage::Subscribe { subscription }) => {
                                handle_subscribe(
                                    &mut socket,
                                    conn_id,
                                    &registry,
                                    tx_for_registry.clone(),
                                    subscription.base,
                                    subscription.quote,
                                    subscription.amount,
                                )
                                .await;
                            }
                            Ok(ClientMessage::Unsubscribe { subscription_id }) => {
                                registry.write().await.remove_subscription(conn_id, subscription_id);
                                // No reply required per spec.
                            }
                        }
                    }
                    Some(Ok(Message::Binary(_))) => {
                        // Binary frames are not part of the protocol; ignore.
                    }
                }
            }

            // ----------------------------------------------------------------
            // Outbound message from the broadcaster / other tasks
            // ----------------------------------------------------------------
            maybe_outbound = rx.recv() => {
                match maybe_outbound {
                    None => {
                        // Channel closed — nothing more to send; exit.
                        break;
                    }
                    Some(msg) => {
                        // Reset backpressure watchdog since we successfully drained one message.
                        backpressure_since = None;

                        if let Ok(json) = serde_json::to_string(&msg) {
                            if socket.send(Message::Text(json)).await.is_err() {
                                break;
                            }
                        }

                        // After draining, check if the channel is still full.
                        update_backpressure_state(&tx, &mut backpressure_since);
                    }
                }
            }

            // ----------------------------------------------------------------
            // Ping timer — send a keepalive ping every 30 s
            // ----------------------------------------------------------------
            _ = ping_timer.tick() => {
                if awaiting_pong {
                    // Previous ping was never answered — close.
                    let _ = socket
                        .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                            code: 1008,
                            reason: "pong_timeout".into(),
                        })))
                        .await;
                    break;
                }
                let _ = socket.send(Message::Ping(vec![])).await;
                awaiting_pong = true;
                pong_deadline = Some(Instant::now() + PONG_TIMEOUT);
            }

            // ----------------------------------------------------------------
            // Pong watchdog — close if pong not received within 10 s
            // ----------------------------------------------------------------
            _ = async {
                if let Some(remaining) = pong_remaining {
                    sleep(remaining).await;
                } else {
                    // No pong pending — sleep forever (will be cancelled by other arms).
                    std::future::pending::<()>().await;
                }
            } => {
                if awaiting_pong {
                    let _ = socket
                        .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                            code: 1008,
                            reason: "pong_timeout".into(),
                        })))
                        .await;
                    break;
                }
            }

            // ----------------------------------------------------------------
            // Backpressure watchdog — close if channel full for >10 s
            // ----------------------------------------------------------------
            _ = async {
                if let Some(remaining) = bp_remaining {
                    sleep(remaining).await;
                } else {
                    std::future::pending::<()>().await;
                }
            } => {
                // Channel has been full for >10 s — close with 1008.
                let _ = socket
                    .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                        code: 1008,
                        reason: "backpressure_timeout".into(),
                    })))
                    .await;
                tracing::warn!(conn_id = %conn_id, "closing connection due to backpressure timeout");
                break;
            }
        }

        // After each iteration, check whether the outbound channel is full
        // and update the backpressure watchdog accordingly.
        update_backpressure_state(&tx, &mut backpressure_since);
    }

    // -----------------------------------------------------------------------
    // Cleanup
    // -----------------------------------------------------------------------
    registry.write().await.remove_connection(conn_id);
    connection_counter.fetch_sub(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Send a [`ServerMessage`] as a UTF-8 JSON text frame.
async fn send_server_message(socket: &mut WebSocket, msg: ServerMessage) {
    if let Ok(json) = serde_json::to_string(&msg) {
        let _ = socket.send(Message::Text(json)).await;
    }
}

/// Classify a JSON parse failure as either `unknown_action` or
/// `invalid_subscription` and return the appropriate error message.
fn classify_parse_error(raw: &str) -> ServerMessage {
    // Try to parse as a raw JSON value to inspect the `action` field.
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) {
        if let Some(action) = v.get("action").and_then(|a| a.as_str()) {
            if action == "subscribe" || action == "unsubscribe" {
                // Recognised action but malformed body.
                return ServerMessage::now(ServerPayload::Error {
                    code: "invalid_subscription".into(),
                    message: "Subscription object is missing or malformed.".into(),
                });
            }
            // Unrecognised action value.
            return ServerMessage::now(ServerPayload::Error {
                code: "unknown_action".into(),
                message: format!("Unknown action: '{action}'."),
            });
        }
    }
    // Completely unparseable JSON or missing action field.
    ServerMessage::now(ServerPayload::Error {
        code: "unknown_action".into(),
        message: "Could not parse client message.".into(),
    })
}

/// Handle a `subscribe` action from the client.
async fn handle_subscribe(
    socket: &mut WebSocket,
    conn_id: ConnId,
    registry: &Arc<RwLock<SubscriptionRegistry>>,
    tx: mpsc::Sender<ServerMessage>,
    base: String,
    quote: String,
    amount: Option<String>,
) {
    let sub_id: Uuid = Uuid::new_v4();
    let sub = Subscription {
        id: sub_id,
        base,
        quote,
        amount,
        last_emitted_price: None,
    };

    let result = registry.write().await.add_subscription(conn_id, tx, sub);

    let reply = match result {
        Ok(()) => ServerMessage::now(ServerPayload::SubscriptionConfirmed {
            subscription_id: sub_id,
        }),
        Err("subscription_limit_exceeded") => ServerMessage::now(ServerPayload::Error {
            code: "subscription_limit_exceeded".into(),
            message: "Maximum of 20 subscriptions per connection reached.".into(),
        }),
        Err(other) => ServerMessage::now(ServerPayload::Error {
            code: other.into(),
            message: "Failed to register subscription.".into(),
        }),
    };

    send_server_message(socket, reply).await;
}

/// Update the backpressure watchdog based on whether the outbound channel is
/// currently full.
///
/// Uses `try_send` with a dummy probe — if it returns `TrySendError::Full`
/// the channel is at capacity.
fn update_backpressure_state(
    tx: &mpsc::Sender<ServerMessage>,
    backpressure_since: &mut Option<Instant>,
) {
    // A channel is "full" when its available capacity is 0.
    if tx.capacity() == 0 {
        if backpressure_since.is_none() {
            *backpressure_since = Some(Instant::now());
        }
    } else {
        *backpressure_since = None;
    }
}
