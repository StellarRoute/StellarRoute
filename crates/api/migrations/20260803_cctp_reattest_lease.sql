-- Reattest lease + provider attempt counter (status unchanged until finalize).

ALTER TABLE cctp_transfers
    ADD COLUMN IF NOT EXISTS reattest_lease_owner_hash TEXT;

ALTER TABLE cctp_transfers
    ADD COLUMN IF NOT EXISTS reattest_lease_until TIMESTAMPTZ;

ALTER TABLE cctp_transfers
    ADD COLUMN IF NOT EXISTS reattest_attempt_count INT NOT NULL DEFAULT 0;

ALTER TABLE cctp_transfers
    ADD COLUMN IF NOT EXISTS reattest_cooldown_until TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_cctp_transfers_reattest_lease
    ON cctp_transfers (status, reattest_lease_until)
    WHERE status = 'attestation_failed';

COMMENT ON COLUMN cctp_transfers.reattest_lease_owner_hash IS
    'SHA-256 hex of lease owner nonce; transfer stays attestation_failed until finalize.';
COMMENT ON COLUMN cctp_transfers.reattest_attempt_count IS
    'Circle Iris reattest provider calls (success or failure); capped separately from retry_count.';
COMMENT ON COLUMN cctp_transfers.reattest_cooldown_until IS
    'Durable cooldown after failed reattest finalize; claim blocked until elapsed.';
