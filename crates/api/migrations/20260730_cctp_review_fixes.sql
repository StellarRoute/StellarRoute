-- CCTP review fixes: mint submitter, prepare payload binding, active prepare locks.

ALTER TABLE cctp_transfers
    ADD COLUMN IF NOT EXISTS mint_submitter TEXT,
    ADD COLUMN IF NOT EXISTS approval_payload_hash TEXT,
    ADD COLUMN IF NOT EXISTS approval_expiration_ledger BIGINT,
    ADD COLUMN IF NOT EXISTS burn_payload_hash TEXT,
    ADD COLUMN IF NOT EXISTS burn_prepare_step TEXT;

CREATE TABLE IF NOT EXISTS cctp_active_prepares (
    source_account TEXT PRIMARY KEY,
    transfer_id UUID NOT NULL,
    prepare_kind TEXT NOT NULL,
    payload_hash TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_cctp_active_prepares_expires
    ON cctp_active_prepares (expires_at);
