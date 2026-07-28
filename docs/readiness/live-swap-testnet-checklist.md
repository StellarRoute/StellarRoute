# Live Swap E2E Checklist — Testnet

Strict, executable checklist proving the live swap path end-to-end on
Stellar Testnet: quote → prepare → sign (Freighter) → submit → confirm.
Complements [`docs/swap-e2e-flow.md`](../swap-e2e-flow.md) (UX spec) with
ordered steps, expected HTTP status codes, and a failure matrix an
operator/integrator can run without reading UX copy.

**Current status (issue #1051 / milestone M4 — Live swap path):**
`POST /api/v1/swap/prepare` and `POST /api/v1/swap/submit` are documented in
the OpenAPI contract (Swagger `swap` tag) and validate their input, but
transaction building and on-chain submission are **not implemented yet** —
both currently return `501 not_implemented` after validation passes. Steps
3–6 below are the checklist to run **once that ships**; steps 1–2 and the
kill-switch/pause checks are runnable today. Do not mark this checklist's
exit criteria met until steps 3–6 pass for real.

## Prerequisites

- StellarRoute API running against Testnet (`STELLAR_HORIZON_URL=https://horizon-testnet.stellar.org`, `STELLAR_NETWORK=testnet`).
- [Freighter wallet](https://www.freighter.app/) browser extension installed, set to **Testnet**.
- Frontend running locally or against a preview deploy with the swap UI flag enabled: `NEXT_PUBLIC_FLAG_SWAP_UI_V2=true` (see [`frontend/docs/FEATURE_FLAGS.md`](../../frontend/docs/FEATURE_FLAGS.md)).
- `sdk-js` example available at [`sdk-js/examples/quickstart-quote.ts`](../../sdk-js/examples/quickstart-quote.ts) as a reference for calling the API directly (a `prepareSwap`/`submitSwap` example should be added alongside it once those SDK methods exist).

### Freighter testnet account funding (no secrets)

1. Open Freighter → switch network to **Testnet** (Settings → Network).
2. Create or select an account; copy its **public key** (`G...`) — never paste or record the secret key/recovery phrase anywhere, including this checklist's appendix.
3. Fund it via Friendbot:
   ```bash
   curl "https://friendbot.stellar.org/?addr=<PUBLIC_KEY>"
   ```
4. Confirm funding:
   ```bash
   curl "https://horizon-testnet.stellar.org/accounts/<PUBLIC_KEY>" | jq '.balances'
   ```
5. Repeat for a second funded account if testing a multi-hop or issued-asset swap (trustline required — see step 2 below).

## Ordered steps & expected status codes

| # | Step | Call | Expected status | Notes |
|---|---|---|---|---|
| 1 | Request a quote | `GET /api/v1/quote/:base/:quote?amount=...` | `200` | Record `quote.expires_at` and `quote.rationale`. Runnable today. |
| 2 | (If needed) establish trustline for issued asset | Freighter: Manage Assets → Add Asset | n/a (on-chain) | Skip for native XLM legs. |
| 3 | Prepare the swap | `POST /api/v1/swap/prepare` with the route from step 1 and `sender` = your public key | `200` (target) / currently `501` | Response is `SwapPrepareResponse { xdr_envelope, expected_output, expires_at }` once implemented. `400` on invalid route/amount — verify this today. |
| 4 | Sign the envelope | Freighter `signTransaction(xdr_envelope)` | n/a (wallet UI) | User approves in the Freighter popup. Do **not** proceed past a rejected signature (see failure matrix). |
| 5 | Submit the signed transaction | `POST /api/v1/swap/submit` with the signed `xdr_envelope` | `200` (target) / currently `501` | Response is `SwapSubmitResponse { tx_hash, status }` once implemented. |
| 6 | Confirm on-chain | `curl "https://horizon-testnet.stellar.org/transactions/<tx_hash>"` | `200`, `successful: true` | Record the `tx_hash` in the appendix below — this is the checklist's exit criterion. |

## Failure matrix

| Scenario | Trigger | Expected behavior | Verify against |
|---|---|---|---|
| **Stale quote** | Wait past `expires_at` (or the market moves) before calling `prepare` | `prepare` rejects with a validation-class error rather than silently using stale pricing; UI shows "Quote expired — update the quote to continue" (see `docs/swap-e2e-flow.md`) | `docs/api/error_taxonomy.md` (`validation_error` / a future `stale_quote`-specific code once `prepare` is implemented) |
| **User rejected sign** | Dismiss/reject the Freighter signing popup at step 4 | No `submit` call is made; UI returns to the pre-confirm state without an error toast (rejection is not a failure) | `docs/swap-e2e-flow.md` loading/retry states |
| **Submit conflict** | Submit the same signed envelope (or same `quote_id`) twice | Second `submit` call is rejected (duplicate-submission guard) rather than double-executing on-chain | `docs/swap-e2e-flow.md` idempotency note; backend should map this to a `409`-class code once `submit` executes for real |
| **Router paused** | Set the kill switch to exclude the relevant source/venue via `POST /api/v1/admin/kill-switch` (see [`docs/RUNBOOK_KILL_SWITCH.md`](../RUNBOOK_KILL_SWITCH.md)) before requesting a quote | `quote`/`prepare` exclude the paused venue (`exclusion_diagnostics`) or return `no_route` if no alternative exists | `docs/RUNBOOK_KILL_SWITCH.md`, `docs/api/error_taxonomy.md` (`no_route`) |

## Links

- API routes: `GET /api/v1/quote/:base/:quote`, `POST /api/v1/swap/prepare`, `POST /api/v1/swap/submit` — see [`docs/api/error_taxonomy.md`](../api/error_taxonomy.md#swap-preparesubmit) and the Swagger `swap` tag at `/swagger-ui`.
- SDK: [`sdk-js/examples/quickstart-quote.ts`](../../sdk-js/examples/quickstart-quote.ts); `StellarRouteClient.executeSwap` in [`sdk-js/src/client.ts`](../../sdk-js/src/client.ts) (currently a documented stub — see `docs/api/error_taxonomy.md`).
- Frontend flag: `swap_ui_v2` — see [`frontend/docs/FEATURE_FLAGS.md`](../../frontend/docs/FEATURE_FLAGS.md).
- UX spec: [`docs/swap-e2e-flow.md`](../swap-e2e-flow.md).
- Kill switch runbook: [`docs/RUNBOOK_KILL_SWITCH.md`](../RUNBOOK_KILL_SWITCH.md).

## Exit criteria

This checklist is **only** satisfied once a real, on-chain testnet swap has
completed end-to-end (steps 1–6 above, with steps 3 and 5 returning `200`,
not `501`). Record the result here:

### Appendix — successful run record (template)

```
Date (UTC):
Operator / integrator:
API commit / version:
Base asset → Quote asset:
Amount:
Sender public key (G...; never record secret keys/seed phrases):
Quote expires_at used:
Prepare response expected_output:
Submit response status:
Transaction hash: 
Horizon confirmation URL: https://horizon-testnet.stellar.org/transactions/<tx_hash>
Notes / anomalies observed:
```
