# Database Connection Pool Tuning Guide

StellarRoute uses [sqlx](https://github.com/launchbadge/sqlx) with a `PgPool`
for both the primary (write) and optional replica (read) pools.  This guide
explains how to size the pools and how to use the runtime introspection
endpoint to validate your settings.

---

## Runtime introspection

```
GET /metrics/pool
```

Returns non-sensitive pool statistics for both pools.  No credentials or
connection strings are included.

**Example response:**

```json
{
  "primary": {
    "max_connections": 20,
    "size": 12,
    "idle": 8,
    "in_use": 4,
    "utilisation": 0.20
  },
  "replica": {
    "max_connections": 40,
    "size": 35,
    "idle": 10,
    "in_use": 25,
    "utilisation": 0.625
  }
}
```

**Field definitions:**

| Field | Description |
|---|---|
| `max_connections` | Hard cap configured via env var |
| `size` | Current open connections (idle + in-use) |
| `idle` | Connections waiting for a query |
| `in_use` | Connections currently executing a query |
| `utilisation` | `in_use / max_connections` (0.0–1.0) |

**Quick curl example:**

```bash
curl -s http://localhost:8080/metrics/pool | jq .
```

---

## Environment variables

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | — | Primary pool connection string (required) |
| `DATABASE_REPLICA_URL` | — | Replica pool connection string (optional) |
| `DATABASE_POOL_MAX_CONNECTIONS` | `20` | Max connections for the primary pool |
| `DATABASE_REPLICA_POOL_MAX_CONNECTIONS` | `40` | Max connections for the replica pool |
| `DATABASE_POOL_MIN_CONNECTIONS` | `2` | Minimum idle connections kept warm |
| `DATABASE_POOL_ACQUIRE_TIMEOUT_SECS` | `5` | Max wait time to acquire a connection |
| `DATABASE_POOL_IDLE_TIMEOUT_SECS` | `600` | Idle connections closed after this many seconds |
| `DATABASE_POOL_MAX_LIFETIME_SECS` | `1800` | Connections recycled after this many seconds |

---

## Sizing heuristics

### Primary (write) pool

The primary pool handles writes and admin queries.  Write throughput is
typically low compared to reads.

```
max_connections ≈ (num_api_instances × avg_concurrent_writes) + headroom
```

A starting point for a single API instance:

```
max_connections = 10–20
```

### Replica (read) pool

The replica pool handles all quote and orderbook reads.  These are the
hot path.

```
max_connections ≈ (num_api_instances × target_rps × avg_query_latency_secs) × 1.5
```

For example, 500 RPS with 10 ms average query latency on a single instance:

```
max_connections ≈ 1 × 500 × 0.010 × 1.5 = 7.5  →  round up to 10–15
```

Add more headroom for burst traffic.  A safe upper bound is the PostgreSQL
`max_connections` setting divided by the number of application instances.

### Utilisation targets

| Utilisation | Action |
|---|---|
| < 0.50 | Pool is oversized; reduce `max_connections` to free server resources |
| 0.50–0.80 | Healthy range |
| > 0.80 | Pool is undersized; increase `max_connections` or add replicas |
| 1.00 | Connections are being queued; immediate action required |

---

## Detecting pool exhaustion

When all connections are in use, new requests wait up to
`DATABASE_POOL_ACQUIRE_TIMEOUT_SECS` before failing with a `503` or a
database timeout error.  Signs of pool exhaustion:

- `utilisation` consistently above 0.90 in `/metrics/pool`
- Increased p99 latency on `/api/v1/quote`
- `sqlx::PoolTimedOut` errors in logs

---

## PostgreSQL server-side limits

Ensure `max_connections` in `postgresql.conf` is at least:

```
sum(max_connections across all app instances) + superuser_reserved_connections (default 3)
```

For PgBouncer or other connection poolers, set the pool size on the pooler
side and keep the application pool small (2–5 connections per instance).

---

## Recommended monitoring

Scrape `/metrics/pool` every 15 seconds and alert when:

- `utilisation > 0.85` for more than 60 seconds
- `idle == 0` (all connections in use)
- `size < min_connections` (connections are being dropped unexpectedly)
