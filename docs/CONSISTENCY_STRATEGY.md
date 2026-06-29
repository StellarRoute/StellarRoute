# Read-After-Write Consistency Strategy

## Overview

This document describes the consistency strategy implemented to prevent quotes from reading pre-commit offer rows during indexer writes.

## Problem

When the indexer writes new SDEX offers or AMM pool reserves, there's a window where:
1. Write transaction has begun but not committed
2. Quote endpoint reads from the same tables
3. Quote endpoint may see uncommitted or partially committed data (dirty reads)

## Solution

We implement consistency guards using PostgreSQL transaction isolation levels and visibility rules.

### Strategies

#### 1. Snapshot Isolation (REPEATABLE READ)
- **How it works**: Each transaction sees a consistent snapshot of the database from the start
- **Benefits**: No dirty reads, consistent view throughout transaction
- **Trade-offs**: May see slightly stale data if write committed after snapshot

#### 2. Version Checking (READ COMMITTED)
- **How it works**: Explicitly checks for row-level locks before reading
- **Benefits**: Always sees latest committed data
- **Trade-offs**: Requires lock detection queries

#### 3. Serializable
- **How it works**: Strictest isolation, detects conflicts automatically
- **Benefits**: Strongest guarantees
- **Trade-offs**: Higher chance of transaction conflicts/retries

## Implementation

### Consistency Guard

Located in `crates/api/src/consistency_guard.rs`:

```rust
let guard = ConsistencyGuard::new(
    ConsistencyStrategy::SnapshotIsolation,
    metrics
);

let mut tx = guard.begin_read_transaction(&pool).await?;
let visible = guard.check_visibility(&mut tx, (base, quote)).await?;
```

### Metrics

Tracked metrics:
- `guarded_reads`: Total number of guarded read transactions
- `stale_reads_prevented`: Number of reads blocked due to ongoing writes
- `conflict_retries`: Number of transaction retries due to conflicts

### Configuration

Set via environment variable:
```bash
CONSISTENCY_STRATEGY=snapshot  # or "version" or "serializable"
```

## Testing

Regression tests in `crates/api/tests/consistency_guard_test.rs` reproduce stale-read scenarios and verify guards work correctly.

Run tests:
```bash
TEST_DATABASE_URL=postgres://localhost/stellarroute_test cargo test --test consistency_guard_test -- --ignored
```

## Performance Impact

- **Snapshot Isolation**: Minimal overhead (~1-2ms per transaction)
- **Version Checking**: Moderate overhead (~5-10ms for lock checks)
- **Serializable**: Higher overhead, may require retries

## Monitoring

Monitor via Prometheus metrics:
- `stellarroute_api_consistency_guarded_reads_total`
- `stellarroute_api_consistency_stale_reads_prevented_total`
- `stellarroute_api_consistency_conflict_retries_total`
