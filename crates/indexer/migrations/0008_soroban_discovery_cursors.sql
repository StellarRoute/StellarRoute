-- StellarRoute - Phase 2.1
-- Durable cursor state for incremental Soroban pool discovery

create table if not exists soroban_sync_cursors (
  job_name text primary key,
  cursor text not null,
  last_seen_ledger bigint,
  status text not null default 'idle',
  updated_at timestamptz not null default now()
);

comment on table soroban_sync_cursors is 'Durable sync cursors for resumable Soroban discovery jobs';
