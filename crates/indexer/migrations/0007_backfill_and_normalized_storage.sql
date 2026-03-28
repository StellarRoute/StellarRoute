-- StellarRoute - Phase 1.5/2.1
-- Unified liquidity surface with e7 precision for routing engine performance

-- 1. Create a table for backfill checkpoints to track resumable jobs
create table if not exists backfill_checkpoints (
  job_name text primary key,
  last_processed_id bigint not null default 0,
  batch_size integer not null default 1000,
  status text not null default 'idle', -- 'running', 'paused', 'completed', 'error'
  last_error text,
  updated_at timestamptz not null default now()
);

-- 2. Create a table for normalized liquidity storage (replaces/complements the view)
-- This table stores pre-calculated price_e7 and amount_e7 for the routing engine.
create table if not exists normalized_liquidity (
  venue_type text not null, -- 'sdex' | 'amm'
  venue_ref text not null,  -- offer_id | pool_address
  selling_asset_id uuid not null references assets(id),
  buying_asset_id uuid not null references assets(id),
  price numeric(30, 14) not null,
  available_amount numeric(30, 14) not null,
  -- Optimized for routing engine:
  price_e7 bigint not null,
  available_amount_e7 bigint not null,
  source_ledger bigint not null,
  updated_at timestamptz not null default now(),
  primary key (venue_type, venue_ref)
);

-- 3. Add indexes for rapid routing lookups
create index if not exists idx_normalized_liquidity_pair_price
  on normalized_liquidity (selling_asset_id, buying_asset_id, price_e7 asc);

create index if not exists idx_normalized_liquidity_updated
  on normalized_liquidity (updated_at desc);

-- 4. Function to update checkpoints
create or replace function update_backfill_checkpoint(
  p_job_name text,
  p_last_id bigint,
  p_status text,
  p_error text default null
)
returns void as $$
begin
  insert into backfill_checkpoints (job_name, last_processed_id, status, last_error, updated_at)
  values (p_job_name, p_last_id, p_status, p_error, now())
  on conflict (job_name)
  do update set
    last_processed_id = excluded.last_processed_id,
    status = excluded.status,
    last_error = excluded.last_error,
    updated_at = now();
end;
$$ language plpgsql;

-- 5. Add comments
comment on table backfill_checkpoints is 'Tracks progress of long-running data migrations and normalization jobs';
comment on table normalized_liquidity is 'Unified price and amount storage with pre-calculated e7 precision for routing engine performance';
comment on column normalized_liquidity.price_e7 is 'Price scaled by 10^7 (bigint) for fast pathfinding comparisons';
comment on column normalized_liquidity.available_amount_e7 is 'Amount scaled by 10^7 (bigint) for fast pathfinding calculations';
