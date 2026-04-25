# API Versioning and Deprecation Policy

StellarRoute treats the `/api/v1/*` surface as a stable contract for integrators. Breaking changes are introduced deliberately, announced in advance, and paired with an explicit migration path.

## Stability Rules

- Existing response fields are not silently removed or renamed without a documented deprecation period.
- Breaking behavior changes ship behind a successor endpoint or a new versioned route instead of mutating the old contract in place.
- Deprecations are announced in repository docs and surfaced at runtime with HTTP headers so API clients can detect them automatically.
- The default notice period for a deprecated route or field is at least 90 days before sunset, unless a critical security issue requires a faster removal.

## Runtime Deprecation Headers

Deprecated routes emit:

- `Deprecation: true`
- `Sunset: <RFC 1123 timestamp>`
- `Link: <successor>; rel="successor-version", <migration-guide>; rel="deprecation"`

This allows SDKs, gateways, and observability tooling to flag usage of retiring endpoints before they are removed.

## Current Deprecation: `/api/v1/route`

The legacy single-route endpoint:

```text
GET /api/v1/route/:base/:quote
```

is being retired in favor of:

```text
GET /api/v1/routes/:base/:quote
```

### Sunset

`/api/v1/route` is deprecated immediately and scheduled to sunset on `Wed, 01 Jul 2026 00:00:00 GMT`.

### Why the replacement exists

`/api/v1/routes` exposes ranked route candidates and route-scoring metadata, which makes it a better long-term contract for wallets, UIs, and routing-aware integrations.

## Migration Guide for Integrators

### Old request

```text
GET /api/v1/route/native/USDC?amount=100&slippage_bps=50
```

### New request

```text
GET /api/v1/routes/native/USDC?amount=100&limit=5&max_hops=3
```

### Migration notes

- Keep the same `base`, `quote`, and `amount` parameters.
- Continue passing `slippage_bps` when you need the same slippage ceiling semantics.
- Switch downstream parsing from a single `path` field to the first entry in the `routes[]` array when you only need the best route.
- Prefer consuming the full ranked `routes[]` list if your product wants route selection, fallback routing, or richer diagnostics.
- Update alerts and dashboards to treat `Deprecation` and `Sunset` headers on `/api/v1/route` as actionable migration signals.

### Response mapping

| Legacy `/api/v1/route` | Replacement `/api/v1/routes` |
|------------------------|------------------------------|
| `base_asset` | `base_asset` |
| `quote_asset` | `quote_asset` |
| `amount` | `amount` |
| `path` | `routes[0].path` |
| `slippage_bps` | request parameter remains supported |
| `timestamp` | `timestamp` |

For more detail on the replacement response shape, see [`routes_endpoint.md`](./routes_endpoint.md).
