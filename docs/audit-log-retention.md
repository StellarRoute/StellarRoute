# Route Audit Log — Retention Policy & Storage Cost Guide

## Overview

The `route_audit_log` table stores a structured, privacy-safe record of every
route decision.  This document covers:

1. [Default retention policy](#default-retention-policy)
2. [Storage cost estimates](#storage-cost-estimates)
3. [Tuning options](#tuning-options)
4. [Pruning operations](#pruning-operations)
5. [Privacy guarantees](#privacy-guarantees)
6. [Schema reference](#schema-reference)

---

## Default Retention Policy

| Parameter         | Value      |
|-------------------|------------|
| Retention window  | **30 days** |
| Enforcement       | `retained_until` generated column (`logged_at + 30 days`) |
| Pruning trigger   | Application background task or `pg_cron` job |
| Pruning method    | `DELETE FROM route_audit_log WHERE retained_until <= NOW()` |

The `retained_until` column is a PostgreSQL **generated column** — it is
computed automatically and never needs to be set by the application.

---

## Storage Cost Estimates

Row size depends on the number of exclusions and path hops.  A typical
single-hop quote with 2–3 exclusions produces a row of approximately **800 bytes**.

| Throughput | Rows/day | Rows/30 days | Raw storage (30 days) |
|------------|----------|--------------|----------------------|
| 10 req/s   | 864 K    | 25.9 M       | ~21 GB               |
| 100 req/s  | 8.6 M    | 259 M        | ~207 GB              |
| 500 req/s  | 43.2 M   | 1.3 B        | ~1.0 TB              |
| 1 000 req/s| 86.4 M   | 2.6 B        | ~2.1 TB              |

> **Note:** These are uncompressed estimates.  PostgreSQL TOAST compression
> typically reduces JSONB column sizes by 30–60%.

---

## Tuning Options

### Option 1 — Reduce the retention window

Change the generated column expression in a migration:

```sql
-- Reduce to 7 days
ALTER TABLE route_audit_log
  DROP COLUMN retained_until;

ALTER TABLE route_audit_log
  ADD COLUMN retained_until TIMESTAMPTZ NOT NULL
    GENERATED ALWAYS AS (logged_at + INTERVAL '7 days') STORED;
```

Or prune more aggressively from the application:

```rust
// Prune entries older than 7 days
audit_store.prune_older_than(chrono::Duration::days(7)).await?;
```

### Option 2 — Partition by day

For high-throughput deployments, range-partition the table by `logged_at`.
This makes pruning a fast `DROP TABLE` on the oldest partition rather than
a row-by-row `DELETE`.

```sql
-- Example: convert to partitioned table (requires downtime or pg_partman)
CREATE TABLE route_audit_log_partitioned (
    LIKE route_audit_log INCLUDING ALL
) PARTITION BY RANGE (logged_at);

-- Create daily partitions
CREATE TABLE route_audit_log_2026_04_25
    PARTITION OF route_audit_log_partitioned
    FOR VALUES FROM ('2026-04-25') TO ('2026-04-26');
```

### Option 3 — Sampling

Only log a fraction of normal requests while always logging errors and
no-route outcomes.  Set `AUDIT_LOG_ENABLED=false` and implement selective
emission in the quote handler:

```rust
// Always log non-success outcomes; sample 1-in-10 successes
let should_log = outcome != AuditOutcome::Success || rand::random::<u8>() < 26; // ~10%
if should_log {
    state.audit_writer.emit(/* ... */);
}
```

### Option 4 — Offload to object storage

Use `COPY … TO` to export old rows to S3/GCS before deleting them:

```sql
COPY (
    SELECT * FROM route_audit_log
    WHERE logged_at < NOW() - INTERVAL '7 days'
) TO PROGRAM 'aws s3 cp - s3://my-bucket/audit/$(date +%Y-%m-%d).csv'
WITH (FORMAT CSV, HEADER);

DELETE FROM route_audit_log
WHERE logged_at < NOW() - INTERVAL '7 days';
```

---

## Pruning Operations

### Application-level pruning (recommended)

The `AuditStore` exposes two pruning methods:

```rust
// Delete all rows past their retention deadline (uses the generated column)
let deleted = audit_store.prune_expired().await?;

// Delete rows older than a custom duration
let deleted = audit_store.prune_older_than(chrono::Duration::days(7)).await?;
```

Schedule this from a background task in `AppState::new()`:

```rust
let store = AuditStore::new(db.write_pool().clone());
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(3600)); // hourly
    loop {
        interval.tick().await;
        match store.prune_expired().await {
            Ok(n) => tracing::info!(deleted = n, "Audit log pruned"),
            Err(e) => tracing::warn!(error = %e, "Audit log pruning failed"),
        }
    }
});
```

### pg_cron (PostgreSQL-native scheduling)

```sql
-- Requires pg_cron extension
SELECT cron.schedule(
    'prune-audit-log',
    '0 3 * * *',  -- 03:00 UTC daily
    $$DELETE FROM route_audit_log WHERE retained_until <= NOW()$$
);
```

---

## Privacy Guarantees

All entries are redacted by [`AuditRedactor`] before insertion:

| Field                        | Treatment                                      |
|------------------------------|------------------------------------------------|
| `inputs.base` / `inputs.quote` | Issuer suffix replaced with `[REDACTED]`     |
| `selected.path[*].from/to`   | Issuer suffix replaced with `[REDACTED]`       |
| `venue_ref`                  | **Not redacted** — public on-chain data        |
| `price`, `amount`            | **Not redacted** — non-identifying numerics    |
| `request_id`, `trace_id`     | **Not redacted** — required for correlation    |

Example:
- Before: `"USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5"`
- After:  `"USDC:[REDACTED]"`

The redaction is **idempotent** — applying it twice produces the same result.

---

## Schema Reference

```sql
CREATE TABLE route_audit_log (
    id              BIGSERIAL   PRIMARY KEY,
    request_id      TEXT        NOT NULL,          -- x-request-id header
    trace_id        TEXT        NOT NULL DEFAULT '', -- W3C trace ID (hex)
    logged_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    latency_ms      INTEGER     NOT NULL DEFAULT 0,
    outcome         TEXT        NOT NULL            -- success|no_route|stale_data|error
                    CHECK (outcome IN ('success', 'no_route', 'stale_data', 'error')),
    cache_hit       BOOLEAN     NOT NULL DEFAULT FALSE,
    inputs          JSONB       NOT NULL,           -- redacted request inputs
    selected        JSONB,                          -- redacted selected route (NULL on error)
    exclusions      JSONB       NOT NULL DEFAULT '[]',
    retained_until  TIMESTAMPTZ NOT NULL            -- logged_at + 30 days (generated)
                    GENERATED ALWAYS AS (logged_at + INTERVAL '30 days') STORED
);
```

### Indexes

| Index                         | Purpose                                    |
|-------------------------------|--------------------------------------------|
| `idx_audit_request_id`        | Lookup by `x-request-id` for correlation   |
| `idx_audit_trace_id`          | Lookup by W3C trace ID                     |
| `idx_audit_logged_at`         | Time-range queries                         |
| `idx_audit_retention`         | Fast pruning (partial, `retained_until <= NOW()`) |
| `idx_audit_outcome_time`      | Outcome-filtered time-range queries        |

---

## Environment Variables

| Variable            | Default | Description                                      |
|---------------------|---------|--------------------------------------------------|
| `AUDIT_LOG_ENABLED` | `true`  | Set to `false` or `0` to disable audit logging   |
