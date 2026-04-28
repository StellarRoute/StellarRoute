-- StellarRoute - Phase 1.5/2.1 Finalization
-- Move from view to table for normalized liquidity with automatic synchronization

-- 1. Safely rename the legacy view if it exists
do $$
begin
    if exists (select 1 from pg_views where viewname = 'normalized_liquidity') then
        execute 'alter view normalized_liquidity rename to normalized_liquidity_legacy_view';
    end if;
end $$;

-- 2. Trigger function to sync from SDEX offers
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
            updated_at = EXCLUDED.updated_at;
        return NEW;
    end if;
end;
$$ language plpgsql;

-- 3. Trigger function to sync from AMM pool reserves
create or replace function sync_normalized_liquidity_from_amm()
returns trigger as $$
declare
    v_price numeric;
begin
    if (TG_OP = 'DELETE') then
        delete from normalized_liquidity
        where venue_type = 'amm' and venue_ref = OLD.pool_address;
        return OLD;
    else
        v_price := NEW.reserve_buying / nullif(NEW.reserve_selling, 0);
        
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
            updated_at
        )
        values (
            'amm',
            NEW.pool_address,
            NEW.selling_asset_id,
            NEW.buying_asset_id,
            v_price,
            NEW.reserve_selling,
            (v_price * 10000000)::bigint,
            (NEW.reserve_selling * 10000000)::bigint,
            NEW.last_updated_ledger,
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
            updated_at = EXCLUDED.updated_at;
        return NEW;
    end if;
end;
$$ language plpgsql;

-- 4. Create triggers
drop trigger if exists trg_sync_normalized_sdex on sdex_offers;
create trigger trg_sync_normalized_sdex
after insert or update or delete on sdex_offers
for each row execute function sync_normalized_liquidity_from_sdex();

drop trigger if exists trg_sync_normalized_amm on amm_pool_reserves;
create trigger trg_sync_normalized_amm
after insert or update or delete on amm_pool_reserves
for each row execute function sync_normalized_liquidity_from_amm();

-- 5. Initial Backfill
insert into normalized_liquidity (
    venue_type, venue_ref, selling_asset_id, buying_asset_id, 
    price, available_amount, price_e7, available_amount_e7, 
    source_ledger, updated_at
)
select 
    'sdex', offer_id::text, selling_asset_id, buying_asset_id, 
    price, amount, (price * 10000000)::bigint, (amount * 10000000)::bigint, 
    last_modified_ledger, updated_at
from sdex_offers
on conflict do nothing;

insert into normalized_liquidity (
    venue_type, venue_ref, selling_asset_id, buying_asset_id, 
    price, available_amount, price_e7, available_amount_e7, 
    source_ledger, updated_at
)
select 
    'amm', pool_address, selling_asset_id, buying_asset_id, 
    (reserve_buying / nullif(reserve_selling, 0)), reserve_selling, 
    ((reserve_buying / nullif(reserve_selling, 0)) * 10000000)::bigint, 
    (reserve_selling * 10000000)::bigint, 
    last_updated_ledger, updated_at
from amm_pool_reserves
where reserve_selling > 0
on conflict do nothing;
