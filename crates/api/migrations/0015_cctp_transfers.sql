-- CCTP v2 transfer saga persistence (separate from swap_prepared_quotes).
-- Stores attestation polling state only; no private keys or signed payloads.

CREATE TABLE IF NOT EXISTS cctp_transfers (
    transfer_id UUID PRIMARY KEY,
    support_reference_id TEXT NOT NULL,
    corridor_id TEXT NOT NULL,
    provider TEXT NOT NULL,
    direction TEXT NOT NULL CHECK (direction IN ('stellar_to_evm', 'evm_to_stellar')),
    source_chain_id TEXT NOT NULL,
    destination_chain_id TEXT NOT NULL,
    source_asset TEXT NOT NULL,
    source_asset_canonical TEXT NOT NULL,
    destination_asset TEXT NOT NULL,
    destination_asset_canonical TEXT NOT NULL,
    sender TEXT NOT NULL DEFAULT '',
    recipient TEXT NOT NULL,
    amount TEXT NOT NULL,
    destination_amount TEXT NOT NULL,
    finality TEXT NOT NULL CHECK (finality IN ('standard', 'fast')),
    runtime_fee_quote TEXT,
    max_fee TEXT,
    fee_expires_at TIMESTAMPTZ,
    quote_expires_at TIMESTAMPTZ NOT NULL,
    status TEXT NOT NULL,
    source_tx_hash TEXT,
    destination_tx_hash TEXT,
    iris_message_hash TEXT,
    message_nonce TEXT,
    raw_message BYTEA,
    attestation BYTEA,
    retry_count INTEGER NOT NULL DEFAULT 0,
    last_provider_error TEXT,
    last_provider_code TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    terminal_at TIMESTAMPTZ
);

-- One burn tx hash cannot bind to two transfers.
CREATE UNIQUE INDEX IF NOT EXISTS idx_cctp_source_tx_hash_unique
    ON cctp_transfers (source_tx_hash)
    WHERE source_tx_hash IS NOT NULL;

-- Active polling by saga status.
CREATE INDEX IF NOT EXISTS idx_cctp_status_polling
    ON cctp_transfers (status, updated_at)
    WHERE terminal_at IS NULL;

-- Nonce tracking per source chain when present.
CREATE UNIQUE INDEX IF NOT EXISTS idx_cctp_message_nonce_source
    ON cctp_transfers (source_chain_id, message_nonce)
    WHERE message_nonce IS NOT NULL;

COMMENT ON TABLE cctp_transfers IS
    'Circle CCTP v2 bridge transfer saga. No signing material or RPC secrets.';
COMMENT ON COLUMN cctp_transfers.raw_message IS
    'Untrusted raw CCTP message bytes from Iris; validated before state transitions.';
COMMENT ON COLUMN cctp_transfers.attestation IS
    'Untrusted attestation bytes from Iris; never logged in plaintext.';
