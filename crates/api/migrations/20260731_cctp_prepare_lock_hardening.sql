-- Harden cctp_active_prepares: payload cache, FK, constraints, timestamps.

ALTER TABLE cctp_active_prepares
    ADD COLUMN IF NOT EXISTS prepared_payload TEXT,
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Backfill updated_at for rows created before column existed.
UPDATE cctp_active_prepares SET updated_at = created_at WHERE updated_at IS NULL;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'cctp_active_prepares_kind_check'
    ) THEN
        ALTER TABLE cctp_active_prepares
            ADD CONSTRAINT cctp_active_prepares_kind_check
            CHECK (prepare_kind IN ('approval', 'burn', 'mint'));
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'cctp_active_prepares_hash_len'
    ) THEN
        ALTER TABLE cctp_active_prepares
            ADD CONSTRAINT cctp_active_prepares_hash_len
            CHECK (char_length(payload_hash) BETWEEN 1 AND 128);
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'cctp_active_prepares_payload_len'
    ) THEN
        ALTER TABLE cctp_active_prepares
            ADD CONSTRAINT cctp_active_prepares_payload_len
            CHECK (
                prepared_payload IS NULL
                OR char_length(prepared_payload) BETWEEN 1 AND 131072
            );
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'cctp_active_prepares_transfer_fk'
    ) THEN
        ALTER TABLE cctp_active_prepares
            ADD CONSTRAINT cctp_active_prepares_transfer_fk
            FOREIGN KEY (transfer_id) REFERENCES cctp_transfers(transfer_id)
            ON DELETE CASCADE;
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_cctp_active_prepares_transfer
    ON cctp_active_prepares (transfer_id);
