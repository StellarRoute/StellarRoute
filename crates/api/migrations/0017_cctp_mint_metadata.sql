-- CCTP mint preparation binding metadata (no signed payloads).

ALTER TABLE cctp_transfers ADD COLUMN IF NOT EXISTS mint_payload_hash TEXT;
ALTER TABLE cctp_transfers ADD COLUMN IF NOT EXISTS mint_payload_expires_at TIMESTAMPTZ;

CREATE UNIQUE INDEX IF NOT EXISTS idx_cctp_destination_tx_hash_unique
    ON cctp_transfers (destination_tx_hash)
    WHERE destination_tx_hash IS NOT NULL;

ALTER TABLE cctp_transfers DROP CONSTRAINT IF EXISTS cctp_transfers_mint_payload_hash_len;
ALTER TABLE cctp_transfers ADD CONSTRAINT cctp_transfers_mint_payload_hash_len
    CHECK (mint_payload_hash IS NULL OR length(mint_payload_hash) <= 128);

COMMENT ON COLUMN cctp_transfers.mint_payload_hash IS
    'SHA-256 hex of prepared unsigned mint payload; no signed material stored.';
