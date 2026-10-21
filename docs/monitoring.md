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

### Swap Prepare / Submit

- **Metrics**:
  - `stellarroute_swap_prepare_total` (counter)
  - `stellarroute_swap_submit_total` (counter)
  - `stellarroute_swap_prepare_duration_seconds` (histogram)
  - `stellarroute_swap_submit_duration_seconds` (histogram)
  - `stellarroute_swap_inflight` (gauge)
- **Labels**:
  - `outcome`: "success" or "error"
  - `error_class`: machine-readable error category (e.g. `none`, `validation`, `simulation_failed`, `bad_signature`, `timeout`, `rpc_error`, `internal`)
  - `phase`: "prepare" or "submit" (on the `stellarroute_swap_inflight` gauge)
- **Description**: Outcomes, latency, and concurrency of the two-phase swap flow (`POST /swap/prepare` and `POST /swap/submit`).

| Error Class | Phase | Meaning |
|-------------|-------|---------|
| `none` | both | Request succeeded |
| `validation` | both | Request validation failed |
| `quote_expired` | prepare | Referenced quote is stale/expired |
| `quote_not_found` | prepare | Referenced `quote_id` does not exist |
| `simulation_failed` | prepare | Soroban simulation failed |
| `build_failed` | prepare | Transaction build failed |
| `duplicate_quote` | submit | `quote_id` was already submitted |
| `bad_signature` | submit | Supplied signature is invalid |
| `insufficient_fee` | submit | Transaction fee too low |
| `insufficient_balance` | submit | Source account lacks funds |
| `slippage_exceeded` | submit | On-chain execution exceeded slippage |
| `timeout` | both | Upstream Soroban/Horizon timeout |
| `rpc_error` | both | Generic RPC error |
| `internal` | both | Internal/unexpected error |

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

## Recording Rules

Pre-computed series for the quote-latency and indexer-lag panels live in
[`monitoring/prometheus/recording-rules.yaml`](../monitoring/prometheus/recording-rules.yaml).
They derive new series from metrics the API and indexer already export — no
underlying metric is renamed or replaced — so adding the file is safe on a
running Prometheus.

```yaml
rule_files:
  - 'monitoring/prometheus/recording-rules.yaml'
  - 'monitoring/prometheus/slo-alerts.yml'
```

| Recorded series | Derived from | Use |
|-----------------|--------------|-----|
| `job:stellarroute_quote_request_duration_seconds:p95_5m` | `stellarroute_quote_request_duration_seconds_bucket` | Quote P95 against the 500ms SLO |
| `job:stellarroute_quote_request_duration_seconds:p95_30m` | same | Long window for burn-rate panels |
| `job_outcome:stellarroute_quote_request_duration_seconds:p95_5m` | same | P95 split by `outcome` |
| `job_cache_hit:stellarroute_quote_request_duration_seconds:p95_5m` | same | Cold-path (cache miss) P95 |
| `job:stellarroute_indexer_lag_ledgers:max` | `stellarroute_indexer_lag_ledgers` | Worst lag across sources, for a status tile |
| `source:stellarroute_indexer_lag_ledgers:max` | same | Per-source lag, aggregated across replicas |
| `source:stellarroute_indexer_lag_ledgers:avg_5m` | same | Smoothed lag for trend panels |
| `source:stellarroute_indexer_lag_seconds:max` | `stellarroute_indexer_lag_seconds` | Wall-clock staleness per source |

Names follow the Prometheus `level:metric:operations` convention. Grafana panels
can swap a `histogram_quantile(...)` expression for the recorded series directly:

```promql
# Before
histogram_quantile(0.95, rate(stellarroute_quote_request_duration_seconds_bucket[5m]))
# After
job:stellarroute_quote_request_duration_seconds:p95_5m
```

### Validating the rules

`promtool` ships with Prometheus. CI does not run it — validate locally before
changing the file:

```bash
promtool check rules monitoring/prometheus/recording-rules.yaml
# SUCCESS: 8 rules found
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
| Swap Prepare Success Rate | > 99% | 5m | 99.9% | 2x over 30m | 4x over 30m |
| Swap Submit Success Rate | > 99% | 5m | 99.9% | 2x over 30m | 4x over 30m |

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
| `SLOSwapPrepareFailureRateBurnWarning` | warning | prepare failure rate > 1% (1m & 30m windows) | 1m |
| `SLOSwapPrepareFailureRateBurnCritical` | critical | prepare failure rate > 1% (5m & 30m windows) | 1m |
| `SLOSwapSubmitFailureRateBurnWarning` | warning | submit failure rate > 1% (1m & 30m windows) | 1m |
| `SLOSwapSubmitFailureRateBurnCritical` | critical | submit failure rate > 1% (5m & 30m windows) | 1m |

### Direct Threshold Alerts (`stellarroute_direct_alerts`)

| Alert | Severity | Condition | For |
|-------|----------|-----------|-----|
| `HighQuoteLatency` | warning | P95 > 1s over 5m | 5m |
| `LowCacheHitRatio` | warning | hit ratio < 50% over 5m | 10m |
| `HighSwapPrepareFailureRate` | warning | prepare failure rate > 5% over 5m | 5m |
| `HighSwapSubmitFailureRate` | warning | submit failure rate > 5% over 5m | 5m |

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

**Swap Prepare Failure Rate:**
```promql
rate(stellarroute_swap_prepare_total{outcome="error"}[5m]) / rate(stellarroute_swap_prepare_total[5m])
```

**Swap Submit Failure Rate:**
```promql
rate(stellarroute_swap_submit_total{outcome="error"}[5m]) / rate(stellarroute_swap_submit_total[5m])
```

**Swap Failure Rate by Error Class:**
```promql
sum by (error_class) (rate(stellarroute_swap_prepare_total{outcome="error"}[5m]))
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

## Observability Map: Metric → Grafana Panel → SLO

This table is the single source of truth mapping every emitted Prometheus metric to its
dashboard panel and the SLO it serves.  **Do not rename metric names** — doing so silently
breaks all dashboard queries and alert rules.

### Quote & Route

| Metric | Type | Dashboard | Panel Title | SLO | Healthy Threshold |
|---|---|---|---|---|---|
| `stellarroute_quote_request_duration_seconds` | Histogram | [`slo-dashboard.json`](../monitoring/grafana/slo-dashboard.json) | **Quote Latency P50 / P95 / P99** | `quote_p95_latency`, `quote_p99_latency` | P95 < 500 ms, P99 < 2 s |
| `stellarroute_quote_request_duration_seconds` | Histogram | [`slo-dashboard.json`](../monitoring/grafana/slo-dashboard.json) | **SLO Burn Rate – Quote P95 Latency** | `quote_p95_latency` | 1 m + 30 m windows both < 500 ms |
| `stellarroute_quote_requests_total` | Counter | [`slo-dashboard.json`](../monitoring/grafana/slo-dashboard.json) | **Quote Error Rate** | `quote_error_rate` | error rate < 1 % |
| `stellarroute_quote_requests_total` | Counter | [`slo-dashboard.json`](../monitoring/grafana/slo-dashboard.json) | **SLO Burn Rate – Quote Error Rate** | `quote_error_rate` | 1 m + 30 m windows both < 1 % |
| `stellarroute_quote_requests_total` | Counter | [`slo-dashboard.json`](../monitoring/grafana/slo-dashboard.json) | **SLO Compliance (30d burn rate)** | `quote_error_rate` | ≥ 99.9 % over 30 d |
| `stellarroute_route_compute_duration_seconds` | Histogram | [`slo-dashboard.json`](../monitoring/grafana/slo-dashboard.json) | **Route Compute Time P95** | `route_compute_p95_latency` | P95 < 1 s |

### Cache

| Metric | Type | Dashboard | Panel Title | SLO | Healthy Threshold |
|---|---|---|---|---|---|
| `stellarroute_cache_hits_total` | Counter | [`slo-dashboard.json`](../monitoring/grafana/slo-dashboard.json) | **Cache Hit Ratio** | `cache_hit_ratio` | ≥ 70 % over 10 m |
| `stellarroute_cache_misses_total` | Counter | [`slo-dashboard.json`](../monitoring/grafana/slo-dashboard.json) | **Cache Hit Ratio** | `cache_hit_ratio` | ≥ 70 % over 10 m |

### Swap Prepare / Submit

> [!NOTE]
> The panels below exist in [`swap-indexer-panels.json`](../monitoring/grafana/swap-indexer-panels.json)
> (new dedicated dashboard). Import it alongside `slo-dashboard.json` for full swap coverage.
> The `slo-dashboard.json` SLO compliance panel tracks the aggregated error rates; the new
> dashboard adds per-`error_class` breakdown and in-flight concurrency views.

| Metric | Type | Dashboard | Panel Title | SLO | Healthy Threshold |
|---|---|---|---|---|---|
| `stellarroute_swap_prepare_total` | Counter | [`swap-indexer-panels.json`](../monitoring/grafana/swap-indexer-panels.json) | **Swap Prepare Success / Error Rate** | `swap_prepare_success_rate` | error rate < 1 % |
| `stellarroute_swap_prepare_total` | Counter | [`swap-indexer-panels.json`](../monitoring/grafana/swap-indexer-panels.json) | **Swap Prepare Errors by Class** | `swap_prepare_success_rate` | `simulation_failed`, `timeout` near zero |
| `stellarroute_swap_prepare_duration_seconds` | Histogram | [`swap-indexer-panels.json`](../monitoring/grafana/swap-indexer-panels.json) | **Swap Prepare Latency P50 / P95** | — | P95 < 2 s (informational) |
| `stellarroute_swap_submit_total` | Counter | [`swap-indexer-panels.json`](../monitoring/grafana/swap-indexer-panels.json) | **Swap Submit Success / Error Rate** | `swap_submit_success_rate` | error rate < 1 % |
| `stellarroute_swap_submit_total` | Counter | [`swap-indexer-panels.json`](../monitoring/grafana/swap-indexer-panels.json) | **Swap Submit Errors by Class** | `swap_submit_success_rate` | `bad_signature`, `slippage_exceeded` near zero |
| `stellarroute_swap_submit_duration_seconds` | Histogram | [`swap-indexer-panels.json`](../monitoring/grafana/swap-indexer-panels.json) | **Swap Submit Latency P50 / P95** | — | P95 < 5 s (informational) |
| `stellarroute_swap_inflight` | Gauge | [`swap-indexer-panels.json`](../monitoring/grafana/swap-indexer-panels.json) | **Swap In-Flight Concurrency** | — | No runaway accumulation |

### Indexer Lag & Sync

> [!NOTE]
> The dedicated [`indexer-lag-dashboard.json`](../monitoring/grafana/indexer-lag-dashboard.json)
> provides the primary view. The `slo-dashboard.json` also includes summary Indexer Lag and Sync
> Status panels at row y=24. Both dashboards query the same metrics — no duplication of data.
> The panels below marked _(lag dash)_ live in `indexer-lag-dashboard.json`; those marked
> _(slo dash)_ live in `slo-dashboard.json`.

| Metric | Type | Dashboard | Panel Title | SLO | Healthy Threshold |
|---|---|---|---|---|---|
| `stellarroute_indexer_lag_ledgers` | Gauge | [`indexer-lag-dashboard.json`](../monitoring/grafana/indexer-lag-dashboard.json) _(lag dash)_ | **Indexer Lag (ledgers)** | `indexer_sync_health` | < 10 ledgers |
| `stellarroute_indexer_lag_ledgers` | Gauge | [`indexer-lag-dashboard.json`](../monitoring/grafana/indexer-lag-dashboard.json) _(lag dash)_ | **Indexer Lag by Source** | `indexer_sync_health` | < 10 ledgers, both `sdex` and `amm` |
| `stellarroute_indexer_lag_ledgers` | Gauge | [`slo-dashboard.json`](../monitoring/grafana/slo-dashboard.json) _(slo dash)_ | **Indexer Lag (ledgers)** | `indexer_sync_health` | < 10 ledgers |
| `stellarroute_indexer_lag_seconds` | Gauge | [`indexer-lag-dashboard.json`](../monitoring/grafana/indexer-lag-dashboard.json) _(lag dash)_ | _(PromQL reference)_ | `indexer_sync_health` | < 50 s |
| `stellarroute_indexer_sync_status` | Gauge | [`indexer-lag-dashboard.json`](../monitoring/grafana/indexer-lag-dashboard.json) _(lag dash)_ | **Indexer Sync Status** | `indexer_sync_health` | `1` (OK) for all sources |
| `stellarroute_indexer_sync_status` | Gauge | [`indexer-lag-dashboard.json`](../monitoring/grafana/indexer-lag-dashboard.json) _(lag dash)_ | **Critical or Unknown Sources** | `indexer_sync_health` | Flat / zero |
| `stellarroute_indexer_sync_status` | Gauge | [`slo-dashboard.json`](../monitoring/grafana/slo-dashboard.json) _(slo dash)_ | **Indexer Sync Status** | `indexer_sync_health` | `1` (OK) for all sources |
| `stellarroute_indexer_last_indexed_ledger` | Gauge | [`indexer-lag-dashboard.json`](../monitoring/grafana/indexer-lag-dashboard.json) _(lag dash)_ | _(PromQL reference)_ | `indexer_sync_health` | Always advancing |
| `stellarroute_indexer_horizon_ledger` | Gauge | [`indexer-lag-dashboard.json`](../monitoring/grafana/indexer-lag-dashboard.json) _(lag dash)_ | _(PromQL reference)_ | `indexer_sync_health` | Always advancing |

### Indexer Ingestion Counters

| Metric | Type | Dashboard | Panel Title | SLO | Healthy Threshold |
|---|---|---|---|---|---|
| `stellarroute_indexer_offers_indexed_total` | Counter | — _(no dedicated panel; query ad-hoc)_ | — | — | Rate > 0 when SDEX loop runs |
| `stellarroute_indexer_sse_events_received_total` | Counter | — | — | — | Rate > 0; drops indicate Horizon disconnect |
| `stellarroute_indexer_sse_disconnects_total` | Counter | — | — | — | Near zero; spikes indicate Horizon SSE instability |
| `stellarroute_indexer_horizon_throttle_events_total` | Counter | — | — | — | Near zero; spikes = Horizon 429s |
| `stellarroute_indexer_horizon_throttle_wait_ms_total` | Counter | — | — | — | Near zero |
| `stellarroute_indexer_horizon_consecutive_429s` | Gauge | — | — | — | 0; non-zero = active back-pressure |

### SLO Reference

The table below cross-references each SLO defined in [`config/slo.yaml`](../config/slo.yaml)
with the metric(s) that back it.

| SLO Name | Backing Metrics | Target | Alert |
|---|---|---|---|
| `quote_p95_latency` | `stellarroute_quote_request_duration_seconds` | P95 < 500 ms | `SLOQuoteP95LatencyBurnWarning/Critical` |
| `quote_p99_latency` | `stellarroute_quote_request_duration_seconds` | P99 < 2 s | `SLOQuoteP99LatencyBurnWarning/Critical` |
| `quote_error_rate` | `stellarroute_quote_requests_total` | error rate < 1 % | `SLOQuoteErrorRateBurnWarning/Critical` |
| `route_compute_p95_latency` | `stellarroute_route_compute_duration_seconds` | P95 < 1 s | `SLORouteComputeP95LatencyBurnWarning` |
| `cache_hit_ratio` | `stellarroute_cache_hits_total`, `stellarroute_cache_misses_total` | ≥ 70 % | `SLOCacheHitRatioBurnWarning` |
| `indexer_sync_health` | `stellarroute_indexer_sync_status`, `stellarroute_indexer_lag_ledgers` | status ≥ 0 | `SLOIndexerSyncCritical` |
| `swap_prepare_success_rate` | `stellarroute_swap_prepare_total` | error rate < 1 % | `SLOSwapPrepareFailureRateBurnWarning/Critical` |
| `swap_submit_success_rate` | `stellarroute_swap_submit_total` | error rate < 1 % | `SLOSwapSubmitFailureRateBurnWarning/Critical` |

---

## References

- **SLO definitions**: [`config/slo.yaml`](../config/slo.yaml)
- **Prometheus recording rules**: [`monitoring/prometheus/recording-rules.yaml`](../monitoring/prometheus/recording-rules.yaml)
- **Prometheus alerting rules**: [`monitoring/prometheus/slo-alerts.yml`](../monitoring/prometheus/slo-alerts.yml)
- **Grafana SLO dashboard**: [`monitoring/grafana/slo-dashboard.json`](../monitoring/grafana/slo-dashboard.json)
- **Grafana indexer lag dashboard**: [`monitoring/grafana/indexer-lag-dashboard.json`](../monitoring/grafana/indexer-lag-dashboard.json)
- **Grafana swap + indexer detail dashboard**: [`monitoring/grafana/swap-indexer-panels.json`](../monitoring/grafana/swap-indexer-panels.json)
- **Probe runner script**: [`scripts/slo-probe.sh`](../scripts/slo-probe.sh)
- **CI workflow**: [`.github/workflows/slo-probes.yml`](../.github/workflows/slo-probes.yml)
- **Indexer lag monitoring**: [`docs/indexer-lag-monitoring.md`](indexer-lag-monitoring.md)
- **AMM refresh failures runbook**: [`docs/runbooks/amm-refresh-failures.md`](runbooks/amm-refresh-failures.md)

