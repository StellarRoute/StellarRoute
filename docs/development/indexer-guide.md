# Indexer Service Operations and Troubleshooting Guide

This guide explains how to run, validate, and troubleshoot the StellarRoute indexer service.

## What the Indexer Does

The indexer service continuously ingests two liquidity sources:

- SDEX offers via Horizon
- Soroban AMM pool state via Soroban RPC

It writes normalized liquidity data into Postgres for API quote/routing reads.

## Prerequisites

- Docker and Docker Compose
- Rust toolchain (see [SETUP.md](./SETUP.md))
- A local copy of this repository

## 1. Start Local Dependencies

From the repository root:

```bash
docker-compose up -d
```

This starts:

- PostgreSQL on `localhost:5432`
- Redis on `localhost:6379`

## 2. Configure Environment Variables

The indexer requires these variables:

- `DATABASE_URL`
- `STELLAR_HORIZON_URL`
- `SOROBAN_RPC_URL`
- `ROUTER_CONTRACT_ADDRESS`

Optional operational variables:

- `STARTUP_CREDENTIAL_CHECK=true` to run startup reachability checks for DB/Horizon/Soroban
- `RUST_LOG=stellarroute_indexer=info` (or `debug`) for log verbosity
- `LOG_FORMAT=json` for structured JSON logs

PowerShell example:

```powershell
$env:DATABASE_URL = "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute"
$env:STELLAR_HORIZON_URL = "https://horizon-testnet.stellar.org"
$env:SOROBAN_RPC_URL = "https://soroban-rpc.testnet.stellar.org"
$env:ROUTER_CONTRACT_ADDRESS = "<your-router-contract-address>"
$env:STARTUP_CREDENTIAL_CHECK = "true"
```

Bash example:

```bash
export DATABASE_URL="postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute"
export STELLAR_HORIZON_URL="https://horizon-testnet.stellar.org"
export SOROBAN_RPC_URL="https://soroban-rpc.testnet.stellar.org"
export ROUTER_CONTRACT_ADDRESS="<your-router-contract-address>"
export STARTUP_CREDENTIAL_CHECK=true
```

## 3. Run the Indexer

From the repository root:

```bash
cargo run -p stellarroute-indexer
```

On startup, the service:

1. Loads environment configuration
2. Connects to Postgres
3. Runs indexer migrations automatically
4. Starts SDEX indexing and AMM aggregation loops

## 4. Polling vs Streaming (SSE) Mode

The indexer supports two ingestion modes for SDEX offers, controlled by the `HORIZON_MODE` environment variable.

### Configuration

| Variable | Values | Default |
|---|---|---|
| `HORIZON_MODE` | `poll` \| `sse` | `poll` |
| `POLL_INTERVAL_SECS` | integer seconds | `2` (Horizon fetch timeout) |

The binary maps the config value to an internal mode and passes it to `SdexIndexer::with_mode()`:

```bash
# Polling mode (default) — fetches offers in a loop every ~5 seconds
export HORIZON_MODE=poll

# SSE streaming mode — subscribes to Horizon's server-sent event stream
export HORIZON_MODE=sse
```

### Polling mode

The indexer calls `GET /offers` from Horizon on a fixed cadence (loop sleeps 5 seconds between runs). Each iteration:
1. Fetches a page of offers (up to `horizon_limit`, default 200 per page).
2. Upserts assets and offers into Postgres.
3. Records `stellarroute_indexer_offers_indexed_total` for the batch.

Polling is safe to use with any Horizon instance and is the recommended mode for most deployments.

### SSE streaming mode

When `HORIZON_MODE=sse`, the indexer opens a persistent HTTP connection to the Horizon `/offers` stream endpoint. Each incoming event:
1. Increments `stellarroute_indexer_sse_events_received_total{source="sdex"}`.
2. Advances an in-memory cursor (paging token) for reconnect continuity.
3. Upserts the offer in real time without a polling delay.

**Automatic fallback to polling:** If the SSE connection fails to establish 3 consecutive times, the indexer logs a warning and switches permanently to polling mode for the remainder of the process lifetime:

```
WARN SSE connection failed consistently; falling back to polling
```

An unexpected stream close (server-side disconnect) is treated as a single failure event, increments `stellarroute_indexer_sse_disconnects_total{source="sdex"}`, and triggers a reconnect loop without counting toward the 3-failure threshold.

### SSE metrics

Two Prometheus counters track SSE health:

| Metric | Label | Description |
|---|---|---|
| `stellarroute_indexer_sse_events_received_total` | `source="sdex"` | Total offer events received over SSE |
| `stellarroute_indexer_sse_disconnects_total` | `source="sdex"` | Total times the SSE stream closed unexpectedly |

Expose these via `GET /metrics` (served by the API binary when Prometheus is configured) or scrape with a Prometheus instance pointed at the indexer metrics port.

Example Prometheus query for disconnect rate:

```promql
rate(stellarroute_indexer_sse_disconnects_total{source="sdex"}[5m])
```

If this rate is consistently above zero, the Horizon endpoint is unstable or rate-limiting the persistent connection — switch to polling mode or reduce load.

### Troubleshooting: Horizon rate limits

**Symptoms**
- Logs contain `SDEX polling rate-limited; preserving cursor and waiting` (polling mode) or frequent `SSE connection failed` messages (SSE mode).
- `stellarroute_indexer_horizon_throttle_events_total` counter increases rapidly.
- `sdex_offers.updated_at` stops advancing.

**What the indexer does automatically**
- Honors the `Retry-After` response header when present.
- Applies adaptive jittered exponential backoff on consecutive 429 responses.
- In polling mode: preserves the current cursor so no offers are skipped.
- In SSE mode: falls back to polling after 3 connection failures (which may include 429s that close the stream).

**Operator actions**
1. Check `stellarroute_indexer_horizon_consecutive_429s{source="sdex"}` — if it stays elevated, reduce poll frequency or switch to a non-rate-limited Horizon endpoint.
2. If using a public testnet Horizon instance, consider running your own via Stellar's `quickstart` Docker image to get a higher rate limit.
3. For SSE mode: if the stream disconnects repeatedly but polling succeeds, set `HORIZON_MODE=poll` and monitor `sdex_offers.updated_at` for recovery.
4. Confirm recovery by querying:

```sql
SELECT MAX(updated_at) AS sdex_last_update, NOW() - MAX(updated_at) AS age
FROM sdex_offers;
```

Age should return to under 30 seconds within a few polling cycles after the rate limit window passes.

## 5. Database Surfaces Written by the Indexer

Primary tables/views to inspect:

- `sdex_offers`: latest indexed SDEX offers
- `amm_pool_reserves`: latest indexed AMM reserve state per pool
- `normalized_liquidity` (view): unified `sdex + amm` read model
- `soroban_sync_cursors`: durable Soroban discovery cursor state
- `db_health_metrics`: database health metrics emitted by monitoring jobs

Related architecture references:

- [database-schema.md](../architecture/database-schema.md)
- [RECONCILIATION.md](../architecture/RECONCILIATION.md)

Quick inspection queries:

```sql
-- Freshness and volume signals
SELECT COUNT(*) AS sdex_offer_count, MAX(updated_at) AS sdex_last_update
FROM sdex_offers;

SELECT COUNT(*) AS amm_pool_count, MAX(updated_at) AS amm_last_update
FROM amm_pool_reserves;

SELECT venue_type, COUNT(*) AS rows, MAX(updated_at) AS last_update
FROM normalized_liquidity
GROUP BY venue_type;

-- Soroban discovery cursor status
SELECT job_name, cursor, last_seen_ledger, status, updated_at
FROM soroban_sync_cursors
ORDER BY updated_at DESC;
```

## 6. Reconciliation Overview

Reconciliation compares cross-source data consistency (staleness, price drift, ledger alignment, and more) and documents operational SQL for drift/repair analysis.

Use:

- [RECONCILIATION.md](../architecture/RECONCILIATION.md)

Key reconciliation artifacts to monitor when enabled:

- `reconciliation_checks`
- `drift_events`
- `repair_actions`
- `reconciliation_runs`
- `critical_issues` (view)

## 7. Common Failure Modes and Remediation

### A. Horizon rate limits (`429 Too Many Requests`)

Symptoms:

- Logs indicating backoff/rate-limit events
- Slower offer ingestion cadence

What the indexer does automatically:

- Honors `Retry-After` when present
- Applies adaptive jittered backoff
- Preserves cursor progress instead of advancing on `429`

Operator actions:

1. Keep the service running unless there is sustained failure.
2. Reduce concurrent load against the same Horizon endpoint.
3. Confirm forward movement in `sdex_offers.updated_at` after the backoff window.

### B. Cursor stalls or gaps in Soroban discovery

Symptoms:

- `soroban_sync_cursors.updated_at` stale for long periods
- `last_seen_ledger` not advancing while chain activity exists

Checks:

```sql
SELECT job_name, cursor, last_seen_ledger, status, updated_at,
       NOW() - updated_at AS cursor_age
FROM soroban_sync_cursors
WHERE job_name = 'soroban_pool_discovery';
```

Remediation:

1. Verify `SOROBAN_RPC_URL` reachability.
2. Restart the indexer process.
3. Re-check cursor advancement and `amm_pool_reserves.updated_at` freshness.

### C. Stale offers or stale AMM reserves

Symptoms:

- `MAX(updated_at)` for `sdex_offers` or `amm_pool_reserves` is old
- API quote quality degradation or missing routes

Checks:

```sql
SELECT
  MAX(updated_at) AS sdex_last_update,
  NOW() - MAX(updated_at) AS sdex_age
FROM sdex_offers;

SELECT
  MAX(updated_at) AS amm_last_update,
  NOW() - MAX(updated_at) AS amm_age
FROM amm_pool_reserves;
```

Remediation:

1. Confirm DB connectivity and free connections.
2. Confirm Horizon/Soroban endpoints are reachable.
3. Restart indexer if ingestion does not recover.
4. Use reconciliation diagnostics for deeper drift analysis.

## 8. Health Verification Checklist

After startup or incident recovery, verify:

1. Container dependencies are healthy (`docker-compose ps`).
2. Indexer process is running and logging periodic indexing activity.
3. `sdex_offers` and `amm_pool_reserves` both show recent `updated_at` values.
4. `soroban_sync_cursors` shows advancing `last_seen_ledger`.
5. `normalized_liquidity` contains rows for expected venue types.
6. (Optional) If API is running, confirm `GET /health` is healthy.

Example API check:

```bash
curl http://localhost:3000/health
```

## 9. Operational Notes

- Migrations are run automatically by the indexer binary on startup. Applied in order:
  - `0001_init.sql` — base schema (assets, sdex_offers)
  - `0002_performance_indexes.sql` — query indexes
  - `0003_trading_pairs_and_snapshots.sql` — trading pairs + orderbook snapshots
  - `0004_normalized_liquidity.sql` — unified liquidity view
  - `0005_venue_health_scores.sql` — venue health tracking
  - `0006_maintenance_policies.sql` — retention/archival policies
  - `0007_backfill_and_normalized_storage.sql` — backfill + materialized storage
  - `0008_soroban_discovery_cursors.sql` — Soroban event cursors
  - `0009_finalize_unified_liquidity.sql` — unified liquidity finalization
  - `0010_asset_metadata.sql` — asset metadata fields
  - `0011_trace_context_provenance.sql` — trace context + provenance columns
  - `0012_contract_swap_activity.sql` — on-chain swap activity log
  - `0013_amm_pools.sql` — AMM pool registry table
- The indexer can continue running through transient ingestion errors; investigate sustained staleness rather than brief blips.
- For architecture-level data quality strategy and SQL diagnostics, use [RECONCILIATION.md](../architecture/RECONCILIATION.md) as the source of truth.