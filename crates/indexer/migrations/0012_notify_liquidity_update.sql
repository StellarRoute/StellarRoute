-- StellarRoute - NOTIFY-driven GraphManager sync (closes #653)
--
-- Adds pg_notify('liquidity_update', ...) to both the SDEX sync trigger and
-- the AMM upsert function so the API-side GraphManager wakes up immediately on
-- liquidity changes rather than waiting for the 10 s polling fallback.

-- 1. AMM: replace upsert function to emit notify after write
create or replace function upsert_amm_pool_reserve(
  p_pool_address text,
  p_selling_asset_id uuid,
  p_buying_asset_id uuid,
  p_reserve_selling numeric,
  p_reserve_buying numeric,
  p_fee_bps integer,
  p_last_updated_ledger bigint,
  p_source_trace_id text default '',
  p_source_span_id text default ''
)
returns void as $$
begin
  insert into amm_pool_reserves (
    pool_address,
    selling_asset_id,
    buying_asset_id,
    reserve_selling,
    reserve_buying,
    fee_bps,
    last_updated_ledger,
    source_trace_id,
    source_span_id,
    updated_at
  )
  values (
    p_pool_address,
    p_selling_asset_id,
    p_buying_asset_id,
    p_reserve_selling,
    p_reserve_buying,
    p_fee_bps,
    p_last_updated_ledger,
    p_source_trace_id,
    p_source_span_id,
    now()
  )
  on conflict (pool_address)
  do update set
    selling_asset_id = excluded.selling_asset_id,
    buying_asset_id = excluded.buying_asset_id,
    reserve_selling = excluded.reserve_selling,
    reserve_buying = excluded.reserve_buying,
    fee_bps = excluded.fee_bps,
    last_updated_ledger = excluded.last_updated_ledger,
    source_trace_id = excluded.source_trace_id,
    source_span_id = excluded.source_span_id,
    updated_at = now();

  perform pg_notify('liquidity_update', 'amm:' || p_pool_address);
end;
$$ language plpgsql;

-- 2. SDEX: replace sync trigger function to emit notify after write
create or replace function sync_normalized_liquidity_from_sdex()
returns trigger as $$
begin
    if (TG_OP = 'DELETE') then
        delete from normalized_liquidity
        where venue_type = 'sdex' and venue_ref = OLD.offer_id::text;
        return OLD;
    else
        insert into normalized_liquidity (
            venue_type,
            venue_ref,
            selling_asset_id,
            buying_asset_id,
            price,
            available_amount,
            price_e7,
            available_amount_e7,
            source_ledger,
            source_trace_id,
            source_span_id,
            updated_at
        )
        values (
            'sdex',
            NEW.offer_id::text,
            NEW.selling_asset_id,
            NEW.buying_asset_id,
            NEW.price,
            NEW.amount,
            (NEW.price * 10000000)::bigint,
            (NEW.amount * 10000000)::bigint,
            NEW.last_modified_ledger,
            NEW.source_trace_id,
            NEW.source_span_id,
            NEW.updated_at
        )
        on conflict (venue_type, venue_ref)
        do update set
            selling_asset_id = EXCLUDED.selling_asset_id,
            buying_asset_id = EXCLUDED.buying_asset_id,
            price = EXCLUDED.price,
            available_amount = EXCLUDED.available_amount,
            price_e7 = EXCLUDED.price_e7,
            available_amount_e7 = EXCLUDED.available_amount_e7,
            source_ledger = EXCLUDED.source_ledger,
            source_trace_id = EXCLUDED.source_trace_id,
            source_span_id = EXCLUDED.source_span_id,
            updated_at = EXCLUDED.updated_at;

        perform pg_notify('liquidity_update', 'sdex:' || NEW.offer_id::text);
        return NEW;
    end if;
end;
$$ language plpgsql;
