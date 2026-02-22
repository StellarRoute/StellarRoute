# Database Schema Documentation

## Overview

StellarRoute uses PostgreSQL to store SDEX orderbook data, trading pairs, and historical snapshots. The schema is designed for high performance with proper normalization and strategic denormalization where needed.

## Entity Relationship Diagram

```
┌─────────────────────┐
│       assets        │
├─────────────────────┤
│ id (PK)            │
│ asset_type         │
│ asset_code         │
│ asset_issuer       │
│ created_at         │
└─────────────────────┘
         │ 1
         │
         │ N
         ▼
┌─────────────────────────────────┐         ┌─────────────────────────┐
│         sdex_offers             │         │     trading_pairs       │
├─────────────────────────────────┤         ├─────────────────────────┤
│ offer_id (PK)                  │         │ id (PK)                │
│ seller                         │         │ base_asset_id (FK)     │───┐
│ selling_asset_id (FK)          │───┐     │ counter_asset_id (FK)  │───┤
│ buying_asset_id (FK)           │───┤     │ is_active              │   │
│ amount                         │   │     │ total_offers           │   │
│ price                          │   │     │ total_volume           │   │
│ price_n                        │   │     │ last_trade_at          │   │
│ price_d                        │   │     │ created_at             │   │
│ last_modified_ledger           │   │     │ updated_at             │   │
│ paging_token                   │   │     └─────────────────────────┘   │
│ updated_at                     │   │              │ 1                  │
└─────────────────────────────────┘   │              │                    │
         │                            │              │ N                  │
         │                            │              ▼                    │
         │                            │     ┌─────────────────────────┐  │
         │                            └────▶│  orderbook_snapshots    │  │
         │                                  ├─────────────────────────┤  │
         │                                  │ id (PK)                │  │
         │                                  │ trading_pair_id (FK)   │  │
         ▼                                  │ snapshot_time          │  │
┌─────────────────────────┐                │ bids (JSONB)           │  │
│   archived_offers       │                │ asks (JSONB)           │  │
├─────────────────────────┤                │ bid_count              │  │
│ offer_id (PK)          │                │ ask_count              │  │
│ seller                 │                │ spread                 │  │
│ selling_asset_type     │                │ mid_price              │  │
│ selling_asset_code     │                │ total_bid_volume       │  │
│ selling_asset_issuer   │                │ total_ask_volume       │  │
│ buying_asset_type      │                │ ledger_sequence        │  │
│ buying_asset_code      │                │ created_at             │  │
│ buying_asset_issuer    │                └─────────────────────────┘  │
│ amount                 │                                             │
│ price                  │                                             │
│ price_n                │                                             │
│ price_d                │                                             │
│ last_modified_ledger   │                                             │
│ archived_at            │                                             │
│ archive_reason         │                                             │
└─────────────────────────┘                                             │
                                                                        │
┌─────────────────────────┐                                             │
│   ingestion_state       │                                             │
├─────────────────────────┤                                             │
│ key (PK)               │                                             │
│ value                  │                                             │
│ updated_at             │                                             │
└─────────────────────────┘                                             │
                                                                        │
┌─────────────────────────┐                                             │
│   db_health_metrics     │                                             │
├─────────────────────────┤                                             │
│ id (PK)                │                                             │
│ metric_name            │                                             │
│ metric_value           │                                             │
│ metric_unit            │                                             │
│ metadata (JSONB)       │                                             │
│ recorded_at            │                                             │
└─────────────────────────┘                                             │
                                                                        │
                        Referenced by all tables ──────────────────────┘
```

## Core Tables

### assets

Stores normalized asset information for all assets traded on SDEX.

**Columns:**

- `id` (UUID, PK): Unique identifier
- `asset_type` (TEXT): Type of asset - "native", "credit_alphanum4", or "credit_alphanum12"
- `asset_code` (TEXT, nullable): Asset code (e.g., "USDC", "BTC")
- `asset_issuer` (TEXT, nullable): Stellar address of asset issuer
- `created_at` (TIMESTAMPTZ): Record creation timestamp

**Constraints:**

- Unique constraint on (asset_type, asset_code, asset_issuer)

**Indexes:**

- `idx_assets_type`: On asset_type
- `idx_assets_code`: On asset_code (partial, where not null)
- `idx_assets_issuer`: On asset_issuer (partial, where not null)

---

### sdex_offers

Main table storing active SDEX offers from Stellar Horizon.

**Columns:**

- `offer_id` (BIGINT, PK): Horizon offer ID
- `seller` (TEXT): Stellar account address of seller
- `selling_asset_id` (UUID, FK → assets): Asset being sold
- `buying_asset_id` (UUID, FK → assets): Asset being bought
- `amount` (NUMERIC(30,14)): Amount of selling asset
- `price` (NUMERIC(30,14)): Price ratio (buying/selling)
- `price_n` (BIGINT): Price numerator
- `price_d` (BIGINT): Price denominator
- `last_modified_ledger` (BIGINT): Ledger sequence when offer was last modified
- `paging_token` (TEXT, nullable): Horizon pagination token
- `updated_at` (TIMESTAMPTZ): Last update timestamp

**Foreign Keys:**

- `selling_asset_id` → assets(id)
- `buying_asset_id` → assets(id)

**Indexes:**

- `idx_sdex_offers_pair`: On (selling_asset_id, buying_asset_id)
- `idx_sdex_offers_seller`: On seller
- `idx_sdex_offers_ledger`: On last_modified_ledger DESC
- `idx_sdex_offers_updated_at`: On updated_at DESC
- `idx_sdex_offers_seller_pair`: On (seller, selling_asset_id, buying_asset_id)
- `idx_sdex_offers_price`: On (selling_asset_id, buying_asset_id, price)

**Query Patterns:**

- Find offers for specific trading pair
- Get best price for pair
- List offers by seller
- Find recent offers by ledger or timestamp

---

### trading_pairs

Tracks active trading pairs with aggregated statistics.

**Columns:**

- `id` (UUID, PK): Unique identifier
- `base_asset_id` (UUID, FK → assets): Base asset in the pair
- `counter_asset_id` (UUID, FK → assets): Counter/quote asset
- `is_active` (BOOLEAN): Whether pair is currently active
- `total_offers` (INTEGER): Current number of active offers
- `total_volume` (NUMERIC(30,14)): Cumulative trading volume
- `last_trade_at` (TIMESTAMPTZ, nullable): Last trade timestamp
- `created_at` (TIMESTAMPTZ): Record creation timestamp
- `updated_at` (TIMESTAMPTZ): Last update timestamp

**Constraints:**

- Unique constraint on (base_asset_id, counter_asset_id)
- Check constraint: base_asset_id != counter_asset_id

**Foreign Keys:**

- `base_asset_id` → assets(id)
- `counter_asset_id` → assets(id)

**Indexes:**

- `idx_trading_pairs_base`: On base_asset_id
- `idx_trading_pairs_counter`: On counter_asset_id
- `idx_trading_pairs_active`: On (is_active, updated_at DESC)
- `idx_trading_pairs_volume`: On total_volume DESC (partial, where is_active)

**Query Patterns:**

- List all active trading pairs
- Find pairs by asset
- Get most traded pairs by volume

---

### orderbook_snapshots

Historical point-in-time snapshots of orderbooks for analytics.

**Columns:**

- `id` (UUID, PK): Unique identifier
- `trading_pair_id` (UUID, FK → trading_pairs): Associated trading pair
- `snapshot_time` (TIMESTAMPTZ): Time of snapshot
- `bids` (JSONB): Array of bid orders with price/amount
- `asks` (JSONB): Array of ask orders with price/amount
- `bid_count` (INTEGER): Number of bid orders
- `ask_count` (INTEGER): Number of ask orders
- `spread` (NUMERIC(30,14), nullable): Bid-ask spread
- `mid_price` (NUMERIC(30,14), nullable): Mid-market price
- `total_bid_volume` (NUMERIC(30,14)): Total bid volume
- `total_ask_volume` (NUMERIC(30,14)): Total ask volume
- `ledger_sequence` (BIGINT): Stellar ledger sequence number
- `created_at` (TIMESTAMPTZ): Record creation timestamp

**Constraints:**

- Check constraint: bid_count >= 0 AND ask_count >= 0
- ON DELETE CASCADE with trading_pairs

**Foreign Keys:**

- `trading_pair_id` → trading_pairs(id) ON DELETE CASCADE

**Indexes:**

- `idx_orderbook_snapshots_pair_time`: On (trading_pair_id, snapshot_time DESC)
- `idx_orderbook_snapshots_time`: On snapshot_time DESC
- `idx_orderbook_snapshots_ledger`: On ledger_sequence DESC

**JSONB Structure:**

```json
{
  "bids": [
    { "price": "1.5000", "amount": "100.00", "offer_id": 12345 },
    { "price": "1.4900", "amount": "250.00", "offer_id": 12346 }
  ],
  "asks": [
    { "price": "1.5100", "amount": "150.00", "offer_id": 12347 },
    { "price": "1.5200", "amount": "200.00", "offer_id": 12348 }
  ]
}
```

**Query Patterns:**

- Get latest snapshot for trading pair
- Historical price analysis
- Volume trends over time
- Spread analysis

---

## Supporting Tables

### archived_offers

Archive table for old/inactive offers to maintain main table performance.

**Purpose:** Keeps sdex_offers table lean by moving historical data

**Columns:** Similar to sdex_offers but denormalized (includes asset types directly)

**Indexes:**

- `idx_archived_offers_archived_at`: On archived_at DESC
- `idx_archived_offers_seller`: On seller

---

### ingestion_state

Tracks indexer state for resumable synchronization.

**Columns:**

- `key` (TEXT, PK): State key (e.g., "last_cursor", "last_ledger")
- `value` (TEXT): State value
- `updated_at` (TIMESTAMPTZ): Last update timestamp

---

### db_health_metrics

Stores database health and performance metrics over time.

**Columns:**

- `id` (UUID, PK): Unique identifier
- `metric_name` (TEXT): Name of metric
- `metric_value` (NUMERIC): Metric value
- `metric_unit` (TEXT, nullable): Unit (count, bytes, ms)
- `metadata` (JSONB, nullable): Additional context
- `recorded_at` (TIMESTAMPTZ): Recording timestamp

**Indexes:**

- `idx_db_health_metrics_recorded_at`: On recorded_at DESC
- `idx_db_health_metrics_name`: On (metric_name, recorded_at DESC)

---

## Views

### active_offers

Denormalized view joining offers with asset information for easier querying.

```sql
SELECT
  o.offer_id, o.seller,
  sa.asset_type as selling_asset_type,
  sa.asset_code as selling_asset_code,
  sa.asset_issuer as selling_asset_issuer,
  ba.asset_type as buying_asset_type,
  ba.asset_code as buying_asset_code,
  ba.asset_issuer as buying_asset_issuer,
  o.amount, o.price, o.price_n, o.price_d,
  o.last_modified_ledger, o.updated_at
FROM sdex_offers o
JOIN assets sa ON o.selling_asset_id = sa.id
JOIN assets ba ON o.buying_asset_id = ba.id;
```

---

### latest_orderbook_snapshots

Most recent snapshot for each trading pair.

```sql
SELECT DISTINCT ON (trading_pair_id)
  s.*, tp.base_asset_id, tp.counter_asset_id
FROM orderbook_snapshots s
JOIN trading_pairs tp ON s.trading_pair_id = tp.id
ORDER BY trading_pair_id, snapshot_time DESC;
```

---

## Materialized Views

### orderbook_summary

Pre-aggregated orderbook statistics for fast queries.

**Columns:**

- selling_asset_id, buying_asset_id
- offer_count, min_price, max_price, avg_price
- total_amount, last_updated

**Refresh:** Call `refresh_orderbook_summary()` function or use `REFRESH MATERIALIZED VIEW CONCURRENTLY`

---

## Stored Functions

### capture_orderbook_snapshot(base_asset_id, counter_asset_id, ledger_sequence)

Captures a point-in-time snapshot of an orderbook.

**Returns:** UUID of created snapshot

**Logic:**

1. Gets or creates trading_pair record
2. Collects bids (base → counter offers)
3. Collects asks (counter → base offers, inverted)
4. Calculates spread and mid-price
5. Inserts snapshot with JSONB data
6. Updates trading_pair statistics

---

### archive_old_offers(days_old)

Archives offers older than specified days.

**Returns:** Count of archived offers

**Default:** 30 days

---

### cleanup_old_snapshots(days_to_keep)

Removes old snapshot records to manage storage.

**Returns:** Count of deleted snapshots

**Default:** 7 days

---

### get_db_health_metrics()

Returns current database health metrics.

**Returns Table:**

- metric_name (TEXT)
- metric_value (NUMERIC)
- metric_unit (TEXT)

**Metrics Provided:**

- total_offers, total_assets, total_archived_offers
- Table sizes (bytes)
- Database size

---

## Migration Files

1. **0001_init.sql** - Core schema (assets, sdex_offers, ingestion_state)
2. **0002_performance_indexes.sql** - Performance indexes, archival, health metrics
3. **0003_trading_pairs_and_snapshots.sql** - Trading pairs and orderbook snapshots

**Run migrations:**

```bash
sqlx migrate run --database-url postgresql://user:pass@localhost/stellarroute
```

---

## Performance Considerations

### Indexes Strategy

- **Composite indexes** for common multi-column queries
- **Partial indexes** on nullable columns to reduce size
- **Descending indexes** for time-series queries
- **Covering indexes** where beneficial

### Data Retention

- **Active offers:** Keep in main table
- **Old offers:** Archive after 30 days (configurable)
- **Snapshots:** Keep 7 days (configurable)
- **Metrics:** Retain based on monitoring needs

### Query Optimization

- Use materialized views for expensive aggregations
- Leverage views for common denormalized queries
- JSONB for flexible nested data (orderbook snapshots)
- Numeric(30,14) for precise financial calculations

### Maintenance

**Regular tasks:**

```sql
-- Refresh materialized view
SELECT refresh_orderbook_summary();

-- Archive old offers
SELECT archive_old_offers(30);

-- Clean old snapshots
SELECT cleanup_old_snapshots(7);

-- Check health
SELECT * FROM get_db_health_metrics();
```

---

## Example Queries

### Get orderbook for a trading pair

```sql
SELECT * FROM active_offers
WHERE selling_asset_id = 'uuid-of-xlm'
  AND buying_asset_id = 'uuid-of-usdc'
ORDER BY price ASC;
```

### Find best price for trade

```sql
SELECT price, amount FROM sdex_offers
WHERE selling_asset_id = $1 AND buying_asset_id = $2
ORDER BY price ASC
LIMIT 1;
```

### Get latest snapshot

```sql
SELECT * FROM latest_orderbook_snapshots
WHERE base_asset_id = $1 AND counter_asset_id = $2;
```

### List active trading pairs

```sql
SELECT
  tp.*,
  ba.asset_code as base_code,
  ca.asset_code as counter_code
FROM trading_pairs tp
JOIN assets ba ON tp.base_asset_id = ba.id
JOIN assets ca ON tp.counter_asset_id = ca.id
WHERE tp.is_active = true
ORDER BY tp.total_volume DESC;
```

---

## References

- [Stellar Horizon API](https://developers.stellar.org/api/horizon/resources/list-all-offers)
- [sqlx migrations](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli)
- [PostgreSQL JSONB](https://www.postgresql.org/docs/current/datatype-json.html)
- [PostgreSQL Numeric Types](https://www.postgresql.org/docs/current/datatype-numeric.html)
