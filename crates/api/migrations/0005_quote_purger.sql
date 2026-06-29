-- StellarRoute API - Quote Purger with Observability (Phase 1.6)
--
-- Implements automated stale-quote artifact purging with:
-- 1. Configurable retention policies for replay_artifacts and route_audit_log
-- 2. Safeguards against over-aggressive deletion (batch limits, rate controls)
-- 3. Observability hooks: purge metrics, age distributions, deleted counts
-- 4. Administrative functions for manual tuning and incident response

-- ── Purge Metrics Table ────────────────────────────────────────────────────────────
-- Tracks each purge operation for observability and debugging

CREATE TABLE IF NOT EXISTS quote_purge_metrics (
    id                  BIGSERIAL        PRIMARY KEY,
    
    -- Purge metadata
    purge_type          TEXT             NOT NULL,  -- 'replay_artifacts' | 'route_audit_log'
    started_at          TIMESTAMPTZ      NOT NULL DEFAULT NOW(),
    completed_at        TIMESTAMPTZ,
    
    -- Deletion statistics
    deleted_count       BIGINT           NOT NULL DEFAULT 0,
    scanned_count       BIGINT           NOT NULL DEFAULT 0,  -- rows examined before delete
    duration_ms         INTEGER,
    
    -- Age distribution (percentiles of deleted rows)
    age_min_days        NUMERIC,  -- minimum age of deleted rows (most recent)
    age_max_days        NUMERIC,  -- maximum age of deleted rows (oldest)
    age_p50_days        NUMERIC,  -- median age
    age_p95_days        NUMERIC,  -- 95th percentile
    age_p99_days        NUMERIC,  -- 99th percentile
    
    -- Safety metrics
    rows_retained       BIGINT,   -- rows remaining after purge
    batch_size_used     INTEGER,
    was_rate_limited    BOOLEAN  DEFAULT FALSE,
    
    -- Status and errors
    status              TEXT      NOT NULL DEFAULT 'pending',  -- pending | success | partial | failed
    error_message       TEXT,
    operator_note       TEXT      -- manual annotations for incident response
);

-- Indexes for operational queries
CREATE INDEX IF NOT EXISTS idx_purge_metrics_type_time
    ON quote_purge_metrics(purge_type, started_at DESC);

CREATE INDEX IF NOT EXISTS idx_purge_metrics_status
    ON quote_purge_metrics(status)
    WHERE status != 'success';

-- ── Purge Functions ──────────────────────────────────────────────────────────────────

-- Function: purge_replay_artifacts_older_than
--
-- Deletes replay_artifacts older than specified days with safeguards:
-- - Limits batch size to prevent long locks
-- - Returns purge metrics for observability
-- - Tracks age distribution of deleted rows
--
-- Usage:
--   SELECT * FROM purge_replay_artifacts_older_than(30, 1000);
--
CREATE OR REPLACE FUNCTION purge_replay_artifacts_older_than(
    p_retention_days     INTEGER DEFAULT 30,
    p_batch_size         INTEGER DEFAULT 1000,
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
    -- Calculate age distribution BEFORE deletion (for observability)
    SELECT
        ROUND(MIN(EXTRACT(EPOCH FROM (NOW() - captured_at)) / 86400), 2),
        ROUND(MAX(EXTRACT(EPOCH FROM (NOW() - captured_at)) / 86400), 2),
        ROUND(PERCENTILE_CONT(0.50) WITHIN GROUP (ORDER BY EXTRACT(EPOCH FROM (NOW() - captured_at)) / 86400), 2),
        ROUND(PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY EXTRACT(EPOCH FROM (NOW() - captured_at)) / 86400), 2),
        ROUND(PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY EXTRACT(EPOCH FROM (NOW() - captured_at)) / 86400), 2)
    INTO v_age_min, v_age_max, v_age_p50, v_age_p95, v_age_p99
    FROM replay_artifacts
    WHERE captured_at < NOW() - (INTERVAL '1 day' * p_retention_days);

    -- Batch deletion loop with iteration limit as safeguard
    LOOP
        v_iteration := v_iteration + 1;
        
        IF v_iteration > p_max_iterations THEN
            v_was_rate_limited := TRUE;
            EXIT;
        END IF;

        -- Delete batch of old artifacts
        DELETE FROM replay_artifacts
        WHERE id IN (
            SELECT id FROM replay_artifacts
            WHERE captured_at < NOW() - (INTERVAL '1 day' * p_retention_days)
            LIMIT p_batch_size
        );

        GET DIAGNOSTICS v_batch_deleted = ROW_COUNT;
        v_total_deleted := v_total_deleted + v_batch_deleted;
        v_total_scanned := v_total_scanned + p_batch_size;

        EXIT WHEN v_batch_deleted = 0;
        
        -- Add small delay between batches if we're doing many iterations
        -- This prevents starving other queries
        IF v_iteration % 10 = 0 THEN
            PERFORM pg_sleep(0.1);
        END IF;
    END LOOP;

    -- Get final retained count
    SELECT COUNT(*) INTO v_rows_retained FROM replay_artifacts;

    -- Return all metrics
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

    -- Log metrics to audit table
    INSERT INTO quote_purge_metrics (
        purge_type, deleted_count, scanned_count, duration_ms,
        age_min_days, age_max_days, age_p50_days, age_p95_days, age_p99_days,
        rows_retained, batch_size_used, was_rate_limited, status, completed_at
    ) VALUES (
        'replay_artifacts',
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

-- Function: purge_route_audit_log_older_than
--
-- Deletes route_audit_log entries older than specified days with safeguards.
-- Respects the computed `retained_until` column and provides observability.
--
CREATE OR REPLACE FUNCTION purge_route_audit_log_older_than(
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
    FROM route_audit_log
    WHERE retained_until <= NOW();

    -- Batch deletion loop with iteration limit
    LOOP
        v_iteration := v_iteration + 1;
        
        IF v_iteration > p_max_iterations THEN
            v_was_rate_limited := TRUE;
            EXIT;
        END IF;

        -- Delete batch of expired audit logs
        DELETE FROM route_audit_log
        WHERE id IN (
            SELECT id FROM route_audit_log
            WHERE retained_until <= NOW()
            ORDER BY id  -- deterministic deletion order
            LIMIT p_batch_size
        );

        GET DIAGNOSTICS v_batch_deleted = ROW_COUNT;
        v_total_deleted := v_total_deleted + v_batch_deleted;
        v_total_scanned := v_total_scanned + p_batch_size;

        EXIT WHEN v_batch_deleted = 0;
        
        -- Add small delay between batches
        IF v_iteration % 10 = 0 THEN
            PERFORM pg_sleep(0.1);
        END IF;
    END LOOP;

    -- Get final retained count
    SELECT COUNT(*) INTO v_rows_retained FROM route_audit_log;

    -- Return all metrics
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

    -- Log metrics to audit table
    INSERT INTO quote_purge_metrics (
        purge_type, deleted_count, scanned_count, duration_ms,
        age_min_days, age_max_days, age_p50_days, age_p95_days, age_p99_days,
        rows_retained, batch_size_used, was_rate_limited, status, completed_at
    ) VALUES (
        'route_audit_log',
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

-- Function: get_quote_purge_status
--
-- Returns current purge metrics and status for dashboarding and alerting
--
CREATE OR REPLACE FUNCTION get_quote_purge_status()
RETURNS TABLE (
    purge_type              TEXT,
    last_purge_at           TIMESTAMPTZ,
    last_deleted_count      BIGINT,
    last_duration_ms        INTEGER,
    rows_currently_in_table BIGINT,
    last_age_p99_days       NUMERIC
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        m.purge_type,
        MAX(m.completed_at),
        (ARRAY_AGG(m.deleted_count ORDER BY m.completed_at DESC))[1]::BIGINT,
        (ARRAY_AGG(m.duration_ms ORDER BY m.completed_at DESC))[1]::INTEGER,
        CASE 
            WHEN m.purge_type = 'replay_artifacts' THEN (SELECT COUNT(*) FROM replay_artifacts)
            WHEN m.purge_type = 'route_audit_log' THEN (SELECT COUNT(*) FROM route_audit_log)
            ELSE 0
        END,
        (ARRAY_AGG(m.age_p99_days ORDER BY m.completed_at DESC))[1]::NUMERIC
    FROM quote_purge_metrics m
    WHERE m.status IN ('success', 'partial')
    GROUP BY m.purge_type;
END;
$$ LANGUAGE plpgsql;

-- ── Comments ──────────────────────────────────────────────────────────────────

COMMENT ON TABLE quote_purge_metrics IS
    'Audit trail for all quote artifact purge operations. '
    'Tracks deleted counts, age distributions, duration, and rate-limiting events. '
    'Used for alerting, dashboarding, and post-incident analysis.';

COMMENT ON FUNCTION purge_replay_artifacts_older_than IS
    'Deletes replay_artifacts older than retention_days in batches. '
    'Returns age distribution and metrics for observability. '
    'Parameters: retention_days (default 30), batch_size (default 1000), max_iterations (default 100).';

COMMENT ON FUNCTION purge_route_audit_log_older_than IS
    'Deletes route_audit_log entries past retained_until in batches. '
    'Returns age distribution and metrics for observability. '
    'Parameters: retention_days (default 30), batch_size (default 5000), max_iterations (default 100).';

COMMENT ON FUNCTION get_quote_purge_status IS
    'Returns latest purge metrics per table type for operational dashboards. '
    'Shows last purge timestamp, deleted count, duration, and 99th percentile age.';
