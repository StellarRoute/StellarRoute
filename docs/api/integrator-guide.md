# Integrator API Guide

This guide documents integration-specific API behavior for partner systems.

## Quote Expiration Webhooks

Integrators can register an HTTPS webhook endpoint to receive quote expiration events.

### Register or update webhook

- **Method:** `POST`
- **Path:** `/api/v1/integrator/webhooks/quote-expiration`
- **Required header:** `X-API-Key`

Request body:

```json
{
  "webhook_url": "https://integrator.example/webhooks/quotes",
  "signing_secret": "optional-shared-secret",
  "enabled": true
}
```

Behavior:

- If `signing_secret` is omitted or blank, the API generates one and returns it once in `generated_signing_secret`.
- Registrations are stored per consumer (`X-API-Key`).
- `webhook_url` must use `https://`.

Successful response:

```json
{
  "v": 1,
  "timestamp": 1740312000000,
  "request_id": "req_01hyxk6bzv4n9p8m8j1f4c0a2r",
  "data": {
    "consumer_id": "api_key:your-key",
    "webhook_url": "https://integrator.example/webhooks/quotes",
    "enabled": true,
    "generated_signing_secret": "2d4ad5fd-99d1-4d6a-9d84-7bbcc90a2d9c"
  }
}
```

### Webhook event payload

Events are posted with JSON payload:

```json
{
  "event_id": "f30f7d86-c604-4a0a-bd4a-7381f09542f1",
  "consumer_id": "api_key:your-key",
  "quote_id": "native:USDC:1740312000000:100",
  "pair": "native/USDC",
  "reason": "ttl_expired",
  "expired_at": 1740312002000
}
```

`reason` values:

- `ttl_expired` when a quote naturally reaches its TTL.
- `cache_invalidated` when underlying liquidity changes invalidate cached quotes.

### Signature verification

Each webhook request includes:

- `X-StellarRoute-Event: quote.expired`
- `X-StellarRoute-Consumer: <consumer_id>`
- `X-StellarRoute-Signature: sha256=<hex_digest>`

Signature is computed as `HMAC-SHA256(secret, raw_request_body)`.

### Retry policy

Failed deliveries are retried with exponential backoff:

- attempt 1: immediate
- attempt 2: 500ms
- attempt 3: 1000ms
- attempt 4: 2000ms

A delivery is considered successful for any HTTP 2xx response.
