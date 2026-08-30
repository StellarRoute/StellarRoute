# Operator Runbook: Indexer AMM Refresh Failures & Degradation

This runbook guides operators through diagnosing, mitigating, and resolving repeated Soroban AMM pool refresh failures in the `stellarroute-indexer`.

---

## Core Operational & Product Principles

1. **Stale AMM data must NOT hard-block SDEX swaps.**
   - StellarRoute aggregates liquidity from both Classic SDEX orderbooks and Soroban AMM liquidity pools via the `normalized_liquidity` unified surface.
   - When Soroban RPC calls fail or AMM ingestion lags, quotes must gracefully fallback to available SDEX liquidity or return degraded quotes (`"degraded": true`), never hard-failing (`5xx` / unhandled errors) live user swap requests.
   - Classic one-hop SDEX execution (`PathPaymentStrictSend`) remains the primary fallback and must stay fully operational during AMM ingestion outages.
2. **Fail-safe degradation over erroneous routing:**
   - If an AMM pool's reserves have not been updated within `stale_threshold_secs` (default: 300s / 5 minutes), the indexer cleanup routine removes stale entries from `amm_pool_reserves`, preventing the routing engine from proposing trades against stale pool reserves.
   - Quote requests during partial AMM outages will execute against SDEX or provide degraded warnings without halting swap capability.

---

## Metrics to Watch

The following Prometheus metrics (exposed at `GET /metrics` on the API and tracked by the lag monitor) are critical when diagnosing AMM refresh failures:

| Metric Name | Type | Labels | Description | Healthy Value |
|---|---|---|---|---|
| `stellarroute_indexer_lag_ledgers` | Gauge | `source="amm"` | Number of ledgers AMM indexer is behind live Horizon/Soroban sequence | `< 10` ledgers |
| `stellarroute_indexer_lag_seconds` | Gauge | `source="amm"` | Estimated wall-clock lag in seconds for AMM data | `< 50s` |
| `stellarroute_indexer_sync_status` | Gauge | `source="amm"` | AMM sync health: `1` (OK), `0` (Warning), `-1` (Critical), `-2` (Unknown) | `1` |
| `stellarroute_indexer_last_indexed_ledger` | Gauge | `source="amm"` | Most recently indexed ledger sequence by the AMM loop | Advancing ~every 5s |
| `stellarroute_indexer_horizon_ledger` | Gauge | `instance="default"` | Current Horizon latest ledger | Advancing ~every 5s |
| `stellarroute_reconciliation_drift_events_total` | Counter | — | Total drift anomalies detected between SDEX and AMM ledger alignment | Flat / minimal |
| `stellarroute_reconciliation_repairs_total` | Counter | — | Total automatic repair operations executed | Flat / slow increment |
| `stellarroute_quote_requests_total` | Counter | `outcome`, `cache_hit` | Quote request outcomes to monitor overall health | `outcome="success"` |

---

## Relevant Logs & Error Signatures

Inspect indexer container logs (`stellarroute-indexer`) for the following log signatures:

### 1. Repeated Aggregation Cycle Failure
```text
ERROR stellarroute_indexer::amm: AMM aggregation cycle failed: <reason>
ERROR stellarroute_indexer::amm: Initial AMM aggregation failed: <reason>
ERROR stellarroute-indexer: AMM aggregator error: <reason>
```
- **Meaning:** The entire AMM poll loop iteration failed (e.g., Soroban RPC unreachable, database connection error, or unrecoverable RPC JSON error).

### 2. Single Pool Batch / Reserve Fetch Failure
```text
WARN stellarroute_indexer::amm: Failed to process pool batch: <reason>
WARN stellarroute_indexer::amm: Failed to process pool <address>: <reason>
```
- **Meaning:** Transient error fetching contract data for a specific pool or parsing XDR entry. Other pools in the batch may still proceed.

### 3. Event Lookback / Cursor Retention Exceeded
```text
WARN stellarroute_indexer::amm: AMM discovery cursor older than RPC retention lookback; clamping
WARN stellarroute_indexer::amm: AMM pool discovery getEvents failed; continuing with registry fallback
```
- **Meaning:** The cursor stored in `soroban_sync_cursors` is further back than the Soroban RPC node's historical event retention window (e.g., error `-32602: startLedger must be within history retention`). The indexer automatically clamps the cursor to `current_ledger - 10000` and falls back to operator registry pools.

### 4. Stale Pool Cleanup Notification
```text
INFO stellarroute_indexer::amm: Cleaned up N stale pool entries
```
- **Meaning:** Pools whose `updated_at` fell behind `stale_threshold_secs` were deleted from `amm_pool_reserves` to avoid executing trades against stale reserves.

---

## Alerting & Paging Tiers

| Severity | Condition | User Impact | Action / Paging |
|---|---|---|---|
| **Sev-3 (Warning)** | `stellarroute_indexer_lag_ledgers{source="amm"} > 10` for > 5m, or intermittent batch failures | None to Low: AMM quotes marked `degraded: true` or routing selects SDEX | **Do NOT page off-hours.** Post notice to `#ops-alerts` during business hours. Monitor for self-recovery. |
| **Sev-2 (Degraded)** | `stellarroute_indexer_lag_ledgers{source="amm"} > 60` (> 300s) for > 5m, or Soroban RPC rate-limited | Medium: Stale AMM pools purged; swaps route exclusively through SDEX. Quotes show degraded warning. | **Notify secondary on-call / Slack alert.** Verify Soroban RPC health and database availability. |
| **Sev-1 (Critical)** | Indexer process crashed entirely, DB unreachable, or both `sdex` and `amm` sync status = `-1` | High: Orderbook data stale across all venues, API health returns 503. | **Page primary on-call immediately.** Follow emergency indexer restart and DB verification procedure. |

> [!IMPORTANT]
> Stale AMM pool state alone is a **Sev-2 (Degraded)** event, NOT a total system outage. Do not panic-restart the API or purge SDEX tables while triaging AMM refresh failures.

---

## Triage & Diagnostic Steps

### Step 1: Check System & Component Health
Query the API's component health endpoint:
```bash
curl -s http://localhost:8080/health/deps | jq .
```
Look for `indexer_lag_amm` component status:
- If `"indexer_lag_amm": "warning (lag: X ledgers)"` or `"indexer_lag_amm": "critical"`, proceed to Step 2.
- Verify `database` is `"healthy"`.

### Step 2: Query Indexer Lag & Sync Gauges
Check current Prometheus metrics:
```bash
curl -s http://localhost:8080/metrics | grep -E "stellarroute_indexer_(lag|sync_status)"
```
Example degraded output:
```text
stellarroute_indexer_lag_ledgers{source="amm"} 72
stellarroute_indexer_lag_seconds{source="amm"} 360
stellarroute_indexer_sync_status{source="amm"} -1
stellarroute_indexer_sync_status{source="sdex"} 1
```

### Step 3: Inspect Indexer Logs
View the last 100 lines of indexer logs:
```bash
docker compose logs --tail=100 indexer
```
Filter specifically for AMM warnings and errors:
```bash
docker compose logs --tail=200 indexer | grep -E "(AMM|soroban|pool)"
```

### Step 4: Verify Database State
Connect to Postgres (`psql $DATABASE_URL`) to inspect cursors and active pool reserves:

1. **Check discovery cursor status:**
   ```sql
   SELECT job_name, cursor, last_seen_ledger, status, updated_at
   FROM soroban_sync_cursors
   WHERE job_name = 'soroban_pool_discovery';
   ```

2. **Check last updated AMM reserves:**
   ```sql
   SELECT pool_address, reserve_a, reserve_b, last_updated_ledger, updated_at
   FROM amm_pool_reserves
   ORDER BY updated_at DESC
   LIMIT 10;
   ```

3. **Check count of active pools in unified liquidity:**
   ```sql
   SELECT venue_type, count(*), max(updated_at)
   FROM normalized_liquidity
   GROUP BY venue_type;
   ```

4. **Verify SDEX is unaffected:**
   ```sql
   SELECT count(*), max(last_modified_ledger)
   FROM sdex_offers;
   ```

---

## Remediation Procedures

### Procedure A: Upstream Soroban RPC Connectivity / Rate Limiting
If logs show HTTP 429 (Too Many Requests), connection timeouts, or RPC JSON errors:

1. Check that `SOROBAN_RPC_URL` is accessible from the indexer host:
   ```bash
   curl -s -X POST -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","id":1,"method":"getLatestLedger"}' \
     "$SOROBAN_RPC_URL" | jq .
   ```
2. If the current RPC endpoint is failing or rate-limiting:
   - Switch to a backup Soroban RPC provider in `.env` / deployment configuration:
     ```env
     SOROBAN_RPC_URL=https://soroban-testnet.stellar.org
     ```
   - Restart the indexer container:
     ```bash
     docker compose restart indexer
     ```

### Procedure B: Stalled Discovery Cursor / Retention Window Mismatch
If logs show `getEvents` failing due to ledger range errors and automatic clamping did not resolve the stall:

1. Check current live ledger on Soroban:
   ```bash
   CURRENT_LEDGER=$(curl -s -X POST -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","id":1,"method":"getLatestLedger"}' \
     "$SOROBAN_RPC_URL" | jq -r '.result.sequence')
   echo "Current ledger: $CURRENT_LEDGER"
   ```
2. Reset the discovery cursor in Postgres to the current ledger sequence:
   ```sql
   UPDATE soroban_sync_cursors
   SET cursor = '<CURRENT_LEDGER>',
       last_seen_ledger = <CURRENT_LEDGER>,
       status = 'running',
       updated_at = now()
   WHERE job_name = 'soroban_pool_discovery';
   ```
3. Restart indexer to trigger bootstrap from registry:
   ```bash
   docker compose restart indexer
   ```

### Procedure C: Re-seed Pools from Fallback Registry
If contract event discovery is temporarily broken on the testnet router contract:

1. Ensure target pools are listed in the `amm_pools` table:
   ```sql
   SELECT pool_address, active FROM amm_pools;
   ```
2. If missing, register known pool contracts:
   ```sql
   INSERT INTO amm_pools (pool_address, network, active, metadata)
   VALUES ('<POOL_CONTRACT_ADDRESS>', 'testnet', true, '{}'::jsonb)
   ON CONFLICT (pool_address) DO UPDATE SET active = true, updated_at = now();
   ```
3. Alternatively, supply comma-separated addresses via `AMM_POOLS` environment variable in the indexer service configuration.

### Procedure D: Safe Restart of the Indexer Service
To cleanly restart the indexer without disrupting active DB connections:
```bash
docker compose -f docker-compose.yml -f docker-compose.app.yml --profile indexer restart indexer
```
Verify startup logs:
```bash
docker compose logs -f --tail=50 indexer
```
Look for:
```text
INFO stellarroute_indexer::amm: Starting AMM pool aggregation loop
INFO stellarroute_indexer::amm: Using N pools from registry fallback
```

---

## Recovery Verification Checklist

Before resolving an incident, verify all of the following:

- [ ] `stellarroute_indexer_lag_ledgers{source="amm"}` dropped below 10 ledgers.
- [ ] `stellarroute_indexer_sync_status{source="amm"}` returned to `1` (OK).
- [ ] `GET /health/deps` returns `"status": "ok"` with healthy lag indicators.
- [ ] `amm_pool_reserves` rows show `updated_at` within the last 60 seconds.
- [ ] `normalized_liquidity` contains active `amm` rows with valid price and reserve figures.
- [ ] Quote endpoint (`GET /api/v1/quote/:base/:quote`) returns fresh quotes without unexpected error codes.
- [ ] SDEX quotes and one-hop swap executions remained intact throughout the mitigation.
