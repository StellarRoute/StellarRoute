# StellarRoute Emergency Operations: Kill Switch Runbook

This runbook describes how to use the API-level kill switches to disable unstable or problematic routing sources and venues without redeploying the application.

## Authentication

All kill switch endpoints require the `ADMIN_AUTH_TOKEN` secret. Pass it via the
`x-admin-token` header (or as a `Bearer` token in `Authorization`).

In every `curl` example below, set the token first:

```bash
ADMIN_TOKEN="<your-ADMIN_AUTH_TOKEN>"
```

See `docs/key_rotation.md` for how to rotate the token without downtime.

## Overview

The kill switch allows operational control over which liquidity sources (SDEX, AMM) and specific venues (individual AMM pools or SDEX pairs) are used by the routing engine. Changes take effect within 5 seconds across all API instances via Redis synchronization.

## Authentication (issue #1053)

Both endpoints live under `/api/v1/admin/kill-switch` and require the admin
token (`ADMIN_AUTH_TOKEN`), sent as either the `x-admin-token` header or
`Authorization: Bearer <token>`:

| Method | Dev/test default | Production default |
|---|---|---|
| `GET` (view state) | Public — no token required | Requires `ADMIN_AUTH_TOKEN` |
| `POST` (update state) | Requires `ADMIN_AUTH_TOKEN` | Requires `ADMIN_AUTH_TOKEN` |

`GET` is intentionally left public in dev/test so operators can inspect
state locally without configuring a token, but is gated the same as `POST`
whenever `STELLARROUTE_ENV=production`. This is a deliberate policy
decision, not an oversight — see
[`docs/api/production-exposure.md`](api/production-exposure.md) for the
full inventory alongside `/metrics` and `/api/v1/replay/*`, which share the
same guard.

**Misconfiguration guard:** if `STELLARROUTE_ENV=production` and
`ADMIN_AUTH_TOKEN` is unset, the API refuses to start (rather than booting
into a state where the kill switch — and every other admin/system route —
silently denies every request, including legitimate operators). Set
`ADMIN_AUTH_TOKEN` before starting in production.

Requests without a valid token receive `401 Unauthorized`.

## Scenarios

- **Unstable AMM Protocol:** If a specific AMM protocol is experiencing issues (e.g., Soroban RPC latency, contract bugs), disable the entire `amm` source.
- **Problematic Pool:** If a specific pool is providing bad quotes or has stale data that the automated health scorer hasn't caught yet, disable that specific `venue_ref`.
- **Maintenance:** Disable specific sources during scheduled maintenance.

## Operations

### 1. View Current Kill Switch State

**Endpoint:** `GET /api/v1/admin/kill-switch`

Requires admin authentication. Returns the current set of forced-exclude
overrides for sources and venues.

**Example Request:**
```bash
curl -H "x-admin-token: $ADMIN_TOKEN" \
  http://localhost:8080/api/v1/admin/kill-switch
```

**Example Request (production — token required):**
```bash
curl http://localhost:8080/api/v1/admin/kill-switch \
  -H "x-admin-token: $ADMIN_AUTH_TOKEN"
```

**Example Response:**
```json
{
  "sources": {
    "amm": "force_exclude"
  },
  "venues": {
    "amm:0x123...": "force_exclude"
  }
}
```

### 2. Disable a Source or Venue

**Endpoint:** `POST /api/v1/admin/kill-switch`

**To disable all AMMs:**
```bash
curl -X POST http://localhost:8080/api/v1/admin/kill-switch \
  -H "x-admin-token: $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -H "x-admin-token: $ADMIN_AUTH_TOKEN" \
  -d '{
    "sources": {
      "amm": "force_exclude"
    },
    "venues": {}
  }'
```

**To disable a specific venue:**
```bash
curl -X POST http://localhost:8080/api/v1/admin/kill-switch \
  -H "x-admin-token: $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -H "x-admin-token: $ADMIN_AUTH_TOKEN" \
  -d '{
    "sources": {},
    "venues": {
      "amm:0x123...": "force_exclude"
    }
  }'
```

### 3. Re-enable a Source or Venue

Send a `POST` request with an empty state or with the specific entry removed.

```bash
curl -X POST http://localhost:8080/api/v1/admin/kill-switch \
  -H "x-admin-token: $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -H "x-admin-token: $ADMIN_AUTH_TOKEN" \
  -d '{
    "sources": {},
    "venues": {}
  }'
```

## Key Rotation During an Incident

If you suspect the admin token has been compromised:

1. Generate a new token: `openssl rand -hex 32`
2. Set the new value in all instances' `ADMIN_AUTH_TOKEN` environment variable
   (follow `docs/key_rotation.md` for zero-downtime rotation).
3. Verify old token is rejected: send a request with the old token and confirm
   you receive `401 Unauthorized`.
4. Re-issue any in-flight kill switch commands with the new token.

## Monitoring & Observability

- **Logs:** Look for "Admin updating kill switch state" in the API logs.
- **Metrics:**
    - `stellarroute_kill_switch_status{type="source", name="amm"}`: Value `1` if disabled, `0` if enabled.
    - `stellarroute_kill_switch_status{type="venue", name="..."}`: Value `1` if disabled.
- **Quotes:** The `exclusion_diagnostics` field in the `/api/v1/quote` response will list venues excluded due to `override`.

## Troubleshooting

- **401 Unauthorized:** Check that `ADMIN_AUTH_TOKEN` is set on the API instance and that you are using the correct value in `x-admin-token`.
- **State not syncing:** Ensure Redis is reachable and all API instances have a connection to the same Redis cluster.
- **Immediate effect not seen:** Propagation delay is up to 5 seconds. If longer, check API instance connectivity.
- **`401 Unauthorized`:** Confirm `x-admin-token` (or `Authorization: Bearer`) matches the server's `ADMIN_AUTH_TOKEN` exactly. In production this now applies to `GET` as well as `POST`.
- **API won't start in production:** If logs show a refusal to boot citing `ADMIN_AUTH_TOKEN`, set that variable — production requires it unconditionally (see Authentication section above).
