-- Route decision audit log
--
-- Stores a structured, privacy-safe record of every route decision for
-- debugging and post-incident analysis.  All sensitive fields (asset issuers,
-- wallet addresses) are redacted before insertion.
--
-- ── Schema notes ─────────────────────────────────────────────────────────────
--
-- • `request_id`  – correlates with the HTTP `x-request-id` header.
-- • `trace_id`    – W3C traceparent trace ID (hex, 32 chars); empty string
--                   when no distributed trace is active.
-- • `outcome`     – one of: 'success', 'no_route', 'stale_data', 'error'
-- • `inputs`      – JSONB: base/quote assets (issuers redacted), amount,
--                   slippage_bps, quote_type.
-- • `selected`    – JSONB: chosen venue, price, path (issuers redacted).
--                   NULL when outcome != 'success'.
-- • `exclusions`  – JSONB array of {venue_ref, reason} objects.
--                   Empty array when no venues were excluded.
-- • `latency_ms`  – wall-clock duration of the quote pipeline in ms.
-- • `cache_hit`   – true when the response was served from cache.
--
-- ── Retention policy ─────────────────────────────────────────────────────────
--
-- Default retention: 30 days.
-- Entries older than `retained_until` are eligible for pruning.
-- The `prune_audit_log_older_than` function (below) should be called by a
-- scheduled job (e.g. pg_cron or an application-level background task).
--
-- Storage estimate (at 500 req/s sustained):
--   ~500 rows/s × 86 400 s/day × 30 days ≈ 1.3 billion rows
--   Average row size ≈ 800 bytes → ~1 TB raw.
--
-- For high-throughput deployments, consider:
--   1. Reducing retention to 7 days (≈ 240 GB).
--   2. Enabling table partitioning by `logged_at` (range, daily).
--   3. Sampling: only log 1-in-N normal requests; always log errors/no-route.
--   4. Offloading to an append-only object store (S3/GCS) via COPY … TO.
--
-- These options are documented in docs/audit-log-retention.md.

CREATE TABLE IF NOT EXISTS route_audit_log (
    id              BIGSERIAL   PRIMARY KEY,

    -- Correlation
    request_id      TEXT        NOT NULL,
    trace_id        TEXT        NOT NULL DEFAULT '',

    -- Timing
    logged_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    latency_ms      INTEGER     NOT NULL DEFAULT 0,

    -- Outcome
    outcome         TEXT        NOT NULL
                    CHECK (outcome IN ('success', 'no_route', 'stale_data', 'error')),
    cache_hit       BOOLEAN     NOT NULL DEFAULT FALSE,

    -- Request inputs (issuers redacted)
    inputs          JSONB       NOT NULL,

    -- Selected route (NULL on non-success outcomes)
    selected        JSONB,

    -- Exclusion reasons (empty array when none)
    exclusions      JSONB       NOT NULL DEFAULT '[]'::jsonb,

    -- Retention
    retained_until  TIMESTAMPTZ NOT NULL
                    GENERATED ALWAYS AS (logged_at + INTERVAL '30 days') STORED
);

-- ── Indexes ───────────────────────────────────────────────────────────────────

-- Primary lookup: by request_id for incident correlation
CREATE INDEX IF NOT EXISTS idx_audit_request_id
    ON route_audit_log(request_id);

-- Trace correlation
CREATE INDEX IF NOT EXISTS idx_audit_trace_id
    ON route_audit_log(trace_id)
    WHERE trace_id <> '';

-- Time-range queries and pruning
CREATE INDEX IF NOT EXISTS idx_audit_logged_at
    ON route_audit_log(logged_at DESC);

-- Retention pruning (partial index — only rows eligible for deletion)
CREATE INDEX IF NOT EXISTS idx_audit_retention
    ON route_audit_log(retained_until)
    WHERE retained_until <= NOW();

-- Outcome-based filtering (e.g. "show me all no_route events in the last hour")
CREATE INDEX IF NOT EXISTS idx_audit_outcome_time
    ON route_audit_log(outcome, logged_at DESC);

-- ── Comments ──────────────────────────────────────────────────────────────────

COMMENT ON TABLE route_audit_log IS
    'Privacy-safe structured audit log for every route decision. '
    'All asset_issuer values are replaced with [REDACTED] before insertion. '
    'Default retention: 30 days. See docs/audit-log-retention.md for tuning guidance.';

COMMENT ON COLUMN route_audit_log.request_id IS
    'HTTP x-request-id header value; correlates with API access logs.';
COMMENT ON COLUMN route_audit_log.trace_id IS
    'W3C traceparent trace ID (32-char hex). Empty string when no trace is active.';
COMMENT ON COLUMN route_audit_log.outcome IS
    'success | no_route | stale_data | error';
COMMENT ON COLUMN route_audit_log.inputs IS
    'Redacted request inputs: {base, quote, amount, slippage_bps, quote_type}. '
    'asset_issuer values replaced with [REDACTED].';
COMMENT ON COLUMN route_audit_log.selected IS
    'Redacted selected route: {venue_ref, venue_type, price, path}. NULL on non-success.';
COMMENT ON COLUMN route_audit_log.exclusions IS
    'Array of {venue_ref, reason} objects for venues excluded from routing.';
COMMENT ON COLUMN route_audit_log.retained_until IS
    'Computed retention deadline (logged_at + 30 days). Rows past this date are prunable.';
