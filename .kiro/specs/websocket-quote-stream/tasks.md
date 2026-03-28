# Implementation Plan: WebSocket Quote Stream

## Overview

Implement the WebSocket quote streaming endpoint for the StellarRoute API in `crates/api`. The work is broken into discrete layers: message types, subscription registry, connection task, broadcaster, rate limiting, route wiring, and integration tests.

## Tasks

- [x] 1. Define message types and serialization
  - [x] 1.1 Create `crates/api/src/routes/ws/messages.rs` with `ClientMessage`, `ServerMessage`, `ServerPayload`, and `SubscriptionRequest` types
    - Implement `Serialize`/`Deserialize` derives with `serde(tag = "action")` for `ClientMessage` and `serde(tag = "type")` for `ServerPayload`
    - `ServerMessage` must include `v: u8 = 1` and `timestamp: i64` (Unix ms) fields
    - _Requirements: 3.1, 3.2, 3.5_
  - [ ]* 1.2 Write property test `prop_server_message_envelope_fields` in `messages.rs`
    - **Property 1: ServerMessage envelope invariant**
    - **Validates: Requirements 3.1, 3.5**
    - Tag: `// Feature: websocket-quote-stream, Property 1: ServerMessage envelope invariant`
  - [ ]* 1.3 Write property test `prop_server_message_round_trip` in `messages.rs`
    - **Property 2: ServerMessage serialization round-trip**
    - **Validates: Requirements 3.2, 7.6**
    - Tag: `// Feature: websocket-quote-stream, Property 2: ServerMessage serialization round-trip`

- [x] 2. Implement subscription registry
  - [x] 2.1 Create `crates/api/src/routes/ws/registry.rs` with `SubscriptionRegistry`, `ConnectionEntry`, and `Subscription` structs
    - `SubscriptionRegistry` wraps `HashMap<ConnId, ConnectionEntry>` behind `Arc<RwLock<...>>`
    - `ConnectionEntry` holds `subscriptions: Vec<Subscription>` and `tx: mpsc::Sender<ServerMessage>`
    - `Subscription` holds `id`, `base`, `quote`, `amount: Option<String>`, `last_emitted_price: Option<f64>`
    - Implement `add_subscription`, `remove_subscription`, `remove_connection`, and `get_connections_for_pair` methods
    - _Requirements: 2.1, 2.2, 2.5, 2.6_
  - [ ]* 2.2 Write property test `prop_subscribe_unsubscribe_removes_entry` in `registry.rs`
    - **Property 4: Subscribe then unsubscribe removes subscription**
    - **Validates: Requirements 2.2, 7.4**
    - Tag: `// Feature: websocket-quote-stream, Property 4: Subscribe then unsubscribe removes subscription`
  - [ ]* 2.3 Write property test `prop_connection_cleanup_on_close` in `registry.rs`
    - **Property 7: Connection cleanup on close**
    - **Validates: Requirements 1.5, 2.6**
    - Tag: `// Feature: websocket-quote-stream, Property 7: Connection cleanup on close`

- [x] 3. Implement per-connection message rate limiter
  - [x] 3.1 Create `crates/api/src/routes/ws/rate_limit.rs` with `MessageRateLimiter`
    - Sliding-window counter: 60 messages per 60-second window
    - `check_and_increment(&mut self) -> bool` returns `true` if within limit
    - _Requirements: 5.2_

- [x] 4. Implement the connection task
  - [x] 4.1 Create `crates/api/src/routes/ws/connection.rs` with `ConnectionTask` and `run_connection` async function
    - `tokio::select!` loop over: inbound WS frames, outbound `mpsc::Receiver<ServerMessage>`, ping timer (30 s), pong watchdog (10 s), backpressure watchdog (10 s)
    - Parse inbound frames as `ClientMessage` JSON; dispatch to subscribe/unsubscribe handlers
    - Enforce 20-subscription-per-connection limit; reply with appropriate `ServerMessage` errors
    - Enforce per-connection message rate limit via `MessageRateLimiter`; send `rate_limit_exceeded` error and close with code `1008` on breach
    - Send outbound `ServerMessage` values as UTF-8 JSON text frames
    - On connection close, call `registry.remove_connection` and decrement `ConnectionCounter`
    - _Requirements: 1.5, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 5.2, 6.3, 6.4_
  - [ ]* 4.2 Write property test `prop_unknown_action_returns_error` in `connection.rs`
    - **Property 5: Unknown action returns error**
    - **Validates: Requirements 2.3**
    - Tag: `// Feature: websocket-quote-stream, Property 5: Unknown action returns error`
  - [ ]* 4.3 Write property test `prop_malformed_subscription_returns_error` in `connection.rs`
    - **Property 6: Malformed subscription returns error**
    - **Validates: Requirements 2.4**
    - Tag: `// Feature: websocket-quote-stream, Property 6: Malformed subscription returns error`
  - [ ]* 4.4 Write property test `prop_backpressure_drop_policy` in `connection.rs`
    - **Property 10: Backpressure drop policy**
    - **Validates: Requirements 6.1, 6.2**
    - Tag: `// Feature: websocket-quote-stream, Property 10: Backpressure drop policy`
  - [ ]* 4.5 Write property test `prop_backpressure_timeout_closes` in `connection.rs`
    - **Property 11: Backpressure timeout closes connection**
    - **Validates: Requirements 6.3**
    - Tag: `// Feature: websocket-quote-stream, Property 11: Backpressure timeout closes connection`
  - [ ]* 4.6 Write property test `prop_pong_timeout_closes` in `connection.rs`
    - **Property 12: Ping/pong timeout closes connection**
    - **Validates: Requirements 6.4**
    - Tag: `// Feature: websocket-quote-stream, Property 12: Ping/pong timeout closes connection`

- [x] 5. Checkpoint — Ensure all unit tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 6. Implement the quote broadcaster
  - [x] 6.1 Create `crates/api/src/routes/ws/broadcaster.rs` with `QuoteBroadcaster` and `run_broadcaster` async function
    - Poll loop sleeping `WS_POLL_INTERVAL_MS` (default 1000 ms)
    - Read all subscriptions from registry (read lock), group by `(base, quote)` pair
    - Call `get_liquidity_revision` to detect ledger changes; call extracted `compute_quote` from `routes/quote.rs` on change or first poll after subscription
    - Apply dedup: skip emission if price unchanged; apply 0.01% threshold when `amount` is set
    - Send `ServerMessage::QuoteUpdate` to each matching connection's `mpsc::Sender`; on `Err` (channel full/closed) apply drop policy or remove dead connection
    - Emit `error { code: "no_route_found" }` when `compute_quote` returns `ApiError::NoRouteFound`
    - Wrap task in restart loop: on panic, log error and restart after 1 s
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 6.1, 6.2_
  - [ ]* 6.2 Write property test `prop_dedup_threshold` in `broadcaster.rs`
    - **Property 8: Amount-based dedup threshold**
    - **Validates: Requirements 2.7, 4.4**
    - Tag: `// Feature: websocket-quote-stream, Property 8: Amount-based dedup threshold`

- [x] 7. Implement `WsState` and wire into `AppState`
  - [x] 7.1 Create `WsState` struct in `crates/api/src/routes/ws/mod.rs`
    - Fields: `registry`, `connection_counter: Arc<AtomicUsize>`, `max_connections: usize`, `ip_rate_limiter`
    - Read `WS_MAX_CONNECTIONS` (default 500), `WS_POLL_INTERVAL_MS`, `WS_PING_INTERVAL_SECS`, `WS_PONG_TIMEOUT_SECS`, `WS_BACKPRESSURE_TIMEOUT_SECS` from environment
    - _Requirements: 5.3_
  - [x] 7.2 Add `pub ws: Option<Arc<WsState>>` field to `AppState` in `crates/api/src/state.rs`
    - _Requirements: 1.4_

- [x] 8. Implement the WS upgrade handler and register the route
  - [x] 8.1 Implement `ws_handler` in `crates/api/src/routes/ws/mod.rs`
    - Check `connection_counter` against `max_connections`; return HTTP 503 JSON on breach
    - Check IP rate limit (10 new conns/min); return HTTP 429 on breach
    - Increment `connection_counter`, call `.on_upgrade(move |socket| run_connection(...))`, spawn `run_broadcaster` if not already running
    - _Requirements: 1.1, 1.2, 1.3, 5.1, 5.3, 5.4_
  - [x] 8.2 Register `.route("/ws", get(ws::ws_handler))` in `crates/api/src/routes/mod.rs`
    - _Requirements: 1.4_

- [x] 9. Checkpoint — Ensure all unit tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 10. Write integration tests
  - [x] 10.1 Create `crates/api/tests/ws_integration.rs` with unit tests covering the full lifecycle
    - `test_ws_upgrade_succeeds` — Req 1.1, 1.2
    - `test_connection_limit_returns_503` — Req 1.3, 5.3
    - `test_subscribe_invalid_pair_returns_no_route` — Req 4.5, 7.2
    - `test_subscription_limit_exceeded` — Req 2.5, 7.3
    - `test_full_lifecycle` — Req 7.1
    - `test_message_rate_limit_closes_connection` — Req 5.2, 7.5
    - `test_ip_rate_limit_rejects_connections` — Req 5.1
    - `test_channel_capacity_is_32` — Req 6.1
    - Use `tokio-tungstenite` for WS client connections
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_
  - [ ]* 10.2 Write property test `prop_subscribe_returns_confirmed` in `ws_integration.rs`
    - **Property 3: Subscribe produces subscription_confirmed**
    - **Validates: Requirements 2.1**
    - Tag: `// Feature: websocket-quote-stream, Property 3: Subscribe produces subscription_confirmed`
  - [ ]* 10.3 Write property test `prop_initial_quote_update_on_subscribe` in `ws_integration.rs`
    - **Property 9: Initial quote_update on subscribe**
    - **Validates: Requirements 4.3**
    - Tag: `// Feature: websocket-quote-stream, Property 9: Initial quote_update on subscribe`

- [x] 11. Final checkpoint — Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for a faster MVP
- Each task references specific requirements for traceability
- Property tests use `proptest = "1"` and must include the feature/property comment tag
- Integration tests use `tokio-tungstenite` for WebSocket client connections
