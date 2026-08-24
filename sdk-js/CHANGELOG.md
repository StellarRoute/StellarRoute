# Changelog

All notable changes to `@stellarroute/sdk-js` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
package adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html). While
the major version is `0`, minor bumps may contain breaking changes — every one is
listed under a **Breaking** heading below.

## [Unreleased]

### Added

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

### Breaking

- `ExecuteSwapParams.signTransaction` is required.
- `ExecuteSwapParams.networkPassphrase` is required (string or async callback);
  prepare vs integrator mismatch throws typed `network_mismatch` before sign/submit.
- `ExecuteSwapResult` includes `quote_id`, `execution_mode`, `tx_hash`, `status`,
  and optional `min_output`.
- Non-`classic_path_payment` prepare responses fail closed before signing.

## [0.1.0] — Initial release

### Added

- `StellarRouteClient` covering health, pairs, orderbook, quote, routes, simulate,
  price history, and batch helpers.
- WebSocket client for streaming quote updates.
- `StellarRouteApiError` + staleness helpers.
- Dual ESM/CJS builds with bundled type declarations.
