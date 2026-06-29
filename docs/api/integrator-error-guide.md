# API Integrator Error Guide

This guide helps integrators handle StellarRoute API errors in production clients, SDKs, and UIs.

## Retry vs fail-fast matrix

| Error code | HTTP status | Action | Retry guidance |
|---|---|---|---|
| `bad_request` | 400 | Fail fast | Do not retry; fix the request payload. |
| `invalid_asset` | 400 | Fail fast | Do not retry; validate asset identifiers before retrying. |
| `validation_error` | 400 | Fail fast | Do not retry; present validation messages and correct inputs. |
| `not_found` | 404 | Fail fast | Do not retry; the resource is missing or the pair is unsupported. |
| `no_route` | 404 | Fail fast | Do not retry immediately; the market is currently untradeable. |
| `stale_market_data` | 422 | Retry with refresh | Refresh market data and retry after a short delay. |
| `rate_limit_exceeded` | 429 | Retry with backoff | Retry after `Retry-After` / reset header or exponential backoff. |
| `overloaded` | 503 | Retry with backoff | Retry after a longer delay and stop after a few attempts. |
| `internal_error` | 500 | Retry carefully | Retry once or twice for transient failures, but fail fast if repeated. |

## Recommended backoff strategy

### `rate_limit_exceeded`

- Prefer `Retry-After` when the API returns it.
- Otherwise, use an exponential backoff sequence: `1s`, `2s`, `4s`.
- Limit retries to a small fixed number (e.g. 3 attempts) to avoid traffic bursts.
- Example: if you receive `X-RateLimit-Reset`, schedule the next attempt for that reset time.

### `overloaded`

- Treat this as temporary operational backpressure.
- Wait `1s` then retry, then increase to `2s`, `4s`, and stop after 3–4 attempts.
- If the service remains overloaded, surface a safe fallback path instead of retrying endlessly.

### `internal_error`

- Consider a short early retry for transient failures, but avoid retry loops.
- Example: retry once after `500ms` and give up if the same error persists.

## Handling `stale_market_data`

- This error means the quote could not be fulfilled because the API's market snapshot was stale.
- Integrators should refresh their quote input and retry after a short pause.
- Example flow:
  1. Detect `stale_market_data`.
  2. Re-fetch the latest available pair/orderbook data if applicable.
  3. Retry the quote request after `500ms` to `1s`.
  4. Fail gracefully if the error repeats more than 2–3 times.

> Note: `stale_market_data` is not the same as an invalid request. It is a transient market freshness issue.

## JS SDK examples

Use SDK helpers to branch on API errors cleanly.

```ts
import {
  StellarRouteClient,
  StellarRouteApiError,
  isStellarRouteApiError,
} from '@stellarroute/sdk-js';

const client = new StellarRouteClient({ baseUrl: 'https://api.stellarroute.io' });

try {
  await client.getQuote('native', 'USDC', 100);
} catch (err) {
  if (!isStellarRouteApiError(err)) {
    console.error('Unexpected failure', err);
    throw err;
  }

  if (err.isValidationError()) {
    console.warn('Invalid quote request:', err.message);
  } else if (err.isStaleMarketData()) {
    console.warn('Market data stale, retrying...');
    // Refresh market data and retry after a short delay.
  } else if (err.isRateLimited()) {
    console.warn('Request was rate limited, backoff and retry later.');
  } else if (err.isOverloaded()) {
    console.warn('Service overloaded, retry with backoff or degrade gracefully.');
  } else if (err.isNotFound()) {
    console.warn('Pair not available.');
  } else {
    console.error('API error:', err.code, err.message);
  }
}
```

### Useful JS SDK helpers

- `err.isNotFound()` — 404 / `not_found`
- `err.isValidationError()` — 400 / `validation_error` or `invalid_asset`
- `err.isRateLimited()` — 429 / `rate_limit_exceeded`
- `err.isOverloaded()` — 503 / `overloaded`
- `err.isStaleMarketData()` — 422 / `stale_market_data`
- `err.isNetworkError()` — network failure / timeout

## Rust SDK examples

Use the typed `SdkError` enum and the `ApiErrorCode` values.

```rust
use stellarroute_sdk::{ApiErrorCode, ClientBuilder, QuoteRequest, QuoteType, SdkError};

let client = ClientBuilder::new("http://127.0.0.1:3000").build()?;

match client.quote(QuoteRequest::sell("native", "USDC")).await {
    Ok(quote) => println!("price: {}", quote.price),
    Err(err) => match err {
        SdkError::RateLimited { info } => {
            println!("rate limited until {:?}", info.reset);
        }
        SdkError::Api { code, message, status } => {
            if code == ApiErrorCode::StaleMarketData {
                println!("market data stale, refresh and retry");
            } else if code == ApiErrorCode::Overloaded {
                println!("service overloaded, try again later");
            } else if code == ApiErrorCode::NotFound {
                println!("pair not found");
            } else {
                println!("api error {} {}", status, message);
            }
        }
        SdkError::Http(_) => eprintln!("transport failure"),
        SdkError::InvalidConfig(_) => eprintln!("invalid SDK config"),
        SdkError::Deserialization(_) => eprintln!("invalid response payload"),
    },
}
```

### Rust SDK convenience helpers

- `err.is_not_found()`
- `err.is_validation_error()`
- `err.is_rate_limited()`
- `err.is_stale_market_data()`
- `err.is_overloaded()`

## Sample JSON error responses

```json
{ "error": "bad_request", "message": "Malformed request" }
```

```json
{
  "error": "invalid_asset",
  "message": "Invalid asset identifier",
  "details": { "asset": "USDC:INVALID" }
}
```

```json
{
  "error": "validation_error",
  "message": "Amount must be a positive integer",
  "details": { "field": "amount", "reason": "must be greater than zero" }
}
```

```json
{ "error": "not_found", "message": "Trading pair not found" }
```

```json
{ "error": "no_route", "message": "No trading route available for this pair" }
```

```json
{
  "error": "stale_market_data",
  "message": "Quote data is too old",
  "details": { "last_updated": "2026-05-31T14:33:00Z" }
}
```

```json
{
  "error": "rate_limit_exceeded",
  "message": "Too many requests",
  "details": { "retry_after_seconds": 10 }
}
```

```json
{ "error": "overloaded", "message": "Service overloaded, please retry later" }
```

```json
{ "error": "internal_error", "message": "Unexpected server failure" }
```

## Versioning and deprecation guidance

Integrators should pin an explicit API version path, for example `/api/v1/...`.
Follow the release and deprecation headers described in [API Versioning and Deprecation Policy](versioning-policy.md).

## Frontend guidance

Frontend teams should translate API error codes into trader-facing copy instead of exposing raw codes.
If there is an active design error copy issue, use it as the source of truth for messaging.

- Use `rate_limit_exceeded`/`overloaded` to show polite retry messaging.
- Use `stale_market_data` to show a refresh or retry prompt with minimal friction.
- Use `validation_error` and `invalid_asset` to surface actionable input corrections.
- Use `not_found` and `no_route` to explain that the requested market is unavailable.
