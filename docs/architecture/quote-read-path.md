# Quote Read Path — Database Schema Guide

This document maps every table and view the live **quote** and **orderbook** API
paths read from. Use it as a quick reference so you can trace a quote failure
back to a table without reading raw migrations.

> **Wave contributor warning:** `normalized_liquidity` is on the critical path for
> every quote. Do not rename columns, change column types, or drop rows in Wave
> tickets without a backfill plan and maintainer approval. The API and routing
> engine depend on the exact shape documented below.

---

## Tables and views the quote path reads

### 1. `normalized_liquidity` (table)

The **primary read surface** for all quote and routing decisions. Originally a
`UNION ALL` view over `sdex_offers` and `amm_pool_reserves` (migration 0004),
it is now a standalone **table** with trigger-based synchronization (migrations
0007, 0009, 0011, 0012).

| Column | Type | Notes |
|---|---|---|
| `venue_type` | `text` | `'sdex'` or `'amm'` |
| `venue_ref` | `text` | Offer ID (sdex) or pool address (amm) |
| `selling_asset_id` | `uuid` | FK → `assets.id` |
| `buying_asset_id` | `uuid` | FK → `assets.id` |
| `price` | `numeric(30,14)` | Human-readable price |
| `available_amount` | `numeric(30,14)` | Human-readable amount |
| `price_e7` | `bigint` | Price × 10⁷ (fast pathfinding) |
| `available_amount_e7` | `bigint` | Amount × 10⁷ (fast pathfinding) |
| `source_ledger` | `bigint` | Last ledger that touched this row |
| `source_trace_id` | `text` | Indexer trace for debugging |
| `source_span_id` | `text` | Indexer span for debugging |
| `updated_at` | `timestamptz` | Row update timestamp |

**Primary key:** `(venue_type, venue_ref)`

**Indexes used by quote reads:**
- `idx_normalized_liquidity_pair_price` — `(selling_asset_id, buying_asset_id, price_e7 ASC)`
- `idx_normalized_liquidity_updated` — `(updated_at DESC)`

**How it stays in sync:** Triggers on `sdex_offers` and `amm_pool_reserves`
fire `sync_normalized_liquidity_from_sdex()` and
`sync_normalized_liquidity_from_amm()` (migration 0009/0011/0012). The AMM
upsert function also emits `pg_notify('liquidity_update', ...)` so the
API-side `GraphManager` wakes up immediately.

**Key code paths:**
- `crates/api/src/routes/quote.rs` → `fetch_source_candidates()` (line ~1508)
  and `get_liquidity_revision()` (line ~1571)
- `crates/api/src/graph.rs` → `sync_graph()` (line ~152) — also LEFT JOINs
  `amm_pool_reserves` for `fee_bps`

---

### 2. `assets`

Resolved at quote time to map asset codes/issuers to internal UUIDs.

| Column | Type | Notes |
|---|---|---|
| `id` | `uuid` | PK |
| `asset_type` | `text` | `native`, `credit_alphanum4`, `credit_alphanum12` |
| `asset_code` | `text` | NULL for native |
| `asset_issuer` | `text` | NULL for native |
| `created_at` | `timestamptz` | |

**Key code path:**
- `crates/api/src/routes/quote.rs` → `find_asset_id()` (line ~1594) — for
  native assets, it LEFT JOINs `sdex_offers` to pick the row backed by live
  offers.

---

### 3. `sdex_offers`

Read **indirectly** in two places:

1. **Native asset resolution** — `find_asset_id()` JOINs `sdex_offers` to
   disambiguate multiple `assets` rows for native XLM.
2. **Graph sync** — `graph.rs` does not query `sdex_offers` directly; it reads
   from `normalized_liquidity` which already contains SDEX rows.

| Column | Type | Notes |
|---|---|---|
| `offer_id` | `bigint` | PK |
| `selling_asset_id` | `uuid` | FK → `assets.id` |
| `buying_asset_id` | `uuid` | FK → `assets.id` |
| `price` | `numeric` | |
| `amount` | `numeric` | Available amount |
| `last_modified_ledger` | `bigint` | |
| `source_trace_id` | `text` | Added in migration 0011 |
| `source_span_id` | `text` | Added in migration 0011 |
| `updated_at` | `timestamptz` | |

---

### 4. `amm_pool_reserves`

Read **indirectly** via the `graph.rs` sync query:

```sql
SELECT ... amm.fee_bps as fee_bps
FROM normalized_liquidity nl
LEFT JOIN amm_pool_reserves amm
  ON nl.venue_type = 'amm' AND nl.venue_ref = amm.pool_address
WHERE nl.available_amount > 0
```

The `fee_bps` column is the only field read from this table at runtime. All
other AMM data flows through `normalized_liquidity`.

| Column | Type | Notes |
|---|---|---|
| `pool_address` | `text` | PK |
| `selling_asset_id` | `uuid` | FK → `assets.id` |
| `buying_asset_id` | `uuid` | FK → `assets.id` |
| `reserve_selling` | `numeric(38,18)` | |
| `reserve_buying` | `numeric(38,18)` | |
| `fee_bps` | `integer` | Pool fee in basis points |
| `last_updated_ledger` | `bigint` | |
| `source_trace_id` | `text` | Added in migration 0011 |
| `source_span_id` | `text` | Added in migration 0011 |
| `updated_at` | `timestamptz` | |

---

### 5. `orderbook_snapshots` (orderbook endpoint only)

Used by the `/api/v1/orderbook` endpoint for orderbook depth, not by the
quote path. Listed here because orderbook reads are a common contributor
touch-point.

| Column | Type | Notes |
|---|---|---|
| `id` | `uuid` | PK |
| `trading_pair_id` | `uuid` | FK → `trading_pairs.id` |
| `bids` | `jsonb` | Array of `{price, amount, total}` |
| `asks` | `jsonb` | Array of `{price, amount, total}` |
| `mid_price` | `numeric` | Used by price-history endpoint |
| `ledger_sequence` | `bigint` | |

---

## Read-path data flow

```
                 ┌──────────────────────────────────────────────┐
                 │            Indexer writes                     │
                 │  sdex_offers ──trigger──▶ normalized_liquidity│
                 │  amm_pool_reserves ─trigger──▶ (same table)   │
                 │                         pg_notify ──▶ GraphManager
                 └──────────────────────────────────────────────┘
                                      │
                                      ▼
                 ┌──────────────────────────────────────────────┐
                 │            API reads                          │
                 │  assets ← find_asset_id()                    │
                 │  normalized_liquidity ← fetch_source_candidates()
                 │  normalized_liquidity ← get_liquidity_revision()
                 │  normalized_liquidity ← graph.rs sync_graph() │
                 │  amm_pool_reserves ← graph.rs (fee_bps only) │
                 └──────────────────────────────────────────────┘
```

---

## What NOT to change in Wave tickets

- Do not rename or drop columns in `normalized_liquidity`, `assets`,
  `sdex_offers`, or `amm_pool_reserves`.
- Do not change the `price_e7` / `available_amount_e7` scaling factor
  (currently × 10⁷).
- Do not alter the `sync_normalized_liquidity_from_sdex()` or
  `sync_normalized_liquidity_from_amm()` trigger functions without a
  migration that backfills existing rows.
- Do not remove the `pg_notify('liquidity_update', ...)` calls — the
  `GraphManager` depends on them for real-time graph updates.

Allowed additive changes: new columns with `DEFAULT` values, new indexes,
new tables, and new migrations that do not modify existing column semantics.
