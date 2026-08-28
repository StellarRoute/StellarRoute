-- Prepared swap quotes and submission lifecycle (prepare → sign → submit).
--
-- `quote_id` correlates prepare/submit pairs and enforces idempotent submission:
-- a successful submit transitions `submission_status` to `submitted` exactly once.

CREATE TABLE IF NOT EXISTS swap_prepared_quotes (
    quote_id            TEXT        PRIMARY KEY,
    sender_account_hash TEXT        NOT NULL,
    unsigned_xdr_hash   TEXT        NOT NULL,
    expires_at          TIMESTAMPTZ NOT NULL,
    estimated_output    TEXT        NOT NULL DEFAULT '',
    min_output          TEXT        NOT NULL DEFAULT '',
    valid_until_ledger  BIGINT,
    submission_status   TEXT        NOT NULL DEFAULT 'prepared'
                        CHECK (submission_status IN ('prepared', 'submitting', 'submitted', 'failed')),
    tx_hash             TEXT,
    submitted_at        TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_swap_prepared_quotes_expires_at
    ON swap_prepared_quotes(expires_at);

CREATE INDEX IF NOT EXISTS idx_swap_prepared_quotes_status
    ON swap_prepared_quotes(submission_status);

COMMENT ON TABLE swap_prepared_quotes IS
    'Server-side swap prepare/submit lifecycle keyed by quote_id for expiry and idempotency.';
