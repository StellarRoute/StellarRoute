# Changelog

All notable changes to `@stellarroute/sdk-js` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
package adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html). While
the major version is `0`, minor bumps may contain breaking changes — every one is
listed under a **Breaking** heading below.

## [Unreleased]

### Fixed

- Preserve per-call headers (including `x-cctp-transfer-access` and `idempotency-key`) across network-error and HTTP-status retries in `StellarRouteClient`, and forward `idempotencyKey` across all CCTP mutation helpers.
- Honor caller `AbortSignal` cancellation immediately with `StellarRouteApiError` (`code === "aborted"`) without retrying, check pre-aborted signals before `fetch`, and remove `abort` listeners on every settled attempt and backoff wait.
- Gate automatic `POST` retries on `429` / `5xx` / network errors so non-idempotent `POST` calls only retry when an `Idempotency-Key` header is present (read-only batch/simulation/validation `POST` helpers remain retryable).

### Added

- `StellarRouteClientOptions.horizonUrl` and `stellarNetwork` for default `confirmSwap` Horizon polling; `DEFAULT_PUBLIC_HORIZON_URL` export.
- Chain-aware asset foundation (`ChainAsset`, `canonicalizeAssetId`, `looksLikeCaip`).
- Classic live-swap methods: `prepareSwap`, `submitSwap`, `confirmSwap`, and
  `executeSwap` with required `signTransaction`.
- Prepare/submit/confirm types including `execution_mode: classic_path_payment`,
  `quote_id`, `expected_output`, `min_output`, and Horizon confirmation result.
- `parseApiErrorBody` supporting flat and `{ data: { error, message, details } }`
  envelopes; `API_ERROR_CODES` includes `dependency_unavailable`,
  `unsupported_execution_mode`, and `unsupported_route`.
- Explicit ambiguous-submit retries that reuse the same `{ quote_id, signed_xdr }`
  without re-prepare/re-sign.
- `StellarRouteClientOptions.network` (`'testnet' | 'public'`) and
  `StellarRouteClientOptions.horizonUrl` set the Horizon instance `confirmSwap`
  polls by default; `DEFAULT_PUBLIC_HORIZON_URL` constant.
- `prepareSwap`'s `options.idempotencyKey` sends an `Idempotency-Key` header
  (`IDEMPOTENCY_KEY_HEADER`), so a retried prepare call can be resent safely
  with the same key.

### Fixed

- `confirmSwap` can be cancelled during the wait between polls: aborting
  `options.signal` now rejects right away instead of after `pollIntervalMs`.
  The same signal is forwarded to the in-flight Horizon `fetch`.

### Breaking

- `confirmSwap` no longer silently defaults to testnet Horizon. It uses
  `options.horizonUrl`, then the client's `horizonUrl` / `network`, then the network
  of the last `executeSwap`, and otherwise throws `validation_error`
  (`details.status: 'missing_horizon_url'`).
- `ExecuteSwapParams.signTransaction` is required.
- `ExecuteSwapParams.networkPassphrase` is required (string or async callback);
  prepare vs integrator mismatch throws typed `network_mismatch` before sign/submit.
- `ExecuteSwapResult` includes `quote_id`, `execution_mode`, `tx_hash`, `status`,
  and optional `min_output`.
- Non-`classic_path_payment` prepare responses fail closed before signing.
- `prepareSwap`'s second parameter changed from a bare `signal?: AbortSignal`
  to `options?: PrepareSwapOptions` (`{ signal?, idempotencyKey? }`). Update
  call sites that pass a signal positionally to `prepareSwap(params, { signal })`.

## [0.1.0] — Initial release

### Added

- `StellarRouteClient` covering health, pairs, orderbook, quote, routes, simulate,
  price history, and batch helpers.
- WebSocket client for streaming quote updates.
- `StellarRouteApiError` + staleness helpers.
- Dual ESM/CJS builds with bundled type declarations.
