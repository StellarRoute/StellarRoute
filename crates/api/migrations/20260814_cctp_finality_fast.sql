-- Allow CCTP Fast finality for EVM→Stellar (Iris prices threshold 1000).
-- 0016 had narrowed the check to Standard-only during fail-closed rollout.

ALTER TABLE cctp_transfers DROP CONSTRAINT IF EXISTS cctp_transfers_finality_check;
ALTER TABLE cctp_transfers ADD CONSTRAINT cctp_transfers_finality_check
    CHECK (finality IN ('standard', 'fast'));
