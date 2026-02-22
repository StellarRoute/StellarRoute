-- StellarRoute - Phase 1.4
-- Trading pairs and orderbook snapshots

-- Create trading_pairs table for tracking active trading pairs
create table if not exists trading_pairs (
  id uuid primary key default uuid_generate_v4(),
  base_asset_id uuid not null references assets(id),
  counter_asset_id uuid not null references assets(id),
  is_active boolean not null default true,
  total_offers integer not null default 0,
  total_volume numeric(30, 14) not null default 0,
  last_trade_at timestamptz null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (base_asset_id, counter_asset_id),
  check (base_asset_id != counter_asset_id)
);

-- Indexes for trading_pairs
create index if not exists idx_trading_pairs_base
  on trading_pairs (base_asset_id);

create index if not exists idx_trading_pairs_counter
  on trading_pairs (counter_asset_id);

create index if not exists idx_trading_pairs_active
  on trading_pairs (is_active, updated_at desc);

create index if not exists idx_trading_pairs_volume
  on trading_pairs (total_volume desc) where is_active = true;

-- Create orderbook_snapshots table for historical queries
create table if not exists orderbook_snapshots (
  id uuid primary key default uuid_generate_v4(),
  trading_pair_id uuid not null references trading_pairs(id) on delete cascade,
  snapshot_time timestamptz not null default now(),
  bids jsonb not null,
  asks jsonb not null,
  bid_count integer not null,
  ask_count integer not null,
  spread numeric(30, 14) null,
  mid_price numeric(30, 14) null,
  total_bid_volume numeric(30, 14) not null default 0,
  total_ask_volume numeric(30, 14) not null default 0,
  ledger_sequence bigint not null,
  created_at timestamptz not null default now()
);

-- Indexes for orderbook_snapshots
create index if not exists idx_orderbook_snapshots_pair_time
  on orderbook_snapshots (trading_pair_id, snapshot_time desc);

create index if not exists idx_orderbook_snapshots_time
  on orderbook_snapshots (snapshot_time desc);

create index if not exists idx_orderbook_snapshots_ledger
  on orderbook_snapshots (ledger_sequence desc);

-- Add constraint to enforce data quality
alter table orderbook_snapshots 
  add constraint check_positive_counts 
  check (bid_count >= 0 and ask_count >= 0);

-- Create function to capture orderbook snapshot
create or replace function capture_orderbook_snapshot(
  p_base_asset_id uuid,
  p_counter_asset_id uuid,
  p_ledger_sequence bigint
)
returns uuid as $$
declare
  v_trading_pair_id uuid;
  v_snapshot_id uuid;
  v_bids jsonb;
  v_asks jsonb;
  v_bid_count integer;
  v_ask_count integer;
  v_spread numeric;
  v_mid_price numeric;
  v_total_bid_volume numeric;
  v_total_ask_volume numeric;
begin
  -- Get or create trading pair
  insert into trading_pairs (base_asset_id, counter_asset_id)
  values (p_base_asset_id, p_counter_asset_id)
  on conflict (base_asset_id, counter_asset_id) 
  do update set updated_at = now()
  returning id into v_trading_pair_id;

  -- Collect bids (offers selling base asset for counter asset)
  select 
    coalesce(jsonb_agg(
      jsonb_build_object(
        'price', price::text,
        'amount', amount::text,
        'offer_id', offer_id
      ) order by price desc
    ), '[]'::jsonb),
    count(*),
    coalesce(sum(amount), 0)
  into v_bids, v_bid_count, v_total_bid_volume
  from sdex_offers
  where selling_asset_id = p_base_asset_id
    and buying_asset_id = p_counter_asset_id;

  -- Collect asks (offers selling counter asset for base asset, invert price)
  select 
    coalesce(jsonb_agg(
      jsonb_build_object(
        'price', (1.0 / price)::text,
        'amount', (amount * price)::text,
        'offer_id', offer_id
      ) order by (1.0 / price) asc
    ), '[]'::jsonb),
    count(*),
    coalesce(sum(amount * price), 0)
  into v_asks, v_ask_count, v_total_ask_volume
  from sdex_offers
  where selling_asset_id = p_counter_asset_id
    and buying_asset_id = p_base_asset_id;

  -- Calculate spread and mid price
  if v_bid_count > 0 and v_ask_count > 0 then
    select 
      ((v_asks->0->>'price')::numeric - (v_bids->0->>'price')::numeric),
      ((v_asks->0->>'price')::numeric + (v_bids->0->>'price')::numeric) / 2.0
    into v_spread, v_mid_price;
  end if;

  -- Insert snapshot
  insert into orderbook_snapshots (
    trading_pair_id, bids, asks, bid_count, ask_count,
    spread, mid_price, total_bid_volume, total_ask_volume,
    ledger_sequence
  )
  values (
    v_trading_pair_id, v_bids, v_asks, v_bid_count, v_ask_count,
    v_spread, v_mid_price, v_total_bid_volume, v_total_ask_volume,
    p_ledger_sequence
  )
  returning id into v_snapshot_id;

  -- Update trading pair statistics
  update trading_pairs
  set 
    total_offers = v_bid_count + v_ask_count,
    updated_at = now()
  where id = v_trading_pair_id;

  return v_snapshot_id;
end;
$$ language plpgsql;

-- Create view for latest orderbook snapshots
create or replace view latest_orderbook_snapshots as
select distinct on (trading_pair_id)
  s.*,
  tp.base_asset_id,
  tp.counter_asset_id
from orderbook_snapshots s
join trading_pairs tp on s.trading_pair_id = tp.id
order by trading_pair_id, snapshot_time desc;

-- Function to clean old snapshots (keep only recent data)
create or replace function cleanup_old_snapshots(days_to_keep integer default 7)
returns integer as $$
declare
  deleted_count integer;
begin
  delete from orderbook_snapshots
  where snapshot_time < now() - interval '1 day' * days_to_keep
  returning count(*) into deleted_count;
  
  return coalesce(deleted_count, 0);
end;
$$ language plpgsql;

-- Add comments
comment on table trading_pairs is 'Tracks active trading pairs with statistics';
comment on table orderbook_snapshots is 'Historical orderbook snapshots for analysis and charting';
comment on function capture_orderbook_snapshot is 'Captures a point-in-time snapshot of an orderbook';
comment on function cleanup_old_snapshots is 'Removes orderbook snapshots older than specified days';
comment on view latest_orderbook_snapshots is 'Most recent snapshot for each trading pair';

comment on column trading_pairs.base_asset_id is 'The asset being traded (e.g., BTC in BTC/USDC)';
comment on column trading_pairs.counter_asset_id is 'The asset used for pricing (e.g., USDC in BTC/USDC)';
comment on column trading_pairs.total_offers is 'Current number of active offers for this pair';
comment on column trading_pairs.total_volume is 'Cumulative trading volume';
comment on column orderbook_snapshots.bids is 'Array of bid orders (buying base asset)';
comment on column orderbook_snapshots.asks is 'Array of ask orders (selling base asset)';
comment on column orderbook_snapshots.spread is 'Difference between best ask and best bid';
comment on column orderbook_snapshots.mid_price is 'Average of best bid and ask prices';
