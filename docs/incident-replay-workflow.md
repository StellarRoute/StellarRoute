# Incident Replay Workflow

This document describes how to use the deterministic quote replay system during a routing incident or regression investigation.

---

## Overview

Every successful quote computation automatically stores a **replay artifact** — a redacted snapshot of all inputs (liquidity candidates, health config, request parameters) and the original output. When an anomaly is reported, you can:

1. Find the `artifact_id` from logs
2. Fetch the artifact
3. Run the replay pipeline against the stored snapshot
4. Diff the replay output against the original to determine whether the routing logic has changed

---

## Prerequisites

- `DATABASE_URL` environment variable pointing to the StellarRoute PostgreSQL instance
- `replay-cli` binary built: `cargo build --bin replay-cli --release`
- Or: API server running at `$API_URL` (e.g. `https://api.stellarroute.io`)

---

## Step 1: Find the artifact_id from logs

Every captured artifact emits a structured log line at `DEBUG` level:

```
DEBUG replay artifact captured artifact_id=<UUID>
```

Search your log aggregator (e.g. Datadog, CloudWatch) for the request that triggered the anomaly:

```
# Example: find artifacts for a specific trading pair around the incident time
artifact_id AND base=native AND quote=USDC AND timestamp:[2026-03-27T10:00:00 TO 2026-03-27T10:05:00]
```

If the artifact was tagged with an incident ID at capture time, you can also list by incident:

```bash
replay-cli list --incident INC-20260327-001
```

---

## Step 2: Fetch the artifact

### Using the CLI

```bash
export ARTIFACT_ID="550e8400-e29b-41d4-a716-446655440000"
replay-cli fetch $ARTIFACT_ID | jq .
```

### Using the API

```bash
curl -s "$API_URL/api/v1/replay/$ARTIFACT_ID" | jq .
```

Example output (truncated):

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "schema_version": 1,
  "incident_id": "INC-20260327-001",
  "captured_at": "2026-03-27T10:02:34.123Z",
  "base": "native",
  "quote": "USDC:[REDACTED]",
  "amount": "1000.0000000",
  "slippage_bps": 50,
  "quote_type": "sell",
  "liquidity_snapshot": [
    { "venue_type": "sdex", "venue_ref": "offer1", "price": "0.9950000", "available_amount": "5000.0000000" },
    { "venue_type": "amm",  "venue_ref": "pool1",  "price": "0.9960000", "available_amount": "50000.0000000" }
  ],
  "original_output": {
    "price": "0.9950000",
    "selected_source": "sdex:offer1"
  }
}
```

---

## Step 3: Run the replay

```bash
replay-cli run $ARTIFACT_ID | jq .
```

Or via API:

```bash
curl -s -X POST "$API_URL/api/v1/replay/$ARTIFACT_ID/run" | jq .
```

---

## Step 4: Diff the replay against the original

```bash
replay-cli diff $ARTIFACT_ID | jq .
```

Or via API:

```bash
curl -s -X POST "$API_URL/api/v1/replay/$ARTIFACT_ID/diff" | jq .
```

### Case A: No divergence (routing logic unchanged)

```json
{
  "artifact_id": "550e8400-e29b-41d4-a716-446655440000",
  "is_identical": true,
  "divergences": []
}
```

**Interpretation**: The routing logic has not changed since the incident. The anomaly was caused by a data issue (e.g. stale liquidity, missing offers) rather than a code regression. Investigate the `liquidity_snapshot` in the artifact for unusual prices or missing venues.

### Case B: Divergence found (routing logic changed)

```json
{
  "artifact_id": "550e8400-e29b-41d4-a716-446655440000",
  "is_identical": false,
  "divergences": [
    {
      "field": "selected_source",
      "original": "sdex:offer1",
      "replayed": "amm:pool1"
    },
    {
      "field": "price",
      "original": "0.9950000",
      "replayed": "0.9960000"
    }
  ]
}
```

**Interpretation**: The replay selected a different venue (`amm:pool1` instead of `sdex:offer1`). This indicates a change in the routing or scoring logic since the artifact was captured. Check recent commits to `crates/api/src/routes/quote.rs` or `crates/routing/` for changes to the venue selection comparator or health scoring thresholds.

---

## Step 5: List artifacts for an incident

To review all artifacts captured during an incident window:

```bash
replay-cli list --incident INC-20260327-001 --limit 50
```

Or filter by trading pair:

```bash
replay-cli list --base native --quote USDC --limit 20
```

---

## Step 6: Attach artifact to incident ticket

Export the artifact as a JSON file and attach it to your incident ticket:

```bash
replay-cli fetch $ARTIFACT_ID > artifact-$ARTIFACT_ID.json
```

The artifact is safe to share: all `asset_issuer` values are replaced with `[REDACTED]` before storage.

---

## Enabling capture in production

Artifact capture is disabled by default. Enable it by setting:

```bash
REPLAY_CAPTURE_ENABLED=true
```

Capture is non-blocking — it never adds latency to live quote responses. If the DB write fails, a `WARN` log is emitted and the quote response is unaffected.

---

## Retention

Artifacts are retained for 30 days by default (configurable via `REPLAY_RETENTION_DAYS`). To manually prune old artifacts, call `ReplayArtifact::prune_older_than` from a maintenance job or run a direct SQL query:

```sql
DELETE FROM replay_artifacts WHERE captured_at < NOW() - INTERVAL '30 days';
```
