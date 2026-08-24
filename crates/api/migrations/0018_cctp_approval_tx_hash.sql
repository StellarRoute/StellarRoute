-- Track Stellar/EVM token approval tx separately from burn tx (ordered prepare flow).

ALTER TABLE cctp_transfers ADD COLUMN IF NOT EXISTS source_approval_tx_hash TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_cctp_source_approval_tx_hash_unique
    ON cctp_transfers (source_approval_tx_hash)
    WHERE source_approval_tx_hash IS NOT NULL;

ALTER TABLE cctp_transfers DROP CONSTRAINT IF EXISTS cctp_transfers_source_approval_tx_hash_len;
ALTER TABLE cctp_transfers ADD CONSTRAINT cctp_transfers_source_approval_tx_hash_len
    CHECK (source_approval_tx_hash IS NULL OR length(source_approval_tx_hash) <= 128);

COMMENT ON COLUMN cctp_transfers.source_approval_tx_hash IS
    'On-chain approval tx hash when a separate approve step preceded burn.';
