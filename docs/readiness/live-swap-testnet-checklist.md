# Live Swap E2E Checklist — Testnet

Strict, executable checklist proving the live **classic** swap path end-to-end on
Stellar Testnet: quote → prepare → sign (Freighter) → submit → confirm.
Complements [`docs/swap-e2e-flow.md`](../swap-e2e-flow.md) (UX spec) with
ordered steps, expected HTTP status codes, **commands that emit evidence**,
and a failure matrix an operator/integrator can run without reading UX copy.

**Current execution scope:** `POST /api/v1/swap/prepare` and
`POST /api/v1/swap/submit` implement **classic SDEX `PathPaymentStrictSend` only**
(single hop). Successful prepare returns `execution_mode: classic_path_payment`.
**Soroban / AMM / router / multi-hop are unsupported** (`unsupported_execution_mode`
/ `unsupported_route`). Do not treat Soroban as shippable.

**Operator note — sender lock:** At most one active `prepared`/`submitting`
quote per sender. Stuck `submitting` rows are **not** TTL-expired and block new
prepares until reconcile or guarded operator release. See
[`docs/runbooks/swap-submitting-sender-lock.md`](../runbooks/swap-submitting-sender-lock.md).

| Surface | Status |
|---|---|
| OpenAPI prepare/submit | Documented (`swap` tag) |
| Classic one-hop prepare/submit | Backend contract on active root |
| `sdk-js` prepare/submit/execute/confirm | Unit-tested; nested error envelope parsing |
| Frontend (`real_xdr`) | `frontend/lib/swap/api-execution.ts` — signs server XDR only |
| Dry-run smoke | `scripts/live-swap-api-smoke.mjs` (testnet quote+prepare; no secrets) |

Do **not** mark exit criteria met until steps 3–6 pass for a real on-chain
testnet classic swap with a recorded Horizon `tx_hash`.

## Prerequisites

- StellarRoute API running against Testnet (`STELLAR_HORIZON_URL=https://horizon-testnet.stellar.org`, `STELLAR_NETWORK=testnet` or unset; passphrase defaults to testnet).
- Migration `0014_swap_prepared_quotes_security.sql` applied (security metadata + active-sender index).
- [Freighter wallet](https://www.freighter.app/) browser extension installed, set to **Testnet** (never paste secret keys into the repo or checklist appendix).
- Frontend flags: `NEXT_PUBLIC_FLAG_SWAP_UI_V2=true`, `NEXT_PUBLIC_FLAG_REAL_XDR=true` (product default when unset; production fails closed if disabled — see [`frontend/docs/FEATURE_FLAGS.md`](../../frontend/docs/FEATURE_FLAGS.md) and `frontend/.env.example`).
- `sdk-js` example available at [`sdk-js/examples/quickstart-quote.ts`](../../sdk-js/examples/quickstart-quote.ts); live path via `prepareSwap` / `submitSwap` / `executeSwap` / `confirmSwap`.

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
5. Issued-asset legs require a trustline (Manage Assets → Add Asset). Multi-hop classic prepare is **not** supported in this build.

## Ordered steps & expected status codes

| # | Step | Call | Expected status | Notes |
|---|---|---|---|---|
| 1 | Request a quote | `GET /api/v1/quote/:base/:quote?amount=...` | `200` | Prefer a direct SDEX venue. Record amounts for prepare. |
| 2 | (If needed) establish trustline for issued asset | Freighter: Manage Assets → Add Asset | n/a (on-chain) | Skip for native XLM legs. |
| 3 | Prepare the swap | `POST /api/v1/swap/prepare` with a **single** `sdex`/`horizon` hop, `sender` = public key | `200` | Response: `quote_id`, `xdr_envelope`, `expected_output`, `min_output`, `expires_at`, `execution_mode: classic_path_payment`. AMM → `422 unsupported_execution_mode`. Multi-hop → `422 unsupported_route`. Concurrent prepare for same sender → `409` / `details.status=active_prepare_exists`. |
| 4 | Sign the envelope | Freighter `signTransaction(xdr_envelope)` on **Testnet** | n/a (wallet UI) | Network passphrase must match the server / wallet-reported Testnet passphrase. Rejection → no submit. |
| 5 | Submit the signed transaction | `POST /api/v1/swap/submit` with `{ quote_id, signed_xdr }` | `200` / `202` | Binds deterministic `tx_hash` at claim before broadcast. On ambiguous timeout, retry **same** signed envelope (reconcile); do not prepare a second quote for the same sender until the first settles or is operator-released. |
| 6 | Confirm on-chain | `curl "https://horizon-testnet.stellar.org/transactions/<tx_hash>"` | `200`, `successful: true` | Record the `tx_hash` in the appendix — checklist exit criterion. |

### Evidence commands

Dry-run (quote + prepare; no secrets; rejects mainnet):

```bash
STELLARROUTE_API_URL=http://localhost:8080 \
STELLARROUTE_SENDER=<PUBLIC_KEY> \
STELLAR_NETWORK=testnet \
STELLARROUTE_SMOKE_EVIDENCE_PATH=./tmp/live-swap-smoke-evidence.json \
node scripts/live-swap-api-smoke.mjs
```

Deterministic unit coverage:

```bash
npm --prefix sdk-js run test -- src/client.test.ts src/types.test.ts src/chain_asset.test.ts
npm --prefix frontend run test -- lib/swap/api-execution.test.ts
```

### Manual Freighter full path (no repository secret-key path)

1. Complete dry-run successfully.
2. In the UI with `NEXT_PUBLIC_FLAG_REAL_XDR=true`, confirm a one-hop SDEX swap.
3. Approve Freighter on Testnet; do not dismiss the popup if you intend to submit.
4. Record `tx_hash` from the UI / submit response and confirm on
   `https://horizon-testnet.stellar.org/transactions/<tx_hash>`.

## Failure matrix

| Scenario | Trigger | Expected behavior | Verify against |
|---|---|---|---|
| **Stale prepare** | Wait past prepare `expires_at` while still `prepared`, then submit | `422 quote_expired`; quote marked failed | `docs/api/error_taxonomy.md` |
| **User rejected sign** | Dismiss Freighter popup at step 4 | No `submit`; prepare may still hold the sender lock until TTL expire of **prepared** | `docs/swap-e2e-flow.md` |
| **Submit conflict** | Submit again after success / in-progress | `409` (`already_submitted` / `in_progress` / `permanently_failed`) | error taxonomy |
| **Timeout after claim** | Horizon timeout/5xx after claim | Quote stays `submitting` with bound `tx_hash`; retry submit same envelope; pending reconcile if dependency stays unavailable | runbook below |
| **Active prepare exists** | Second prepare for same sender | `409` / `details.status=active_prepare_exists` | runbook if stuck in `submitting` |
| **Router/venue paused** | Kill switch excludes SDEX venue/source | `422 not_executable` | [`docs/RUNBOOK_KILL_SWITCH.md`](../RUNBOOK_KILL_SWITCH.md) |
| **AMM / Soroban / multi-hop** | AMM hop or >1 hop | `422 unsupported_execution_mode` / `unsupported_route`; UI CTA disabled with safe copy | error taxonomy |
| **Dependency unavailable** | Horizon/DB dependency down | `503 dependency_unavailable` | error taxonomy |

## Stuck `submitting` / sender-lock (operator)

If a sender cannot prepare again and the quote is `submitting`:

1. **First** reconcile the bound `tx_hash` against Horizon.
2. **Only if** Horizon has no accepted tx **and** the quote’s `timebounds_max` has elapsed, mark failed with the guarded SQL in the runbook (releases the sender lock).
3. Always write a `swap_submit_audit_log` row; never log full accounts or XDR.
4. **Never** clear a lock if the transaction might still be accepted.
5. Horizon **hash-integrity mismatch** → leave `submitting` for investigation.

Full procedure (copy-paste SQL, Horizon curls, anti-patterns):
[`docs/runbooks/swap-submitting-sender-lock.md`](../runbooks/swap-submitting-sender-lock.md).

## Links

- Error taxonomy: [`docs/api/error_taxonomy.md`](../api/error_taxonomy.md#swap-preparesubmit) and the Swagger `swap` tag at `/swagger-ui`.
- Sender-lock runbook: [`docs/runbooks/swap-submitting-sender-lock.md`](../runbooks/swap-submitting-sender-lock.md).
- SDK: `sdk-js` `prepareSwap` / `submitSwap` / `executeSwap` / `confirmSwap` — see [`sdk-js/examples/swap-submit.ts`](../../sdk-js/examples/swap-submit.ts).
- Frontend: `frontend/lib/swap/api-execution.ts` (`real_xdr`) — see [`frontend/docs/FEATURE_FLAGS.md`](../../frontend/docs/FEATURE_FLAGS.md).
- Audit readiness: [`audit/readiness-evidence.md`](../../audit/readiness-evidence.md).
- UX spec: [`docs/swap-e2e-flow.md`](../swap-e2e-flow.md).
- Kill switch runbook: [`docs/RUNBOOK_KILL_SWITCH.md`](../RUNBOOK_KILL_SWITCH.md).

## Exit criteria

Satisfied only after a real on-chain **testnet** classic path-payment swap
(steps 1–6). Record:

```
Date (UTC):
Operator / integrator:
API commit / version:
SDK / frontend commit:
Evidence JSON path:
Base → Quote:
Amount:
Sender public key (G... only):
Prepare quote_id:
execution_mode (must be classic_path_payment):
expected_output / min_output:
Submit status:
Transaction hash:
Horizon URL: https://horizon-testnet.stellar.org/transactions/<tx_hash>
successful: true/false
Notes:
```
