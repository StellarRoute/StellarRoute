# Database Migration Runbook — Zero-Downtime Live Deploys

StellarRoute cannot take long outages for schema changes. This runbook defines
the **expand/contract** pattern used to evolve the Postgres schema while keeping
the API available.

## Scope

Applies to:

- `crates/api/migrations` (API state: quotes, audit logs, replay artifacts)
- `crates/indexer/migrations` (indexer state: offers, AMM reserves, pairs)

For contract-storage migrations (Soroban `StorageKey` changes), see the
**Data Migration Strategy** section of [`docs/deployment/README.md`](README.md).

## Migration principles

1. **Backward-compatible schema first** — never drop a column or change a
   non-null constraint until *all* deployed code can live without it.
2. **Additive changes only during deploy** — new columns, tables, indexes, and
   generated columns are safe to add while old code is still running.
3. **Expand → dual-write → flip reads → contract** — the canonical four-phase
   pattern.
4. **Migrations run before code** — the schema must be compatible with both the
   old and new application version before traffic hits the new version.
5. **Test rollbacks at every step** — each phase should be reversible without
   data loss.

## The expand/contract pattern

### Phase 1 — Expand (additive migration)

Add the new structure without breaking old code.

```sql
-- Example: add a new nullable column with a default that old code ignores
ALTER TABLE sdex_offers
  ADD COLUMNIF NOT EXISTS flags INTEGER NULL DEFAULT 0;

-- Add an index concurrently so it does not block reads/writes
CREATE INDEX CONCURRENTLY idx_sdex_offers_flags
  ON sdex_offers(flags)
  WHERE flags IS NOT NULL;
```

Rules:

- Columns must be **nullable** (or have a `DEFAULT`) so old `INSERT` statements
  that omit them still succeed.
- New `CHECK` constraints should be `NOT VALID` initially, then validated in a
  separate transaction after backfill.
- Create indexes `CONCURRENTLY` outside a transaction to avoid long locks.

### Phase 2 — Dual-write

Deploy code that writes to **both** old and new representations.

```rust
// Pseudocode inside the indexer
let flags = compute_flags(offer);
sqlx::query(
    "UPDATE sdex_offers SET amount = $1, price = $2, flags = $3 WHERE offer_id = $4"
)
.bind(&offer.amount)
.bind(&offer.price)
.bind(flags)
.bind(offer.offer_id)
.execute(&pool)
.await?;
```

Old code ignores `flags`; new code populates it. Reads still use the old path.

### Phase 3 — Backfill

Populate the new column for historical rows.

```sql
-- Backfill in small batches to avoid long transactions and lock escalation.
UPDATE sdex_offers
SET flags = 0
WHERE flags IS NULL
  AND offer_id IN (
    SELECT offer_id FROM sdex_offers
    WHERE flags IS NULL
    ORDER BY offer_id
    LIMIT 10000
  );
```

Repeat until `COUNT(*) WHERE flags IS NULL` is zero.

### Phase 4 — Flip reads

Deploy code that reads from the new representation.

```rust
// New code path
let row = sqlx::query("SELECT offer_id, amount, price, flags FROM sdex_offers WHERE offer_id = $1")
    .bind(offer_id)
    .fetch_one(&pool)
    .await?;
```

### Phase 5 — Contract (cleanup migration)

Once all code has been reading the new path for at least one full retention
window, drop the old representation.

```sql
ALTER TABLE sdex_offers DROP COLUMN legacy_flag_source;
```

Never contract in the same deploy that flips reads. Wait for a subsequent
release so rollback can revert to the previous code without schema changes.

## Production runbook

### Pre-deploy

1. Open a maintenance window announcement (even though the service stays up).
2. Verify the migration set: `sqlx migrate info --source crates/api/migrations`.
3. Run migrations against a staging database that mirrors production size.
4. Confirm query plans for any new indexes (`EXPLAIN ANALYZE`).
5. Confirm backward compatibility: deploy the migration, then run the
   *previous* production container against the migrated schema for 5 minutes.

### Deploy

1. **Schema first** — run migrations from a job or init container:
   ```bash
   sqlx migrate run --source crates/api/migrations
   sqlx migrate run --source crates/indexer/migrations
   ```
2. **Pause auto-scaling** to prevent pod churn during the schema flip.
3. **Canary** — roll out the new code to 5% of traffic.
4. **Dual-write verification** — compare old vs new query results with a
   shadow-read query for 10 minutes.
5. **Full rollout** — increase canary to 100%.
6. **Backfill** if not already done by dual-write.
7. **Monitor** indexer lag, API p95 latency, and error rate for 30 minutes.

### Rollback at each phase

| Phase | Rollback action |
|-------|-----------------|
| Expand | Drop the new column/index in a follow-up migration. Old code is unaffected. |
| Dual-write | Revert to the previous container. New column remains but is ignored. |
| Flip reads | Revert to the previous container. Reads old column again. |
| Contract | **Not safely reversible without restoring from backup.** This is why contract is its own release. |

### Post-deploy

1. Schedule the contract migration for the next release window.
2. Document the new schema in `docs/architecture/database-schema.md`.
3. Update runbooks that reference the affected tables.

## CI / migration runbook

### PR checks

Every migration PR must include:

1. The migration file(s) under `crates/*/migrations/`.
2. A note in the PR description explaining whether the change is **expand**,
   **contract**, or **flip-reads**.
3. Proof of backward compatibility — either:
   - a new unit test that exercises both old and new column paths, or
   - a CI step that runs `sqlx migrate run` and then starts the *previous*
     Docker image against the migrated database.

### CI pipeline

```yaml
# Suggested CI step (excerpt)
- name: Run API migrations
  run: sqlx migrate run --source crates/api/migrations
  env:
    DATABASE_URL: postgres://postgres:postgres@localhost/stellarroute_test

- name: Run indexer migrations
  run: sqlx migrate run --source crates/indexer/migrations
  env:
    DATABASE_URL: postgres://postgres:postgres@localhost/stellarroute_test

- name: Smoke test with old code against new schema
  run: |
    docker run --rm --network host \
      ghcr.io/stellarroute/api:previous-release \
      /app/api --smoke-test --database-url $DATABASE_URL
```

### Ordering

Migrations must run in this order in production:

1. `crates/indexer/migrations` — the indexer owns the raw offer/pair data.
2. `crates/api/migrations` — the API consumes indexed data and adds its own
   state (quotes, audit logs, replay artifacts).

Never run API migrations that depend on indexer schema changes before the
indexer migrations have completed.

## Backward-compatible migration example

Here is a complete, safe evolution that adds a `fee_bps` column to
`trading_pairs`.

### Migration A — expand

```sql
-- 0015_add_trading_pairs_fee_bps.sql
ALTER TABLE trading_pairs
  ADD COLUMN fee_bps INTEGER NULL DEFAULT 0;

CREATE INDEX CONCURRENTLY idx_trading_pairs_fee_bps
  ON trading_pairs(fee_bps)
  WHERE fee_bps IS NOT NULL;

COMMENT ON COLUMN trading_pairs.fee_bps IS
  'Fee in basis points. NULL means "not yet populated". Default 0.';
```

### Code change — dual-write

```rust
// In the indexer or admin path that writes trading_pairs
sqlx::query(
    "INSERT INTO trading_pairs (base_asset_id, counter_asset_id, fee_bps)
     VALUES ($1, $2, $3)
     ON CONFLICT (base_asset_id, counter_asset_id)
     DO UPDATE SET is_active = true, fee_bps = EXCLUDED.fee_bps"
)
.bind(base_asset_id)
.bind(counter_asset_id)
.bind(fee_bps)
.execute(&pool)
.await?;
```

### Backfill

```sql
-- Run from a one-off job or pg_cron until count reaches zero
UPDATE trading_pairs
SET fee_bps = 0
WHERE fee_bps IS NULL
  AND id IN (
    SELECT id FROM trading_pairs WHERE fee_bps IS NULL ORDER BY id LIMIT 10000
  );
```

### Code change — flip reads

```rust
let row = sqlx::query(
    "SELECT base_asset_id, counter_asset_id, fee_bps FROM trading_pairs WHERE id = $1"
)
.bind(pair_id)
.fetch_one(&pool)
.await?;
let fee_bps: i32 = row.try_get("fee_bps")?.unwrap_or(0);
```

### Migration B — contract

```sql
-- 0016_make_trading_pairs_fee_bps_not_null.sql
ALTER TABLE trading_pairs
  ALTER COLUMN fee_bps SET NOT NULL;

-- Only drop the old representation after the new one has been authoritative
-- for at least one release cycle.
-- ALTER TABLE trading_pairs DROP COLUMN old_fee_source;
```

## Things to avoid

| Anti-pattern | Why it is dangerous | Safe alternative |
|--------------|---------------------|------------------|
| `ALTER TABLE … ALTER COLUMN TYPE` on a large table | Rewrites the table under an `ACCESS EXCLUSIVE` lock | Add new column, backfill, flip reads, drop old column |
| Adding `NOT NULL` without a default in one migration | Breaks old `INSERT` paths | Add nullable/default first, validate later |
| Creating non-concurrent indexes on hot tables | Blocks writes for minutes or hours | `CREATE INDEX CONCURRENTLY` |
| Dropping a column still read by the previous release | Immediate outage | Wait one release between flip-reads and contract |
| Running migrations and code flip in the same deploy | Cannot roll back code without reverting schema | Schema-first deploy, then code canary |

## References

- [`docs/architecture/database-schema.md`](../../architecture/database-schema.md)
- [`docs/deployment/README.md`](README.md) — general deployment and contract
  storage migration guidance
- [sqlx migrate](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli)
- [PostgreSQL: ALTER TABLE](https://www.postgresql.org/docs/current/sql-altertable.html)
