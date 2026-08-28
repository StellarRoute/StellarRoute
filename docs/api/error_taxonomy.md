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
| `invalid_amount` | 400 | The requested amount is invalid (e.g. non-numeric, zero, or negative). |
| `invalid_slippage` | 400 | The requested slippage tolerance is invalid. |
| `invalid_asset_format` | 400 | An asset identifier is malformed (wrong shape, not a parse failure of the value itself). |
| `validation_error` | 400 | The request parameters failed validation (e.g. amount <= 0). |
| `unauthorized` | 401 | The request lacks valid authentication credentials. |
| `not_found` | 404 | The requested resource (pair, orderbook, etc.) was not found. |
| `no_route` | 404 | No trading route was found for the given pair. |
| `stale_market_data` | 422 | The quote could not be generated because the underlying market data is too stale. |
| `not_executable` | 422 | The route would fail execution on-chain (simulation detected failure). |
| `quote_expired` | 422 | The referenced prepare quote has expired; call `prepare` again before submitting. |
| `rate_limit_exceeded` | 429 | Too many requests have been made in a short period. |
| `internal_error` | 500 | An unexpected error occurred on the server. |
| `not_implemented` | 501 | The requested operation is part of the documented API contract but not yet available. |
| `overloaded` | 503 | The server is currently processing too many requests. |
| `dependency_unavailable` | 503 | An upstream dependency (e.g. Horizon) is unavailable or its circuit breaker is open. |
| `quote_not_found` | 404 | The referenced `quote_id` is unknown or no longer valid. |
| `duplicate_quote` | 409 | The prepare quote was already submitted or is currently being submitted. |
| `unsupported_execution_mode` | 422 | The route requires AMM/Soroban/router execution, which is not enabled; classic PathPaymentStrictSend only. |
| `unsupported_route` | 422 | The classic route shape is not supported by this prepare build (currently single SDEX hop only; multi-hop is rejected). |
| `cctp_not_enabled` | 503 | Circle CCTP bridge settlement is not enabled on this deployment. |
| `unsupported_corridor` | 400 | The requested CCTP corridor or provider is unknown or unsupported. |
| `invalid_finality` | 400 | CCTP finality mode is invalid or unsupported for this corridor. |
| `invalid_recipient` | 400 | The CCTP recipient address failed validation. |
| `fee_quote_unavailable` | 503 | A runtime CCTP fee quote could not be produced. |
| `attestation_pending` | 422 | CCTP attestation is still pending for this transfer (saga guard). |
| `attestation_expired` | 422 | CCTP attestation has expired; request re-attestation before minting. |
| `mint_retryable` | 422 | CCTP mint failed but may be retried (`mint_failed_retryable` saga state). |
| `transfer_not_found` | 404 | The referenced CCTP `transfer_id` is unknown. |
| `provider_killed` | 503 | The CCTP provider kill-switch is active. |

This table is the canonical list backing `crates/api/src/models/response.rs`'s
`ApiErrorCode::ALL` and sdk-js's `API_ERROR_CODES` (`sdk-js/src/types.ts`).
`crates/api/tests/openapi_swap_contract.rs` fails the build if any of the
three drift apart — update all three together when adding a new code.

## Swap prepare/submit

`POST /api/v1/swap/prepare` and `POST /api/v1/swap/submit` (tag `swap` in
Swagger UI) implement the live classic swap path: `prepare` validates a
**single SDEX hop**, prices it authoritatively, and returns an unsigned
`PathPaymentStrictSend` envelope plus `quote_id` with
`execution_mode: classic_path_payment` and `network_passphrase` (the network
the envelope was built for — clients must compare to the wallet before
signing). Multi-hop and AMM/Soroban routes are rejected
(`unsupported_route` / `unsupported_execution_mode`). `submit`
cryptographically verifies the signed envelope against the prepared quote and
broadcasts via Horizon with hash-bound idempotency.

Wire note: hop `from_asset` / `to_asset` accept **either** canonical object
JSON (`{asset_code, asset_issuer}`) **or** legacy string JSON
(`"native"` / `"CODE:ISSUER"`) — fail-closed otherwise. OpenAPI documents this
as `AssetPath` `oneOf`.

Focused PR CI coverage (no external DB):
`cargo test -p stellarroute-api --test swap_integration --test swap_submit_integration --test openapi_swap_contract`.

Allowlisted conflict `details.status` values surfaced to traders (sanitized;
no arbitrary detail blobs): `active_prepare_exists`, `already_submitted`,
`in_progress`, `pending_reconcile`, `confirm_timeout`, `bad_sequence`,
`missing_network_passphrase`, `submitting_without_hash` (plus client-side
`network_mismatch` before Freighter signing).

See
[`docs/readiness/live-swap-testnet-checklist.md`](../readiness/live-swap-testnet-checklist.md)
for the operational checklist and
[`docs/runbooks/swap-submitting-sender-lock.md`](../runbooks/swap-submitting-sender-lock.md)
for stuck-`submitting` sender-lock recovery.

## SDK Mapping

The JS SDK (`@stellarroute/sdk-js`) maps these codes to the `StellarRouteApiError` class.

| SDK Method | Logic |
|:-----------|:------|
| `isNotFound()` | `status === 404 \|\| code === 'not_found'` |
| `isRateLimited()` | `status === 429 \|\| code === 'rate_limit_exceeded'` |
| `isValidationError()` | `status === 400 \|\| ['validation_error', 'invalid_asset'].includes(code)` |

The SDK also synthesizes three client-side error codes for transport/configuration failures that never reach the server:
- `network_error`: The request failed at the transport layer (e.g. connection timeout, CORS, or DNS failure).
- `network_mismatch`: The prepare quote's network passphrase does not match the active wallet's network (surfaced before signing).
- `unknown_error`: An unclassified error occurred during SDK execution.

## WebSocket Errors

WebSocket endpoints use the same error codes as REST endpoints, plus additional WebSocket-specific codes:

| Code | Description |
|------|-------------|
| `unknown_action` | The `action` field in a client message is not recognized. |
| `invalid_subscription` | Subscription object is malformed or missing required fields. |
| `too_many_subscriptions` | Connection has reached the maximum subscriptions per connection limit. (not currently emitted) |

See [WebSocket Quote Stream API](websocket.md) for complete WebSocket protocol documentation and error handling guidance.

## Freshness SLO & Configuration Knobs

To prevent execution on stale rates, the StellarRoute API implements strict freshness checks:
- **SLO**: Market quotes/routes are rejected if the underlying data sources (SDEX offers or Soroban pool reserves) have not been updated within configured thresholds.
- **Error**: Rejections return HTTP 422 with the code `stale_market_data` containing details about the stale vs. fresh inputs.
- **Config Knobs**:
  - `freshness_threshold_secs.sdex`: Maximum age of SDEX offer data (default: 60s) before it is considered stale.
  - `freshness_threshold_secs.amm`: Maximum age of Soroban AMM state (default: 30s) before it is considered stale.
  - `staleness_threshold_secs`: Ultimate cutoff beyond which any source is rejected (default: 300s).

## Integration guidance

For practical retry semantics, backoff guidance, SDK helper examples, and frontend messaging recommendations, see [API Integrator Error Guide](integrator-error-guide.md).
