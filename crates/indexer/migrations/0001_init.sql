-- StellarRoute - Phase 1.2
-- Initial schema for SDEX indexing

create extension if not exists "uuid-ossp";

create table if not exists assets (
  id uuid primary key default uuid_generate_v4(),
  asset_type text not null, -- "native" | "credit_alphanum4" | "credit_alphanum12"
  asset_code text null,
  asset_issuer text null,
  created_at timestamptz not null default now(),
  unique (asset_type, asset_code, asset_issuer)
);

create table if not exists sdex_offers (
  offer_id bigint primary key, -- Horizon offer id
  seller text not null,
  selling_asset_id uuid not null references assets(id),
  buying_asset_id uuid not null references assets(id),
  amount numeric(30, 14) not null,
  price numeric(30, 14) not null,
  price_n bigint null,
  price_d bigint null,
  last_modified_ledger bigint not null,
  paging_token text null,
  updated_at timestamptz not null default now()
);

create index if not exists idx_sdex_offers_pair
  on sdex_offers (selling_asset_id, buying_asset_id);

create table if not exists ingestion_state (
  key text primary key,
  value text not null,
  updated_at timestamptz not null default now()
);

