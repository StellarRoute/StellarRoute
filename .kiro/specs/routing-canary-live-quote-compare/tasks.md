# Implementation Tasks

## Task 1: Add LiveCompare data types to API models

**File:** `crates/api/src/models/response.rs`

Add the following types after the existing `AssetMetadataBulkResponse` struct:

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

Also add these types to the public re-exports in `crates/api/src/models/mod.rs` (wherever other response types are re-exported).

**Verification:** `cargo build -p stellarroute-api` compiles without errors.

---

## Task 2: Add live_compare_history to AppState

**Depends on:** Task 1

**File:** `crates/api/src/state.rs`

1. Add the import at the top alongside the existing canary import:
   ```rust
   use crate::models::LiveCompareResult;
   ```

2. Add the field to the `AppState` struct after `canary_history`:
   ```rust
   /// Live-compare history buffer: results from the external canary comparison job.
   /// Capped at 1,000 entries; newest at the back, oldest evicted from the front.
   pub live_compare_history: Arc<tokio::sync::RwLock<std::collections::VecDeque<LiveCompareResult>>>,
   ```

3. Initialise the field in **both** `new_with_policy` and `with_cache_and_policy` (the two `Self { ... }` blocks), alongside the existing `canary_history` initialisation:
   ```rust
   live_compare_history: Arc::new(tokio::sync::RwLock::new(
       std::collections::VecDeque::with_capacity(1000),
   )),
   ```

**Verification:** `cargo build -p stellarroute-api` compiles without errors.

---

## Task 3: Add canary live-compare Prometheus metrics

**Depends on:** Task 1

**File:** `crates/api/src/metrics.rs`

1. Add a new section at the end of the existing `lazy_static!` block (before the closing `}`):

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

2. Add a helper function at the end of the file (after the existing helpers):

```rust
// ── Canary live-compare metric helpers ───────────────────────────────────────

/// Update canary live-compare Prometheus metrics after a result is ingested.
/// `outcome` must be one of "ok", "diverged", "error".
/// When outcome is "error", divergence_bps is forced to 0.0.
pub fn record_live_compare_result(pair: &str, divergence_bps: f64, outcome: &str) {
    CANARY_QUOTE_DIVERGENCE_BPS
        .with_label_values(&[pair])
        .set(divergence_bps);
    CANARY_COMPARISON_TOTAL
        .with_label_values(&[pair, outcome])
        .inc();
}
```

**Verification:** `cargo build -p stellarroute-api` compiles without errors.

---

## Task 4: Add live-compare route handlers

**Depends on:** Tasks 1, 2, 3

**File:** `crates/api/src/routes/canary.rs`

1. Extend the imports at the top of the file:
   ```rust
   use crate::models::{LiveCompareIngestResponse, LiveCompareOutcome, LiveCompareReportResponse, LiveCompareResult};
   ```

2. Append these two handlers after the existing `update_config` function:

```rust
/// POST /api/v1/system/canary/live-compare
///
/// Accepts a comparison result from the external canary script, updates
/// Prometheus metrics, and appends to the in-memory history buffer.
/// Requires AdminAuth. Returns HTTP 422 automatically when the JSON body
/// is missing required fields or contains unexpected types (serde rejection).
pub async fn ingest_live_compare(
    State(state): State<Arc<AppState>>,
    _admin: AdminAuth,
    Json(result): Json<LiveCompareResult>,
) -> Result<Json<LiveCompareIngestResponse>> {
    let outcome_str = match result.outcome {
        LiveCompareOutcome::Ok => "ok",
        LiveCompareOutcome::Diverged => "diverged",
        LiveCompareOutcome::Error => "error",
    };

    // When outcome is error, divergence is unknown — record 0 to avoid negative gauge.
    let bps = if result.outcome == LiveCompareOutcome::Error {
        0.0
    } else {
        result.divergence_bps.max(0.0)
    };

    crate::metrics::record_live_compare_result(&result.pair, bps, outcome_str);

    let mut history = state.live_compare_history.write().await;
    if history.len() == 1000 {
        history.pop_front();
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
/// Returns recent comparison history newest first. Requires AdminAuth.
pub async fn live_compare_report(
    State(state): State<Arc<AppState>>,
    _admin: AdminAuth,
) -> Result<Json<LiveCompareReportResponse>> {
    let history = state.live_compare_history.read().await;
    let results: Vec<LiveCompareResult> = history.iter().rev().cloned().collect();
    let total_entries = results.len();
    Ok(Json(LiveCompareReportResponse {
        total_entries,
        results,
    }))
}
```

**Verification:** `cargo build -p stellarroute-api` compiles without errors.

---

## Task 5: Register new routes in the router

**Depends on:** Task 4

**File:** `crates/api/src/routes/mod.rs`

1. Add `live_compare_report` to the `operator_routes` block (alongside the existing `canary::get_report` entry), so it gets the `production_admin_guard` middleware:

```rust
.route(
    "/api/v1/system/canary/live-compare/report",
    get(canary::live_compare_report),
)
```

2. Add `ingest_live_compare` as a standalone route (POST always requires `AdminAuth` via its extractor, same as `canary::update_config`). Place it next to the existing canary config route:

```rust
.route(
    "/api/v1/system/canary/live-compare",
    post(canary::ingest_live_compare),
)
```

**Verification:** `cargo build -p stellarroute-api` compiles without errors. `cargo test -p stellarroute-api` passes.

---

## Task 6: Write canary comparison Python script

**File:** `monitoring/canary-compare/canary_compare.py`

Create the zero-dependency comparison script. The script must:

- Use only Python 3 standard library (`urllib`, `json`, `argparse`, `os`, `sys`, `datetime`, `math`)
- Follow the exact pattern of `monitoring/synthetic-probes/probe_runner.py` (structured JSON stdout, env-var + CLI config, runbook links, exit 0/1)
- Support configuration via env vars with CLI override:

| Env var | CLI flag | Default |
|---|---|---|
| `CANARY_SR_BASE_URL` | `--sr-base-url` | `http://localhost:3000` |
| `CANARY_HORIZON_BASE_URL` | `--horizon-base-url` | `https://horizon.stellar.org` |
| `CANARY_BASE_ASSET` | `--base-asset` | `native` |
| `CANARY_QUOTE_ASSET` | `--quote-asset` | `USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN` |
| `CANARY_AMOUNT` | `--amount` | `1000.0` |
| `CANARY_TIMEOUT` | `--timeout` | `10.0` |
| `CANARY_DIVERGENCE_THRESHOLD_BPS` | `--divergence-threshold` | `50` |
| `CANARY_FAILURE_THRESHOLD` | `--failure-threshold` | `3` |
| `CANARY_ADMIN_TOKEN` | `--admin-token` | *(required)* |
| `CANARY_COUNT_ERRORS_AS_FAILURES` | `--count-errors-as-failures` | `false` |

**Key logic:**

1. `fetch_sr_price(base_url, base, quote, amount, timeout)` — calls `GET /api/v1/quote/{base}/{quote}?amount={amount}`, extracts `data.price` from the JSON envelope. Returns `(price_str, None)` on success, `(None, error_msg)` on failure.

2. `fetch_horizon_price(horizon_base, base_asset, quote_asset, timeout)` — constructs the Horizon order_book URL. For a `CODE:ISSUER` quote asset vs native base:
   ```
   {horizon_base}/order_book
     ?selling_asset_type=native
     &buying_asset_type=credit_alphanum4  (or credit_alphanum12 for len > 4)
     &buying_asset_code={CODE}
     &buying_asset_issuer={ISSUER}
     &limit=5
   ```
   Extracts `asks[0]["price"]`. Returns `(price_str, None)` on success or empty asks, `(None, error_msg)` on HTTP error.

3. Main comparison loop:
   - compute `divergence_bps = round(abs(float(sr) - float(ref)) / float(ref) * 10_000, 2)`
   - outcome = `"ok"` / `"diverged"` / `"error"`
   - emit structured JSON log line to stdout (fields: `timestamp`, `pair`, `stellarroute_price`, `reference_price`, `divergence_bps`, `outcome`, `consecutive_failures`)
   - POST result to `{sr_base_url}/api/v1/system/canary/live-compare` with `Authorization: Bearer {admin_token}` header — failure here is a warning only (logged, does not affect exit code)
   - increment `consecutive_failures` on `"diverged"` (and optionally on `"error"` if `count_errors_as_failures=true`), reset to 0 otherwise
   - if `consecutive_failures >= failure_threshold`: emit alert JSON log (with `"alert": true` and `runbook_url`), then `sys.exit(1)`

4. Normal completion: `sys.exit(0)`

**Runbook URL constant:**
```python
RUNBOOK_URL = "https://links.internal/runbooks/canary-divergence"
```

**Verification:** `python3 monitoring/canary-compare/canary_compare.py --help` runs without error. `python3 -m py_compile monitoring/canary-compare/canary_compare.py` passes.

---

## Task 7: Write canary-compare README

**Depends on:** Task 6

**File:** `monitoring/canary-compare/README.md`

Write a concise README covering:
- What the script does (one paragraph)
- Configuration table (env var, CLI flag, default, description) matching Task 6
- How to run locally against a dev API
- How to run against production (using env vars)
- What the exit codes mean
- Where to find alerts (link to `monitoring/prometheus/canary-alerts.yml`)
- Where to find the operator runbook (link to `docs/routing_canary.md`)

---

## Task 8: Write Prometheus alert rules

**File:** `monitoring/prometheus/canary-alerts.yml`

Create this file with the content specified in the design doc:

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

**Verification:** File is valid YAML (`python3 -c "import yaml, sys; yaml.safe_load(open('monitoring/prometheus/canary-alerts.yml'))"` or equivalent).

---

## Task 9: Write GitHub Actions workflow

**File:** `.github/workflows/canary-compare.yml`

Create the workflow as specified in the design doc. Key requirements:
- Triggers: `schedule` (cron `*/5 * * * *`) and `workflow_dispatch`
- `workflow_dispatch` inputs: `base_url` (string, default `https://api.stellarroute.io`) and `verbose` (boolean, default false)
- Job `canary-compare` on `ubuntu-latest`
- Step 1: verify `STELLARROUTE_CANARY_ADMIN_TOKEN` secret is set; fail with descriptive error if not
- Step 2: run `python3 monitoring/canary-compare/canary_compare.py` with env vars set from secrets and inputs
- Include inline comments on each step linking to `docs/routing_canary.md`
- No `pip install` or dependency installation steps

**Verification:** File is valid YAML. `actions/checkout@v4` is referenced correctly.

---

## Task 10: Update docs/routing_canary.md

**File:** `docs/routing_canary.md`

Add two sections to the existing document:

**1. System overview table** — insert near the top of the document, after the opening paragraph, before "Features":

```markdown
## Canary Subsystems

| Subsystem | Purpose | Endpoints | Prometheus metrics |
|---|---|---|---|
| Internal routing canary | Compare candidate routing policy vs production baseline | `GET /api/v1/system/canary/report`, `POST /api/v1/system/canary/config` | *(none — tracked via in-memory VecDeque only)* |
| Live quote comparison | Cross-check StellarRoute prices vs Horizon reference | `POST /api/v1/system/canary/live-compare`, `GET /api/v1/system/canary/live-compare/report` | `stellarroute_canary_quote_divergence_bps`, `stellarroute_canary_comparison_total` |
```

**2. "Live Quote Comparison Job" section** — append at the end of the document with these subsections:
- What it does (brief description referencing `monitoring/canary-compare/canary_compare.py`)
- Configuration table (env vars, CLI flags, defaults from Task 6)
- Default thresholds: warning 50 bps, critical 200 bps, consecutive failure threshold 3 — plus guidance on when to adjust them
- Example `curl` to manually invoke the script (as a shell command with env vars set)
- Alert runbook subsection: steps for `CanaryQuoteDivergenceWarning` and `CanaryQuoteDivergenceCritical`, referencing `GET /api/v1/system/canary/live-compare/report`
- Reference to `monitoring/prometheus/canary-alerts.yml`

**Verification:** The file renders correctly as Markdown and contains the string `live-compare` (verifiable with `rg -n 'canary' docs/routing_canary.md`).
