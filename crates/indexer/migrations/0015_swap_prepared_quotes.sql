-- Classic prepare → sign → submit lifecycle tables.
-- Owned by the API store, but applied by the indexer migrator because the API
-- does not run migrations on boot and staging/prod only auto-apply indexer SQL.

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

ALTER TABLE swap_prepared_quotes
    ADD COLUMN IF NOT EXISTS sender_account TEXT,
    ADD COLUMN IF NOT EXISTS amount_in TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS execution_mode TEXT NOT NULL DEFAULT 'classic_path_payment',
    ADD COLUMN IF NOT EXISTS network_passphrase TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS route_digest TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS price_digest TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS source_sequence BIGINT,
    ADD COLUMN IF NOT EXISTS timebounds_max BIGINT,
    ADD COLUMN IF NOT EXISTS base_fee INTEGER;

CREATE UNIQUE INDEX IF NOT EXISTS idx_swap_prepared_active_sender
    ON swap_prepared_quotes (sender_account)
    WHERE submission_status IN ('prepared', 'submitting')
      AND sender_account IS NOT NULL;

COMMENT ON TABLE swap_prepared_quotes IS
    'Server-side swap prepare/submit lifecycle keyed by quote_id for expiry and idempotency.';

-- Privacy-safe prepare/submit audit log (API writes; table must exist with quotes).
CREATE TABLE IF NOT EXISTS swap_submit_audit_log (
    id              BIGSERIAL   PRIMARY KEY,
    quote_id        TEXT        NOT NULL,
    tx_hash         TEXT,
    account         TEXT        NOT NULL,
    request_id      TEXT        NOT NULL,
    trace_id        TEXT        NOT NULL DEFAULT '',
    logged_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    latency_ms      INTEGER     NOT NULL DEFAULT 0,
    outcome         TEXT        NOT NULL
                    CHECK (outcome IN ('prepared', 'submitted', 'failed')),
    error_class     TEXT        NOT NULL DEFAULT '',
    metadata        JSONB       NOT NULL DEFAULT '{}'::jsonb,
    retained_until  TIMESTAMPTZ NOT NULL
                    GENERATED ALWAYS AS (logged_at + INTERVAL '30 days') STORED
);

CREATE INDEX IF NOT EXISTS idx_swap_submit_audit_quote_id
    ON swap_submit_audit_log(quote_id);

CREATE INDEX IF NOT EXISTS idx_swap_submit_audit_tx_hash
    ON swap_submit_audit_log(tx_hash)
    WHERE tx_hash IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_swap_submit_audit_logged_at
    ON swap_submit_audit_log(logged_at DESC);
