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

## 4. Polling vs Streaming Modes

- The runtime binary currently starts SDEX indexing in polling mode.
- The SDEX indexer library also supports a streaming mode API (`IndexingMode::Streaming`).
- Horizon stream support is implemented with a polling-backed stream abstraction today and is documented as SSE-ready in code.

Operationally, use this guide assuming polling mode for production runs of the current binary.

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

- Migrations are run automatically by the indexer binary on startup.
- The indexer can continue running through transient ingestion errors; investigate sustained staleness rather than brief blips.
- For architecture-level data quality strategy and SQL diagnostics, use [RECONCILIATION.md](../architecture/RECONCILIATION.md) as the source of truth.