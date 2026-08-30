# Design Document — Routing Canary: Live Quote Comparison

## Overview

This feature adds an external, continuously-scheduled pipeline that cross-checks StellarRoute quote
prices against Stellar Horizon's public SDEX order-book API. It does not replace the existing
internal canary (which compares two routing policies against each other) — it adds a separate
subsystem with a different purpose: detecting when StellarRoute prices diverge from observable
on-chain reality.

The design is deliberately minimal: no new database tables, no new background threads inside the
API process, and no new Rust dependencies. The comparison logic lives in a zero-dependency Python
script that runs externally (via GitHub Actions cron) and pushes results back into the API's
existing Prometheus metrics and in-memory history buffer via a new admin endpoint.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│  GitHub Actions (every 5 min)                                       │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  canary_compare.py                                          │   │
│  │                                                             │   │
│  │  1. GET /api/v1/quote/{base}/{quote}?amount=X  ──────────►  │   │
│  │     StellarRoute API  ◄──── sr_price                        │   │
│  │                                                             │   │
│  │  2. GET horizon.stellar.org/order_book?…  ───────────────►  │   │
│  │     Horizon API  ◄──── ref_price (best ask)                 │   │
│  │                                                             │   │
│  │  3. divergence_bps = abs(sr - ref) / ref * 10_000           │   │
│  │                                                             │   │
│  │  4. POST /api/v1/system/canary/live-compare  ────────────►  │   │
│  │     { pair, sr_price, ref_price, divergence_bps, outcome }  │   │
│  │                                                             │   │
│  │  5. stdout: structured JSON log line                        │   │
│  │  6. exit 0 (ok) or exit 1 (sustained divergence)           │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  StellarRoute API process                                           │
│                                                                     │
│  POST /api/v1/system/canary/live-compare  (AdminAuth)               │
│    ├── validates Live_Compare_Result schema                         │
│    ├── updates CANARY_QUOTE_DIVERGENCE_BPS gauge  (Prometheus)      │
│    ├── increments CANARY_COMPARISON_TOTAL counter (Prometheus)      │
│    └── appends to live_compare_history: VecDeque (capped at 1,000) │
│                                                                     │
│  GET /api/v1/system/canary/live-compare/report  (AdminAuth)         │
│    └── returns live_compare_history newest-first + total_entries    │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  Prometheus scrapes /metrics                                        │
│    stellarroute_canary_quote_divergence_bps{pair="XLM/USDC"}       │
│    stellarroute_canary_comparison_total{pair,outcome}               │
│                                                                     │
│  Alert rules (monitoring/prometheus/canary-alerts.yml)              │
│    CanaryQuoteDivergenceWarning  >50 bps for 5 min                  │
│    CanaryQuoteDivergenceCritical >200 bps for 2 min                 │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Components

### 1. Python comparison script — `monitoring/canary-compare/canary_compare.py`

Follows the zero-dependency pattern of `monitoring/synthetic-probes/probe_runner.py` exactly:
`urllib` for HTTP, `json`/`argparse`/`os`/`sys`/`datetime` from the standard library only.

**Configuration** (env var → CLI flag → default):

| Env var | CLI flag | Default | Notes |
|---|---|---|---|
| `CANARY_SR_BASE_URL` | `--sr-base-url` | `http://localhost:3000` | StellarRoute API root |
| `CANARY_HORIZON_BASE_URL` | `--horizon-base-url` | `https://horizon.stellar.org` | Horizon root |
| `CANARY_BASE_ASSET` | `--base-asset` | `native` | Selling asset (XLM by default) |
| `CANARY_QUOTE_ASSET` | `--quote-asset` | `USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN` | `CODE:ISSUER` or native |
| `CANARY_AMOUNT` | `--amount` | `1000.0` | Trade size for quote fetch |
| `CANARY_TIMEOUT` | `--timeout` | `10.0` | HTTP timeout in seconds |
| `CANARY_DIVERGENCE_THRESHOLD_BPS` | `--divergence-threshold` | `50` | bps above which a run is `"diverged"` |
| `CANARY_FAILURE_THRESHOLD` | `--failure-threshold` | `3` | consecutive failures before exit 1 |
| `CANARY_ADMIN_TOKEN` | `--admin-token` | *(required)* | Bearer token for ingest endpoint |
| `CANARY_COUNT_ERRORS_AS_FAILURES` | `--count-errors-as-failures` | `false` | whether HTTP errors count toward failure threshold |

**Control flow per run:**

```
fetch_sr_quote()      → sr_price (str) or OutcomeError
fetch_horizon_price() → ref_price (str) or OutcomeError

if either is OutcomeError:
    outcome = "error"
    divergence_bps = None
elif abs(float(sr_price) - float(ref_price)) / float(ref_price) * 10_000 > threshold:
    outcome = "diverged"
    divergence_bps = <computed>
else:
    outcome = "ok"
    divergence_bps = <computed>

emit_log(outcome, divergence_bps, consecutive_failures)
post_result_to_api(...)   # fire-and-forget; failure is logged but doesn't change exit code

if outcome == "diverged":
    consecutive_failures += 1
else:
    consecutive_failures = 0   # errors only count if count_errors_as_failures=true

if consecutive_failures >= failure_threshold:
    emit_alert_log(runbook_url)
    sys.exit(1)
```

**Horizon order-book URL construction** for a `CODE:ISSUER` quote asset against native XLM:

```
{horizon_base}/order_book
  ?selling_asset_type=native
  &buying_asset_type=credit_alphanum4   (or credit_alphanum12 for codes >4 chars)
  &buying_asset_code={CODE}
  &buying_asset_issuer={ISSUER}
  &limit=5
```

For a native/native pair (not valid in practice; guarded at startup with a config error).

**`fetch_horizon_price()` — best-ask extraction:**

```python
data = json.loads(response_body)
asks = data.get("asks", [])
if not asks:
    return OutcomeError("order_book_empty")
ref_price = asks[0]["price"]          # string decimal, e.g. "0.1234567"
```

**Structured JSON log line schema** (stdout, one line per run):

```json
{
  "timestamp": "2026-07-28T12:00:00.000000Z",
  "pair": "native/USDC:GA5Z...",
  "stellarroute_price": "0.1234567",
  "reference_price": "0.1230000",
  "divergence_bps": 3.79,
  "outcome": "ok",
  "consecutive_failures": 0
}
```

Alert log (emitted before exit 1, in addition to the regular log line):

```json
{
  "timestamp": "...",
  "alert": true,
  "message": "Sustained divergence: 3 consecutive failures exceeded threshold of 3",
  "pair": "...",
  "runbook_url": "https://links.internal/runbooks/canary-divergence"
}
```

---

### 2. API changes — `crates/api/`

#### 2a. New data types — `crates/api/src/models/response.rs`

```rust
/// Outcome of a single external canary comparison run.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LiveCompareOutcome {
    Ok,
    Diverged,
    Error,
}

/// Payload pushed by the external canary script after each comparison run.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LiveCompareResult {
    /// Canonical pair string, e.g. "native/USDC:GA5Z…"
    pub pair: String,
    /// StellarRoute quote price (decimal string)
    pub stellarroute_price: String,
    /// Horizon best-ask price (decimal string; empty string when outcome is "error")
    pub reference_price: String,
    /// Absolute divergence in basis points; 0.0 when outcome is "error"
    pub divergence_bps: f64,
    /// Machine-readable outcome
    pub outcome: LiveCompareOutcome,
    /// ISO 8601 UTC timestamp of when the comparison was performed
    pub timestamp: String,
}

/// Response body for POST /api/v1/system/canary/live-compare
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LiveCompareIngestResponse {
    pub status: String,
    pub entries: usize,
}

/// Response body for GET /api/v1/system/canary/live-compare/report
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LiveCompareReportResponse {
    pub total_entries: usize,
    pub results: Vec<LiveCompareResult>,
}
```

#### 2b. AppState field — `crates/api/src/state.rs`

Add one field alongside the existing `canary_history`:

```rust
/// Live-compare history buffer: results from the external canary comparison job.
/// Capped at 1,000 entries; newest at the back, oldest evicted from the front.
pub live_compare_history: Arc<tokio::sync::RwLock<std::collections::VecDeque<LiveCompareResult>>>,
```

Initialised in both `new_with_policy` and `with_cache_and_policy`:

```rust
live_compare_history: Arc::new(tokio::sync::RwLock::new(
    std::collections::VecDeque::with_capacity(1000),
)),
```

#### 2c. New Prometheus metrics — `crates/api/src/metrics.rs`

Appended to the existing `lazy_static!` block:

```rust
// ── Canary live-compare metrics ───────────────────────────────────────────────

/// Latest canary quote divergence from Horizon reference price, in basis points.
/// Label: pair (e.g. "native/USDC:GA5Z…")
/// Updated on every POST /api/v1/system/canary/live-compare call.
/// Set to 0 when outcome is "error" (divergence unknown).
pub static ref CANARY_QUOTE_DIVERGENCE_BPS: prometheus::GaugeVec = prometheus::register_gauge_vec!(
    "stellarroute_canary_quote_divergence_bps",
    "Latest canary quote divergence from Horizon reference price in basis points",
    &["pair"]
)
.expect("Can't create CANARY_QUOTE_DIVERGENCE_BPS gauge");

/// Total canary live-comparison runs by outcome.
/// Labels: pair, outcome ("ok" | "diverged" | "error")
pub static ref CANARY_COMPARISON_TOTAL: IntCounterVec = register_int_counter_vec!(
    "stellarroute_canary_comparison_total",
    "Total canary live-comparison runs by outcome",
    &["pair", "outcome"]
)
.expect("Can't create CANARY_COMPARISON_TOTAL counter");
```

Helper functions added to `metrics.rs`:

```rust
/// Update canary live-compare metrics after a result is ingested.
pub fn record_live_compare_result(pair: &str, divergence_bps: f64, outcome: &str) {
    CANARY_QUOTE_DIVERGENCE_BPS
        .with_label_values(&[pair])
        .set(divergence_bps);
    CANARY_COMPARISON_TOTAL
        .with_label_values(&[pair, outcome])
        .inc();
}
```

#### 2d. New route handlers — `crates/api/src/routes/canary.rs`

Two new handler functions appended to the existing file:

```rust
/// POST /api/v1/system/canary/live-compare
///
/// Accepts a comparison result from the external canary script, updates
/// Prometheus metrics, and appends to the in-memory history buffer.
/// Requires AdminAuth.
pub async fn ingest_live_compare(
    State(state): State<Arc<AppState>>,
    _admin: AdminAuth,
    Json(result): Json<LiveCompareResult>,
) -> Result<Json<LiveCompareIngestResponse>> {
    // outcome string for Prometheus label
    let outcome_str = match result.outcome {
        LiveCompareOutcome::Ok => "ok",
        LiveCompareOutcome::Diverged => "diverged",
        LiveCompareOutcome::Error => "error",
    };

    // divergence_bps is 0 when outcome is error (Req 3.5)
    let bps = if result.outcome == LiveCompareOutcome::Error {
        0.0
    } else {
        result.divergence_bps.max(0.0)  // never negative (Req 3.6)
    };

    crate::metrics::record_live_compare_result(&result.pair, bps, outcome_str);

    let mut history = state.live_compare_history.write().await;
    if history.len() == 1000 {
        history.pop_front();  // evict oldest (Req 2.6)
    }
    history.push_back(result);
    let entries = history.len();

    Ok(Json(LiveCompareIngestResponse {
        status: "ok".to_string(),
        entries,
    }))
}

/// GET /api/v1/system/canary/live-compare/report
///
/// Returns recent comparison history, newest first. Requires AdminAuth.
pub async fn live_compare_report(
    State(state): State<Arc<AppState>>,
    _admin: AdminAuth,
) -> Result<Json<LiveCompareReportResponse>> {
    let history = state.live_compare_history.read().await;
    // Collect in reverse (newest first)
    let results: Vec<LiveCompareResult> = history.iter().rev().cloned().collect();
    let total_entries = results.len();
    Ok(Json(LiveCompareReportResponse { total_entries, results }))
}
```

**Validation note:** Axum's `Json` extractor with `serde` deserialization handles missing required
fields and wrong types automatically, returning HTTP 422 via axum's rejection handling — no
custom validation middleware is needed. The `LiveCompareOutcome` enum's `#[serde(rename_all =
"snake_case")]` ensures that values other than `"ok"`, `"diverged"`, `"error"` fail deserialization
with a 422, satisfying Req 2.8 and 2.9.

#### 2e. Route registration — `crates/api/src/routes/mod.rs`

Add to the `operator_routes` block (which already wraps `production_admin_guard`):

```rust
.route(
    "/api/v1/system/canary/live-compare/report",
    get(canary::live_compare_report),
)
```

Add separately (POST always requires `AdminAuth` extractor regardless of environment, same pattern
as `POST /api/v1/system/canary/config`):

```rust
.route(
    "/api/v1/system/canary/live-compare",
    post(canary::ingest_live_compare),
)
```

---

### 3. Prometheus alert rules — `monitoring/prometheus/canary-alerts.yml`

A new file, included by the Prometheus configuration alongside `slo-alerts.yml`:

```yaml
groups:
  - name: stellarroute_canary_live_compare
    interval: 30s
    rules:
      - alert: CanaryQuoteDivergenceWarning
        expr: stellarroute_canary_quote_divergence_bps > 50
        for: 5m
        labels:
          severity: warning
          subsystem: canary_live_compare
        annotations:
          summary: >
            Canary: {{ $labels.pair }} divergence {{ $value | humanize }}bps
            exceeds warning threshold (50bps)
          description: >
            StellarRoute quote price has diverged from the Horizon reference price
            by {{ $value | humanize }}bps for {{ $labels.pair }} continuously for
            5 minutes. This may indicate stale indexer data or a routing anomaly.
            Check GET /api/v1/system/canary/live-compare/report for recent results.
          runbook_url: "https://links.internal/runbooks/canary-divergence"

      - alert: CanaryQuoteDivergenceCritical
        expr: stellarroute_canary_quote_divergence_bps > 200
        for: 2m
        labels:
          severity: critical
          subsystem: canary_live_compare
        annotations:
          summary: >
            Canary CRITICAL: {{ $labels.pair }} divergence {{ $value | humanize }}bps
            exceeds critical threshold (200bps)
          description: >
            StellarRoute quote price has diverged from the Horizon reference price
            by {{ $value | humanize }}bps for {{ $labels.pair }} continuously for
            2 minutes. This level of divergence indicates a probable routing bug,
            mis-indexed liquidity, or stale data. Immediate investigation required.
            Check GET /api/v1/system/canary/live-compare/report and the indexer sync
            status (stellarroute_indexer_sync_status) for root-cause clues.
          runbook_url: "https://links.internal/runbooks/canary-divergence"
```

---

### 4. GitHub Actions workflow — `.github/workflows/canary-compare.yml`

```yaml
name: Canary Live Quote Compare
# Runs every 5 minutes against production to detect quote price divergence.
# See docs/routing_canary.md — "Live Quote Comparison Job" for full context.

on:
  schedule:
    - cron: '*/5 * * * *'
  workflow_dispatch:
    inputs:
      base_url:
        description: 'Target StellarRoute API base URL'
        required: false
        default: 'https://api.stellarroute.io'
      verbose:
        description: 'Enable verbose output'
        type: boolean
        default: false

jobs:
  canary-compare:
    name: Compare Live Quotes vs Horizon
    runs-on: ubuntu-latest  # Python 3 is pre-installed; no pip install needed
    steps:
      - uses: actions/checkout@v4

      # Fail early with a clear message if the secret is not configured.
      - name: Verify admin token secret is set
        run: |
          if [ -z "${{ secrets.STELLARROUTE_CANARY_ADMIN_TOKEN }}" ]; then
            echo "ERROR: STELLARROUTE_CANARY_ADMIN_TOKEN secret is not configured."
            echo "See docs/routing_canary.md for setup instructions."
            exit 1
          fi

      # Run the comparison script. Exit code 1 = sustained divergence detected.
      - name: Run canary comparison
        env:
          CANARY_SR_BASE_URL: ${{ github.event.inputs.base_url || 'https://api.stellarroute.io' }}
          CANARY_ADMIN_TOKEN: ${{ secrets.STELLARROUTE_CANARY_ADMIN_TOKEN }}
          CANARY_VERBOSE: ${{ github.event.inputs.verbose || 'false' }}
        run: |
          python3 monitoring/canary-compare/canary_compare.py \
            --sr-base-url "$CANARY_SR_BASE_URL" \
            $( [ "$CANARY_VERBOSE" = "true" ] && echo "--verbose" )
```

---

### 5. Documentation update — `docs/routing_canary.md`

A new section is appended to the existing document. The additions are:

1. **System overview table** at the top of the file listing both canary subsystems.
2. **"Live Quote Comparison Job"** section covering: what it does, config reference table, default
   thresholds, manual invocation example, and a runbook subsection for the two alert levels.

The table differentiating the two systems:

| Subsystem | Purpose | Endpoints | Prometheus metrics |
|---|---|---|---|
| Internal routing canary | Compare candidate routing policy vs production baseline | `GET /api/v1/system/canary/report`, `POST /api/v1/system/canary/config` | *(none yet — tracked via in-memory VecDeque only)* |
| Live quote comparison | Cross-check StellarRoute prices vs Horizon reference | `POST /api/v1/system/canary/live-compare`, `GET /api/v1/system/canary/live-compare/report` | `stellarroute_canary_quote_divergence_bps`, `stellarroute_canary_comparison_total` |

---

## Data Flow Detail

### Happy path (no divergence)

```
canary_compare.py invoked (GitHub Actions, every 5 min)
  │
  ├── GET /api/v1/quote/native/USDC?amount=1000.0  → sr_price = "0.1250000"
  ├── GET horizon…/order_book?…                    → ref_price = "0.1251000"
  ├── divergence_bps = abs(0.125 - 0.1251) / 0.1251 * 10000 = 0.80
  ├── 0.80 < 50 (threshold) → outcome = "ok", consecutive_failures = 0
  ├── emit log: {"outcome":"ok","divergence_bps":0.80,...}
  └── POST /api/v1/system/canary/live-compare
        body: {pair, sr_price, ref_price, divergence_bps: 0.80, outcome: "ok", timestamp}
        → API: CANARY_QUOTE_DIVERGENCE_BPS{pair}.set(0.80)
        → API: CANARY_COMPARISON_TOTAL{pair,outcome="ok"}.inc()
        → API: live_compare_history.push_back(result)
        → 200 {"status":"ok","entries":N}
  exit 0
```

### Sustained divergence path

```
Run 1: divergence_bps = 75.0 > 50 → outcome = "diverged", consecutive_failures = 1
Run 2: divergence_bps = 82.0 > 50 → outcome = "diverged", consecutive_failures = 2
Run 3: divergence_bps = 91.0 > 50 → outcome = "diverged", consecutive_failures = 3 = threshold
  → emit_alert_log(alert=true, runbook_url=…)
  → POST result to API (divergence_bps: 91.0, outcome: "diverged")
  exit 1  ← GitHub Actions step marked FAILED
```

### Horizon unavailable path

```
GET horizon…/order_book → HTTP 503
outcome = "error", consecutive_failures unchanged (count_errors_as_failures=false, default)
emit log: {"outcome":"error","error":"Horizon returned HTTP 503",...}
POST /api/v1/system/canary/live-compare
  → API: CANARY_QUOTE_DIVERGENCE_BPS{pair}.set(0)   ← Req 3.5: 0 when error
  → API: CANARY_COMPARISON_TOTAL{pair,outcome="error"}.inc()
exit 0  ← not a failure unless consecutive_failures reaches threshold
```

---

## Error Handling

| Failure scenario | Script behaviour | API behaviour |
|---|---|---|
| StellarRoute returns non-200 | outcome = "error"; logged; POST to API | Records outcome="error" in metrics |
| StellarRoute JSON unparseable | outcome = "error"; logged; POST to API | Records outcome="error" in metrics |
| Horizon returns non-200 | outcome = "error"; logged; POST to API | Records outcome="error" in metrics |
| Horizon asks array empty | outcome = "error"; logged; POST to API | Records outcome="error"; divergence_bps = 0 |
| POST to `/live-compare` fails | warning logged; exit code unaffected | N/A |
| POST body missing required field | N/A | HTTP 422; no metric or history update |
| POST `outcome` has unknown value | N/A | HTTP 422 (serde deserialization failure) |
| POST without admin token | N/A | HTTP 401 (AdminAuth extractor rejects) |

---

## Files Changed

| File | Change type | Notes |
|---|---|---|
| `monitoring/canary-compare/canary_compare.py` | **New** | Zero-dependency Python comparison script |
| `monitoring/canary-compare/README.md` | **New** | Brief usage docs |
| `monitoring/prometheus/canary-alerts.yml` | **New** | Two Prometheus alert rules |
| `.github/workflows/canary-compare.yml` | **New** | Scheduled GHA workflow |
| `crates/api/src/models/response.rs` | **Modify** | Add `LiveCompareResult`, `LiveCompareOutcome`, `LiveCompareIngestResponse`, `LiveCompareReportResponse` |
| `crates/api/src/state.rs` | **Modify** | Add `live_compare_history` field to `AppState` |
| `crates/api/src/metrics.rs` | **Modify** | Add `CANARY_QUOTE_DIVERGENCE_BPS`, `CANARY_COMPARISON_TOTAL`, `record_live_compare_result()` |
| `crates/api/src/routes/canary.rs` | **Modify** | Add `ingest_live_compare` and `live_compare_report` handlers |
| `crates/api/src/routes/mod.rs` | **Modify** | Register two new routes |
| `docs/routing_canary.md` | **Modify** | Add system overview table + "Live Quote Comparison Job" section |

---

## Design Decisions

**Why not run the comparison inside the API process as a background tokio task?**
The comparison makes HTTP calls to Horizon. Running that inside the API process couples the API's
availability to Horizon's response time and adds complexity around cancellation, backpressure, and
startup ordering. An external script keeps the concerns separated: the API only receives results,
never initiates external fetches for quality validation.

**Why push results back to the API instead of writing directly to Prometheus pushgateway?**
The existing monitoring stack uses a pull model (Prometheus scrapes `/metrics`). A pushgateway
would be a new infrastructure dependency. Pushing through the API endpoint reuses the existing
`/metrics` scrape path and keeps the history buffer accessible for operator inspection via
`GET /api/v1/system/canary/live-compare/report`.

**Why `GaugeVec` for divergence (not a histogram)?**
The alert fires on a sustained high *current* value, not on rate or percentile. A gauge is the
correct Prometheus primitive for "current value that may go up and down." The history buffer
provides the time-series detail if an operator needs to look back further than Prometheus's
retention window.

**Why not validate the `pair` string format in the API?**
The `pair` label is free-form. Constraining it in the API would require duplicating the canonical
pair format logic from `crates/routing`. The script already constructs the pair string
deterministically (`{base_asset}/{quote_asset}`). Prometheus label cardinality is bounded by the
number of distinct pairs the script is configured to monitor (typically 2–5).

**Why set divergence_bps = 0 on error instead of omitting the metric?**
Prometheus gauges must always have a value once the label set is initialized. Setting to 0
disambiguates "unknown due to error" from "actually 0bps divergence" via the
`CANARY_COMPARISON_TOTAL{outcome="error"}` counter. Alert rules for divergence are on `> 50` so
a 0 value never triggers a false positive.
