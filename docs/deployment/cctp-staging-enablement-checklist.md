# CCTP staging enablement checklist (testnet)

**Purpose:** Flip public reverse + forward CCTP on Wave 0 / EC2 staging after a signed-live Sepolia→Stellar UI proof. Repo defaults stay fail-closed (`CCTP_ENABLED=false`).

**Date completed:** _YYYY-MM-DD_  
**Operator:** _name_  
**Staging API:** `https://34.224.110.144.sslip.io` (or current host)

## Preconditions

- [ ] Signed-live reverse proof published: `docs/cctp/signed-live-sepolia-to-stellar.md` + evidence JSON
- [ ] Forward Stellar→Sepolia proof still valid (no regression claims)
- [ ] Staging host has current `main` (or release) with trustline prepare-mint support
- [ ] `.env.prod` present on host (from `deploy/env.prod.example`)

## Required env (API)

| Variable | Required | Notes |
| --- | --- | --- |
| `CCTP_ENABLED` | yes | Set `true` only after probes below are green |
| `CCTP_ACCESS_TOKEN_HMAC_KEY` | yes | ≥32 decoded bytes; script generates if missing |
| `CCTP_SEPOLIA_RPC_URL` or `SEPOLIA_RPC_URL` | yes | Explicit HTTPS JSON-RPC — never silent `rpc.sepolia.org` |
| `CCTP_STELLAR_RPC_URL` / Soroban RPC | yes | Testnet Soroban reachable from host |
| `STELLAR_HORIZON_URL` | yes | Testnet Horizon (trustline probe) |
| `CCTP_IRIS_BASE_URL` | optional | Defaults to Iris sandbox |

## Kill switches / CORS / frontend

- [ ] Provider kill switch off for `circle-cctp`
- [ ] CORS allowlist includes `https://stellarroute.app` and `https://www.stellarroute.app`
- [ ] Frontend `NEXT_PUBLIC_API_URL_TESTNET` (or equivalent) points at staging HTTPS API
- [ ] `/cross-chain` reachable and CCTP readiness fetch succeeds

## Enable procedure

1. SSH to staging host; repo root with `.env.prod`.
2. Run:

```bash
CCTP_SEPOLIA_RPC_URL='https://…' bash deploy/aws/scripts/enable-cctp-staging.sh
```

3. Confirm API health and corridors:

```bash
curl -skS 'https://34.224.110.144.sslip.io/api/v2' | jq '.data.supported_corridors'
```

- [ ] `stellar_to_evm` → `executable: true`
- [ ] `evm_to_stellar` → `executable: true`

4. Smoke (no large burns required):

- [ ] Quote `evm_to_stellar` from staging-backed UI (or curl with access token path)
- [ ] Confirm UI no longer shows `cctp_not_enabled` for reverse

## Rollback

```bash
# On host: set CCTP_ENABLED=false in .env.prod and recreate API container
```

Or re-run enable script inverted by manually upserting `CCTP_ENABLED=false` then `docker compose … up -d api`.

## Record

| Field | Value |
| --- | --- |
| Enable commit / image | |
| `executable` JSON snippet | |
| Smoke quote transfer_id (optional) | |
| Notes | |
