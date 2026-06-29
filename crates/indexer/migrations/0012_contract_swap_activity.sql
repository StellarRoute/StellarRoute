CREATE TABLE IF NOT EXISTS contract_swap_activity (
    event_id TEXT PRIMARY KEY,
    contract_id TEXT NOT NULL,
    ledger BIGINT NOT NULL,
    ledger_closed_at TIMESTAMPTZ,
    paging_token TEXT NOT NULL,
    sender TEXT NOT NULL,
    amount_in NUMERIC NOT NULL,
    amount_out NUMERIC NOT NULL,
    fee_amount NUMERIC NOT NULL,
    route JSONB NOT NULL DEFAULT '{}'::jsonb,
    source_asset TEXT,
    destination_asset TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_contract_swap_activity_ledger
    ON contract_swap_activity (ledger DESC, event_id);

CREATE INDEX IF NOT EXISTS idx_contract_swap_activity_sender
    ON contract_swap_activity (sender, ledger DESC);
