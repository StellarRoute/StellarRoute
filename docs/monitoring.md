# Monitoring and Metrics

StellarRoute exposes Prometheus metrics for monitoring system performance and health.

## Metrics Endpoints

- **Prometheus format**: `GET /metrics`
- **Cache metrics (JSON)**: `GET /metrics/cache`

## Exposed Metrics

### Quote Request Latency

- **Metric**: `stellarroute_quote_request_duration_seconds`
- **Type**: Histogram
- **Labels**:
  - `outcome`: "success" or "error"
  - `cache_hit`: "true" or "false"
- **Description**: Time taken to process quote requests
- **Buckets**: 0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0 seconds

### Route Computation Time

- **Metric**: `stellarroute_route_compute_duration_seconds`
- **Type**: Histogram
- **Labels**:
  - `environment`: "production", "analysis", "realtime", "testing"
- **Description**: Time taken to compute optimal routes
- **Buckets**: 0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0 seconds

### Cache Operations

- **Metrics**:
  - `stellarroute_cache_hits_total` (counter)
  - `stellarroute_cache_misses_total` (counter)
- **Labels**:
  - `type`: "quote"
- **Description**: Cache hit and miss counts

### Quote Requests

- **Metric**: `stellarroute_quote_requests_total`
- **Type**: Counter
- **Labels**:
  - `outcome`: "success" or "error"
  - `cache_hit`: "true" or "false"
- **Description**: Total number of quote requests

### Indexer Synchronization Lag

- **Metrics**:
  - `stellarroute_indexer_lag_ledgers` (gauge): Number of ledgers the local index is behind the live Horizon sequence.
  - `stellarroute_indexer_lag_seconds` (gauge): Estimated wall-clock lag of the local index behind Horizon (seconds).
  - `stellarroute_indexer_last_indexed_ledger` (gauge): Most recently indexed ledger sequence number.
  - `stellarroute_indexer_horizon_ledger` (gauge): Current Horizon latest ledger sequence number (cached).
  - `stellarroute_indexer_sync_status` (gauge): Indexer sync health: 1=ok, 0=warning, -1=critical, -2=unknown.
- **Labels**:
  - `source`: "sdex" or "amm"
- **Thresholds**:
  - `ok`: < 10 ledgers (< 50s)
  - `warning`: 10-60 ledgers (50s-300s)
  - `critical`: > 60 ledgers (> 300s)
- **Behavior**:
  - When lag is `critical`, `/health/deps` returns `degraded`.
  - When lag is `critical`, API quote requests are rejected with `stale_market_data` error.

## Prometheus Configuration

Add the following to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: "stellarroute"
    static_configs:
      - targets: ["your-stellarroute-host:3000"]
    metrics_path: "/metrics"
```

## Grafana Dashboard

### P50/P95 Quote Latency

```json
{
  "title": "Quote Latency P50/P95",
  "targets": [
    {
      "expr": "histogram_quantile(0.50, rate(stellarroute_quote_request_duration_seconds_bucket[5m]))",
      "legendFormat": "P50"
    },
    {
      "expr": "histogram_quantile(0.95, rate(stellarroute_quote_request_duration_seconds_bucket[5m]))",
      "legendFormat": "P95"
    }
  ]
}
```

### Route Compute Time

```json
{
  "title": "Route Computation Time",
  "targets": [
    {
      "expr": "rate(stellarroute_route_compute_duration_seconds_sum[5m]) / rate(stellarroute_route_compute_duration_seconds_count[5m])",
      "legendFormat": "Average Compute Time"
    }
  ]
}
```

### Cache Hit Ratio

```json
{
  "title": "Cache Hit Ratio",
  "targets": [
    {
      "expr": "rate(stellarroute_cache_hits_total[5m]) / (rate(stellarroute_cache_hits_total[5m]) + rate(stellarroute_cache_misses_total[5m]))",
      "legendFormat": "Cache Hit Ratio"
    }
  ]
}
```

## Alerting

### High Quote Latency

```prometheus
alert: HighQuoteLatency
expr: histogram_quantile(0.95, rate(stellarroute_quote_request_duration_seconds_bucket[5m])) > 1.0
for: 5m
labels:
  severity: warning
annotations:
  summary: "Quote latency P95 is high"
  description: "95th percentile quote latency is {{ $value }}s"
```

### Low Cache Hit Ratio

```prometheus
alert: LowCacheHitRatio
expr: rate(stellarroute_cache_hits_total[5m]) / (rate(stellarroute_cache_hits_total[5m]) + rate(stellarroute_cache_misses_total[5m])) < 0.5
for: 10m
labels:
  severity: warning
annotations:
  summary: "Cache hit ratio is low"
  description: "Cache hit ratio dropped below 50%"
```

## External Dependency Circuit Breakers

`GET /health/deps` now performs lightweight probes with independent breakers:

- Horizon probe: `GET {STELLAR_HORIZON_URL}/health`
- Soroban probe: JSON-RPC `getHealth` to `SOROBAN_RPC_URL`

Each dependency has its own breaker state (`closed`, `open`, `half_open`), so one provider can degrade while the other remains healthy.

### Half-open Recovery Behavior

- When a breaker opens, active probes are suppressed and the dependency is reported as `degraded (circuit_open)`.
- After `recovery_timeout_secs`, the breaker transitions to half-open automatically.
- In half-open, a normal health probe is attempted.
- Consecutive probe successes (`success_threshold`) close the breaker.
- Any failure during half-open re-opens the breaker immediately.

This keeps Soroban RPC outages isolated from Horizon health while still allowing automatic, probe-driven recovery.
