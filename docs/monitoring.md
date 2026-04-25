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

## Prometheus Configuration

Add the following to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'stellarroute'
    static_configs:
      - targets: ['your-stellarroute-host:3000']
    metrics_path: '/metrics'
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