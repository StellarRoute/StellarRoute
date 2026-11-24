-- StellarRoute - Phase 1.6
-- Reconciliation engine tables for drift detection and repair tracking

create table if not exists reconciliation_thresholds (
  check_type text primary key,
  staleness_threshold_secs integer,
  price_divergence_pct numeric(10, 4),
  liquidity_change_pct numeric(10, 4),
  ledger_lag_threshold integer,
  enabled boolean not null default true,
  updated_at timestamptz not null default now()
);

insert into reconciliation_thresholds (check_type, staleness_threshold_secs, enabled)
values ('data_staleness', 300, true)
on conflict (check_type) do nothing;

insert into reconciliation_thresholds (check_type, price_divergence_pct, enabled)
values ('price_divergence', 2.5, true)
on conflict (check_type) do nothing;

insert into reconciliation_thresholds (check_type, liquidity_change_pct, enabled)
values ('liquidity_anomaly', 15.0, true)
on conflict (check_type) do nothing;

insert into reconciliation_thresholds (check_type, ledger_lag_threshold, enabled)
values ('ledger_alignment', 100, true)
on conflict (check_type) do nothing;

create table if not exists reconciliation_checks (
  id uuid primary key default uuid_generate_v4(),
  check_type text not null,
  entity_type text not null,
  entity_ref text not null,
  expected_value jsonb not null default '{}'::jsonb,
  actual_value jsonb not null default '{}'::jsonb,
  drift_severity text not null,
  drift_percentage numeric(12, 4),
  extra_context jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now()
);

create index if not exists idx_reconciliation_checks_type_time
  on reconciliation_checks (check_type, created_at desc);

create index if not exists idx_reconciliation_checks_severity_time
  on reconciliation_checks (drift_severity, created_at desc);

create table if not exists drift_events (
  id uuid primary key default uuid_generate_v4(),
  check_id uuid references reconciliation_checks(id) on delete set null,
  entity_type text not null,
  entity_ref text not null,
  drift_category text not null,
  metric_name text not null,
  metric_value numeric(12, 4),
  metric_unit text,
  threshold_value numeric(12, 4),
  breach boolean not null default false,
  metadata jsonb not null default '{}'::jsonb,
  recorded_at timestamptz not null default now()
);

create index if not exists idx_drift_events_category_time
  on drift_events (drift_category, recorded_at desc);

create table if not exists repair_actions (
  id uuid primary key default uuid_generate_v4(),
  check_id uuid references reconciliation_checks(id) on delete set null,
  action_type text not null,
  entity_type text not null,
  entity_ref text not null,
  reason text not null,
  action_details jsonb not null default '{}'::jsonb,
  success boolean not null default false,
  error_message text,
  affected_rows integer not null default 0,
  created_at timestamptz not null default now(),
  executed_at timestamptz not null default now()
);

create index if not exists idx_repair_actions_type_time
  on repair_actions (action_type, executed_at desc);

create table if not exists reconciliation_runs (
  id uuid primary key default uuid_generate_v4(),
  run_started_at timestamptz not null,
  run_completed_at timestamptz not null,
  checks_requested integer not null default 0,
  checks_executed integer not null default 0,
  checks_passed integer not null default 0,
  checks_failed integer not null default 0,
  total_drift_events integer not null default 0,
  critical_drift_events integer not null default 0,
  total_repairs_attempted integer not null default 0,
  successful_repairs integer not null default 0,
  failed_repairs integer not null default 0,
  duration_ms bigint not null default 0
);

create index if not exists idx_reconciliation_runs_started
  on reconciliation_runs (run_started_at desc);

-- Reserve history used to detect sudden liquidity changes between updates
create table if not exists amm_pool_reserve_history (
  id uuid primary key default uuid_generate_v4(),
  pool_address text not null,
  reserve_selling numeric(38, 18) not null check (reserve_selling > 0),
  reserve_buying numeric(38, 18) not null check (reserve_buying > 0),
  recorded_at timestamptz not null default now()
);

create index if not exists idx_amm_pool_reserve_history_pool_time
  on amm_pool_reserve_history (pool_address, recorded_at desc);
