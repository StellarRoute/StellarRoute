# Multi-Region Read Replica Operational Runbook

**Version**: 1.0  
**Last Updated**: 2026-03-25  
**Status**: Production Ready  

## Executive Summary

This runbook provides operational guidance for the multi-region read replica system supporting the StellarRoute quote API. The system automatically routes read requests across three regional PostgreSQL replicas (US-East primary, EU-West secondary, AP-Southeast tertiary) with sophisticated health monitoring and failover strategies.

**Key Capabilities**:
- Automatic failover across regions with <500ms latency impact
- Circuit breakers prevent cascade failures
- Data version tracking prevents split-brain scenarios
- Configurable consistency constraints (strong, eventual, session)
- Comprehensive health metrics and alerting

---

## Architecture Overview

### Regional Replica Configuration

| Region | Database URL Var | Priority | Max Lag | Max Staleness | Pool Size |
|--------|------------------|----------|---------|---------------|-----------|
| **US-East** (Primary) | `DATABASE_URL` | 0 | 5s | 10s | 5 |
| **EU-West** (Secondary) | `DATABASE_URL_EU_WEST` | 1 | 5s | 10s | 5 |
| **AP-Southeast** (Tertiary) | `DATABASE_URL_AP_SOUTHEAST` | 2 | 5s | 10s | 5 |

### Failover Chain

```
Quote Request
    ↓
[Consistency Constraint Check]
    ↓
Try Primary (US-East)
    ↓ Success? → Return with decision metadata
    ✗ Failure
    ↓
Try Secondary (EU-West) 
    ↓ Success? → Return (marked as fallback)
    ✗ Failure
    ↓
Try Tertiary (AP-Southeast)
    ↓ Success? → Return (marked as fallback)
    ✗ Failure
    ↓
Return 500 Internal Server Error
```

### Consistency Model

Three consistency levels available via request-level configuration:

1. **Strong Consistency**
   - Max age: 1 second
   - Requires primary region
   - No degraded regions accepted
   - Use for: Critical price updates, large trades

2. **Session Consistency** (Default)
   - Max age: Configurable (default 10s)
   - Prefers primary but allows secondary
   - No degraded regions accepted
   - Use for: Standard quote requests

3. **Eventual Consistency**
   - Max age: Configurable (default 60s)
   - Accepts any healthy or degraded region
   - Best availability
   - Use for: Informational quotes, low-value trades

---

## Monitoring and Alerting

### Health Check Metrics

The system continuously monitors each region:

```
Health Check Interval: 3 seconds
Circuit Breaker Threshold: 3 consecutive failures
Circuit Breaker Timeout: 30 seconds
```

**Health Statuses**:
- 🟢 **Healthy**: Responding normally, lag < threshold
- 🟡 **Degraded**: Slow responses OR replica lag > threshold
- 🔴 **Unhealthy**: Failing health checks
- ⚫ **CircuitOpen**: Failing too consistently, circuit breaker active

### Key Metrics to Monitor

```sql
-- Check current health status of all regions
SELECT 
    region_id,
    status,
    consecutive_failures,
    avg_response_time_ms,
    replica_lag_secs
FROM regional_health_snapshots()
ORDER BY priority;

-- Routing statistics
SELECT 
    total_decisions,
    primary_percentage,
    fallback_percentage,
    circuit_breaker_blocks
FROM routing_metrics();

-- Version divergence detection
SELECT 
    current_version,
    version_drift,
    is_converged
FROM version_status();
```

### Alert Conditions

**CRITICAL**: Immediate escalation
- All regions circuit open (complete outage)
- Version drift > 100 ledgers (split-brain risk)
- Primary region unhealthy >5 minutes

**WARNING**: Investigate within 15 minutes
- Any region circuit open (>10 minutes)
- Replica lag > 30 seconds
- Any region degraded >5 minutes
- Fallback ratio > 20% in 5min window

**INFO**: Informational, no action required
- Fallback occurred
- Health transition between states
- Replica lag > threshold but < 30s

---

## Failover Procedures

### Scenario 1: Primary Region Failure

**Symptoms**:
- Quote requests returning 5XX errors
- `routing_metrics().primary_percentage` → 0%
- US-East region status: CircuitOpen

**Diagnosis**:
```bash
# Check primary region health
curl -s http://stellar-route-api/health/regions | jq '.regions[] | select(.region_id == "us-east")'

# Check query on primary (attempt direct connection)
psql $DATABASE_URL -c "SELECT COUNT(*) FROM sdex_offers;"

# Check replication on secondary
psql $DATABASE_URL_EU_WEST -c "SELECT slot_name, restart_lsn FROM pg_replication_slots;"
```

**Recovery Steps**:

1. **Immediate** (0-5 minutes):
   - Confirm primary database is down (not just network issue)
   - Check AWS health dashboard for region issues
   - Verify secondary regions receiving traffic (fallback working)

2. **Short-term** (5-15 minutes):
   - If temporary: Wait for automatic recovery. Circuit opens for 30 seconds, then retries with half-open state.
   - If network issue: Check security groups, RDS endpoint accessibility
   - Restart primary RDS instance if frozen

3. **Long-term** (>15 minutes):
   - Initiate failover: Promote EU-West replica to primary
   - Update DNS/connection strings to point to EU-West
   - Rebuild US-East as new secondary (from EU-West backup)
   - Verify version convergence before re-enabling

**Rollback** (if service restored):
```bash
# Once US-East is healthy again
1. Point connection pool back to US-East
2. Allow 60 seconds for replication catch-up
3. Monitor version_drift() - should be < 10 ledgers
4. If drift > 10: Re-sync EU-West from US-East backup
5. Verify routing metrics: primary_percentage → 100%
```

**Expected Impact**:
- Latency increase: +50-200ms (routing to EU-West)
- Availability: Maintained (replicas healthy)
- User impact: Transparent (fallback automatic)

### Scenario 2: Replica Lag Spike

**Symptoms**:
- `health_snapshots().replica_lag_secs` → 15+ seconds
- Primary region status: Degraded
- Quote requests slow but succeeding

**Diagnosis**:
```bash
# Check replication status on secondary
psql $DATABASE_URL_EU_WEST -c "
  SELECT 
    client_addr,
    state,
    sync_state,
    write_lag,
    flush_lag,
    replay_lag
  FROM pg_stat_replication;
"

# Check query load on primary
psql $DATABASE_URL -c "SELECT usename, query_start, query FROM pg_stat_activity WHERE wait_event IS NOT NULL LIMIT 5;"

# Check disk I/O on primary
# AWS RDS: Check "Disk I/O" metric in CloudWatch
```

**Recovery Steps**:

1. **Immediate**:
   - Reduce write load on primary if possible (pause indexer)
   - Scale up read replicas' compute (increase DB parameter group buffer_pool)

2. **Investigation**:
   - Check if specific queries blocking replication (long-running transactions)
   - Verify network bandwidth between regions
   - Check autovacuum progress: `SHOW autovacuum;`

3. **Resolution**:
   ```bash
   # If transactions blocking replication
   psql $DATABASE_URL -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE backend_xip_xid != '0' ORDER BY xact_start LIMIT 1;"
   
   # If autovacuum slow, kickstart manually
   psql $DATABASE_URL -c "VACUUM ANALYZE normalized_liquidity;"
   
   # Monitor catch-up
   watch -n 1 'psql $DATABASE_URL_EU_WEST -c "SELECT replay_lag FROM pg_stat_replication;"'
   ```

**Expected Impact**:
- User impact: None if lag < max_staleness (10s default)
- Quote quality: Acceptable if within staleness threshold
- Requests may get routed to secondary earlier than usual

### Scenario 3: Version Divergence (Split-Brain Risk)

**Symptoms**:
- `version_drift()` > 100 ledgers
- `is_converged(tolerance=10)` → False
- Version tracker warning in logs

**Critical**: This indicates potential data inconsistency. Pause increased traffic immediately.

**Diagnosis**:
```bash
# Check ledger sequences across all regions
echo "US-East:" && psql $DATABASE_URL -c "SELECT MAX(source_ledger) FROM sdex_offers;"
echo "EU-West:" && psql $DATABASE_URL_EU_WEST -c "SELECT MAX(source_ledger) FROM sdex_offers;"
echo "AP-Southeast:" && psql $DATABASE_URL_AP_SOUTHEAST -c "SELECT MAX(source_ledger) FROM sdex_offers;"

# Check replication status (is replica catching up?)
psql $DATABASE_URL_EU_WEST -c "
  SELECT 
    lsn_distance(pg_current_wal_insert_lsn(), replay_lsn) AS lag_bytes,
    replay_lag
  FROM pg_stat_replication;
"
```

**Recovery Steps**:

1. **CRITICAL - Stop further divergence**:
   ```bash
   # Pause indexer on primary to halt writes
   # This allows replicas to catch up without new changes
   ```

2. **Verify data integrity**:
   ```bash
   # Run consistency check
   SELECT 
     (SELECT COUNT(*) FROM sdex_offers) as primary_rows,
     COUNT(*) as eu_west_rows
   FROM 
     dblink('dbname=postgres host=eu-west-replica user=app', 
            'SELECT * FROM sdex_offers') 
   AS t(offer_id bigint, ...);
   ```

3. **If data matches**: 
   - Resume indexer, monitor convergence
   - Should converge within 60 seconds

4. **If data differs**:
   - 🛑 STOP: Do not proceed without investigation
   - Engage database team: possible corruption or silent failure
   - May require point-in-time recovery (PITR)

**Expected Impact**:
- Availability impact: Service paused for investigation
- Duration: 5-30 minutes depending on root cause
- Recovery method: Likely PITR to consistent snapshot

---

## Configuration Changes

### Updating Consistency Constraint

**For normal operation** (recommended):
```rust
let constraint = ConsistencyConstraint::session(10); // 10 second staleness max
```

**For critical operations**:
```rust
let constraint = ConsistencyConstraint::strong(); // 1 second, primary only
```

**To enable degraded replicas** (maximum availability):
```rust
let constraint = ConsistencyConstraint::eventual(60); // Accept 60s old data
```

### Scaling Connection Pools

Adjust per-region in configuration:

```env
# Max connections per region (default: 5)
# For high-load scenarios
REGION_POOL_SIZE=10

# Health check interval (default: 3 seconds)
REGION_HEALTH_CHECK_INTERVAL_SECS=5

# Circuit breaker settings
REGION_CIRCUIT_BREAKER_THRESHOLD=5      # Default: 3
REGION_CIRCUIT_BREAKER_TIMEOUT_SECS=60  # Default: 30
```

### Disabling a Region Temporarily

Set environment variable to disable reads from a specific region:

```bash
# Disable EU-West (e.g., for maintenance)
REGION_EU_WEST_ENABLED=false

# Restart service
systemctl restart stellarroute-api
```

All reads will attempt primary, then fall back to AP-Southeast.

---

## Testing and Validation

### Test 1: Normal Failover

```bash
# Simulate primary failure by blocking traffic
iptables -I INPUT -s <primary-ip> -j DROP

# Observe:
# - Requests still succeed (routing to secondary)
# - Logs: "Fallback to eu-west"

# Restore
iptables -D INPUT -s <primary-ip> -j DROP

# Observe:
# - Routing returns to primary
# - Logs: "Primary region successful"
```

### Test 2: Cascading Failures

```bash
# Kill all replica connections
pkill -f "replica"

# Observe:
# - All regions report unhealthy
# - Requests begin failing with 500
# - Fallback counters exhausted

# Restore connections
systemctl restart postgresql

# Monitor recovery through health metrics
```

### Test 3: Replica Lag Simulation

```bash
-- On primary, hold transaction to block replication
psql $DATABASE_URL -c "BEGIN; SELECT * FROM sdex_offers LIMIT 1; PERFORM pg_sleep(30); ROLLBACK;"

# Observe:
# - Replica lag increases
# - Secondary status → Degraded
# - Quote requests still route to primary

# Transaction completes, lag catches up
# - Secondary status → Healthy
```

### Test 4: Horizon/Soroban Dependency Outage Simulation

Use the API harness to simulate partial and full upstream dependency outages:

```rust
use stellarroute_api::load_test::{DegradationScenario, HarnessConfig};

// Partial Horizon outage
let partial = HarnessConfig {
    degradation: DegradationScenario {
        horizon_error_rate: 0.5,
        ..Default::default()
    },
    ..Default::default()
};

// Full Horizon + Soroban outage
let full = HarnessConfig {
    degradation: DegradationScenario {
        horizon_error_rate: 1.0,
        soroban_error_rate: 1.0,
        ..Default::default()
    },
    ..Default::default()
};
```

**Observed failure modes**:
- Partial outage: mixed success/failure responses as expected for degraded dependencies.
- Full outage: all requests fail with explicit simulated dependency failure markers.
- Recovery path: clearing outage rates (`0.0`) restores successful request handling.

---

## Runbook Response Time SLAs

| Scenario | Detection | Response | Resolution | Total |
|----------|-----------|----------|-----------|-------|
| Primary failure | 3s | 2m | 5m | <15m |
| Replica lag spike | 3s | 5m | 10m | <30m |
| Version divergence | 5s | 1m | 15m | <45m |
| Network partition | 3s | 3m | 20m | <60m |

---

## Appendix: Useful Queries

### Health Status Overview
```sql
SELECT 
    region_id,
    status,
    consecutive_failures,
    last_success_ts,
    avg_response_time_ms,
    replica_lag_secs
FROM regional_health_snapshots()
ORDER BY priority;
```

### Routing Distribution
```sql
SELECT 
    DATE_TRUNC('minute', event_time) as minute,
    region_id,
    COUNT(*) as request_count
FROM routing_decisions
WHERE event_time > NOW() - INTERVAL '1 hour'
GROUP BY 1, 2
ORDER BY 1 DESC, 3 DESC;
```

### Version Convergence Status
```sql
SELECT 
    region_id,
    ledger_sequence,
    NOW() - last_update_time as age,
    CASE 
        WHEN ABS(MAX(ledger_sequence) OVER () - ledger_sequence) <= 10 THEN 'Converged'
        ELSE 'Diverged: ' || (MAX(ledger_sequence) OVER () - ledger_sequence)
    END as convergence_status
FROM regional_versions;
```

### Alert: Regions Failing Health Checks
```sql
SELECT 
    region_id,
    COUNT(*) as consecutive_failures,
    MAX(failure_time) as last_failure
FROM health_check_log
WHERE status = 'FAILURE'
GROUP BY region_id
HAVING COUNT(*) >= 3
ORDER BY consecutive_failures DESC;
```

---

## Support Escalation

- **Tier 1**: On-call engineer - Follow runbook steps 1-2
- **Tier 2**: Database specialized team - For data integrity issues
- **Tier 3**: Architecture team - For design-level decisions (failover to different region, etc.)

---

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-03-25 | Initial release |
