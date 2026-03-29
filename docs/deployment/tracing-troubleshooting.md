# Distributed Tracing Troubleshooting Guide

This document provides guidance for troubleshooting issues using StellarRoute's distributed tracing capabilities.

## Overview

StellarRoute implements end-to-end request tracing using OpenTelemetry, propagating trace context across:
- API layer (HTTP requests)
- Cache operations (Redis lookups/stores)
- Router/Pathfinder (route search and optimization)
- Indexer (data ingestion)

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `OTEL_EXPORTER_OTLP_ENDPOINT` | OTLP collector endpoint (e.g., `http://localhost:4317`) | (disabled) |
| `OTEL_SERVICE_NAME` | Service name for spans | `stellarroute` / `stellarroute-indexer` |
| `OTEL_SAMPLING_RATIO` | Sampling ratio (0.0 to 1.0) | `1.0` |
| `RUST_LOG` | Log level filter | `info` |
| `LOG_FORMAT` | Output format (`json` or `pretty`) | `pretty` |

### Enabling Distributed Tracing

```bash
# Production with Jaeger
export OTEL_EXPORTER_OTLP_ENDPOINT=http://jaeger:4317
export OTEL_SERVICE_NAME=stellarroute-api
export OTEL_SAMPLING_RATIO=0.1
export LOG_FORMAT=json
```

### Sampling Strategies

| Environment | Recommended Ratio | Notes |
|-------------|-------------------|-------|
| Development | `1.0` | Capture all traces |
| Staging | `0.5` | Balance coverage and cost |
| Production | `0.01` - `0.1` | Reduce overhead; increase for debugging |

## Trace Structure

### Span Hierarchy

```
http.request (API)
  └── quote.request
       ├── cache.lookup
       ├── route.search (Router)
       │    └── find_paths
       ├── route.optimize
       │    └── find_optimal_routes
       └── cache.store
```

### Key Span Attributes

| Span | Attributes |
|------|------------|
| `http.request` | `http.method`, `http.target`, `http.status_code` |
| `cache.lookup` | `cache.hit`, `key` |
| `route.search` | `route.from`, `route.to`, `route.paths_found` |
| `route.optimize` | `route.paths_evaluated`, `route.compute_time_ms` |

## Common Issues

### Issue: Traces Not Appearing in Collector

1. Verify OTLP endpoint is reachable:
   ```bash
   curl -v http://collector:4317/v1/traces
   ```

2. Check service logs for connection errors:
   ```bash
   RUST_LOG=opentelemetry=debug ./stellarroute-api
   ```

3. Ensure sampling ratio is not `0.0`:
   ```bash
   echo $OTEL_SAMPLING_RATIO
   ```

### Issue: Broken Trace Context (Missing Parent Spans)

1. Verify `traceparent` header is being forwarded between services
2. Check that all services use `TraceContextPropagator`
3. Look for spans with `otel.kind=server` that should link to client spans

### Issue: High Latency in Trace Export

1. Switch to batch export (default) instead of simple export
2. Reduce sampling ratio in production
3. Use async export to avoid blocking request threads

### Issue: Missing Spans in Trace

1. Verify instrumentation is applied:
   ```bash
   RUST_LOG=stellarroute_api=trace ./stellarroute-api
   ```

2. Check span filters aren't dropping spans
3. Ensure async operations use `.instrument()` for context propagation

## Investigating Slow Requests

1. Filter by high latency:
   - In Jaeger: `minDuration > 500ms`
   - In logs: search for `latency_ms > 500`

2. Identify bottleneck span:
   ```
   Look for spans with duration >> children duration sum
   ```

3. Common bottlenecks:
   - `cache.lookup` miss leading to computation
   - `route.search` with many edges
   - Database queries (`db.query` spans)

## Investigating Failed Requests

1. Search for error spans:
   - `error=true` attribute
   - `error_class` field in logs

2. Error categories:
   | error_class | Meaning |
   |-------------|---------|
   | `validation` | Invalid request parameters |
   | `not_found` | No route found |
   | `stale_market_data` | Market data too old |
   | `internal` | Unexpected server error |

3. Correlate with request_id:
   ```bash
   grep "request_id=abc-123" /var/log/stellarroute/*.log
   ```

## Metrics from Traces

Key metrics derivable from trace data:

- **P50/P95/P99 latency** by endpoint
- **Cache hit ratio** from `cache.hit` attribute
- **Route computation time** from `route.compute_time_ms`
- **Error rate** by `error_class`

## Integration with Monitoring Tools

### Jaeger

```yaml
# docker-compose.yml
services:
  jaeger:
    image: jaegertracing/all-in-one:1.50
    ports:
      - "16686:16686"  # UI
      - "4317:4317"    # OTLP gRPC
```

### Grafana Tempo

```yaml
services:
  tempo:
    image: grafana/tempo:latest
    ports:
      - "4317:4317"
```

## Log Correlation

Trace context is automatically included in structured logs when `LOG_FORMAT=json`:

```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "level": "INFO",
  "message": "Quote pipeline completed",
  "trace_id": "abc123...",
  "span_id": "def456...",
  "request_id": "uuid...",
  "latency_ms": 45,
  "cache_hit": true
}
```

Query logs by trace ID:
```bash
jq 'select(.trace_id == "abc123...")' /var/log/stellarroute/api.json
```
