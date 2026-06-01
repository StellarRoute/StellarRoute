# Indexer Lag Monitoring

This document covers the lag metrics emitted by StellarRoute for tracking
Horizon sync drift, the threshold-based warning system, and ready-to-use
Grafana/Prometheus dashboard snippets.

---

## Overview

The `IndexerLagMonitor` runs as a background task in the API process.  Every
**30 seconds** it:

1. Fetches the current latest ledger sequence from Horizon (`GET /ledgers?order=desc&limit=1`).
2. Queries the local database for the most recently indexed ledger for each source:
   - **SDEX**: `MAX(last_modified_ledger)` from `sdex_offers`
   - **AMM**: `last_seen_ledger` from `soroban_sync_cursors` (job `soroban_pool_discovery`)
3. Computes `lag_ledgers = horizon_ledger - local_ledger` and
   `lag_seconds ≈ lag_ledgers × 5` (Stellar targets ~5 s/ledger).
4. Classifies the lag into `ok` / `warning` / `critical` and updates Prometheus gauges.

---

## Prometheus Metrics

| Metric name                                  | Type    | Labels         | Description                                                  |
|----------------------------------------------|---------|----------------|--------------------------------------------------------------|
| `stellarroute_indexer_lag_ledgers`           | Gauge   | `source`       | Ledger count behind Horizon (`sdex` or `amm`)                |
| `stellarroute_indexer_lag_seconds`           | Gauge   | `source`       | Estimated wall-clock lag in seconds                          |
| `stellarroute_indexer_last_indexed_ledger`   | Gauge   | `source`       | Most recently indexed ledger sequence number                 |
| `stellarroute_indexer_horizon_ledger`        | Gauge   | `instance`     | Current Horizon latest ledger (cached from last measurement) |
| `stellarroute_indexer_sync_status`           | Gauge   | `source`       | 1 = ok, 0 = warning, -1 = critical, -2 = unknown             |

All metrics are exposed at `GET /metrics` in Prometheus text format.

---

## Threshold-Based Warning Levels

| Level      | Lag (ledgers) | Lag (seconds) | Prometheus value | Action                                      |
|------------|---------------|---------------|------------------|---------------------------------------------|
| `ok`       | < 10          | < 50 s        | 1                | Normal operation                            |
| `warning`  | 10 – 60       | 50 – 300 s    | 0                | Log warning; alert if sustained > 5 min     |
| `critical` | > 60          | > 300 s       | -1               | Log error; page on-call                     |
| `unknown`  | —             | —             | -2               | Horizon unreachable or no data yet          |

Thresholds are configurable via `LagThresholds` in `crates/api/src/indexer_lag.rs`.

---

## Health Endpoint

The `/health` and `/health/deps` endpoints include per-source lag components:

```json
GET /health
{
  "data": {
    "status": "healthy",
    "components": {
      "database": "healthy",
      "redis": "healthy",
      "indexer_lag_sdex": "healthy",
      "indexer_lag_amm": "warning (lag: 15 ledgers)"
    }
  }
}
```

```json
GET /health/deps
{
  "data": {
    "status": "ok",
    "components": {
      "database": "healthy",
      "redis": "healthy",
      "indexer_lag_sdex": "ok (lag: 3 ledgers)",
      "indexer_lag_amm": "ok (lag: 7 ledgers)"
    }
  }
}
```

**HTTP status codes:**
- `warning` lag → HTTP 200 (soft signal, does not flip overall status)
- `critical` lag → HTTP 503 (hard failure, flips `all_healthy = false`)

---

## PromQL Query Examples

### Current lag in ledgers

```promql
stellarroute_indexer_lag_ledgers
```

### Lag in seconds by source

```promql
stellarroute_indexer_lag_seconds{source="sdex"}
stellarroute_indexer_lag_seconds{source="amm"}
```

### Sync status (1=ok, 0=warning, -1=critical)

```promql
stellarroute_indexer_sync_status
```

### Alert: lag exceeds warning threshold for 5 minutes

```promql
stellarroute_indexer_lag_ledgers > 10
```

### Alert: lag exceeds critical threshold

```promql
stellarroute_indexer_lag_ledgers > 60
```

### Rate of ledger indexing (ledgers/second over 5m window)

```promql
rate(stellarroute_indexer_last_indexed_ledger[5m])
```

### How far behind Horizon (percentage)

```promql
(stellarroute_indexer_horizon_ledger{instance="default"} - stellarroute_indexer_last_indexed_ledger)
  / stellarroute_indexer_horizon_ledger{instance="default"} * 100
```

---

## Grafana Dashboard Snippet

Paste this JSON into a Grafana dashboard panel (Time series):

```json
{
  "title": "Indexer Lag (ledgers)",
  "type": "timeseries",
  "targets": [
    {
      "expr": "stellarroute_indexer_lag_ledgers{source=\"sdex\"}",
      "legendFormat": "SDEX lag"
    },
    {
      "expr": "stellarroute_indexer_lag_ledgers{source=\"amm\"}",
      "legendFormat": "AMM lag"
    }
  ],
  "thresholds": {
    "mode": "absolute",
    "steps": [
      { "color": "green", "value": null },
      { "color": "yellow", "value": 10 },
      { "color": "red",    "value": 60 }
    ]
  },
  "fieldConfig": {
    "defaults": {
      "unit": "short",
      "custom": { "lineWidth": 2 }
    }
  }
}
```

Stat panel for sync status:

```json
{
  "title": "Indexer Sync Status",
  "type": "stat",
  "targets": [
    {
      "expr": "stellarroute_indexer_sync_status",
      "legendFormat": "{{source}}"
    }
  ],
  "fieldConfig": {
    "defaults": {
      "mappings": [
        { "type": "value", "value": "1",  "text": "OK",       "color": "green"  },
        { "type": "value", "value": "0",  "text": "WARNING",  "color": "yellow" },
        { "type": "value", "value": "-1", "text": "CRITICAL", "color": "red"    },
        { "type": "value", "value": "-2", "text": "UNKNOWN",  "color": "gray"   }
      ]
    }
  }
}
```

---

## Alertmanager Rules

Add to your `prometheus/rules/stellarroute.yml`:

```yaml
groups:
  - name: stellarroute_indexer_lag
    rules:

      # Warning: lag elevated for 5 minutes
      - alert: IndexerLagWarning
        expr: stellarroute_indexer_lag_ledgers > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "StellarRoute indexer lag elevated ({{ $labels.source }})"
          description: >
            The {{ $labels.source }} indexer is {{ $value }} ledgers behind Horizon.
            Estimated lag: {{ with query "stellarroute_indexer_lag_seconds" }}
            {{ . | first | value | humanizeDuration }}{{ end }}.
            Threshold: 10 ledgers (50 s).

      # Critical: lag exceeds 60 ledgers (5 minutes)
      - alert: IndexerLagCritical
        expr: stellarroute_indexer_lag_ledgers > 60
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "StellarRoute indexer lag CRITICAL ({{ $labels.source }})"
          description: >
            The {{ $labels.source }} indexer is {{ $value }} ledgers behind Horizon
            (≥ 5 minutes of data loss). Immediate investigation required.

      # Stall: indexer has not advanced in 10 minutes
      - alert: IndexerStalled
        expr: increase(stellarroute_indexer_last_indexed_ledger[10m]) == 0
        for: 10m
        labels:
          severity: critical
        annotations:
          summary: "StellarRoute indexer stalled ({{ $labels.source }})"
          description: >
            The {{ $labels.source }} indexer has not advanced its ledger cursor
            in the last 10 minutes. The indexer process may have crashed or
            lost connectivity to Horizon.

      # Unknown: Horizon unreachable
      - alert: IndexerHorizonUnreachable
        expr: stellarroute_indexer_sync_status == -2
        for: 3m
        labels:
          severity: warning
        annotations:
          summary: "StellarRoute cannot reach Horizon for lag measurement"
          description: >
            The lag monitor cannot fetch the latest ledger from Horizon.
            Lag metrics are unavailable. Check STELLAR_HORIZON_URL connectivity.
```

---

## Configuration

| Environment variable      | Default                          | Description                                    |
|---------------------------|----------------------------------|------------------------------------------------|
| `STELLAR_HORIZON_URL`     | `https://horizon.stellar.org`    | Horizon base URL for lag measurement           |

Thresholds can be adjusted in code via `LagThresholds`:

```rust
let thresholds = LagThresholds {
    warning_ledgers: 10,   // default
    critical_ledgers: 60,  // default
};
let monitor = IndexerLagMonitor::new(db, horizon_url, thresholds);
```

The polling interval is set in `AppState::new_with_policy`:

```rust
indexer_lag.clone().start_polling(Duration::from_secs(30)); // default: 30s
```

---

## Storage Cost

Each measurement cycle writes 5 Prometheus gauge values × 2 sources = 10 time-series
data points.  At 30-second intervals this is negligible (< 1 KB/day in TSDB).
