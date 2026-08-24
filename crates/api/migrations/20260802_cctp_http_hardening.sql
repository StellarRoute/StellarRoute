-- CCTP HTTP gate hardening: idempotency state machine, poll lease, no secret persistence.

-- Idempotency: drop plaintext response storage and transfer FK (transfer created on finalize).
ALTER TABLE cctp_quote_idempotency
    DROP CONSTRAINT IF EXISTS cctp_quote_idempotency_transfer_id_fkey;

ALTER TABLE cctp_quote_idempotency
    DROP CONSTRAINT IF EXISTS cctp_quote_idempotency_response_len;

ALTER TABLE cctp_quote_idempotency
    DROP COLUMN IF EXISTS response_json;

ALTER TABLE cctp_quote_idempotency
    ADD COLUMN IF NOT EXISTS state TEXT NOT NULL DEFAULT 'pending';

ALTER TABLE cctp_quote_idempotency
    ADD COLUMN IF NOT EXISTS lease_owner_hash TEXT;

ALTER TABLE cctp_quote_idempotency
    ADD COLUMN IF NOT EXISTS lease_expires_at TIMESTAMPTZ;

ALTER TABLE cctp_quote_idempotency
    DROP CONSTRAINT IF EXISTS cctp_quote_idempotency_state_chk;

ALTER TABLE cctp_quote_idempotency
    ADD CONSTRAINT cctp_quote_idempotency_state_chk
    CHECK (state IN ('pending', 'completed'));

CREATE INDEX IF NOT EXISTS idx_cctp_quote_idempotency_state_lease
    ON cctp_quote_idempotency (state, lease_expires_at)
    WHERE state = 'pending';

-- Poll amplification guard on transfers.
ALTER TABLE cctp_transfers
    ADD COLUMN IF NOT EXISTS last_polled_at TIMESTAMPTZ;

ALTER TABLE cctp_transfers
    ADD COLUMN IF NOT EXISTS poll_lease_until TIMESTAMPTZ;

COMMENT ON COLUMN cctp_quote_idempotency.state IS
    'pending = quote in flight; completed = transfer row finalized.';
COMMENT ON COLUMN cctp_quote_idempotency.lease_owner_hash IS
    'SHA-256 hex of non-secret lease owner nonce; not an access token.';
COMMENT ON COLUMN cctp_transfers.poll_lease_until IS
    'Exclusive poll lease; concurrent GETs skip external Iris while held.';
