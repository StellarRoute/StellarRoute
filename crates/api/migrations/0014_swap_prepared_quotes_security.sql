-- Security metadata for classic prepare/submit binding and reconciliation.
-- Backward compatible: new columns are nullable or have defaults for existing rows.
--
-- network_passphrase semantics:
--   * New prepares MUST persist the exact passphrase used to build the unsigned tx.
--   * Empty / missing passphrase on legacy rows is NOT backfilled from the environment.
--   * Submit fails closed for empty passphrase and requires a fresh prepare.
--   * Operators may manually UPDATE rows only when the historical passphrase is known.

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

-- At most one active prepare/submit in flight per sender account.
-- Submitting rows remain active past prepare TTL (reconcilable).
CREATE UNIQUE INDEX IF NOT EXISTS idx_swap_prepared_active_sender
    ON swap_prepared_quotes (sender_account)
    WHERE submission_status IN ('prepared', 'submitting')
      AND sender_account IS NOT NULL;

COMMENT ON COLUMN swap_prepared_quotes.sender_account IS
    'G-address source account used for signature verification (not logged in audit as raw key).';
COMMENT ON COLUMN swap_prepared_quotes.network_passphrase IS
    'Exact Stellar network passphrase bound at prepare. Empty means legacy/unusable; submit must fail closed.';
COMMENT ON COLUMN swap_prepared_quotes.route_digest IS
    'SHA-256 digest of normalized classic route hops.';
COMMENT ON COLUMN swap_prepared_quotes.price_digest IS
    'SHA-256 digest binding prepare to authoritative server pricing.';
COMMENT ON COLUMN swap_prepared_quotes.tx_hash IS
    'Deterministic transaction hash persisted atomically on claim (submitting must never have NULL tx_hash).';
