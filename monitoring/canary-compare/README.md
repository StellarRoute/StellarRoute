# StellarRoute Canary Live Quote Comparison

Continuously cross-checks StellarRoute quote prices against Stellar Horizon's SDEX order-book API.
Detects when StellarRoute prices diverge from observable on-chain reality — catching routing bugs,
stale indexer data, or liquidity mis-indexing that the internal canary cannot see (because both
internal policies would be equally wrong).

See [`docs/routing_canary.md`](../../docs/routing_canary.md) for the full operational runbook.

---

## How it works

For each invocation the script:

1. Fetches `GET /api/v1/quote/{base}/{quote}?amount={amount}` from StellarRoute.
2. Fetches the Horizon best-ask price via `GET {horizon}/order_book?…`.
3. Computes `divergence_bps = abs(sr_price - ref_price) / ref_price × 10,000`.
4. Emits a structured JSON log line to stdout.
5. POSTs the result to `POST /api/v1/system/canary/live-compare` (fire-and-forget).
6. Exits `1` if consecutive divergence failures reach the configured threshold.

---

## Configuration

All parameters can be set via environment variable or CLI flag (CLI takes precedence).

| Env var | CLI flag | Default | Description |
|---|---|---|---|
| `CANARY_SR_BASE_URL` | `--sr-base-url` | `http://localhost:3000` | StellarRoute API root |
| `CANARY_HORIZON_BASE_URL` | `--horizon-base-url` | `https://horizon.stellar.org` | Horizon root |
| `CANARY_BASE_ASSET` | `--base-asset` | `native` | Selling asset (`native` or `CODE:ISSUER`) |
| `CANARY_QUOTE_ASSET` | `--quote-asset` | `USDC:GA5Z…` | Buying asset (`CODE:ISSUER`) |
| `CANARY_AMOUNT` | `--amount` | `1000.0` | Trade size for the StellarRoute quote |
| `CANARY_TIMEOUT` | `--timeout` | `10.0` | HTTP timeout in seconds |
| `CANARY_DIVERGENCE_THRESHOLD_BPS` | `--divergence-threshold` | `50` | BPS above which a run is `"diverged"` |
| `CANARY_FAILURE_THRESHOLD` | `--failure-threshold` | `3` | Consecutive failures before exit 1 |
| `CANARY_ADMIN_TOKEN` | `--admin-token` | *(required for ingest)* | Bearer token for the ingest endpoint |
| `CANARY_COUNT_ERRORS_AS_FAILURES` | `--count-errors-as-failures` | `false` | Count HTTP errors toward failure count |

---

## Running locally

```bash
# Against a local dev API (no admin token needed for dev)
python3 monitoring/canary-compare/canary_compare.py \
  --sr-base-url http://localhost:3000 \
  --verbose

# With a specific pair and amount
python3 monitoring/canary-compare/canary_compare.py \
  --base-asset native \
  --quote-asset "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN" \
  --amount 500.0 \
  --verbose
```

## Running against production

```bash
export CANARY_SR_BASE_URL="https://api.stellarroute.io"
export CANARY_ADMIN_TOKEN="your-admin-token"
python3 monitoring/canary-compare/canary_compare.py
```

---

## Exit codes

| Code | Meaning |
|---|---|
| `0` | All checks passed (divergence within threshold) |
| `1` | Sustained divergence detected — consecutive failure threshold reached |

---

## Prometheus alerts

Alert rules live in [`monitoring/prometheus/canary-alerts.yml`](../prometheus/canary-alerts.yml):

- `CanaryQuoteDivergenceWarning` — divergence > 50 bps for 5 minutes
- `CanaryQuoteDivergenceCritical` — divergence > 200 bps for 2 minutes

Metrics pushed to the API are scraped by Prometheus at `/metrics`:

- `stellarroute_canary_quote_divergence_bps{pair}` — latest divergence gauge
- `stellarroute_canary_comparison_total{pair,outcome}` — cumulative outcome counter
