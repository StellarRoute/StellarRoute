# Quote Purger Runbook

## Overview

The **Quote Purger** is an automated background task that removes stale quote artifacts and audit logs to maintain database performance and reduce storage costs. It runs on a configurable schedule with built-in safeguards against over-aggressive deletion.

**Related Issue**: #450 [backend] Automated stale-quote purger with observability hooks

### What Gets Purged

1. **`replay_artifacts` table**: Stale snapshots used for deterministic quote replay and post-incident analysis (default retention: 30 days)
2. **`route_audit_log` table**: Audit trail of all quote routing decisions (default retention: 30 days)

Both tables have automatic age-based retention calculated at purge time with comprehensive metrics exported for observability.

---

## Configuration

### Environment Variables

All purger settings are controlled via environment variables with the `QUOTE_PURGER_` prefix:

```bash
# Master control
QUOTE_PURGER_ENABLED=true

# Purge interval in seconds (default: 3600 = 1 hour)
QUOTE_PURGER_INTERVAL_SECS=3600

# Retention policies (days)
QUOTE_PURGER_REPLAY_RETENTION_DAYS=30
QUOTE_PURGER_AUDIT_LOG_RETENTION_DAYS=30

# Safeguards: batch sizes (prevent long-running locks)
QUOTE_PURGER_REPLAY_BATCH_SIZE=1000          # rows per delete batch
QUOTE_PURGER_AUDIT_LOG_BATCH_SIZE=5000       # larger batches safe for append-only table

# Safeguards: iteration limits (prevent runaway purges)
QUOTE_PURGER_MAX_ITERATIONS=100              # max batches before rate-limiting

# Toggle specific purges
QUOTE_PURGER_PURGE_REPLAY_ARTIFACTS=true
QUOTE_PURGER_PURGE_AUDIT_LOG=true

# Observability
QUOTE_PURGER_LOG_METRICS=true                # emit structured logs with metrics

# Alerting thresholds
QUOTE_PURGER_SLOW_PURGE_THRESHOLD_SECS=60   # alert if purge takes >60s
QUOTE_PURGER_ALERT_DELETION_THRESHOLD=1000000  # alert if >1M rows deleted in one run
```

### Example Configurations

#### Development (aggressive purging)
```bash
QUOTE_PURGER_ENABLED=true
QUOTE_PURGER_INTERVAL_SECS=600              # purge every 10 minutes
QUOTE_PURGER_REPLAY_RETENTION_DAYS=1        # keep 1 day only
QUOTE_PURGER_AUDIT_LOG_RETENTION_DAYS=1
```

#### Production (conservative)
```bash
QUOTE_PURGER_ENABLED=true
QUOTE_PURGER_INTERVAL_SECS=3600             # once per hour
QUOTE_PURGER_REPLAY_RETENTION_DAYS=90       # 90 days for compliance/analysis
QUOTE_PURGER_AUDIT_LOG_RETENTION_DAYS=30    # 30 days operational logs
QUOTE_PURGER_REPLAY_BATCH_SIZE=500          # smaller batches to avoid lock contention
QUOTE_PURGER_SLOW_PURGE_THRESHOLD_SECS=30   # alert if slow
```

#### Maintenance/Incident (minimal purging)
```bash
QUOTE_PURGER_ENABLED=true
QUOTE_PURGER_INTERVAL_SECS=86400            # once per day
QUOTE_PURGER_REPLAY_RETENTION_DAYS=180      # preserve long history
QUOTE_PURGER_AUDIT_LOG_RETENTION_DAYS=90
QUOTE_PURGER_MAX_ITERATIONS=10              # very conservative batch limit
```

---

## Observability

### Metrics Table

All purge operations are logged to the `quote_purge_metrics` table:

```sql
SELECT * FROM quote_purge_metrics
WHERE purge_type = 'replay_artifacts'
ORDER BY started_at DESC
LIMIT 10;
```

**Key columns**:
- `purge_type`: 'replay_artifacts' or 'route_audit_log'
- `deleted_count`: rows removed in this purge run
- `scanned_count`: rows examined (may be > deleted if none qualified)
- `duration_ms`: wall-clock time for purge operation
- `age_min_days`, `age_max_days`, `age_p50_days`, `age_p95_days`, `age_p99_days`: age distribution of deleted rows
- `rows_retained`: total rows remaining in table after purge
- `was_rate_limited`: true if hit max_iterations limit (incomplete purge)
- `status`: 'success', 'partial' (rate-limited), or 'failed'

### Structured Logging

When `QUOTE_PURGER_LOG_METRICS=true`, each purge emits a structured log with:

```
metric=stellarroute.api.quote_purge 
purge_type=replay_artifacts 
deleted_count=12345 
duration_ms=1200 
age_p99_days=28.5 
was_rate_limited=false 
alert=true 
alert_reason="Purge took 1.2s (threshold: 1s)"
```

**Parse these logs with**:
```bash
# All purge operations (last 10)
stern stellarroute-api -s 30m | grep 'metric=stellarroute.api.quote_purge' | tail -10

# Alerts only
stern stellarroute-api -s 30m | grep 'metric=stellarroute.api.quote_purge' | grep 'alert=true'
```

### Dashboard Query

Get the latest purge status per table:

```sql
SELECT * FROM get_quote_purge_status();
```

Returns:
- `purge_type`: artifact type
- `last_purge_at`: when last purge ran (null if never)
- `last_deleted_count`: rows deleted in most recent run
- `last_duration_ms`: how long it took
- `rows_currently_in_table`: current row count
- `last_age_p99_days`: 99th percentile age of deleted rows

---

## Operational Procedures

### Manual Purge (Emergency/Testing)

Trigger a purge immediately from any database client:

```sql
-- Purge replay_artifacts
SELECT * FROM purge_replay_artifacts_older_than(
    p_retention_days := 30,      -- days to retain
    p_batch_size := 1000,         -- rows per batch
    p_max_iterations := 100       -- safeguard limit
);

-- Purge route_audit_log
SELECT * FROM purge_route_audit_log_older_than(
    p_retention_days := 30,
    p_batch_size := 5000,
    p_max_iterations := 100
);
```

### Disable Purging (Incident Response)

Stop the purger without restarting the service:

```bash
# Option 1: Kill the server gracefully (restart will not re-enable)
kill -SIGTERM <pid>

# Option 2: Disable via environment before restart
QUOTE_PURGER_ENABLED=false ./stellarroute-api
```

To **re-enable** after an incident, update environment and restart:
```bash
export QUOTE_PURGER_ENABLED=true
# restart API server
```

### Monitor Purge Progress

Check for ongoing or recent purges:

```sql
-- Recent purge runs (last 24h)
SELECT 
    purge_type,
    started_at,
    completed_at,
    deleted_count,
    duration_ms,
    status,
    was_rate_limited
FROM quote_purge_metrics
WHERE started_at > NOW() - INTERVAL '24 hours'
ORDER BY started_at DESC;

-- Alert conditions
SELECT 
    purge_type,
    deleted_count,
    duration_ms,
    was_rate_limited,
    error_message
FROM quote_purge_metrics
WHERE status IN ('partial', 'failed')
  OR duration_ms > 60000
  OR deleted_count > 1000000
ORDER BY started_at DESC;
```

---

## Incident Response

### Scenario 1: Purger causing high lock contention

**Symptoms**: API requests experiencing high query latency, "relation is locked" errors

**Diagnosis**:
```sql
-- Check if purge is running
SELECT pg_stat_activity.* 
FROM pg_stat_activity 
WHERE query LIKE '%purge_%'
  AND query NOT LIKE '%pg_stat_activity%';

-- Check for locks on replay_artifacts / route_audit_log
SELECT * FROM pg_locks 
WHERE relation::regclass::text LIKE '%replay_artifacts%'
   OR relation::regclass::text LIKE '%route_audit_log%';
```

**Resolution**:
1. **Reduce batch size** to minimize lock duration:
   ```bash
   QUOTE_PURGER_REPLAY_BATCH_SIZE=100      # reduce from 1000
   QUOTE_PURGER_AUDIT_LOG_BATCH_SIZE=500   # reduce from 5000
   # Restart API server
   ```

2. **Increase interval** to reduce purge frequency:
   ```bash
   QUOTE_PURGER_INTERVAL_SECS=7200         # increase from 3600 to 2 hours
   # Restart API server
   ```

3. **Temporarily disable** if urgent:
   ```bash
   QUOTE_PURGER_ENABLED=false
   # Restart API server
   # Re-enable after clearing lock queue
   ```

---

### Scenario 2: Purger is rate-limited (incomplete purges)

**Symptoms**: `was_rate_limited=true` in metrics, `rows_retained` growing unbounded

**Diagnosis**:
```sql
-- Check recent rate-limited purges
SELECT purge_type, deleted_count, rows_retained, was_rate_limited
FROM quote_purge_metrics
WHERE was_rate_limited = true
ORDER BY started_at DESC
LIMIT 10;

-- Estimate purge time at current rate
SELECT 
    (SELECT COUNT(*) FROM replay_artifacts) / 1000 * 0.5 AS est_time_mins_at_1k_batch
FROM LIMIT 1;
```

**Resolution**:
1. **Increase max_iterations** to allow larger purges:
   ```bash
   QUOTE_PURGER_MAX_ITERATIONS=500         # increase from 100
   # Restart API server (will catch up on next run)
   ```

2. **Increase batch size** (if lock contention permits):
   ```bash
   QUOTE_PURGER_REPLAY_BATCH_SIZE=5000     # increase from 1000
   QUOTE_PURGER_AUDIT_LOG_BATCH_SIZE=20000
   # Restart API server
   ```

3. **Increase interval** to allow more purge time per cycle:
   ```bash
   QUOTE_PURGER_INTERVAL_SECS=7200         # run less frequently but longer each time
   # Restart API server
   ```

---

### Scenario 3: Purger deleting too aggressively

**Symptoms**: `deleted_count` exceeding threshold, operational logs missing for incident analysis

**Diagnosis**:
```sql
-- Check age of deleted rows
SELECT 
    purge_type,
    started_at,
    age_min_days,
    age_max_days,
    age_p99_days,
    deleted_count
FROM quote_purge_metrics
WHERE deleted_count > 1000000
ORDER BY started_at DESC;
```

**Resolution**:
1. **Increase retention period**:
   ```bash
   QUOTE_PURGER_REPLAY_RETENTION_DAYS=90      # increase from 30
   QUOTE_PURGER_AUDIT_LOG_RETENTION_DAYS=90
   # Restart API server
   ```

2. **Disable specific purges** if one is problematic:
   ```bash
   QUOTE_PURGER_PURGE_REPLAY_ARTIFACTS=false  # disable replay artifacts
   QUOTE_PURGER_PURGE_AUDIT_LOG=true          # keep audit log purging
   # Restart API server
   ```

---

### Scenario 4: Purger failing with errors

**Symptoms**: `status='failed'` in metrics, error logs, no rows being deleted

**Diagnosis**:
```sql
-- Check latest failures
SELECT 
    purge_type,
    started_at,
    status,
    error_message
FROM quote_purge_metrics
WHERE status = 'failed'
ORDER BY started_at DESC
LIMIT 5;
```

**Common causes**:
- **Database not available**: Check connectivity
- **Permissions issue**: Verify `stellarroute_api` role has `DELETE` on tables
- **Disk full**: Check `SELECT pg_database_size('stellarroute')`
- **Transaction deadlock**: Increase `max_iterations` to reduce batch size

**Resolution**:
```bash
# Check connectivity
psql $DATABASE_URL -c "SELECT 1"

# Verify permissions (as superuser)
psql $DATABASE_URL -U postgres -c "
  GRANT DELETE ON replay_artifacts TO stellarroute_api;
  GRANT DELETE ON route_audit_log TO stellarroute_api;
"

# Check disk space
psql $DATABASE_URL -c "SELECT pg_database_size('stellarroute') / 1024^3 AS size_gb"

# Restart API server after fixes
```

---

## Tuning Guide

### For High-Throughput Systems (>1000 req/s)

```bash
# Purge more aggressively to keep tables small
QUOTE_PURGER_INTERVAL_SECS=1800             # every 30 minutes
QUOTE_PURGER_REPLAY_RETENTION_DAYS=14       # 2 weeks only
QUOTE_PURGER_AUDIT_LOG_RETENTION_DAYS=7     # 1 week

# Smaller batches to avoid lock contention
QUOTE_PURGER_REPLAY_BATCH_SIZE=100
QUOTE_PURGER_AUDIT_LOG_BATCH_SIZE=500

# Conservative limits
QUOTE_PURGER_MAX_ITERATIONS=200
QUOTE_PURGER_SLOW_PURGE_THRESHOLD_SECS=30
```

### For Compliance/Audit (need long history)

```bash
# Purge conservatively
QUOTE_PURGER_INTERVAL_SECS=86400            # once per day
QUOTE_PURGER_REPLAY_RETENTION_DAYS=365      # 1 year
QUOTE_PURGER_AUDIT_LOG_RETENTION_DAYS=180   # 6 months

# Larger batches (runs infrequently)
QUOTE_PURGER_REPLAY_BATCH_SIZE=10000
QUOTE_PURGER_AUDIT_LOG_BATCH_SIZE=50000

# Generous thresholds
QUOTE_PURGER_MAX_ITERATIONS=500
QUOTE_PURGER_SLOW_PURGE_THRESHOLD_SECS=300
QUOTE_PURGER_ALERT_DELETION_THRESHOLD=10000000
```

### For Development/Testing

```bash
# Purge very aggressively
QUOTE_PURGER_INTERVAL_SECS=600              # every 10 minutes
QUOTE_PURGER_REPLAY_RETENTION_DAYS=1        # 1 day
QUOTE_PURGER_AUDIT_LOG_RETENTION_DAYS=1

# Moderate batches (dev tables usually small)
QUOTE_PURGER_REPLAY_BATCH_SIZE=500
QUOTE_PURGER_AUDIT_LOG_BATCH_SIZE=2000

# Loose thresholds (expect frequent alerts in dev)
QUOTE_PURGER_SLOW_PURGE_THRESHOLD_SECS=120
QUOTE_PURGER_ALERT_DELETION_THRESHOLD=100000
```

---

## SQL Reference

### Manual Purge Functions

```sql
-- Purge replay_artifacts
-- Returns: deleted_count, total_scanned, rows_retained, age distribution, rate_limited, duration_ms
SELECT * FROM purge_replay_artifacts_older_than(
    p_retention_days INTEGER,
    p_batch_size INTEGER,
    p_max_iterations INTEGER
);

-- Purge route_audit_log
-- Same return signature
SELECT * FROM purge_route_audit_log_older_than(
    p_retention_days INTEGER,
    p_batch_size INTEGER,
    p_max_iterations INTEGER
);

-- Get current purge status per table
SELECT * FROM get_quote_purge_status();
```

### Useful Operational Queries

```sql
-- Current table sizes
SELECT 
    'replay_artifacts' AS table_name,
    COUNT(*) AS row_count,
    pg_size_pretty(pg_total_relation_size('replay_artifacts')) AS size
FROM replay_artifacts
UNION ALL
SELECT 
    'route_audit_log',
    COUNT(*),
    pg_size_pretty(pg_total_relation_size('route_audit_log'))
FROM route_audit_log
UNION ALL
SELECT 
    'quote_purge_metrics',
    COUNT(*),
    pg_size_pretty(pg_total_relation_size('quote_purge_metrics'))
FROM quote_purge_metrics;

-- Purge metrics summary (last 7 days)
SELECT 
    purge_type,
    COUNT(*) AS num_purges,
    SUM(deleted_count) AS total_deleted,
    AVG(duration_ms) AS avg_duration_ms,
    MAX(duration_ms) AS max_duration_ms,
    SUM(CASE WHEN was_rate_limited THEN 1 ELSE 0 END) AS rate_limited_count
FROM quote_purge_metrics
WHERE started_at > NOW() - INTERVAL '7 days'
GROUP BY purge_type;

-- Age distribution of remaining artifacts
SELECT 
    'replay_artifacts' AS table_name,
    ROUND(EXTRACT(EPOCH FROM (NOW() - MIN(captured_at))) / 86400, 1) AS oldest_days,
    ROUND(EXTRACT(EPOCH FROM (NOW() - MAX(captured_at))) / 86400, 1) AS newest_days
FROM replay_artifacts
UNION ALL
SELECT 
    'route_audit_log',
    ROUND(EXTRACT(EPOCH FROM (NOW() - MIN(logged_at))) / 86400, 1),
    ROUND(EXTRACT(EPOCH FROM (NOW() - MAX(logged_at))) / 86400, 1)
FROM route_audit_log;
```

---

## See Also

- **Issue**: #450 [backend] Automated stale-quote purger with observability hooks
- **Migration**: `crates/api/migrations/0005_quote_purger.sql`
- **Implementation**: `crates/api/src/purger/`
- **Retention Policy**: `docs/audit-log-retention.md` (for overall data lifecycle strategy)
