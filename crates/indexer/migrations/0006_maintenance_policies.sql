-- StellarRoute - Phase 1.6
-- Maintenance policies for snapshot compaction and data retention

-- Function to compact orderbook snapshots
-- Keeps only one snapshot per hour for data older than threshold_hours
-- Everything older than retention_days is deleted
create or replace function compact_orderbook_snapshots(
    p_threshold_hours integer default 24,
    p_retention_days integer default 90
)
returns integer as $$
declare
    v_compacted_count integer := 0;
    v_deleted_count integer := 0;
begin
    -- 1. Compaction: For snapshots older than threshold but younger than retention
    -- We keep the snapshot closest to the start of each hour for each trading pair
    with snapshots_to_compact as (
        select id
        from (
            select 
                id,
                trading_pair_id,
                snapshot_time,
                row_number() over (
                    partition by trading_pair_id, date_trunc('hour', snapshot_time) 
                    order by snapshot_time asc
                ) as rank
            from orderbook_snapshots
            where snapshot_time < now() - interval '1 hour' * p_threshold_hours
              and snapshot_time >= now() - interval '1 day' * p_retention_days
        ) s
        where rank > 1
    )
    delete from orderbook_snapshots
    where id in (select id from snapshots_to_compact);
    
    get diagnostics v_compacted_count = row_count;

    -- 2. Retention: Delete everything older than retention_days
    delete from orderbook_snapshots
    where snapshot_time < now() - interval '1 day' * p_retention_days;
    
    get diagnostics v_deleted_count = row_count;

    return v_compacted_count + v_deleted_count;
end;
$$ language plpgsql;

-- Function to apply general retention policies for other tables
create or replace function apply_retention_policies()
returns table (
    table_name text,
    deleted_count bigint
) as $$
begin
    -- Venue health scores: keep 7 days
    delete from venue_health_scores
    where computed_at < now() - interval '7 days';
    get diagnostics deleted_count = row_count;
    table_name := 'venue_health_scores';
    return next;

    -- Route computation jobs: keep 3 days for completed/failed jobs
    delete from route_computation_jobs
    where (status = 'completed' or status = 'failed')
      and updated_at < now() - interval '3 days';
    get diagnostics deleted_count = row_count;
    table_name := 'route_computation_jobs';
    return next;
end;
$$ language plpgsql;

-- Comments
comment on function compact_orderbook_snapshots is 'Reduces snapshot density for older data and enforces max retention';
comment on function apply_retention_policies is 'Enforces time-based retention for health scores and worker jobs';
