# WebSocket Quote Stream API

The StellarRoute WebSocket endpoint provides real-time quote updates for trading pairs. This document describes the connection protocol, message formats, subscription model, and operational guidelines.

---

## Endpoint

**URL**: `ws://localhost:8000/api/v1/stream` (local development)  
**URL**: `wss://api.stellarroute.io/api/v1/stream` (production)  
**Scheme**: WebSocket (RFC 6455)

---

## Connection Lifecycle

### 1. Upgrade Handshake

The connection begins with an HTTP upgrade:

```http
GET /api/v1/stream HTTP/1.1
Host: api.stellarroute.io
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Key: x3JJHMbDL1EzLkh9GBhXDw==
Sec-WebSocket-Version: 13
```

**Possible Responses:**

| Status | Condition | Description |
|--------|-----------|-------------|
| `101 Switching Protocols` | Connection accepted | Upgrade successful; WebSocket is now open. |
| `429 Too Many Requests` | IP rate limit exceeded | Too many new connections from this IP in the last 60 seconds (max 10/min). Retry after 60 seconds. |
| `503 Service Unavailable` | Server at connection limit | Server has reached `WS_MAX_CONNECTIONS` (default 500). Or endpoint is disabled (`WS_ENABLED=false`). |

### 2. Connection Active

Once open, the client may:
- Send `subscribe` messages to register for pair updates
- Send `unsubscribe` messages to remove subscriptions
- Receive `quote_update`, error, and keepalive `ping` messages

### 3. Connection Close

The connection closes when:
- **Client initiated**: Client sends a WebSocket close frame or the TCP socket closes.
- **Server initiated**: Server sends a close frame with a specific code (see [Close Codes](#close-codes)).

---

## Message Protocol

All messages are UTF-8 encoded JSON text frames. Binary frames are ignored.

### Version & Timestamps

Every server-sent message includes:
- `v` (integer): Schema version — always `1` in current implementation.
- `timestamp` (integer): Unix time in milliseconds when the message was produced.

---

## Client → Server Messages

### Subscribe

Subscribe to real-time quote updates for a trading pair.

**Format:**
```json
{
  "action": "subscribe",
  "subscription": {
    "base": "native",
    "quote": "USDC:GABD...",
    "amount": "1000"
  }
}
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `action` | string | Yes | Always `"subscribe"` |
| `subscription.base` | string | Yes | Base asset identifier: `"native"` for XLM, or `"CODE:ISSUER"` for issued assets. |
| `subscription.quote` | string | Yes | Quote asset identifier: `"native"` or `"CODE:ISSUER"`. |
| `subscription.amount` | string | No | Positive decimal string (e.g., `"1000.50"`). When present, quote updates are only emitted if the quoted amount changes by more than **0.01%** relative to the previous emission. Enables client-side deduplication of small price movements. |

**Response:**

On success, server sends:
```json
{
  "v": 1,
  "timestamp": 1700000000000,
  "type": "subscription_confirmed",
  "subscription_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

Use the returned `subscription_id` to later unsubscribe.

**Error Responses:**

| Code | Reason |
|------|--------|
| `invalid_asset` | Base or quote asset identifier is not recognized. |
| `invalid_subscription` | Subscription object is malformed or missing required fields. |
| `too_many_subscriptions` | Connection has reached the max subscriptions per connection (default 10). |

---

### Unsubscribe

Cancel an existing subscription and stop receiving updates for that pair.

**Format:**
```json
{
  "action": "unsubscribe",
  "subscription_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `action` | string | Yes | Always `"unsubscribe"` |
| `subscription_id` | string (UUID) | Yes | The subscription ID returned in the `subscription_confirmed` message. |

**Response:**

On success, no message is sent. The subscription is silently removed.

On invalid subscription ID, server may silently ignore or send an error (implementation-dependent).

---

### Ping

Send a ping frame to keep the connection alive or test responsiveness. WebSocket protocol ping frames are supported in addition to application-level messaging.

**Format:**
```
(WebSocket PING frame)
```

**Response:**

Server responds with a WebSocket PONG frame.

---

## Server → Client Messages

All server messages follow this envelope:
```json
{
  "v": 1,
  "timestamp": <milliseconds>,
  "type": "<message_type>",
  ...payload fields flattened here...
}
```

### Subscription Confirmed

Sent immediately after a successful subscribe request.

**Format:**
```json
{
  "v": 1,
  "timestamp": 1700000000000,
  "type": "subscription_confirmed",
  "subscription_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `subscription_id` | string (UUID) | Unique identifier for this subscription. Use in unsubscribe requests. |

---

### Quote Update

Sent whenever a new quote is computed for a subscribed pair.

**Format:**
```json
{
  "v": 1,
  "timestamp": 1700000000000,
  "type": "quote_update",
  "subscription_id": "550e8400-e29b-41d4-a716-446655440000",
  "quote": {
    "base": "native",
    "quote": "USDC:GA...",
    "amount": "1000.00",
    "total_price": "0.85",
    "venue": "SDEX",
    "path": [...],
    "execution_price": "0.8501",
    "price_impact": "0.12",
    "timestamp": 1700000000000
  }
}
```

**Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `subscription_id` | string (UUID) | Matches the subscription this update belongs to. |
| `quote` | object | Quote response object (see [REST API docs](../api) for full structure). Contains asset pair, exchange rate, execution venue, optimal path, and slippage estimates. |

**Frequency:**

Updates are sent according to the configured poll interval (`WS_POLL_INTERVAL_MS`, default 1000 ms). If an `amount` filter was set during subscription, updates are deduplicated: quotes are only sent if the price changed by more than 0.01% since the last emission.

---

### Error

Sent when an error occurs (e.g., invalid subscription request, rate limit hit).

**Format:**
```json
{
  "v": 1,
  "timestamp": 1700000000000,
  "type": "error",
  "code": "invalid_asset",
  "message": "Asset 'INVALID:CODE' not found"
}
```

**Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `code` | string | Machine-readable error code (e.g., `invalid_asset`, `rate_limit_exceeded`). |
| `message` | string | Human-readable error description. |

**Common Codes:**

See [API Error Taxonomy](error_taxonomy.md) for a full list. WebSocket-specific codes include:

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `invalid_asset` | 400 | One or both assets in the subscription are not recognized. |
| `invalid_subscription` | 400 | Subscription object is malformed or missing required fields. |
| `unknown_action` | 400 | The `action` field is not `subscribe` or `unsubscribe`. |
| `rate_limit_exceeded` | 429 | Per-connection message rate limit exceeded (60 messages/min). Connection will be closed. |
| `too_many_subscriptions` | 400 | Connection subscriptions exceed the per-connection limit (default 10). |

---

### Keepalive Ping

Sent every 30 seconds to ensure the connection remains active and to detect stale connections.

**Format:**
```json
{
  "v": 1,
  "timestamp": 1700000000000,
  "type": "ping"
}
```

**Response:**

The client should respond with a WebSocket PONG frame within 10 seconds. If the server does not receive a PONG, the connection is closed with code `1008` (policy violation) and reason `"pong_timeout"`.

---

## Subscription Model

### Connection-Level Subscriptions

Each WebSocket connection maintains its own set of subscriptions. Subscriptions are **not** shared across connections.

### Per-Connection Limits

| Parameter | Default | Env Variable | Description |
|-----------|---------|--------------|-------------|
| Max subscriptions per connection | 10 | `WS_MAX_SUBSCRIPTIONS` | Maximum number of active subscriptions per connection. |
| Max concurrent connections | 500 | `WS_MAX_CONNECTIONS` | Maximum total concurrent WebSocket connections across all clients. |

### Reconnection Guidance

**If disconnected:**

1. Wait 1–5 seconds (exponential backoff) before attempting to reconnect.
2. Check the close code (see [Close Codes](#close-codes)) to determine if reconnection makes sense.
3. Upon reconnection, re-subscribe to desired pairs (subscriptions are lost on disconnect).

**Graceful Shutdown:**

The server may initiate close with code `1001` (going away) when shutting down or rebalancing load. Clients should reconnect after a delay.

---

## Rate Limits

### Per-Connection Message Rate

**Limit**: 60 messages per 60-second sliding window.

**Behavior**: If exceeded, the server sends an error message with code `rate_limit_exceeded` and closes the connection with code `1008`.

### Per-IP New Connection Rate

**Limit**: 10 new connections per 60-second sliding window per source IP.

**Behavior**: If exceeded, the upgrade handshake returns `429 Too Many Requests`.

### Broadcast Poll Interval

**Parameter**: `WS_POLL_INTERVAL_MS` (default 1000)

**Behavior**: Server polls the database for liquidity changes every N milliseconds and broadcasts updates to all subscribed clients. Shorter intervals increase CPU/DB load; longer intervals increase quote staleness.

---

## Environment Variables

Configure WebSocket behavior via environment variables. All have sensible defaults.

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `WS_ENABLED` | bool | `true` | Enable/disable the WebSocket endpoint. If `false`, all upgrade requests return 503. |
| `WS_MAX_CONNECTIONS` | usize | `500` | Maximum concurrent WebSocket connections. New connections beyond this limit receive 503. |
| `WS_MAX_SUBSCRIPTIONS` | usize | `10` | Maximum subscriptions per connection. |
| `WS_POLL_INTERVAL_MS` | u64 | `1000` | Milliseconds between successive broadcasts of quote updates to all subscribers. |
| `WS_PING_INTERVAL_SECS` | u64 | `30` | Seconds between keepalive ping frames sent by the server. |
| `WS_PONG_TIMEOUT_SECS` | u64 | `10` | Seconds to wait for a PONG response before closing the connection (code 1008). |
| `WS_BACKPRESSURE_TIMEOUT_SECS` | u64 | `10` | Seconds the outbound message channel may remain full before the connection is closed (code 1008). Detects slow/stalled clients. |

---

## Close Codes

The server sends WebSocket close frames with specific codes and reasons to signal different disconnection scenarios:

| Code | Reason | Meaning |
|------|--------|---------|
| `1000` | Normal Closure | Graceful shutdown or client-initiated close. |
| `1001` | Going Away | Server shutting down or rebalancing (client may reconnect later). |
| `1002` | Protocol Error | Malformed WebSocket frame or protocol violation. |
| `1008` | Policy Violation | Rate limit exceeded, pong timeout, or backpressure timeout. Connection should not immediately reconnect; wait before retrying. |

**Backpressure Close Example:**
```
Close code: 1008
Reason: "backpressure_timeout"
```

This occurs when the outbound message queue has been full for longer than `WS_BACKPRESSURE_TIMEOUT_SECS`. The client is consuming messages too slowly; consider reducing subscriptions or increasing processing capacity.

---

## Examples

### JavaScript Client

Connect, subscribe to a pair, and handle real-time updates:

```javascript
const WebSocket = require('ws');

const WS_URL = 'wss://api.stellarroute.io/api/v1/stream';

const ws = new WebSocket(WS_URL);

ws.on('open', () => {
  console.log('Connected to StellarRoute WebSocket');

  // Subscribe to native/USDC updates with deduplication every 0.01%
  ws.send(JSON.stringify({
    action: 'subscribe',
    subscription: {
      base: 'native',
      quote: 'USDC:GBUQWP3BOUZX34CHATTQ7SQ3F5CI6GFGQ7VLRIFQZ4LHFCNVUSNFXFYX',
      amount: '1000.00'
    }
  }));
});

ws.on('message', (data) => {
  const msg = JSON.parse(data);

  switch (msg.type) {
    case 'subscription_confirmed':
      console.log(`Subscribed with ID: ${msg.subscription_id}`);
      break;

    case 'quote_update':
      console.log(`Update for ${msg.quote.base}/${msg.quote.quote}:`);
      console.log(`  Amount: ${msg.quote.amount}`);
      console.log(`  Price: ${msg.quote.total_price}`);
      console.log(`  Venue: ${msg.quote.venue}`);
      console.log(`  Impact: ${msg.quote.price_impact}%`);
      break;

    case 'error':
      console.error(`Error [${msg.code}]: ${msg.message}`);
      break;

    case 'ping':
      console.log('Keepalive ping received');
      break;
  }
});

ws.on('close', (code, reason) => {
  console.log(`Closed: ${code} - ${reason}`);
});

ws.on('error', (err) => {
  console.error('WebSocket error:', err);
});
```

**With TypeScript SDK:**

```typescript
import { StellarRouteClient } from '@stellarroute/sdk-js';

const client = new StellarRouteClient({
  apiUrl: 'https://api.stellarroute.io'
});

client.subscribeToQuoteStream({
  base: 'native',
  quote: 'USDC:GBUQWP3BOUZX34CHATTQ7SQ3F5CI6GFGQ7VLRIFQZ4LHFCNVUSNFXFYX',
  amount: '1000.00'
}, {
  onUpdate: (quote) => {
    console.log(`Price: ${quote.total_price}`, quote);
  },
  onError: (error) => {
    console.error('Stream error:', error);
  },
  onClose: () => {
    console.log('Stream closed');
  }
});
```

---

### cURL + wscat

Use the `wscat` CLI tool for quick WebSocket testing:

**Install wscat:**
```bash
npm install -g wscat
```

**Connect and subscribe:**
```bash
wscat -c wss://api.stellarroute.io/api/v1/stream
```

Then, in the wscat prompt, send:
```json
{"action": "subscribe", "subscription": {"base": "native", "quote": "USDC:GBUQWP3BOUZX34CHATTQ7SQ3F5CI6GFGQ7VLRIFQZ4LHFCNVUSNFXFYX", "amount": "1000"}}
```

Expected output:
```
< {
  "v": 1,
  "timestamp": 1700000000000,
  "type": "subscription_confirmed",
  "subscription_id": "550e8400-e29b-41d4-a716-446655440000"
}

< {
  "v": 1,
  "timestamp": 1700000000001,
  "type": "quote_update",
  "subscription_id": "550e8400-e29b-41d4-a716-446655440000",
  "quote": {
    "base": "native",
    "quote": "USDC:GBUQWP3BOUZX34CHATTQ7SQ3F5CI6GFGQ7VLRIFQZ4LHFCNVUSNFXFYX",
    "amount": "1000.00",
    "total_price": "0.8501",
    "venue": "SDEX",
    ...
  }
}
```

**Unsubscribe:**
```json
{"action": "unsubscribe", "subscription_id": "550e8400-e29b-41d4-a716-446655440000"}
```

---

## Implementation Notes

### Broadcasting Behavior

The quote broadcaster background task runs independently and polls the database every `WS_POLL_INTERVAL_MS`. It computes fresh quotes for every pair with active subscriptions and fans out updates to all connected clients.

**Deduplication:**

If a subscription was created with an `amount` filter, updates are only emitted if the quote price has changed by more than 0.01% since the last emission for that pair. This reduces bandwidth and message noise for stable price ranges.

### Connection Cleanup

When a connection closes:
1. All subscriptions for that connection are removed.
2. The connection counter is decremented.
3. If the broadcaster detects zero active subscriptions, it pauses polling to save resources.

### Error Handling

**Parse errors** (invalid JSON, unknown action):
- Server sends an error message with code `unknown_action` or `invalid_subscription`.
- Connection remains open (unless the error is a rate limit violation).

**Rate limit violations**:
- Server sends an error message with code `rate_limit_exceeded`.
- Connection is immediately closed with code `1008`.

**Backpressure timeout**:
- Server has detected that the outbound message buffer is full and the client is not consuming messages fast enough.
- Connection is closed with code `1008` and reason `"backpressure_timeout"`.

---

## Security Considerations

1. **TLS/SSL**: Always use `wss://` (secure WebSocket) in production. Unencrypted `ws://` should only be used for local development.

2. **Authentication**: Currently, the WebSocket endpoint does not require authentication. Rate limits and connection caps provide basic DDoS mitigation.

3. **Rate Limits**: Per-IP and per-connection rate limits prevent abuse. Monitor for unusual patterns in your client logs.

4. **Data Sensitivity**: Quote streams may leak information about your trading interest and order size. Consider privacy implications.

---

## Troubleshooting

### Connection Refused

**Cause**: WebSocket endpoint is not running or is behind a firewall.

**Solution**: Verify the server is running and the URL is correct. Check firewall rules for port 443 (wss) or 80 (ws).

### 429 Too Many Requests on Upgrade

**Cause**: Per-IP new connection rate limit exceeded (10/min).

**Solution**: Implement exponential backoff (1s, 2s, 4s, …) before retrying. Consider using a connection pool to reuse single connections.

### Pong Timeout (1008)

**Cause**: Client did not respond to server ping within 10 seconds.

**Solution**: Ensure your client has a WebSocket pong handler. On Node.js with `ws` library, pong responses are automatic; on browsers, check that the browser WebSocket implementation supports it.

### Backpressure Timeout (1008)

**Cause**: Client is consuming messages slower than the server can send them (outbound queue full for >10 seconds).

**Solution**: Reduce number of subscriptions, increase client-side message processing speed, or increase `WS_BACKPRESSURE_TIMEOUT_SECS`.

### Rate Limit Exceeded on Message

**Cause**: Sending more than 60 messages per 60 seconds.

**Solution**: Reduce subscription frequency or use the `amount` filter to deduplicate small price movements.

---

## See Also

- [API Error Taxonomy](error_taxonomy.md) — Full error code reference
- [REST API Routes](routes_endpoint.md) — Complementary REST endpoints for quotes and orderbooks
- [API Versioning](versioning.md) — Schema versioning policy

