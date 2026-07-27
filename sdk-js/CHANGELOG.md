# Changelog

All notable changes to `@stellarroute/sdk-js` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
package adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html). While
the major version is `0`, minor bumps may contain breaking changes — every one is
listed under a **Breaking** heading below.

## [Unreleased]

### Added

- Release engineering docs: [`PUBLISHING.md`](./PUBLISHING.md) publish checklist and
  this changelog.
- README quickstart walking through the full quote → swap path.

## [0.1.0] — Initial release

### Added

- `StellarRouteClient` covering `getHealth`, `getPairs`, `getOrderbook`, `getQuote`,
  `getRankedRoutes`, `simulateRoute`, `getPriceHistory`, and batch quote/orderbook.
- `executeSwap(params)` — validates the route via `simulateRoute`, then returns the
  XDR envelope to sign.
- WebSocket client for streaming quote updates (`./websocket.js`).
- `StellarRouteApiError` + `isStellarRouteApiError` type guard for structured error
  handling, and quote-staleness helpers (`isQuoteStale`, `getTimeUntilExpiry`).
- Dual ESM/CJS builds with bundled type declarations.

### Breaking

- None — first published version.

### Known limitations

- `executeSwap` throws `StellarRouteApiError` with `code === "not_implemented"` until
  the server-side swap-build endpoint is deployed. Simulation still runs first, so a
  successful simulate followed by this error means the route is valid and the caller
  should build/sign the transaction with the Stellar SDK directly. Removing that stub
  will be a **breaking** change to `executeSwap`'s failure mode and will land in its
  own minor release with a migration note here.
