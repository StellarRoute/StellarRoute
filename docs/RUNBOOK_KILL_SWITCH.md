# StellarRoute Emergency Operations: Kill Switch Runbook

This runbook describes how to use the API-level kill switches to disable unstable or problematic routing sources and venues without redeploying the application.

## Overview

The kill switch allows operational control over which liquidity sources (SDEX, AMM) and specific venues (individual AMM pools or SDEX pairs) are used by the routing engine. Changes take effect within 5 seconds across all API instances via Redis synchronization.

## Scenarios

- **Unstable AMM Protocol:** If a specific AMM protocol is experiencing issues (e.g., Soroban RPC latency, contract bugs), disable the entire `amm` source.
- **Problematic Pool:** If a specific pool is providing bad quotes or has stale data that the automated health scorer hasn't caught yet, disable that specific `venue_ref`.
- **Maintenance:** Disable specific sources during scheduled maintenance.

## Operations

### 1. View Current Kill Switch State

**Endpoint:** `GET /api/v1/admin/kill-switch`

**Example Request:**
```bash
curl http://localhost:8080/api/v1/admin/kill-switch
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
  -H "Content-Type: application/json" \
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
  -H "Content-Type: application/json" \
  -d '{
    "sources": {},
    "venues": {
      "amm:0x123...": "force_exclude"
    }
  }'
```

### 3. Re-enable a Source or Venue

Send a `POST` request with an empty state or with the specific entry removed/set to `force_include` (though removing it is usually sufficient to return to default behavior).

```bash
curl -X POST http://localhost:8080/api/v1/admin/kill-switch \
  -H "Content-Type: application/json" \
  -d '{
    "sources": {},
    "venues": {}
  }'
```

## Monitoring & Observability

- **Logs:** Look for "Admin updating kill switch state" in the API logs.
- **Metrics:**
    - `stellarroute_kill_switch_status{type="source", name="amm"}`: Value `1` if disabled, `0` if enabled.
    - `stellarroute_kill_switch_status{type="venue", name="..."}`: Value `1` if disabled.
- **Quotes:** The `exclusion_diagnostics` field in the `/api/v1/quote` response will list venues excluded due to `override`.

## Troubleshooting

- **State not syncing:** Ensure Redis is reachable and all API instances have a connection to the same Redis cluster.
- **Immediate effect not seen:** Propagation delay is up to 5 seconds. If longer, check API instance connectivity.
