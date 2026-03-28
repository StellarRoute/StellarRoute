-- Replay artifacts table for deterministic quote replay system
-- Stores redacted snapshots of quote computations for post-incident analysis

CREATE TABLE IF NOT EXISTS replay_artifacts (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    schema_version  INTEGER     NOT NULL DEFAULT 1,
    incident_id     TEXT,
    captured_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Request inputs (denormalised for fast filtering)
    base_asset      TEXT        NOT NULL,
    quote_asset     TEXT        NOT NULL,
    amount          TEXT        NOT NULL,
    slippage_bps    INTEGER     NOT NULL,
    quote_type      TEXT        NOT NULL,

    -- Full artifact payload (redacted) stored as JSONB
    -- Contains: liquidity_snapshot, health_config_snapshot, original_output
    artifact        JSONB       NOT NULL,

    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Efficient lookup by incident for incident-scoped queries
CREATE INDEX IF NOT EXISTS idx_replay_incident
    ON replay_artifacts(incident_id, captured_at DESC)
    WHERE incident_id IS NOT NULL;

-- Efficient lookup by trading pair for regression checks
CREATE INDEX IF NOT EXISTS idx_replay_pair
    ON replay_artifacts(base_asset, quote_asset, captured_at DESC);

-- Efficient pruning and time-range queries
CREATE INDEX IF NOT EXISTS idx_replay_captured_at
    ON replay_artifacts(captured_at DESC);

COMMENT ON TABLE replay_artifacts IS
    'Deterministic replay snapshots for post-incident quote analysis. All sensitive fields are redacted before storage.';
COMMENT ON COLUMN replay_artifacts.schema_version IS
    'Artifact schema version; replayer validates compatibility before executing. Current version: 1.';
COMMENT ON COLUMN replay_artifacts.incident_id IS
    'Optional free-form incident label (e.g. INC-20260327-001) for grouping related artifacts.';
COMMENT ON COLUMN replay_artifacts.artifact IS
    'Full ReplayArtifact JSON with asset_issuer values replaced by [REDACTED]. Contains liquidity_snapshot, health_config_snapshot, and original_output.';
