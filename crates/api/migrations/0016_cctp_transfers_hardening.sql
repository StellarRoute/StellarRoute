-- CCTP transfer hardening: status/finality checks and payload size bounds.

ALTER TABLE cctp_transfers DROP CONSTRAINT IF EXISTS cctp_transfers_finality_check;
ALTER TABLE cctp_transfers ADD CONSTRAINT cctp_transfers_finality_check
    CHECK (finality = 'standard');

ALTER TABLE cctp_transfers DROP CONSTRAINT IF EXISTS cctp_transfers_status_check;
ALTER TABLE cctp_transfers ADD CONSTRAINT cctp_transfers_status_check
    CHECK (status IN (
        'created',
        'burn_prepared',
        'burn_submitted',
        'awaiting_attestation',
        'attestation_ready',
        'mint_prepared',
        'mint_submitted',
        'completed',
        'attestation_failed',
        'mint_failed_retryable',
        'cancelled',
        'provider_killed'
    ));

ALTER TABLE cctp_transfers DROP CONSTRAINT IF EXISTS cctp_transfers_raw_message_len;
ALTER TABLE cctp_transfers ADD CONSTRAINT cctp_transfers_raw_message_len
    CHECK (raw_message IS NULL OR octet_length(raw_message) <= 8192);

ALTER TABLE cctp_transfers DROP CONSTRAINT IF EXISTS cctp_transfers_attestation_len;
ALTER TABLE cctp_transfers ADD CONSTRAINT cctp_transfers_attestation_len
    CHECK (attestation IS NULL OR octet_length(attestation) <= 8192);

ALTER TABLE cctp_transfers DROP CONSTRAINT IF EXISTS cctp_transfers_message_nonce_len;
ALTER TABLE cctp_transfers ADD CONSTRAINT cctp_transfers_message_nonce_len
    CHECK (message_nonce IS NULL OR length(message_nonce) <= 128);
