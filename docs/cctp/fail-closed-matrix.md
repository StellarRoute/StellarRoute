# CCTP fail-closed matrix (frontend + API)

**Status:** Documentation-only  
**Purpose:** Single source of truth for what happens when CCTP is disabled (the public default). Prevents PRs from "enabling it to make the corridor show up" on production without proper operator review.

---

## Summary

Circle CCTP v2 is proven on testnet but **public default is off**. `CCTP_ENABLED=false` is the production default. This matrix documents the expected behavior across all UI routes and API endpoints when CCTP is disabled.

---

## Fail-closed matrix

### API endpoints (`/api/v2/bridge/cctp/*`)

| Env var | API endpoint | Expected response | Required HMAC / RPC | Notes |
|---------|--------------|-------------------|---------------------|-------|
| `CCTP_ENABLED=false` | `POST /api/v2/bridge/cctp/quote` | **503** `cctp_not_enabled` | HMAC + Sepolia RPC **not required** (disabled early) | Quote never generated |
| `CCTP_ENABLED=false` | `POST /api/v2/bridge/cctp/{transfer_id}/prepare-burn` | **503** `cctp_not_enabled` | — | Burn payload never built |
| `CCTP_ENABLED=false` | `POST /api/v2/bridge/cctp/{transfer_id}/submit-burn` | **503** `cctp_not_enabled` | — | Burn hash never recorded |
| `CCTP_ENABLED=false` | `GET /api/v2/bridge/cctp/{transfer_id}` | **503** `cctp_not_enabled` | — | Saga status never returned |
| `CCTP_ENABLED=false` | `POST /api/v2/bridge/cctp/{transfer_id}/prepare-mint` | **503** `cctp_not_enabled` | — | Mint payload never built |
| `CCTP_ENABLED=false` | `POST /api/v2/bridge/cctp/{transfer_id}/submit-mint` | **503** `cctp_not_enabled` | — | Mint hash never recorded |
| `CCTP_ENABLED=false` | `POST /api/v2/bridge/cctp/{transfer_id}/reattest` | **503** `cctp_not_enabled` | — | Re-attestation never triggered |
| `CCTP_ENABLED=false` | `GET /api/v2` (corridor metadata) | `executable: false` for both directions | — | Corridor listed but not executable |

### Frontend UI routes

| Env var / feature flag | UI route / surface | Expected behavior | Required HMAC / RPC | Notes |
|------------------------|--------------------|-------------------|---------------------|-------|
| `CCTP_ENABLED=false` | `/swap` (legacy swap card) | **No change** — classic SDEX prepare → sign → submit flow | None | Frozen (do not modify) |
| `CCTP_ENABLED=false` | `/swap` with `swap_ui_v2` flag on | Cross-chain deck shows **unsupported corridor state** | None | `UnsupportedCorridorState` component renders |
| `CCTP_ENABLED=false` | `/swap` with `swap_ui_v2` + cross-chain deck | Corridor tabs show **disabled/locked** state | None | No quote/mint actions available |
| `NEXT_PUBLIC_FLAG_SWAP_UI_V2=false` | `/swap` (any) | **No change** — legacy swap card only | None | Cross-chain deck never mounted |
| `NEXT_PUBLIC_FLAG_SWAP_UI_V2=true` | `/swap` → cross-chain deck | Deck renders; corridor readiness fetches `/api/v2` and sees `executable: false` | None | UI shows corridor unavailable |

### Classic one-hop SDEX swap (frozen — do not modify)

| Env var | Endpoint | Expected behavior | Notes |
|---------|----------|-------------------|-------|
| Any | `POST /api/v1/swap/prepare` | **200** with unsigned XDR envelope | Single SDEX hop only |
| Any | `POST /api/v1/swap/submit` | **200** with tx hash on success | Classic sign → submit path |

**These paths are frozen.** CCTP changes must not alter prepare/submit behavior for live SDEX swaps.

---

## Env var dependency chain (for reference — do NOT set in production)

When `CCTP_ENABLED=true` (staging only, after operator review):

| Variable | Required | Purpose |
|----------|----------|---------|
| `CCTP_ENABLED` | **Required** | Master switch — must be `true` to enable handlers |
| `CCTP_ACCESS_TOKEN_HMAC_KEY` | **Required** | ≥32 decoded bytes; generates deterministic quote tokens |
| `CCTP_SEPOLIA_RPC_URL` or `SEPOLIA_RPC_URL` | **Required** | Sepolia JSON-RPC for EVM burn/mint builders |
| `CCTP_STELLAR_RPC_URL` / `SOROBAN_RPC_URL` | Required | Soroban RPC for Stellar burn/mint verifiers |
| `CCTP_IRIS_BASE_URL` | Optional | Defaults to Iris sandbox (`iris-api-sandbox.circle.com`) |
| `CCTP_ACCESS_TOKEN_HMAC_PREVIOUS_KEYS` | Optional | Up to 2 prior HMAC keys for rotation |

---

## What this matrix does NOT cover

- **Enabling CCTP in production** — requires staging checklist completion ([`docs/deployment/cctp-staging-enablement-checklist.md`](../deployment/cctp-staging-enablement-checklist.md))
- **Mainnet enablement** — gated behind audit + explicit ops approval
- **Corridor-specific readiness** — direction-specific health checks when enabled
- **Wallet signing adapters** — still required when corridor is enabled

---

## Operator checklist (fail-closed posture)

- [ ] `CCTP_ENABLED=false` in `.env.prod` (production default)
- [ ] No frontend flags (`swap_ui_v2`) enabled in production Vercel
- [ ] `/api/v2/bridge/cctp/*` returns `503 cctp_not_enabled` on public API
- [ ] Classic SDEX `/swap` flow works unchanged
- [ ] Quote selection / ranking unaffected by CCTP flags
- [ ] OpenAPI field names, types, and error codes unchanged
- [ ] CORS allowlists unchanged

---

## Related docs

- **Signed-live proof (forward):** [`signed-live-stellar-to-sepolia.md`](./signed-live-stellar-to-sepolia.md)
- **Signed-live proof (reverse):** [`signed-live-sepolia-to-stellar.md`](./signed-live-sepolia-to-stellar.md)
- **API contract:** [`../api/cctp-v2-contract.md`](../api/cctp-v2-contract.md)
- **Attestation verification:** [`attestation-verification.md`](./attestation-verification.md)
- **Stellar verifier blockers:** [`stellar-verifier-blockers.md`](./stellar-verifier-blockers.md)
- **Error taxonomy:** [`../api/error_taxonomy.md`](../api/error_taxonomy.md)
- **Environment variables:** [`../development/environment-variables.md`](../development/environment-variables.md)
- **Staging enablement checklist:** [`../deployment/cctp-staging-enablement-checklist.md`](../deployment/cctp-staging-enablement-checklist.md)
