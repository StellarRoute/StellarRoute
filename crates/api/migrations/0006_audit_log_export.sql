-- Audit log export pipeline checkpointing
--
-- Tracks incremental export progress from route_audit_log to object storage.
-- See docs/AUDIT_LOG_EXPORT_RUNBOOK.md for operational guidance.

CREATE TABLE IF NOT EXISTS audit_log_export_checkpoints (
    job_name            TEXT        PRIMARY KEY DEFAULT 'default',
    last_exported_id    BIGINT      NOT NULL DEFAULT 0,
    last_export_at      TIMESTAMPTZ,
    last_object_key     TEXT,
    rows_exported_total BIGINT      NOT NULL DEFAULT 0,
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS audit_log_export_runs (
    id                  BIGSERIAL   PRIMARY KEY,
    started_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finished_at         TIMESTAMPTZ,
    status              TEXT        NOT NULL
                        CHECK (status IN ('running', 'success', 'failed')),
    checkpoint_before   BIGINT      NOT NULL,
    checkpoint_after    BIGINT,
    rows_exported       BIGINT      NOT NULL DEFAULT 0,
    object_key          TEXT,
    error_message       TEXT
);

CREATE INDEX IF NOT EXISTS idx_audit_export_runs_started
    ON audit_log_export_runs(started_at DESC);

COMMENT ON TABLE audit_log_export_checkpoints IS
    'Incremental export cursor for route_audit_log → object storage pipeline.';
COMMENT ON TABLE audit_log_export_runs IS
    'History of audit log export runs for observability and incident replay.';
