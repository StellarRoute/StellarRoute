# Routing Canary Validation Pipeline

The Canary Validation Pipeline allows operators to safely test new routing algorithms and policies in production alongside the existing baseline logic. It evaluates the "candidate" policy asynchronously, avoiding user-facing latency, while collecting side-by-side diagnostics on latency and route output quality (slippage, hops, price).

## Canary Subsystems

StellarRoute has two complementary canary subsystems. The internal routing canary compares two
routing policies against each other. The live quote comparison job checks whether StellarRoute
prices match observable on-chain reality — something the internal canary cannot detect, because
a systematic bug would affect both policies equally.

| Subsystem | Purpose | Endpoints | Prometheus metrics |
|---|---|---|---|
| Internal routing canary | Compare candidate routing policy vs production baseline | `GET /api/v1/system/canary/report`, `POST /api/v1/system/canary/config` | *(none — tracked via in-memory VecDeque only)* |
| Live quote comparison | Cross-check StellarRoute prices vs Horizon SDEX reference | `POST /api/v1/system/canary/live-compare`, `GET /api/v1/system/canary/live-compare/report` | `stellarroute_canary_quote_divergence_bps`, `stellarroute_canary_comparison_total` |

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

## Authentication (issue #1055)

Both endpoints require the admin token (`ADMIN_AUTH_TOKEN`), sent as either
the `x-admin-token` header or `Authorization: Bearer <token>`:

| Method | Dev/test default | Production default |
|---|---|---|
| `GET /api/v1/system/canary/report` | Public — no token required | Requires `ADMIN_AUTH_TOKEN` |
| `POST /api/v1/system/canary/config` | Requires `ADMIN_AUTH_TOKEN` | Requires `ADMIN_AUTH_TOKEN` |

`GET` is left public in dev/test so the canary report can be inspected
locally without configuring a token, but is gated the same as `POST`
whenever `STELLARROUTE_ENV=production` — the pipeline's config and drift
history are operationally sensitive (they reveal live routing-trust
signals), so production prefers auth over open access. See
[`docs/api/production-exposure.md`](api/production-exposure.md) for the
full inventory alongside the kill switch and metrics/replay surfaces, which
share the same guard.

Requests without a valid token receive `401 Unauthorized`. If
`STELLARROUTE_ENV=production` and `ADMIN_AUTH_TOKEN` is unset, the API
refuses to start entirely rather than boot with these routes silently
denying every request.

## Operator API

### 1. View Canary Report
Fetch the current pipeline configuration and recent evaluations.

```bash
# Dev/test (no token needed)
curl -X GET http://localhost:3000/api/v1/system/canary/report

# Production
curl -X GET http://localhost:3000/api/v1/system/canary/report \
  -H "x-admin-token: $ADMIN_AUTH_TOKEN"
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
  -H "x-admin-token: $ADMIN_AUTH_TOKEN" \
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
  -H "x-admin-token: $ADMIN_AUTH_TOKEN" \
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

---

## Live Quote Comparison Job

The live quote comparison job (`monitoring/canary-compare/canary_compare.py`) continuously
validates that StellarRoute quote prices are grounded in observable market reality by comparing
them against Stellar Horizon's public SDEX order-book API.

### What it does

1. Fetches `GET /api/v1/quote/{base}/{quote}?amount={amount}` from StellarRoute.
2. Fetches the best-ask price from `GET https://horizon.stellar.org/order_book?…`.
3. Computes `divergence_bps = abs(sr_price - ref_price) / ref_price × 10,000`.
4. Logs a structured JSON result to stdout.
5. POSTs the result to `POST /api/v1/system/canary/live-compare` so Prometheus metrics
   and the history buffer are updated.
6. Exits `1` if consecutive divergence failures reach the configured threshold.

The script runs every 5 minutes via `.github/workflows/canary-compare.yml`.

### Configuration

All parameters can be set via environment variable or CLI flag (CLI takes precedence).

| Env var | CLI flag | Default | Description |
|---|---|---|---|
| `CANARY_SR_BASE_URL` | `--sr-base-url` | `http://localhost:3000` | StellarRoute API root |
| `CANARY_HORIZON_BASE_URL` | `--horizon-base-url` | `https://horizon.stellar.org` | Horizon root |
| `CANARY_BASE_ASSET` | `--base-asset` | `native` | Selling asset |
| `CANARY_QUOTE_ASSET` | `--quote-asset` | `USDC:GA5Z…` | Buying asset (`CODE:ISSUER`) |
| `CANARY_AMOUNT` | `--amount` | `1000.0` | Trade size for StellarRoute quote |
| `CANARY_TIMEOUT` | `--timeout` | `10.0` | HTTP timeout in seconds |
| `CANARY_DIVERGENCE_THRESHOLD_BPS` | `--divergence-threshold` | `50` | BPS above which a run is `"diverged"` |
| `CANARY_FAILURE_THRESHOLD` | `--failure-threshold` | `3` | Consecutive failures before exit 1 |
| `CANARY_ADMIN_TOKEN` | `--admin-token` | *(required)* | Bearer token for the ingest endpoint |
| `CANARY_COUNT_ERRORS_AS_FAILURES` | `--count-errors-as-failures` | `false` | Count HTTP errors toward failure count |

### Divergence thresholds

| Level | Default | `for:` duration | When to adjust |
|---|---|---|---|
| Warning | 50 bps (0.5%) | 5 minutes | Lower to 20–30 bps for stable stablecoin pairs |
| Critical | 200 bps (2.0%) | 2 minutes | Raise for volatile assets where wider spreads are expected |

The consecutive failure threshold (default 3) controls how many sequential runs must diverge
before the script exits non-zero. Keep at 3 for the scheduled workflow; set to 1 for manual
one-shot checks.

### Manual invocation

```bash
# Against a local dev API
python3 monitoring/canary-compare/canary_compare.py \
  --sr-base-url http://localhost:3000 \
  --verbose

# Against production
export CANARY_SR_BASE_URL="https://api.stellarroute.io"
export CANARY_ADMIN_TOKEN="your-admin-token"
python3 monitoring/canary-compare/canary_compare.py --verbose
```

### Alert runbook

#### CanaryQuoteDivergenceWarning (> 50 bps for 5 min)

1. **Inspect recent comparison history:**
   ```bash
   curl -H "x-admin-token: $ADMIN_AUTH_TOKEN" \
     https://api.stellarroute.io/api/v1/system/canary/live-compare/report
   ```
   Check `divergence_bps` and `outcome` across recent entries to see when divergence started.

2. **Check indexer sync status:**
   Look at `stellarroute_indexer_sync_status` in Grafana. A `warning` or `critical` value
   means stale market data is the likely cause — the indexer is falling behind Horizon.

3. **Check the Horizon order book directly:**
   ```bash
   curl "https://horizon.stellar.org/order_book?selling_asset_type=native&buying_asset_type=credit_alphanum4&buying_asset_code=USDC&buying_asset_issuer=GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN&limit=5"
   ```
   Compare `asks[0].price` to what `GET /api/v1/quote/native/USDC?amount=1000` returns.

4. **If indexer is current and divergence persists:** escalate to the routing team — likely a
   routing bug or pool mis-indexing.

#### CanaryQuoteDivergenceCritical (> 200 bps for 2 min)

Follow the same steps as the warning runbook above, then:

5. **Consider activating the kill switch** for affected pairs if users could be materially
   mis-priced. See `docs/RUNBOOK_KILL_SWITCH.md`.

6. **Page the on-call routing engineer** immediately. A 2% deviation sustained for 2 minutes
   is a high-confidence signal of a systemic issue.
