# Redis Chaos Runbook — Quote Burst Degradation

Guide for validating graceful degradation when Redis becomes unavailable during high
quote traffic.

## Expected behavior

When Redis is configured but unreachable mid-operation:

1. **Cache lookups** return `Unavailable` (not confused with a miss-only path).
2. **Quote compute path** continues — responses are computed from PostgreSQL/routing.
3. **No unbounded memory growth** — `SingleFlight` coalesces identical keys and cleans up
   inflight entries after completion or cancellation.
4. **Metrics** show degraded mode activation.

## Key metrics

| Metric | Labels | Meaning |
|--------|--------|---------|
| `stellarroute_cache_degraded_mode` | `type=quote` | `1` = degraded (Redis errors), `0` = normal |
| `stellarroute_cache_unavailable_ops_total` | `operation=get\|set` | Redis operation failures |
| `stellarroute_cache_misses_total` | `type=quote` | Includes fallback compute path during outage |
| `stellarroute_single_flight_coalesced_total` | `type=quote` | Burst coalescing under outage |

## Chaos test procedure

### 1. Baseline (Redis healthy)

```bash
export REDIS_URL=redis://127.0.0.1:6379
export DATABASE_URL=postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute
cargo test -p stellarroute-api redis_chaos -- --nocapture
```

### 2. Simulate outage during burst

```bash
# Start API with Redis
REDIS_URL=redis://127.0.0.1:6379 cargo run -p stellarroute-api &

# Warm cache
for i in $(seq 1 50); do
  curl -s "http://127.0.0.1:3000/api/v1/quote/native/USDC?amount=10" > /dev/null
done

# Kill Redis
docker stop stellarroute-redis   # or: redis-cli shutdown

# Burst identical quotes — should coalesce and compute, not hang
ab -n 200 -c 20 "http://127.0.0.1:3000/api/v1/quote/native/USDC?amount=10"

# Verify metrics
curl -s http://127.0.0.1:3000/metrics | grep -E 'cache_degraded_mode|cache_unavailable'
```

Expected:

- `stellarroute_cache_degraded_mode{type="quote"} 1`
- `stellarroute_cache_unavailable_ops_total` increasing
- All requests return 200/404 (not 503 from cache layer)

### 3. Recovery

Restart Redis and confirm `stellarroute_cache_degraded_mode{type="quote"} 0` after a
successful cache hit.

## Automated test suite

Unit/integration tests live in `crates/api/tests/redis_chaos_test.rs`:

- Cache miss fallback to compute path when lookup returns `Unavailable`
- SingleFlight map bounded under concurrent burst
- Degraded mode metrics increment on unavailable operations

Run:

```bash
cargo test -p stellarroute-api redis_chaos
```

## Related docs

- [`AUDIT_LOG_EXPORT_RUNBOOK.md`](./AUDIT_LOG_EXPORT_RUNBOOK.md)
- [`QUOTE_PURGER_RUNBOOK.md`](./QUOTE_PURGER_RUNBOOK.md)
- [`incident-replay-workflow.md`](./incident-replay-workflow.md)
