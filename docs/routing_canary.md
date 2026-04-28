# Routing Canary Validation Pipeline

The Canary Validation Pipeline allows operators to safely test new routing algorithms and policies in production alongside the existing baseline logic. It evaluates the "candidate" policy asynchronously, avoiding user-facing latency, while collecting side-by-side diagnostics on latency and route output quality (slippage, hops, price).

## Features
- **Zero Impact on Production Requests:** Canary evaluation is offloaded to background threads.
- **Side-by-side Evaluation:** Direct comparison of same-request metrics.
- **Automatic Rollback:** The pipeline automatically disables itself if continuous drift violations occur.
- **Configurable Thresholds:** Operators can configure sampling rates and allowable latency/quality drift.

## How it Works
1. A user requests a trade route (e.g., via `/api/v1/routes/:base/:quote`).
2. The primary `production` policy evaluates the route and returns it to the user.
3. If canary mode is enabled, the pipeline pseudo-randomly samples a subset of requests based on the `evaluation_rate`.
4. The background task executes the `candidate_policy` with the exact same liquidity graph snapshot.
5. A `CanaryEvaluation` is recorded with latency and output drift metrics.
6. The `CanaryEvaluator` detects violations if drift thresholds are exceeded.
7. The evaluation is saved into an in-memory history buffer (up to 1,000 evaluations).

## Operator API

### 1. View Canary Report
Fetch the current pipeline configuration and recent evaluations.

```bash
curl -X GET http://localhost:3000/api/v1/system/canary/report
```

**Response includes:**
- `config`: Current thresholds and policy strings.
- `total_evaluations`: Number of cached evaluation metrics.
- `recent_evaluations`: List of `CanaryEvaluation` DTOs (timestamp, drift metrics, violation reasons).

### 2. Configure Canary Pipeline
Enable/disable the pipeline or adjust thresholds.

```bash
curl -X POST http://localhost:3000/api/v1/system/canary/config \
  -H "Content-Type: application/json" \
  -d '{
    "enabled": true,
    "baseline_policy": "production",
    "candidate_policy": "testing",
    "max_latency_drift_ms": 50,
    "max_output_drift_bps": 10,
    "rollback_trigger_threshold": 5,
    "evaluation_rate": 0.25
  }'
```

### Configuration Fields
| Field | Type | Description |
|---|---|---|
| `enabled` | boolean | Toggle the pipeline on/off. |
| `baseline_policy` | string | The existing policy (default: `production`). |
| `candidate_policy` | string | The new policy to evaluate (e.g., `testing`). |
| `max_latency_drift_ms` | integer | Max allowed additional latency in ms. |
| `max_output_drift_bps` | integer | Max allowed output loss in basis points. |
| `rollback_trigger_threshold` | integer | Consecutive violations before auto-disable. |
| `evaluation_rate` | float | 0.0 to 1.0 (0% to 100% of requests sampled). |

## Emergency Rollback

If you detect severe anomalies in the candidate policy, you can instantly turn off the canary pipeline by sending:

```bash
curl -X POST http://localhost:3000/api/v1/system/canary/config \
  -H "Content-Type: application/json" \
  -d '{
    "enabled": false,
    "baseline_policy": "production",
    "candidate_policy": "testing",
    "max_latency_drift_ms": 50,
    "max_output_drift_bps": 10,
    "rollback_trigger_threshold": 5,
    "evaluation_rate": 0.1
  }'
```

*(Note: The system automatically triggers this same shutdown if `rollback_trigger_threshold` consecutive violations occur).*
