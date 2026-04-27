-- Asset metadata enrichment table
-- Stores decimals, domain, and icon references for assets.
-- Populated by the background metadata enrichment job.

CREATE TABLE IF NOT EXISTS asset_metadata (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    asset_type      TEXT NOT NULL,
    asset_code      TEXT,
    asset_issuer    TEXT,
    -- Enriched fields
    decimals        SMALLINT,
    domain          TEXT,
    icon_url        TEXT,
    -- Source tracking
    source          TEXT NOT NULL DEFAULT 'stellar_toml',
    -- Staleness tracking
    fetched_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Idempotency: one row per canonical asset
    UNIQUE (asset_type, asset_code, asset_issuer)
);

-- Index for fast lookups by asset identity
CREATE INDEX IF NOT EXISTS idx_asset_metadata_identity
    ON asset_metadata (asset_type, asset_code, asset_issuer);

-- Index for staleness-based refresh queries
CREATE INDEX IF NOT EXISTS idx_asset_metadata_fetched_at
    ON asset_metadata (fetched_at ASC);

COMMENT ON TABLE asset_metadata IS
    'Enriched asset metadata (decimals, domain, icon) fetched from stellar.toml and Horizon /assets';
COMMENT ON COLUMN asset_metadata.source IS
    'Source of the metadata: stellar_toml | horizon_assets | manual';
COMMENT ON COLUMN asset_metadata.fetched_at IS
    'Wall-clock time when this row was last refreshed; used for staleness checks';
