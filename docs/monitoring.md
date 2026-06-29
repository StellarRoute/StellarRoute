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

### Indexer Lag

See [indexer-lag-monitoring.md](indexer-lag-monitoring.md) for full documentation of indexer lag metrics (`stellarroute_indexer_lag_ledgers`, `stellarroute_indexer_lag_seconds`, `stellarroute_indexer_sync_status`, etc.).

## Prometheus Configuration

Add the following to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: "stellarroute"
    static_configs:
      - targets: ["your-stellarroute-host:3000"]
    metrics_path: "/metrics"
```

For SLO alerting rules, include `monitoring/prometheus/slo-alerts.yml`:

```yaml
rule_files:
  - 'monitoring/prometheus/slo-alerts.yml'
```

## Service Level Objectives (SLOs)

SLO definitions are maintained as code in [`config/slo.yaml`](../config/slo.yaml). The following objectives are defined:

| SLO | Target | Window | Compliance Target | Burn Rate Warning | Burn Rate Critical |
|-----|--------|--------|-------------------|-------------------|-------------------|
| Quote P95 Latency | < 500ms | 5m | 99.9% | 2x over 30m | 4x over 30m |
| Quote P99 Latency | < 2s | 5m | 99.5% | 2x over 30m | 4x over 30m |
| Quote Error Rate | < 1% | 5m | 99.9% | 2x over 30m | 4x over 30m |
| Route Compute P95 | < 1s | 5m | 99.0% | 2x over 30m | 4x over 30m |
| Cache Hit Ratio | > 70% | 10m | 99.0% | 2x over 30m | 4x over 30m |
| Indexer Sync Health | >= 0 | 5m | 99.5% | — | — |

### Burn-Rate Alerting Strategy

Alerts use a multi-window, multi-burn-rate approach:

- **Warning (2x burn rate)**: Error budget consumed at 2x the expected rate over a 30-minute window. Estimated time to exhaust 30-day budget: ~7.5 hours.
- **Critical (4x burn rate)**: Error budget consumed at 4x the expected rate over a 30-minute window. Estimated time to exhaust 30-day budget: ~3.75 hours.

Both a short window (1-5m) and a long window (30m) must simultaneously breach the SLO target before an alert fires. This prevents flapping from transient spikes while ensuring sustained violations are caught quickly.

## Alerting Rules

Alerting rules are defined in [`monitoring/prometheus/slo-alerts.yml`](../monitoring/prometheus/slo-alerts.yml). They are organized into two groups:

### SLO Burn-Rate Alerts (`stellarroute_slo_alerts`)

| Alert | Severity | Condition | For |
|-------|----------|-----------|-----|
| `SLOQuoteP95LatencyBurnWarning` | warning | P95 > 500ms (1m & 30m windows) | 1m |
| `SLOQuoteP95LatencyBurnCritical` | critical | P95 > 500ms (5m & 30m windows) | 1m |
| `SLOQuoteP99LatencyBurnWarning` | warning | P99 > 2s (1m & 30m windows) | 1m |
| `SLOQuoteP99LatencyBurnCritical` | critical | P99 > 2s (5m & 30m windows) | 1m |
| `SLOQuoteErrorRateBurnWarning` | warning | error rate > 1% (1m & 30m windows) | 1m |
| `SLOQuoteErrorRateBurnCritical` | critical | error rate > 1% (5m & 30m windows) | 1m |
| `SLORouteComputeP95LatencyBurnWarning` | warning | P95 > 1s (1m & 30m windows) | 1m |
| `SLOCacheHitRatioBurnWarning` | warning | hit ratio < 70% (1m & 10m windows) | 2m |
| `SLOIndexerSyncCritical` | critical | sync_status < 0 | 2m |

### Direct Threshold Alerts (`stellarroute_direct_alerts`)

| Alert | Severity | Condition | For |
|-------|----------|-----------|-----|
| `HighQuoteLatency` | warning | P95 > 1s over 5m | 5m |
| `LowCacheHitRatio` | warning | hit ratio < 50% over 5m | 10m |

## Synthetic Probes

Probe definitions are maintained as code in [`config/slo.yaml`](../config/slo.yaml). The following synthetic probes are defined:

| Probe | Endpoint | Interval | Test Cases | Thresholds |
|-------|----------|----------|------------|------------|
| `quote_smoke_test` | GET /api/v1/quote/{base}/{quote} | 5m | XLM/USDC, USDC/XLM, XLM/EURC | max latency 2000ms, 0% error rate |
| `quote_load_probe` | GET /api/v1/quote/{base}/{quote} | 15m | XLM/USDC, USDC/XLM, XLM/EURC, EURC/USDC | P50 < 200ms, P95 < 500ms, P99 < 2s, error rate < 1% |
| `route_smoke_test` | GET /api/v1/route/{base}/{quote} | 5m | XLM/USDC | max latency 5000ms, 0% error rate |

The probe runner script [`scripts/slo-probe.sh`](../scripts/slo-probe.sh) executes the smoke test probes from CI or any shell environment:

```bash
# Run smoke probes against production
./scripts/slo-probe.sh --base-url https://api.stellarroute.io

# Run with verbose output against local dev
./scripts/slo-probe.sh --base-url http://localhost:3000 --verbose

# Quiet mode — only show pass/fail summary
./scripts/slo-probe.sh --base-url https://api.stellarroute.io --quiet
```

Scheduled execution is configured in [`.github/workflows/slo-probes.yml`](../.github/workflows/slo-probes.yml), which runs smoke probes every 5 minutes on the main branch.

## Grafana Dashboard

A comprehensive SLO dashboard is available at [`monitoring/grafana/slo-dashboard.json`](../monitoring/grafana/slo-dashboard.json). Import into Grafana via **Dashboards → Import → Upload JSON file**.

The dashboard includes the following panels:

| Panel | Description | SLO Reference |
|-------|-------------|---------------|
| Quote Latency P50/P95/P99 | Latency percentiles with threshold lines at 500ms and 2s | quote_p95_latency, quote_p99_latency |
| Quote Error Rate | Error rate percentage with threshold at 1% | quote_error_rate |
| Route Compute Time P95 | P95 route computation with threshold at 1s | route_compute_p95_latency |
| Cache Hit Ratio | Cache hit percentage with thresholds at 50% and 70% | cache_hit_ratio |
| SLO Burn Rate – Quote P95 Latency | Multi-window burn rate view (1m vs 30m) | quote_p95_latency |
| SLO Burn Rate – Quote Error Rate | Multi-window burn rate view (1m vs 30m) | quote_error_rate |
| Indexer Sync Status | Stat panel per source (ok/warning/critical/unknown) | indexer_sync_health |
| Indexer Lag (ledgers) | Lag per source with thresholds at 10 and 60 ledgers | indexer_sync_health |
| SLO Compliance (30d burn rate) | 30-day compliance for error rate SLO | quote_error_rate |

### Individual Panel Queries

For ad-hoc Grafana panels, the following PromQL queries can be used:

**P50/P95/P99 Quote Latency:**
```promql
histogram_quantile(0.50, rate(stellarroute_quote_request_duration_seconds_bucket[5m]))
histogram_quantile(0.95, rate(stellarroute_quote_request_duration_seconds_bucket[5m]))
histogram_quantile(0.99, rate(stellarroute_quote_request_duration_seconds_bucket[5m]))
```

**Average Route Compute Time:**
```promql
rate(stellarroute_route_compute_duration_seconds_sum[5m]) / rate(stellarroute_route_compute_duration_seconds_count[5m])
```

**Cache Hit Ratio:**
```promql
rate(stellarroute_cache_hits_total[5m]) / (rate(stellarroute_cache_hits_total[5m]) + rate(stellarroute_cache_misses_total[5m]))
```

**Quote Error Rate:**
```promql
rate(stellarroute_quote_requests_total{outcome="error"}[5m]) / rate(stellarroute_quote_requests_total[5m])
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

## References

- **SLO definitions**: [`config/slo.yaml`](../config/slo.yaml)
- **Prometheus alerting rules**: [`monitoring/prometheus/slo-alerts.yml`](../monitoring/prometheus/slo-alerts.yml)
- **Grafana dashboard**: [`monitoring/grafana/slo-dashboard.json`](../monitoring/grafana/slo-dashboard.json)
- **Probe runner script**: [`scripts/slo-probe.sh`](../scripts/slo-probe.sh)
- **CI workflow**: [`.github/workflows/slo-probes.yml`](../.github/workflows/slo-probes.yml)
- **Indexer lag monitoring**: [`docs/indexer-lag-monitoring.md`](indexer-lag-monitoring.md)
