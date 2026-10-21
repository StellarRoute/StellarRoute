# Integrator Go-Live Checklist (embedding quotes)

This checklist is for partners that **consume StellarRoute quotes** from their own
wallet or dApp. It covers two distinct integration shapes:

- **Quote-only** — display rates / build your own UX from `/api/v1/quote` (no custody,
  no swap execution on our flow).
- **Swap-embed** — also use `/api/v1/swap/prepare` + `/api/v1/swap/submit` to let a
  user sign and broadcast a swap through their own wallet.

Both paths are **read + client-signed**: StellarRoute never custodies funds. The
prepare/submit flow returns an unsigned classic Stellar transaction; the user's wallet
signs it and your client submits it back.

This is an additive-only checklist. It documents existing behavior and does not change
any API contract. All field names, codes, and CORS/CCTP defaults referenced below are
frozen — see `docs/api/integrator-guide.md` and `docs/api/error_taxonomy.md`.

---

## 0. Universal pre-flight (applies to both paths)

1. **Pin the API version.** Always call the `/api/v1` base path. Do not assume
   unversioned routes or future `/api/v2` (non-CCTP) paths exist. Versioning policy:
   `docs/api/versioning.md`, `docs/api/versioning-policy.md`.
2. **Pin the network.** Confirm `network_passphrase` (returned on prepare, and expected
   by your wallet) matches the target network. Testnet vs pubnet is a deployment/contract
   decision on your side; StellarRoute returns the passphrase it built the envelope for.
3. **Confirm CORS origins.** In production StellarRoute enforces a strict CORS allowlist.
   Ensure every browser origin your wallet/dApp loads from is listed in
   `CORS_ALLOWED_ORIGINS` (comma-separated, exact `https://` origins). Wildcard CORS is
   not permitted in production (`REQUIRE_STRICT_CORS` / `STELLARROUTE_ENV=production`).
   For the public testnet/Staging API, include your origin in the allowlist before launch.
4. **Handle rate limits and errors.** Map `429` / `overloaded` to backoff. Map other
   codes via `docs/api/error_taxonomy.md`. Do not hard-fail the UI on a single transient
   `5xx` / `DependencyUnavailable`; retry with backoff.

---

## 1. Quote-only go-live

Use `/api/v1/quote` (and optionally `/api/v1/orderbook/:base/:quote` and
`/api/v1/routes/:base/:quote`) to render prices. No signing, no prepare/submit.

1. **Consume the `expires_at` / `ttl_seconds` fields.** Quote responses include
   `expires_at` (unix ms) and `ttl_seconds`. Treat the quote as stale once `expires_at`
   has passed and re-fetch before showing a user a "ready to swap" state. The cache TTL is
   configurable server-side via `QUOTE_CACHE_TTL_SECONDS`; do not rely on a specific
   constant — read the field from the response.
2. **Show a stale-quote UX from `degraded` + `data_freshness`.** Quotes carry a
   `degraded: bool` flag and a `data_freshness` object (`fresh_count`, `stale_count`,
   `max_staleness_secs`). Stale or unfresh market data does **not** hard-block a quote:
   StellarRoute returns a degraded quote and expects you to notify the user (e.g. "prices
   may be delayed"). Surface `degraded`/`data_freshness` rather than hiding the quote.
3. **Handle `stale_market_data` (HTTP 422).** When quoting is fully blocked by stale data
   the API returns the `stale_market_data` error code. Treat this as a retryable,
   user-visible "refreshing prices" state — not a fatal error.
4. **Idempotent retries (optional).** `POST /api/v1/quote` accepts an optional
   `Idempotency-Key` header (max 128 chars) so a retried request returns the same quote.
   Useful if your client retries on network errors. Details in
   `docs/api/integrator-guide.md` (POST idempotency section).
5. **Use `amount` to size the quote.** Pass the real trade size; larger amounts change
   `price`/`total` via venue-specific impact. Display `price_impact` when present.

---

## 2. Swap-embed go-live (prepare → wallet sign → submit)

Only use this path if your integration actually executes swaps through the user's wallet.

1. **Classic one-hop only.** `/api/v1/swap/prepare` builds an unsigned **classic
   `PathPaymentStrictSend`** transaction. The success response `execution_mode` is always
   `classic_path_payment`. This is the only supported prepare/submit mode today.
2. **AMM / multi-hop prepare is unsupported today.** Soroban AMM and multi-hop settlement
   are not available through prepare/submit. Do **not** send an AMM pool or multi-hop route
   to `/api/v1/swap/prepare` — it is rejected (HTTP `422`, `unsupported_execution_mode` /
   `unsupported_route`). If you source routes from `/api/v1/routes`, only use a single-hop
   `sdex` path for prepare; filter out `amm:*` and multi-leg paths for execution.
3. **Prepare → sign → submit within TTL.** Prepare returns `expires_at` (default prepare
   TTL is 120s) and `network_passphrase`. The wallet must sign the returned `xdr_envelope`
   with the same `network_passphrase` and the `sender` account. Submit the signed XDR to
   `/api/v1/swap/submit` with the `quote_id` before `expires_at`. Submitting remains
   reconcilable past prepare TTL until the on-chain timebounds window closes.
4. **Do not mutate the XDR.** Submit must send back the exact prepared envelope, signed.
   Unsigned or mismatched envelopes are rejected (`422` / validation). The signed hash must
   match the prepared quote.
5. **Handle submit outcomes.** `submit` returns `200` (included) or `202` (pending). On
   `409` conflict (already submitted / submitting / bad sequence) reconcile by re-polling
   submit with the same `quote_id` rather than re-preparing blindly. On `422`
   `quote_expired`, request a fresh prepare.
6. **Stale-quote UX still applies pre-prepare.** If you render a quote before prepare,
   apply Section 1 (degraded flags, `expires_at`) so the user is not shown a stale price
   right before signing.

---

## 3. Pre-launch sign-off

- [ ] Version path pinned to `/api/v1`.
- [ ] Every browser origin in `CORS_ALLOWED_ORIGINS` (production allowlist).
- [ ] Client reads `expires_at`/`ttl_seconds` and re-fetches before swap action.
- [ ] `degraded` / `data_freshness` surfaced to user; not hidden.
- [ ] `stale_market_data` (422) mapped to a retryable "refreshing" state.
- [ ] Swap-embed: only single-hop classic `sdex` routes sent to prepare.
- [ ] Swap-embed: AMM / multi-hop routes excluded from prepare (unsupported today).
- [ ] Wallet signs with matching `network_passphrase` and `sender`.
- [ ] Submit reconciled on `409`; fresh prepare on `422` expired.
- [ ] Error codes mapped via `docs/api/error_taxonomy.md`; `429` backed off.

---

## References

- Endpoints: `docs/api/routes_endpoint.md`
- Webhooks / idempotency / existing swap checklist: `docs/api/integrator-guide.md`
- Error codes: `docs/api/error_taxonomy.md`
- Versioning: `docs/api/versioning.md`, `docs/api/versioning-policy.md`
- OpenAPI contract: `docs/api/openapi.yaml`
