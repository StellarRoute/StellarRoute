# Audit Log Export Runbook

Operational guide for the scheduled `route_audit_log` → object storage export pipeline.

## Overview

The export pipeline incrementally copies privacy-safe audit entries from PostgreSQL to
gzip-compressed JSONL objects on local disk (sync to S3/GCS via your ops tooling). Each
batch is checkpointed so exports resume safely after restarts.

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `AUDIT_EXPORT_ENABLED` | `false` | Enable the background export task |
| `AUDIT_EXPORT_INTERVAL_SECS` | `900` | Seconds between export runs |
| `AUDIT_EXPORT_BATCH_SIZE` | `5000` | Max rows per batch |
| `AUDIT_EXPORT_OBJECT_PREFIX` | `stellarroute/audit-logs` | Object key prefix |
| `AUDIT_EXPORT_STORAGE_PATH` | `./data/audit-exports` | Local storage root |
| `AUDIT_EXPORT_SLOW_THRESHOLD_SECS` | `120` | Alert when a run exceeds this duration |
| `AUDIT_EXPORT_ALERT_FAILURE_STREAK` | `3` | Alert after N consecutive failures |

## Object layout (incident replay)

```
{prefix}/{YYYY}/{MM}/{DD}/batch-{first_id}-{last_id}-{unix_ts}.jsonl.gz
{prefix}/{YYYY}/{MM}/{DD}/batch-{first_id}-{last_id}-{unix_ts}.jsonl.gz.manifest.json
```

Each JSONL line is a redacted audit entry with `id`, `request_id`, and `trace_id` for
correlation with [`incident-replay-workflow.md`](./incident-replay-workflow.md).

### Sync to S3

```bash
aws s3 sync ./data/audit-exports/ s3://my-bucket/stellarroute/audit-logs/ \
  --exclude "*" --include "*.jsonl.gz" --include "*.manifest.json"
```

## Redaction rules

Documented in [`crates/api/src/audit/redactor.rs`](../crates/api/src/audit/redactor.rs).

| Field | Redacted? | Example |
|-------|-----------|---------|
| `inputs.base` / `inputs.quote` issuer | **Yes** | `USDC:GBBD47…` → `USDC:[REDACTED]` |
| `selected.path[*].from` / `.to` issuer | **Yes** | Same as above |
| `venue_ref`, `price`, `amount` | No | Public on-chain / numeric data |
| `request_id`, `trace_id` | No | Required for incident correlation |

Export re-applies redaction idempotently before writing each batch.

## Metrics & alerts

| Metric | Meaning |
|--------|---------|
| `stellarroute_audit_export_rows_total` | Rows exported successfully |
| `stellarroute_audit_export_failures_total` | Failed export runs |
| `stellarroute_audit_export_alerts_total{reason="slow_run\|failure_streak"}` | Alert conditions |

**Alert triggers:**

- Export run duration > `AUDIT_EXPORT_SLOW_THRESHOLD_SECS`
- Consecutive failures ≥ `AUDIT_EXPORT_ALERT_FAILURE_STREAK`

## Troubleshooting

1. **No exports running** — verify `AUDIT_EXPORT_ENABLED=true` and migration `0006_audit_log_export.sql` applied.
2. **Checkpoint stuck** — query `audit_log_export_checkpoints` and `audit_log_export_runs`.
3. **Disk full** — expand `AUDIT_EXPORT_STORAGE_PATH` or sync-and-delete local batches after S3 upload.
