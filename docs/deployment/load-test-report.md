# Load Test Report — Public Quote / Routes Endpoints

This template is filled out after running `scripts/load-test-quote-routes.k6.js`.
It documents whether the StellarRoute API meets the launch performance budget
under expected traffic.

## Performance budget reference

From [`docs/performance_budget.md`](../performance_budget.md):

| Endpoint | Metric | Budget | Pass/Fail |
|----------|--------|--------|-----------|
| `GET /api/v1/quote/{base}/{quote}` | p95 latency | **< 500 ms** | TBD |
| `GET /api/v1/routes/{base}/{quote}` | p95 latency | **< 500 ms** | TBD |
| Both | Error rate | **< 1 %** | TBD |

## Run metadata

| Field | Value |
|-------|-------|
| Date | YYYY-MM-DD |
| Commit / tag | `<sha>` |
| Target environment | `https://...` or `http://localhost:3000` |
| k6 version | `k6 version` |
| VUs | `<from __ENV.VUS>` |
| Ramp duration | `<from __ENV.RAMP_DURATION>` |
| Steady duration | `<from __ENV.DURATION>` |
| Pairs tested | `XLM/USDC, USDC/XLM, XLM/EURC, EURC/USDC` |
| Amount | `100.0` |

## Reproduction commands

```bash
# Local (dev server must be running)
k6 run scripts/load-test-quote-routes.k6.js

# Production-like target
k6 run -e BASE_URL=https://api.stellarroute.io \
       -e VUS=250 \
       -e DURATION=5m \
       scripts/load-test-quote-routes.k6.js

# Custom pairs
k6 run -e PAIRS="XLM/USDC,BTC/USDC" \
       scripts/load-test-quote-routes.k6.js
```

## Results

### Summary metrics

| Metric | p50 | p95 | p99 | Max | Mean | Count |
|--------|-----|-----|-----|-----|------|-------|
| `quote_req_duration_ms` | | | | | | |
| `routes_req_duration_ms` | | | | | | |
| `errors` (rate) | | | | | | |
| `http_req_failed` (rate) | | | | | | |
| Total iterations | | | | | | |

### Pass/fail verdict

| Criterion | Budget | Observed | Status |
|-----------|--------|----------|--------|
| Quote p95 latency | < 500 ms | | |
| Routes p95 latency | < 500 ms | | |
| Error rate | < 1 % | | |

**Overall verdict:** ✅ PASS / ❌ FAIL

## Bottlenecks observed

Document any performance regressions or saturation points observed during the
run. For each bottleneck, include:

1. Symptom (metric, latency spike, error class, resource).  
2. Probable cause.  
3. Mitigation implemented or recommended.

### Bottleneck 1: <name>

- **Symptom:**
- **Probable cause:**
- **Mitigation:**

### Bottleneck 2: <name>

- **Symptom:**
- **Probable cause:**
- **Mitigation:**

## Common mitigation playbook

| Bottleneck area | Symptoms | Mitigation | Reference |
|-----------------|----------|------------|-----------|
| Database pool saturation | p95 latency climbs under load, DB connection wait time high | Increase pool size, add read replicas, or reduce query complexity | [`db-pool-tuning.md`](db-pool-tuning.md) |
| Redis cache miss storm | Cache hit ratio drops, quote latency spikes | Warm cache, increase TTL, or protect with single-flight | [`cache/warming`](../../crates/api/src/cache/) |
| Route compute latency | routes p95 > 500 ms | Pre-compute graphs, tighten AMM discovery, or shard pairs | `crates/routing` |
| Indexer lag | Stale data errors, `sdex/amm` lag gauges high | Scale indexer, optimize ingestion queries | [`indexer-lag-monitoring.md`](../indexer-lag-monitoring.md) |
| Rate limiting / middleware | 429s, latency spikes at boundary | Tune rate-limit buckets, review middleware order | [`middleware/`](../api/middleware.md) |

## Artifacts

- k6 JSON output: `load-test-results.json` (produced by `handleSummary`)
- CI run link: `<URL>`
- Grafana dashboard snapshot: `<URL>`

## Sign-off

| Role | Name | Date | Notes |
|------|------|------|-------|
| Test runner | | | |
| Engineering review | | | |
| Launch approval | | | |
