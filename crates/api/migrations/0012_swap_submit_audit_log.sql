-- Swap submit audit log
--
-- Stores a structured, privacy-safe record of every swap prepare/submit
-- attempt.  Sensitive account information is redacted by the application
-- before insertion; the raw public key or secret is never written here.
--
-- ── Schema notes ─────────────────────────────────────────────────────────────
--
-- • `quote_id`    – client-provided quote identifier; used for idempotency
--                    lookups and correlating prepare/submit pairs.
-- • `tx_hash`     – on-chain transaction hash, available once a submit has
--                    reached the network (NULL on prepare / failure).
-- • `account`     – redacted account identifier; stored as a hash-prefix
--                    fingerprint, never the raw public key.
-- • `request_id`  – correlates with the HTTP `x-request-id` header.
-- • `trace_id`    – W3C traceparent trace ID (hex, 32 chars); empty string
--                    when no distributed trace is active.
-- • `outcome`     – one of: 'prepared', 'submitted', 'failed'.
-- • `error_class` – machine-readable failure class (empty string on success).
-- • `metadata`    – JSONB extensibility bucket for route, amount, fee, etc.
--
-- ── Retention policy ─────────────────────────────────────────────────────────
--
-- Default retention: 30 days.
-- Entries older than `retained_until` are eligible for pruning.
-- The `prune_swap_submit_audit_log_older_than` function (below) should be
-- called by the application purger or a scheduled job (e.g. pg_cron).
--
-- For high-throughput deployments the same tuning options as
-- `route_audit_log` apply; see docs/audit-log-retention.md.

CREATE TABLE IF NOT EXISTS swap_submit_audit_log (
    id              BIGSERIAL   PRIMARY KEY,

    -- Business correlation
    quote_id        TEXT        NOT NULL,
    tx_hash         TEXT,                       -- NULL until submitted / on failure
    account         TEXT        NOT NULL,       -- redacted hash-prefix; never raw key

    -- Request correlation
    request_id      TEXT        NOT NULL,
    trace_id        TEXT        NOT NULL DEFAULT '',

    -- Timing
    logged_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    latency_ms      INTEGER     NOT NULL DEFAULT 0,

    -- Outcome
    outcome         TEXT        NOT NULL
                    CHECK (outcome IN ('prepared', 'submitted', 'failed')),
    error_class     TEXT        NOT NULL DEFAULT '',

    -- Extensibility
    metadata        JSONB       NOT NULL DEFAULT '{}'::jsonb,

    -- Retention
    retained_until  TIMESTAMPTZ NOT NULL
                    GENERATED ALWAYS AS (logged_at + INTERVAL '30 days') STORED
);

-- ── Indexes ───────────────────────────────────────────────────────────────────

-- Primary idempotency/correlation lookup by quote_id
CREATE INDEX IF NOT EXISTS idx_swap_submit_audit_quote_id
    ON swap_submit_audit_log(quote_id);

-- Transaction correlation (only when tx_hash is set)
CREATE INDEX IF NOT EXISTS idx_swap_submit_audit_tx_hash
    ON swap_submit_audit_log(tx_hash)
    WHERE tx_hash IS NOT NULL;

-- Time-range queries
CREATE INDEX IF NOT EXISTS idx_swap_submit_audit_logged_at
    ON swap_submit_audit_log(logged_at DESC);

-- Retention pruning (partial index — only rows eligible for deletion)
CREATE INDEX IF NOT EXISTS idx_swap_submit_audit_retention
    ON swap_submit_audit_log(retained_until)
    WHERE retained_until <= NOW();

-- Outcome-based filtering (e.g. "show me all failed submits in the last hour")
CREATE INDEX IF NOT EXISTS idx_swap_submit_audit_outcome_time
    ON swap_submit_audit_log(outcome, logged_at DESC);

-- ── Comments ──────────────────────────────────────────────────────────────────

COMMENT ON TABLE swap_submit_audit_log IS
    'Privacy-safe structured audit log for swap prepare/submit attempts. '
    'Account identifiers are redacted to a hash-prefix fingerprint before insertion. '
    'Default retention: 30 days. See docs/audit-log-retention.md for tuning guidance.';

COMMENT ON COLUMN swap_submit_audit_log.quote_id IS
    'Client-provided quote identifier; ties prepare and submit attempts together.';
COMMENT ON COLUMN swap_submit_audit_log.tx_hash IS
    'On-chain transaction hash. NULL for prepare records or failed submissions.';
COMMENT ON COLUMN swap_submit_audit_log.account IS
    'Redacted account fingerprint (prefix + hash). The raw public key is never stored.';
COMMENT ON COLUMN swap_submit_audit_log.outcome IS
    'prepared | submitted | failed';
COMMENT ON COLUMN swap_submit_audit_log.error_class IS
    'Machine-readable failure class when outcome = failed; empty otherwise.';
COMMENT ON COLUMN swap_submit_audit_log.metadata IS
    'Extensible JSONB payload for route, amount, fee estimate, etc.';
COMMENT ON COLUMN swap_submit_audit_log.retained_until IS
    'Computed retention deadline (logged_at + 30 days). Rows past this date are prunable.';

-- ── Purge function ────────────────────────────────────────────────────────────
--
-- Mirrors purge_route_audit_log_older_than (0005_quote_purger.sql) but targets
-- swap_submit_audit_log.

CREATE OR REPLACE FUNCTION purge_swap_submit_audit_log_older_than(
    p_retention_days     INTEGER DEFAULT 30,
    p_batch_size         INTEGER DEFAULT 5000,
    p_max_iterations     INTEGER DEFAULT 100
)
RETURNS TABLE (
    deleted_count       BIGINT,
    total_scanned       BIGINT,
    rows_retained       BIGINT,
    age_min_days        NUMERIC,
    age_max_days        NUMERIC,
    age_p50_days        NUMERIC,
    age_p95_days        NUMERIC,
    age_p99_days        NUMERIC,
    was_rate_limited    BOOLEAN,
    duration_ms         INTEGER
) AS $$
DECLARE
    v_total_deleted     BIGINT := 0;
    v_total_scanned     BIGINT := 0;
    v_iteration         INTEGER := 0;
    v_batch_deleted     BIGINT;
    v_start_time        TIMESTAMPTZ := NOW();
    v_age_min           NUMERIC;
    v_age_max           NUMERIC;
    v_age_p50           NUMERIC;
    v_age_p95           NUMERIC;
    v_age_p99           NUMERIC;
    v_rows_retained     BIGINT;
    v_was_rate_limited  BOOLEAN := FALSE;
BEGIN
    -- Calculate age distribution BEFORE deletion
    SELECT
        ROUND(MIN(EXTRACT(EPOCH FROM (NOW() - logged_at)) / 86400), 2),
        ROUND(MAX(EXTRACT(EPOCH FROM (NOW() - logged_at)) / 86400), 2),
        ROUND(PERCENTILE_CONT(0.50) WITHIN GROUP (ORDER BY EXTRACT(EPOCH FROM (NOW() - logged_at)) / 86400), 2),
        ROUND(PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY EXTRACT(EPOCH FROM (NOW() - logged_at)) / 86400), 2),
        ROUND(PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY EXTRACT(EPOCH FROM (NOW() - logged_at)) / 86400), 2)
    INTO v_age_min, v_age_max, v_age_p50, v_age_p95, v_age_p99
    FROM swap_submit_audit_log
    WHERE retained_until <= NOW();

    -- Batch deletion loop with iteration limit
    LOOP
        v_iteration := v_iteration + 1;

        IF v_iteration > p_max_iterations THEN
            v_was_rate_limited := TRUE;
            EXIT;
        END IF;

        DELETE FROM swap_submit_audit_log
        WHERE id IN (
            SELECT id FROM swap_submit_audit_log
            WHERE retained_until <= NOW()
            ORDER BY id
            LIMIT p_batch_size
        );

        GET DIAGNOSTICS v_batch_deleted = ROW_COUNT;
        v_total_deleted := v_total_deleted + v_batch_deleted;
        v_total_scanned := v_total_scanned + p_batch_size;

        EXIT WHEN v_batch_deleted = 0;

        IF v_iteration % 10 = 0 THEN
            PERFORM pg_sleep(0.1);
        END IF;
    END LOOP;

    SELECT COUNT(*) INTO v_rows_retained FROM swap_submit_audit_log;

    RETURN QUERY SELECT
        v_total_deleted,
        v_total_scanned,
        v_rows_retained,
        v_age_min,
        v_age_max,
        v_age_p50,
        v_age_p95,
        v_age_p99,
        v_was_rate_limited,
        CAST(EXTRACT(EPOCH FROM (NOW() - v_start_time)) * 1000 AS INTEGER);

    INSERT INTO quote_purge_metrics (
        purge_type, deleted_count, scanned_count, duration_ms,
        age_min_days, age_max_days, age_p50_days, age_p95_days, age_p99_days,
        rows_retained, batch_size_used, was_rate_limited, status, completed_at
    ) VALUES (
        'swap_submit_audit_log',
        v_total_deleted,
        v_total_scanned,
        CAST(EXTRACT(EPOCH FROM (NOW() - v_start_time)) * 1000 AS INTEGER),
        v_age_min,
        v_age_max,
        v_age_p50,
        v_age_p95,
        v_age_p99,
        v_rows_retained,
        p_batch_size,
        v_was_rate_limited,
        CASE WHEN v_was_rate_limited THEN 'partial' ELSE 'success' END,
        NOW()
    );
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION purge_swap_submit_audit_log_older_than IS
    'Batch-deletes swap_submit_audit_log rows past their retention deadline. '
    'Mirrors purge_route_audit_log_older_than; called by the quote purger.';
