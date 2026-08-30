-- Consolidate duplicate native (XLM) asset rows. NULL code/issuer bypasses the
-- composite unique constraint, so historical indexer runs created thousands of
-- native ids and broke quote resolution.

WITH keeper AS (
    SELECT id
    FROM assets
    WHERE asset_type = 'native'
    ORDER BY created_at ASC
    LIMIT 1
),
dupes AS (
    SELECT id
    FROM assets
    WHERE asset_type = 'native'
      AND id <> (SELECT id FROM keeper)
)
UPDATE sdex_offers o
SET selling_asset_id = (SELECT id FROM keeper)
WHERE selling_asset_id IN (SELECT id FROM dupes);

WITH keeper AS (
    SELECT id
    FROM assets
    WHERE asset_type = 'native'
    ORDER BY created_at ASC
    LIMIT 1
),
dupes AS (
    SELECT id
    FROM assets
    WHERE asset_type = 'native'
      AND id <> (SELECT id FROM keeper)
)
UPDATE sdex_offers o
SET buying_asset_id = (SELECT id FROM keeper)
WHERE buying_asset_id IN (SELECT id FROM dupes);

WITH keeper AS (
    SELECT id
    FROM assets
    WHERE asset_type = 'native'
    ORDER BY created_at ASC
    LIMIT 1
),
dupes AS (
    SELECT id
    FROM assets
    WHERE asset_type = 'native'
      AND id <> (SELECT id FROM keeper)
)
UPDATE normalized_liquidity nl
SET selling_asset_id = (SELECT id FROM keeper)
WHERE selling_asset_id IN (SELECT id FROM dupes);

WITH keeper AS (
    SELECT id
    FROM assets
    WHERE asset_type = 'native'
    ORDER BY created_at ASC
    LIMIT 1
),
dupes AS (
    SELECT id
    FROM assets
    WHERE asset_type = 'native'
      AND id <> (SELECT id FROM keeper)
)
UPDATE normalized_liquidity nl
SET buying_asset_id = (SELECT id FROM keeper)
WHERE buying_asset_id IN (SELECT id FROM dupes);

WITH keeper AS (
    SELECT id
    FROM assets
    WHERE asset_type = 'native'
    ORDER BY created_at ASC
    LIMIT 1
),
dupes AS (
    SELECT id
    FROM assets
    WHERE asset_type = 'native'
      AND id <> (SELECT id FROM keeper)
)
UPDATE amm_pool_reserves r
SET selling_asset_id = (SELECT id FROM keeper)
WHERE selling_asset_id IN (SELECT id FROM dupes);

WITH keeper AS (
    SELECT id
    FROM assets
    WHERE asset_type = 'native'
    ORDER BY created_at ASC
    LIMIT 1
),
dupes AS (
    SELECT id
    FROM assets
    WHERE asset_type = 'native'
      AND id <> (SELECT id FROM keeper)
)
UPDATE amm_pool_reserves r
SET buying_asset_id = (SELECT id FROM keeper)
WHERE buying_asset_id IN (SELECT id FROM dupes);

WITH keeper AS (
    SELECT id
    FROM assets
    WHERE asset_type = 'native'
    ORDER BY created_at ASC
    LIMIT 1
),
dupes AS (
    SELECT id
    FROM assets
    WHERE asset_type = 'native'
      AND id <> (SELECT id FROM keeper)
)
UPDATE trading_pairs tp
SET base_asset_id = (SELECT id FROM keeper)
WHERE base_asset_id IN (SELECT id FROM dupes);

WITH keeper AS (
    SELECT id
    FROM assets
    WHERE asset_type = 'native'
    ORDER BY created_at ASC
    LIMIT 1
),
dupes AS (
    SELECT id
    FROM assets
    WHERE asset_type = 'native'
      AND id <> (SELECT id FROM keeper)
)
UPDATE trading_pairs tp
SET counter_asset_id = (SELECT id FROM keeper)
WHERE counter_asset_id IN (SELECT id FROM dupes);

DELETE FROM assets
WHERE asset_type = 'native'
  AND id <> (
      SELECT id
      FROM assets
      WHERE asset_type = 'native'
      ORDER BY created_at ASC
      LIMIT 1
  );

CREATE UNIQUE INDEX IF NOT EXISTS assets_native_singleton
    ON assets (asset_type)
    WHERE asset_type = 'native';
