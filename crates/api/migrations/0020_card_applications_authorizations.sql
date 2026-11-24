-- Card applications and authorizations tables
-- These tables support card payment processing.
-- They are NOT queried when CARD_ENABLED is false.
-- No PAN, CVV, or track data columns are present in these tables.

CREATE TABLE IF NOT EXISTS card_applications (
  id BIGSERIAL PRIMARY KEY,
  cardholder_name TEXT NOT NULL,
  card_last_four CHAR(4) NOT NULL,
  billing_address TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS card_authorizations (
  id BIGSERIAL PRIMARY KEY,
  card_application_id BIGINT NOT NULL REFERENCES card_applications(id),
  amount_usdc NUMERIC(20, 6) NOT NULL,
  currency TEXT NOT NULL DEFAULT 'USDC',
  status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'held', 'captured', 'released', 'voided')),
  held_amount_usdc NUMERIC(20, 6) DEFAULT 0,
  spent_amount_usdc NUMERIC(20, 6) DEFAULT 0,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_card_auths_application_id
  ON card_authorizations(card_application_id);

CREATE INDEX IF NOT EXISTS idx_card_auths_status
  ON card_authorizations(status);
