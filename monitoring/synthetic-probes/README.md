# StellarRoute Synthetic Monitoring Probes

Synthetic monitoring probes for auditing the status, performance, and schema structure of critical StellarRoute API endpoints.

These probes simulate real-world request patterns externally to ensure service health, check response latencies against specific budgets, and validate contract schema shape and type safety.

---

## Architecture and Design

- **Zero External Dependencies**: Implemented in raw Python 3 using built-in standard libraries (`urllib`, `json`, `argparse`). Designed to run anywhere instantly without needing `pip install` or environment virtualization.
- **Envelope & Contract Checks**: Goes beyond standard HTTP `200` assertions. The probe decodes the API payload and validates that fields match expected types and exact data structure contracts (such as checking `ApiResponse` v1 envelope, `QuoteResponse` pricing representation, and multi-hop paths arrays).
- **Runbook Injection**: If a check fails due to an HTTP failure, slow response, or distorted schema payload, the runner logs a diagnostic JSON payload indicating the failure reason and injects a dedicated **Runbook Link** for the on-call engineer.
- **Exit Status**: Exits with code `0` on success and `1` on failure, integrating smoothly with cron-like agents, health routers, Kubernetes probe handlers, or GitHub Actions.

---

## Configuration

Probes are configured dynamically using **Environment Variables** or **Command-Line Arguments** (CLI arguments take precedence).

| Configuration | CLI Argument | Env Variable | Default | Description |
|---|---|---|---|---|
| **Target URL** | `--target-url` | `TARGET_URL` | `http://127.0.0.1:3000` | Target API server base url. |
| **Timeout** | `--timeout` | `TIMEOUT_SECONDS` | `5.0` | Connection and read timeout in seconds. |
| **Health Threshold** | `--health-threshold` | `HEALTH_THRESHOLD_MS` | `200.0` | Max latency allowed for `/health` (in ms). |
| **Quote Threshold** | `--quote-threshold` | `QUOTE_THRESHOLD_MS` | `500.0` | Max latency allowed for `/quote` (in ms). |
| **Routes Threshold** | `--routes-threshold` | `ROUTES_THRESHOLD_MS` | `800.0` | Max latency allowed for `/routes` (in ms). |
| **Base Asset** | `--base-asset` | `PROBE_BASE_ASSET` | `native` | Base asset identifier for trading. |
| **Quote Asset** | `--quote-asset` | `PROBE_QUOTE_ASSET` | `USDC` | Quote asset identifier for trading. |
| **Trade Amount** | `--amount` | `PROBE_AMOUNT` | `1.0` | Transaction trade volume for quotes/routing. |

---

## How to Run

### 1. View Usage Instructions
```bash
python monitoring/synthetic-probes/probe_runner.py --help
```

### 2. Run All Checks (Local Development)
Ensure your API server is running locally (e.g. `cargo run -p stellarroute-api` on port `3000`).
```bash
python monitoring/synthetic-probes/probe_runner.py --verbose
```

### 3. Run a Single Endpoint Check
To check `/health` alone:
```bash
python monitoring/synthetic-probes/probe_runner.py --endpoint health --verbose
```

To check `/api/v1/quote`:
```bash
python monitoring/synthetic-probes/probe_runner.py --endpoint quote --verbose
```

To check `/api/v1/routes`:
```bash
python monitoring/synthetic-probes/probe_runner.py --endpoint routes --verbose
```

### 4. Overriding Target and Parameters (Local or Production)
```bash
python monitoring/synthetic-probes/probe_runner.py \
  --target-url "https://api.stellarroute.com" \
  --base-asset "native" \
  --quote-asset "EURC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5" \
  --amount "100.0" \
  --verbose
```

---

## Log Output Formats

### Successful Check (JSON StdOut)
```json
{"timestamp": "2026-05-27T14:10:00.000000Z", "endpoint": "health", "success": true, "latency_ms": 12.45}
{"timestamp": "2026-05-27T14:10:00.012543Z", "endpoint": "quote", "success": true, "latency_ms": 145.21}
{"timestamp": "2026-05-27T14:10:00.187652Z", "endpoint": "routes", "success": true, "latency_ms": 284.10}
```

### Failed Check with Runbook Reference
If the database or engine degrades, the failure maps to the appropriate on-call runbook:
```json
{
  "timestamp": "2026-05-27T14:10:00.000000Z",
  "endpoint": "health",
  "success": false,
  "latency_ms": 25.10,
  "error": "System health is degraded: status='unhealthy'",
  "runbook_url": "https://links.internal/runbooks/m5-health-failure"
}
```

If a performance threshold budget is breached (e.g. `/api/v1/routes` takes longer than 800ms):
```json
{
  "timestamp": "2026-05-27T14:10:00.280145Z",
  "endpoint": "routes",
  "success": false,
  "latency_ms": 845.22,
  "error": "Performance degradation: latency 845.22ms exceeded threshold of 800.0ms",
  "runbook_url": "https://links.internal/runbooks/m5-routes-failure"
}
```
