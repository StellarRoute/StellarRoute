//! Integration tests for the WebSocket quote stream endpoint.
//!
//! Unit tests (1–9) run without any external dependencies.
//! Live tests (10–12) require DATABASE_URL and are `#[ignore]`:
//!   DATABASE_URL=postgres://... cargo test -p stellarroute-api --test ws_integration -- --ignored

use serde_json::{Value, json};
use stellarroute_api::routes::ws::{
    messages::{ClientMessage, ServerMessage, ServerPayload, SubscriptionRequest},
    registry::{ConnId, Subscription, SubscriptionRegistry},
    rate_limit::MessageRateLimiter,
};
use tokio::sync::mpsc;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// 1. test_ws_message_types_serialize_correctly
// ---------------------------------------------------------------------------

#[test]
fn test_ws_message_types_serialize_correctly() {
    let sub_id = Uuid::new_v4();
    let msg = ServerMessage {
        v: 1,
        timestamp: 1_700_000_000_000,
        payload: ServerPayload::SubscriptionConfirmed { subscription_id: sub_id },
    };

    let json: Value = serde_json::to_value(&msg).expect("serialization failed");

    assert_eq!(json["v"], 1, "v must be 1");
    assert_eq!(json["timestamp"], 1_700_000_000_000i64, "timestamp must match");
    assert_eq!(json["type"], "subscription_confirmed", "type field must be present");
    assert_eq!(json["subscription_id"], sub_id.to_string());
}

// ---------------------------------------------------------------------------
// 2. test_client_message_subscribe_deserializes
// ---------------------------------------------------------------------------

#[test]
fn test_client_message_subscribe_deserializes() {
    let raw = json!({
        "action": "subscribe",
        "subscription": {
            "base": "native",
            "quote": "USDC:GABC",
            "amount": "100"
        }
    });

    let msg: ClientMessage =
        serde_json::from_value(raw).expect("subscribe deserialization failed");

    match msg {
        ClientMessage::Subscribe { subscription } => {
            assert_eq!(subscription.base, "native");
            assert_eq!(subscription.quote, "USDC:GABC");
            assert_eq!(subscription.amount.as_deref(), Some("100"));
        }
        _ => panic!("expected Subscribe variant"),
    }
}

// ---------------------------------------------------------------------------
// 3. test_client_message_unsubscribe_deserializes
// ---------------------------------------------------------------------------

#[test]
fn test_client_message_unsubscribe_deserializes() {
    let sub_id = Uuid::new_v4();
    let raw = json!({
        "action": "unsubscribe",
        "subscription_id": sub_id.to_string()
    });

    let msg: ClientMessage =
        serde_json::from_value(raw).expect("unsubscribe deserialization failed");

    match msg {
        ClientMessage::Unsubscribe { subscription_id } => {
            assert_eq!(subscription_id, sub_id);
        }
        _ => panic!("expected Unsubscribe variant"),
    }
}

// ---------------------------------------------------------------------------
// 4. test_server_message_error_shape
// ---------------------------------------------------------------------------

#[test]
fn test_server_message_error_shape() {
    let msg = ServerMessage::now(ServerPayload::Error {
        code: "no_route_found".to_string(),
        message: "No liquidity for this pair.".to_string(),
    });

    let json: Value = serde_json::to_value(&msg).expect("serialization failed");

    assert_eq!(json["v"], 1);
    assert_eq!(json["type"], "error");
    assert_eq!(json["code"], "no_route_found");
    assert!(json["message"].as_str().is_some(), "message must be a string");
    assert!(json["timestamp"].as_i64().is_some(), "timestamp must be an integer");
}

// ---------------------------------------------------------------------------
// 5. test_subscription_registry_add_and_remove
// ---------------------------------------------------------------------------

#[test]
fn test_subscription_registry_add_and_remove() {
    let mut registry = SubscriptionRegistry::new();
    let conn_id: ConnId = Uuid::new_v4();
    let sub_id: Uuid = Uuid::new_v4();

    let (tx, _rx) = mpsc::channel::<ServerMessage>(32);

    let sub = Subscription {
        id: sub_id,
        base: "native".to_string(),
        quote: "USDC:GABC".to_string(),
        amount: None,
        last_emitted_price: None,
    };

    registry
        .add_subscription(conn_id, tx, sub)
        .expect("add_subscription should succeed");

    // Verify it's there
    let pairs = registry.get_connections_for_pair("native", "USDC:GABC");
    assert_eq!(pairs.len(), 1, "should have one subscription");
    assert_eq!(pairs[0].2.id, sub_id);

    // Remove it
    registry.remove_subscription(conn_id, sub_id);

    let pairs_after = registry.get_connections_for_pair("native", "USDC:GABC");
    assert!(pairs_after.is_empty(), "subscription should be removed");
}

// ---------------------------------------------------------------------------
// 6. test_subscription_registry_limit
// ---------------------------------------------------------------------------

#[test]
fn test_subscription_registry_limit() {
    let mut registry = SubscriptionRegistry::new();
    let conn_id: ConnId = Uuid::new_v4();
    let (tx, _rx) = mpsc::channel::<ServerMessage>(32);

    // Add 20 subscriptions — all should succeed
    for i in 0..20 {
        let sub = Subscription {
            id: Uuid::new_v4(),
            base: format!("BASE{i}"),
            quote: "USDC:GABC".to_string(),
            amount: None,
            last_emitted_price: None,
        };
        registry
            .add_subscription(conn_id, tx.clone(), sub)
            .unwrap_or_else(|e| panic!("subscription {i} should succeed, got: {e}"));
    }

    // 21st subscription must fail
    let sub_21 = Subscription {
        id: Uuid::new_v4(),
        base: "BASE20".to_string(),
        quote: "USDC:GABC".to_string(),
        amount: None,
        last_emitted_price: None,
    };
    let result = registry.add_subscription(conn_id, tx.clone(), sub_21);
    assert!(result.is_err(), "21st subscription should return an error");
    assert_eq!(result.unwrap_err(), "subscription_limit_exceeded");
}

// ---------------------------------------------------------------------------
// 7. test_subscription_registry_connection_cleanup
// ---------------------------------------------------------------------------

#[test]
fn test_subscription_registry_connection_cleanup() {
    let mut registry = SubscriptionRegistry::new();
    let conn_id: ConnId = Uuid::new_v4();
    let (tx, _rx) = mpsc::channel::<ServerMessage>(32);

    // Add a few subscriptions
    for i in 0..3 {
        let sub = Subscription {
            id: Uuid::new_v4(),
            base: format!("BASE{i}"),
            quote: "USDC:GABC".to_string(),
            amount: None,
            last_emitted_price: None,
        };
        registry.add_subscription(conn_id, tx.clone(), sub).unwrap();
    }

    // Verify they exist
    let all_pairs = registry.all_pairs();
    assert_eq!(all_pairs.len(), 3);

    // Remove the connection
    registry.remove_connection(conn_id);

    // All subscriptions should be gone
    let all_pairs_after = registry.all_pairs();
    assert!(all_pairs_after.is_empty(), "all subscriptions should be cleared after remove_connection");
}

// ---------------------------------------------------------------------------
// 8. test_rate_limiter_allows_60_messages
// ---------------------------------------------------------------------------

#[test]
fn test_rate_limiter_allows_60_messages() {
    let mut limiter = MessageRateLimiter::new();
    for i in 0..60 {
        assert!(
            limiter.check_and_increment(),
            "message {i} should be allowed"
        );
    }
}

// ---------------------------------------------------------------------------
// 9. test_rate_limiter_rejects_61st
// ---------------------------------------------------------------------------

#[test]
fn test_rate_limiter_rejects_61st() {
    let mut limiter = MessageRateLimiter::new();
    for _ in 0..60 {
        limiter.check_and_increment();
    }
    assert!(
        !limiter.check_and_increment(),
        "61st message should be rejected"
    );
}

// ---------------------------------------------------------------------------
// Live tests (require DATABASE_URL + running PostgreSQL)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod live {
    use std::sync::Arc;

    use axum::Router;
    use futures_util::{SinkExt, StreamExt};
    use serde_json::{Value, json};
    use sqlx::PgPool;
    use stellarroute_api::state::AppState;
    use stellarroute_api::server::{Server, ServerConfig};
    use stellarroute_api::routes::ws::WsState;
    use tokio::net::TcpListener;
    use tokio_tungstenite::{connect_async, tungstenite::Message};

    fn default_db_url() -> String {
        std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
        })
    }

    /// Spin up a real TCP listener with the full app router and return its address.
    async fn spawn_test_server(pool: PgPool, max_connections: usize) -> String {
        let ws_state = {
            // Override max_connections for the test
            use std::sync::atomic::AtomicUsize;
            use std::collections::HashMap;
            use tokio::sync::Mutex;
            use stellarroute_api::routes::ws::registry::SubscriptionRegistry;

            Arc::new(stellarroute_api::routes::ws::WsState {
                registry: SubscriptionRegistry::shared(),
                connection_counter: Arc::new(AtomicUsize::new(0)),
                max_connections,
                ip_rate_limiter: Arc::new(Mutex::new(HashMap::new())),
                poll_interval_ms: 1000,
                ping_interval_secs: 30,
                pong_timeout_secs: 10,
                backpressure_timeout_secs: 10,
            })
        };

        let state = Arc::new(AppState::new(pool).with_ws(ws_state));
        let router = stellarroute_api::routes::create_router(state);

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("failed to bind");
        let addr = listener.local_addr().unwrap().to_string();

        tokio::spawn(async move {
            axum::serve(
                listener,
                router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
            )
            .await
            .unwrap();
        });

        addr
    }

    // -----------------------------------------------------------------------
    // 10. test_ws_upgrade_succeeds
    // -----------------------------------------------------------------------

    #[tokio::test]
    #[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
    async fn test_ws_upgrade_succeeds() {
        let pool = PgPool::connect(&default_db_url())
            .await
            .expect("failed to connect to database");

        let addr = spawn_test_server(pool, 500).await;
        let url = format!("ws://{addr}/ws");

        let (mut ws, response) = connect_async(&url)
            .await
            .expect("WebSocket upgrade failed");

        // HTTP 101 Switching Protocols is indicated by a successful connect
        assert_eq!(
            response.status(),
            tokio_tungstenite::tungstenite::http::StatusCode::SWITCHING_PROTOCOLS
        );

        ws.close(None).await.ok();
    }

    // -----------------------------------------------------------------------
    // 11. test_connection_limit_returns_503
    // -----------------------------------------------------------------------

    #[tokio::test]
    #[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
    async fn test_connection_limit_returns_503() {
        let pool = PgPool::connect(&default_db_url())
            .await
            .expect("failed to connect to database");

        // max_connections = 1 so the second attempt is rejected
        let addr = spawn_test_server(pool, 1).await;
        let url = format!("ws://{addr}/ws");

        // First connection should succeed
        let (mut ws1, _) = connect_async(&url)
            .await
            .expect("first WebSocket upgrade should succeed");

        // Second connection should be rejected with 503
        let result = connect_async(&url).await;
        assert!(result.is_err(), "second connection should be rejected");

        if let Err(e) = result {
            let msg = e.to_string();
            assert!(
                msg.contains("503") || msg.contains("Service Unavailable"),
                "expected 503, got: {msg}"
            );
        }

        ws1.close(None).await.ok();
    }

    // -----------------------------------------------------------------------
    // 12. test_full_lifecycle
    // -----------------------------------------------------------------------

    #[tokio::test]
    #[ignore = "requires a running PostgreSQL instance (set DATABASE_URL)"]
    async fn test_full_lifecycle() {
        let pool = PgPool::connect(&default_db_url())
            .await
            .expect("failed to connect to database");

        let addr = spawn_test_server(pool, 500).await;
        let url = format!("ws://{addr}/ws");

        let (mut ws, _) = connect_async(&url)
            .await
            .expect("WebSocket upgrade failed");

        // Send a subscribe message
        let subscribe_msg = json!({
            "action": "subscribe",
            "subscription": {
                "base": "native",
                "quote": "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
            }
        });

        ws.send(Message::Text(subscribe_msg.to_string().into()))
            .await
            .expect("failed to send subscribe");

        // Expect subscription_confirmed
        let msg = ws
            .next()
            .await
            .expect("no message received")
            .expect("ws error");

        let text = match msg {
            Message::Text(t) => t.to_string(),
            other => panic!("expected text frame, got: {other:?}"),
        };

        let json: Value = serde_json::from_str(&text).expect("invalid JSON");
        assert_eq!(json["v"], 1, "v must be 1");
        assert_eq!(json["type"], "subscription_confirmed", "expected subscription_confirmed");
        assert!(json["subscription_id"].as_str().is_some(), "subscription_id must be present");

        // Graceful disconnect
        ws.close(None).await.ok();
    }
}
