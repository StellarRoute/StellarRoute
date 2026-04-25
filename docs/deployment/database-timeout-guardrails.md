# Database Timeout Guardrails

This document describes SQL timeout defaults and tuning guidance for StellarRoute API.

## Defaults

Configured per database connection during pool setup:

- DB_STATEMENT_TIMEOUT_MS: 5000
- DB_LOCK_TIMEOUT_MS: 2000
- DB_IDLE_IN_TXN_TIMEOUT_MS: 5000

Slow query warning threshold:

- DB_SLOW_QUERY_MS: 500

## Why these guardrails exist

- Prevent runaway read queries from hanging workers.
- Bound lock waits under contention.
- Prevent idle transactions from holding resources indefinitely.

## Tuning guidance

- Read-heavy production APIs:
  - Start with statement timeout 3000-7000ms.
- Lock-heavy maintenance windows:
  - Use higher lock timeout only if needed.
- High latency environments:
  - Increase statement timeout conservatively and watch p95/p99.

## Operational checks

- Ensure request logs include request_id for correlation.
- Monitor slow query logs and top SQL fingerprints.
- Verify readiness probes continue to pass under load.
