-- Approval hash alone is insufficient; record when on-chain approval was verified.

ALTER TABLE cctp_transfers ADD COLUMN IF NOT EXISTS source_approval_verified_at TIMESTAMPTZ;

COMMENT ON COLUMN cctp_transfers.source_approval_verified_at IS
    'Timestamp when source_approval_tx_hash was cryptographically verified on-chain.';
