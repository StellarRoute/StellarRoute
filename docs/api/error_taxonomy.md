# API Error Taxonomy

This document defines the standard error taxonomy for the StellarRoute API.

## Error Response Format

All API errors return a consistent JSON body:

```json
{
  "error": "error_code",
  "message": "Human-readable description",
  "details": { ... }
}
```

- `error`: A machine-readable string code in `snake_case`.
- `message`: A descriptive message for developers/users.
- `details`: (Optional) Structured context about the failure (e.g., validation rules, stale counts).

## Error Catalog

| Code | HTTP Status | Description |
|:-----|:------------|:------------|
| `bad_request` | 400 | The request is malformed or contains invalid parameters. |
| `invalid_asset` | 400 | One of the asset identifiers in the request is invalid. |
| `validation_error` | 400 | The request parameters failed validation (e.g. amount <= 0). |
| `unauthorized` | 401 | The request lacks valid authentication credentials. |
| `not_found` | 404 | The requested resource (pair, orderbook, etc.) was not found. |
| `no_route` | 404 | No trading route was found for the given pair. |
| `stale_market_data` | 422 | The quote could not be generated because the underlying market data is too stale. |
| `rate_limit_exceeded` | 429 | Too many requests have been made in a short period. |
| `internal_error` | 500 | An unexpected error occurred on the server. |
| `overloaded` | 503 | The server is currently processing too many requests. |

## SDK Mapping

The JS SDK (`@stellarroute/sdk-js`) maps these codes to the `StellarRouteApiError` class.

| SDK Method | Logic |
|:-----------|:------|
| `isNotFound()` | `status === 404 \|\| code === 'not_found'` |
| `isRateLimited()` | `status === 429 \|\| code === 'rate_limit_exceeded'` |
| `isValidationError()` | `status === 400 \|\| ['validation_error', 'invalid_asset'].includes(code)` |
