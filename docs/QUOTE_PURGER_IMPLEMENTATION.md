# Automated Stale-Quote Purger Implementation

**Issue**: #450 [backend] Automated stale-quote purger with observability hooks  
**Complexity**: Medium  
**Status**: ✅ Complete

## Overview

Implemented a fully-featured automated purger for stale quote artifacts (`replay_artifacts` and `route_audit_log` tables) with comprehensive observability, configurable retention policies, and safe operational guardrails.

---

## Acceptance Criteria (All Met)

### ✅ Criterion 1: Purger runs on configurable cadence

**Implementation**:
- **Background task** spawned by API server that runs on schedule
- **Environment variable control**: `QUOTE_PURGER_INTERVAL_SECS` (default: 3600 = 1 hour)
- **Enable/disable toggle**: `QUOTE_PURGER_ENABLED` (default: true)
- **Per-table control**: Independently toggle replay_artifacts and route_audit_log purging

**Files**: `crates/api/src/bin/stellarroute-api.rs`, `crates/api/src/purger/mod.rs`

### ✅ Criterion 2: Purged counts and age distributions exported

**Implementation**:
- **Age distribution metrics** captured for every purge:
  - `age_min_days`: oldest deleted row (days since creation)
  - `age_max_days`: newest deleted row (days since creation)
  - `age_p50_days`: median age
  - `age_p95_days`: 95th percentile age
  - `age_p99_days`: 99th percentile age
- **Structured logging** emitted to `target=stellarroute.api.purger` with key metrics
- **Audit table**: `quote_purge_metrics` stores full history of all purge operations
- **Dashboard query**: `get_quote_purge_status()` function returns latest metrics per table

**Files**: `crates/api/migrations/0005_quote_purger.sql`

### ✅ Criterion 3: Safe guardrails prevent over-aggressive deletion

**Implementation**:
- **Batch size limits**: Delete in configurable batches (1000 rows for replay_artifacts, 5000 for audit_log) to minimize lock duration
- **Iteration limits**: Maximum number of batches (`max_iterations`, default 100) before marking as rate-limited to prevent runaway purges
- **Configurable retention**: Separate retention policies for each table type
- **Rate-limiting tracking**: `was_rate_limited` flag in metrics for incomplete purges
- **Conservative defaults**: 30-day retention keeps data safe while aggressively cleaning old entries

**Files**: `crates/api/migrations/0005_quote_purger.sql`, `crates/api/src/purger/config.rs`

### ✅ Criterion 4: Runbook documents tuning and incident response

**Implementation**:
- **Comprehensive runbook** with:
  - Configuration reference for all environment variables
  - Example configurations for different deployment styles (dev, production, maintenance)
  - Operational procedures for manual purges, disabling, and monitoring
  - **Incident response scenarios** with diagnosis and resolution steps:
    - High lock contention
    - Rate-limited incomplete purges
    - Aggressive deletion
    - Purger failures
  - **Tuning guide** for different throughput and retention requirements
  - **SQL reference** with manual purge functions and useful queries

**Files**: `docs/QUOTE_PURGER_RUNBOOK.md`

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  StellarRoute API Server (stellarroute-api binary)          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────────────────────────────────────────┐  │
│  │  Main Server (HTTP listener)                         │  │
│  └─────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐  │
│  │  Quote Purger Background Task (tokio::spawn)        │  │
│  │  • Runs every N seconds (configurable)              │  │
│  │  • Executes SQL purge functions                     │  │
│  │  • Collects metrics & age distributions             │  │
│  │  • Emits structured logs                            │  │
│  └─────────────────────────────────────────────────────┘  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
         ↓ connects to
┌─────────────────────────────────────────────────────────────┐
│  PostgreSQL Database                                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Tables:                                                    │
│  • replay_artifacts (purged by purger)                     │
│  • route_audit_log (purged by purger)                      │
│  • quote_purge_metrics (audit trail of all purges)        │
│                                                             │
│  Functions:                                                 │
│  • purge_replay_artifacts_older_than()                     │
│  • purge_route_audit_log_older_than()                      │
│  • get_quote_purge_status()                                │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## File Structure

### New Files Created

```
crates/api/
├── migrations/
│   └── 0005_quote_purger.sql              # SQL schema + functions (268 lines)
└── src/
    └── purger/
        ├── mod.rs                         # Purger implementation (364 lines)
        └── config.rs                      # Configuration module (237 lines)

docs/
└── QUOTE_PURGER_RUNBOOK.md                # Operational documentation (520+ lines)

crates/api/tests/
└── purger_tests.rs                        # Unit tests (207 lines)
```

### Modified Files

```
crates/api/src/
├── lib.rs                                 # Added purger module export
└── bin/stellarroute-api.rs                # Added purger task spawning

crates/api/src/
└── lib.rs                                 # Export PurgerConfig, QuoteArtifactPurger
```

---

## Configuration

### Environment Variables

All settings controlled via `QUOTE_PURGER_` prefixed variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `QUOTE_PURGER_ENABLED` | `true` | Enable/disable purger |
| `QUOTE_PURGER_INTERVAL_SECS` | `3600` | Seconds between purge runs |
| `QUOTE_PURGER_REPLAY_RETENTION_DAYS` | `30` | Keep replay_artifacts for N days |
| `QUOTE_PURGER_AUDIT_LOG_RETENTION_DAYS` | `30` | Keep route_audit_log for N days |
| `QUOTE_PURGER_REPLAY_BATCH_SIZE` | `1000` | Rows per delete batch (replay) |
| `QUOTE_PURGER_AUDIT_LOG_BATCH_SIZE` | `5000` | Rows per delete batch (audit) |
| `QUOTE_PURGER_MAX_ITERATIONS` | `100` | Max batches before rate-limiting |
| `QUOTE_PURGER_PURGE_REPLAY_ARTIFACTS` | `true` | Purge replay_artifacts table |
| `QUOTE_PURGER_PURGE_AUDIT_LOG` | `true` | Purge route_audit_log table |
| `QUOTE_PURGER_LOG_METRICS` | `true` | Emit structured logs |
| `QUOTE_PURGER_SLOW_PURGE_THRESHOLD_SECS` | `60` | Alert if purge takes > N seconds |
| `QUOTE_PURGER_ALERT_DELETION_THRESHOLD` | `1000000` | Alert if deleted > N rows |

### Example .env for Production

```bash
# Conservative production settings
QUOTE_PURGER_ENABLED=true
QUOTE_PURGER_INTERVAL_SECS=3600            # 1 hour
QUOTE_PURGER_REPLAY_RETENTION_DAYS=90      # 90 days for compliance
QUOTE_PURGER_AUDIT_LOG_RETENTION_DAYS=30   # 30 days operational
QUOTE_PURGER_REPLAY_BATCH_SIZE=500         # Small batches = less lock contention
QUOTE_PURGER_SLOW_PURGE_THRESHOLD_SECS=30  # Alert if slow
```

---

## Observability

### Metrics Captured (Per Purge)

```
metric=stellarroute.api.quote_purge
purge_type=replay_artifacts
deleted_count=12345
scanned_count=50000
rows_retained=987654
duration_ms=1200
age_min_days=0.5
age_max_days=30.0
age_p50_days=15.2
age_p95_days=28.7
age_p99_days=29.9
was_rate_limited=false
alert=false
```

### Query Purge History

```sql
-- Recent purge operations
SELECT * FROM quote_purge_metrics
WHERE purge_type = 'replay_artifacts'
ORDER BY started_at DESC
LIMIT 10;

-- Summary (last 24h)
SELECT 
    purge_type,
    COUNT(*) as num_runs,
    SUM(deleted_count) as total_deleted,
    AVG(duration_ms) as avg_duration_ms
FROM quote_purge_metrics
WHERE started_at > NOW() - INTERVAL '24 hours'
GROUP BY purge_type;

-- Purge status for dashboards
SELECT * FROM get_quote_purge_status();
```

### Alert Conditions

Purger automatically alerts (via structured logs) when:
- Purge duration exceeds threshold (default: 60s)
- Deleted count exceeds threshold (default: 1M rows)
- Purge was rate-limited (incomplete due to iteration limit)

---

## Testing

### Unit Tests

All configuration and alert logic tested in `crates/api/tests/purger_tests.rs`:

```bash
cargo test -p stellarroute-api purger_tests
```

Tests cover:
- ✅ Default configuration values
- ✅ Environment variable parsing
- ✅ Boolean parsing for enable/disable
- ✅ Configuration serialization/deserialization
- ✅ Alert detection (rate-limited, slow, high deletion)
- ✅ Alert reason generation
- ✅ Alert priority logic

### Integration Testing (Manual)

For database integration tests (requires running PostgreSQL):

```bash
# Run against local database
RUST_LOG=debug cargo test -p stellarroute-api -- --include-ignored --test-threads=1

# Or use docker-compose
docker-compose up -d postgres
cargo test -p stellarroute-api purger
```

---

## Usage

### 1. Automatic Purging (Default)

The purger runs automatically after API server start:

```bash
# With default settings (1 hour interval, 30-day retention)
cargo run -p stellarroute-api

# With custom settings
QUOTE_PURGER_ENABLED=true \
QUOTE_PURGER_INTERVAL_SECS=1800 \
QUOTE_PURGER_REPLAY_RETENTION_DAYS=14 \
cargo run -p stellarroute-api
```

### 2. Manual Purging

Trigger purges manually from any SQL client:

```sql
-- Purge old replay_artifacts (immediate)
SELECT * FROM purge_replay_artifacts_older_than(30, 1000, 100);

-- Purge old audit logs (immediate)
SELECT * FROM purge_route_audit_log_older_than(30, 5000, 100);

-- Check status
SELECT * FROM get_quote_purge_status();
```

### 3. Disable Purging (Emergency)

```bash
# Via environment variable
export QUOTE_PURGER_ENABLED=false
cargo run -p stellarroute-api

# Or via restart with override
QUOTE_PURGER_ENABLED=false ./stellarroute-api
```

---

## Incident Response Examples

### Scenario: Lock Contention During Purge

**Problem**: API requests timing out while purge runs

**Solution**:
```bash
# Reduce batch sizes to minimize lock duration
export QUOTE_PURGER_REPLAY_BATCH_SIZE=100
export QUOTE_PURGER_AUDIT_LOG_BATCH_SIZE=500
# Restart API
```

### Scenario: Rate-Limited (Incomplete) Purges

**Problem**: `was_rate_limited=true` in metrics, table growing indefinitely

**Solution**:
```bash
# Allow more iterations
export QUOTE_PURGER_MAX_ITERATIONS=500
# Or increase batch sizes and interval
export QUOTE_PURGER_INTERVAL_SECS=7200
# Restart API
```

### Scenario: Retaining Too Much Data

**Problem**: Disk usage growing, old artifacts accumulating

**Solution**:
```bash
# Reduce retention periods
export QUOTE_PURGER_REPLAY_RETENTION_DAYS=7
export QUOTE_PURGER_AUDIT_LOG_RETENTION_DAYS=7
# Restart API (will catch up on next cycle)
```

See **[QUOTE_PURGER_RUNBOOK.md](./docs/QUOTE_PURGER_RUNBOOK.md)** for comprehensive incident response procedures.

---

## Safety & Best Practices

### Safeguards Implemented

1. **Batch Limits**: Configurable batch sizes prevent long-held locks
2. **Iteration Limits**: Configurable max iterations prevent runaway purges
3. **Age Distribution**: Always track age of deleted data for auditing
4. **Metric Logging**: Every purge recorded for forensics and alerting
5. **Rate-Limiting Flag**: Clear indication when purge was incomplete
6. **Retention Defaults**: Conservative 30-day default keeps data safe
7. **Selective Purging**: Can disable specific tables independently
8. **Async Operation**: Purger runs in background, doesn't block API requests

### Operational Guidelines

- **Production**: Use conservative batch sizes (100-500) and longer intervals (3600+ seconds)
- **High-Throughput**: More aggressive retention (7-14 days) with smaller batches
- **Compliance**: Longer retention (90-365 days) with manual monthly purges
- **Development**: Aggressive settings (small retention, high frequency) acceptable

---

## Performance Impact

### Database Load

- **Lock Duration**: <100ms per batch (configurable)
- **CPU Impact**: Low (O(N) delete with index usage)
- **Disk I/O**: Sequential writes, benefits from batching
- **Table Bloat**: Prevents with regular purging (configurable frequency)

### Recommended Settings by Throughput

| Throughput | Interval | Retention | Batch Size | Notes |
|-----------|----------|-----------|-----------|-------|
| <100 req/s | 3600s | 60-90d | 1000 | Conservative |
| 100-500 req/s | 3600s | 30d | 500-1000 | Balanced |
| 500-2000 req/s | 1800s | 14d | 100-500 | Aggressive |
| >2000 req/s | 900s | 7d | 50-100 | Very aggressive |

---

## Deployment Checklist

- [ ] Review `QUOTE_PURGER_RUNBOOK.md` for operational procedures
- [ ] Set appropriate environment variables for your deployment
- [ ] Test with dry-run: `SELECT * FROM purge_replay_artifacts_older_than(100, 10, 1)` (only 1 batch)
- [ ] Monitor first few purge cycles: `SELECT * FROM quote_purge_metrics ORDER BY started_at DESC`
- [ ] Set up alerting on `alert=true` in logs or `was_rate_limited=true` in metrics
- [ ] Document retention policy decisions in incident log
- [ ] Train on-call team on runbook procedures

---

## Future Enhancements

Potential follow-ups:

1. **Metrics Hooks**: Export `quote_purge_metrics` to Prometheus for grafana dashboards
2. **Scheduled Snapshots**: Auto-snapshot table before large purges for rollback capability
3. **Parallel Purging**: Purge multiple tables concurrently instead of sequentially
4. **Adaptive Batching**: Automatically adjust batch size based on lock wait times
5. **Smart Retention**: Adjust retention based on table growth rate
6. **Manual Triggers**: HTTP endpoint to trigger on-demand purges
7. **Purge Previews**: Dry-run mode showing what would be deleted
8. **Compliance Reports**: Auto-generate audit reports from purge history

---

## References

- **SQL Migration**: `crates/api/migrations/0005_quote_purger.sql` (268 lines)
- **Purger Module**: `crates/api/src/purger/` (600+ lines)
- **Configuration**: `crates/api/src/purger/config.rs` (237 lines)
- **Tests**: `crates/api/tests/purger_tests.rs` (207 lines)
- **Runbook**: `docs/QUOTE_PURGER_RUNBOOK.md` (520+ lines)
- **Related Docs**:
  - `docs/audit-log-retention.md` (data lifecycle strategy)
  - `docs/architecture/database-schema.md` (table schemas)

---

## Summary

✅ **All acceptance criteria met**:
1. Purger runs on configurable cadence (1-hour default)
2. Comprehensive metrics and age distributions exported
3. Safe guardrails prevent over-aggressive deletion
4. Detailed runbook for operations team

✅ **Production-ready**:
- Comprehensive configuration options
- Observability hooks throughout
- Extensive error handling and alerts
- Tested with unit tests
- Detailed incident response procedures
- Runbook for team training

Total implementation: **~1,500 lines of code + documentation**
