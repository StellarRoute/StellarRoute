-- Contract version registry table
-- Stores deployed contract metadata for version tracking and SDK pinning

CREATE TABLE IF NOT EXISTS contract_registry (
    id SERIAL PRIMARY KEY,
    contract_name TEXT NOT NULL,
    version TEXT NOT NULL,
    wasm_hash TEXT NOT NULL,
    network TEXT NOT NULL,
    contract_address TEXT,
    deployed_at BIGINT,
    git_commit TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT unique_contract_version UNIQUE (contract_name, version, network)
);

-- Index for fast lookups by contract name
CREATE INDEX IF NOT EXISTS idx_contract_registry_name 
    ON contract_registry(contract_name);

-- Index for network-specific queries
CREATE INDEX IF NOT EXISTS idx_contract_registry_network 
    ON contract_registry(network);

-- Index for latest version queries
CREATE INDEX IF NOT EXISTS idx_contract_registry_deployed 
    ON contract_registry(deployed_at DESC);

-- Update trigger for updated_at
CREATE OR REPLACE FUNCTION update_contract_registry_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER contract_registry_updated_at
    BEFORE UPDATE ON contract_registry
    FOR EACH ROW
    EXECUTE FUNCTION update_contract_registry_updated_at();
