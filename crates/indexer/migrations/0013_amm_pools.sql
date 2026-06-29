-- Migration: Create persistent AMM pool registry for bootstrap and operator management
CREATE TABLE IF NOT EXISTS amm_pools (
    pool_address TEXT PRIMARY KEY,
    network TEXT NOT NULL DEFAULT 'mainnet',
    active BOOLEAN NOT NULL DEFAULT TRUE,
    metadata JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_amm_pools_network_active ON amm_pools (network, active);

CREATE OR REPLACE FUNCTION amm_pools_updated_at_trigger() RETURNS trigger AS $$
BEGIN
  NEW.updated_at = now();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_amm_pools_updated_at ON amm_pools;
CREATE TRIGGER trg_amm_pools_updated_at
BEFORE UPDATE ON amm_pools
FOR EACH ROW EXECUTE PROCEDURE amm_pools_updated_at_trigger();

COMMENT ON TABLE amm_pools IS 'Operator-managed registry of known AMM pool addresses used as bootstrap fallback';
