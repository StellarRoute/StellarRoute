# Stateful Market-Data Reconciliation Engine

**Status**: Phase 1.6 - Data Quality & Reconciliation  
**Issue**: #160 - Stateful market-data reconciliation between Horizon and Soroban RPC  
**Acceptance Criteria**: All 5 implemented and tested

## Executive Summary

The reconciliation engine continuously monitors consistency between Horizon-indexed SDEX orderbook data and Soroban RPC AMM pool state. It detects drifts (mismatches) across five dimensions, emits metrics for operational visibility, and automatically repairs critical issues.

**Key Capabilities:**
- 🔍 **Five consistency checks** with configurable thresholds
- 📊 **Drift metrics** for monitoring and alerting
- 🔧 **Automatic repair workflows** for critical issues
- 📝 **Complete audit trail** of all checks, drifts, and repairs
- ⚠️ **Severity-based response** (info/warning/critical)

## Architecture

### Consistency Checks

The engine implements five orthogonal checks:

#### 1. **Asset Mapping** (Always-On)
Ensures bidirectional integrity of asset references.

**What it checks:**
- All SDEX offers reference existing assets
- All AMM pools reference existing assets
- No orphaned foreign keys

**Severity:**
- 🔴 **Critical** if any references are missing (data corruption risk)

**Repair Action:**
- Log issue and invalidate affected records
- Alert operator for investigation

**Example Query:**
```sql
SELECT entity_type, COUNT(*)
FROM reconciliation_checks
WHERE check_type = 'asset_mapping' 
  AND drift_severity = 'critical'
  AND created_at > NOW() - INTERVAL '24 hours'
```

---

#### 2. **Data Staleness** (Every Cycle)
Detects when data hasn't been updated in configured time window.

**Thresholds:**
- `staleness_threshold_secs`: 300 seconds (5 minutes, configurable)

**Severity:**
- 🟡 **Warning** for SDEX offers (orderbook is active, indirection acceptable)
- 🔴 **Critical** for AMM pools (liquidity operations are real-time)

**Example Scenario:**
```
SDEX indexer crashes at 14:00
At 14:05, reconciliation detects offers not updated (staleness = 5 min)
→ Warning raised, Horizon client restarted
At 14:06, fresh offers indexed, staleness resolved
```

**Repair Action:**
- Mark entity for refetch from source
- Re-fetch is performed by normal indexing loop

---

#### 3. **Price Divergence** (Every Cycle)
Detects when same trading pair prices differ between venues.

**Business Logic:**
For each (selling_asset, buying_asset) pair:
1. Query best SDEX price: `MAX(sdex_offers.price)`
2. Query AMM price: `reserve_buying / reserve_selling`
3. Calculate divergence: `|amm_price - sdex_price| / sdex_price * 100%`

**Thresholds:**
- `price_divergence_pct`: 2.5% (configurable)
- 🟡 **Warning** if 2.5% < divergence < 5.0%
- 🔴 **Critical** if divergence > 5.0%

**Example Scenario:**
```
SDEX best offer: XLM/USDC @ 0.250
AMM pool ratio:  XLM/USDC @ 0.242 (3.2% lower)
→ Price divergence detected (above 2.5% threshold)
→ Indicates possible arbitrage opportunity or market imbalance
→ Alert operations team for investigation
```

**Repair Action:**
- 🟡 **Warning → Alert operator** (price discrepancies usually indicate market conditions, not errors)
- 🔴 **Critical → Alert operator** (may indicate data quality issue)

**Note:** Price divergence typically indicates legitimate market conditions (arbs, spreads) rather than data errors. It's monitored for:
- Arbitrage opportunities (routing optimization)
- Market imbalances (liquidity analysis)
- Data quality issues (if extreme)

---

#### 4. **Liquidity Anomalies** (Every Cycle)
Detects sudden changes in available liquidity (reserves draining).

**Business Logic:**
For each AMM pool, track reserve changes between consecutive updates:
```
change_pct = |current_reserve - previous_reserve| / previous_reserve * 100%
```

**Thresholds:**
- `liquidity_change_pct`: 15.0% (configurable)
- 🟡 **Warning** if 15% < change < 30%
- 🔴 **Critical** if change > 30%

**Example Scenarios:**
```
✅ Normal: Pool reserves stable day-to-day
⚠️ Warning: Single trade removes 20% of reserve (unusual but possible)
🔴 Critical: Reserve drops 50% (drain event or hack?)
```

**Repair Action:**
- Alert operator immediately (potential security issue)
- Refetch latest pool state to verify

---

#### 5. **Ledger Alignment** (Every Cycle)
Monitors ledger sequence drift between indexing services.

**Business Logic:**
```
sdex_ledger = MAX(sdex_offers.last_modified_ledger)
amm_ledger = MAX(amm_pool_reserves.last_updated_ledger)
lag = |sdex_ledger - amm_ledger|
```

**Thresholds:**
- `ledger_lag_threshold`: 100 blocks (configurable)
- 🟡 **Warning** if 100 < lag < 200 blocks
- 🔴 **Critical** if lag > 200 blocks

**Example Scenario:**
```
Horizon indexer is current at ledger 50000
Soroban indexer fell behind at ledger 49700
Lag = 300 blocks → Critical
→ Indicates polling interval mismatch or network issue
→ Refetch data and check indexer health
```

**Repair Action:**
- Refetch stale data from both sources
- Check indexer service health (RPC connection, polling rate)

---

## Drift Metrics & Alerting

### DriftEvent Table Structure

All drift detections are recorded to `drift_events`:

```
┌─────────────────────────────────────────────────────────────┐
│ drift_events                                                │
├─────────────────────────────────────────────────────────────┤
│ id                    UUID PK                               │
│ check_id              UUID FK → reconciliation_checks       │
│ entity_type          TEXT ('sdex_offer', 'amm_pool', ...)  │
│ entity_ref           TEXT (offer_id, pool_address, ...)    │
│ drift_category       TEXT ('price', 'liquidity', ...)      │
│ metric_name          TEXT ('price_divergence_pct', ...)    │
│ metric_value         NUMERIC (actual divergence %)         │
│ threshold_value      NUMERIC (configured threshold)         │
│ breach               BOOLEAN (metric > threshold)           │
│ recorded_at          TIMESTAMPTZ                            │
└─────────────────────────────────────────────────────────────┘
```

### Sample Queries for Operations

**Recent drift summary (last 6 hours):**
```sql
SELECT
  drift_category,
  COUNT(*) as total_events,
  COUNT(*) FILTER (WHERE breach) as breaches,
  ROUND(AVG(metric_value), 2) as avg_value,
  MAX(recorded_at) as latest
FROM drift_events
WHERE recorded_at > NOW() - INTERVAL '6 hours'
GROUP BY drift_category
ORDER BY breaches DESC;
```

**Example Output:**
```
drift_category | total_events | breaches | avg_value | latest
───────────────┼──────────────┼──────────┼───────────┼────────────────
liquidity      | 142          | 5        | 8.3%      | 2026-03-25 15:42
staleness      | 89           | 2        | 4.2%      | 2026-03-25 15:48
price          | 312          | 0        | 1.8%      | 2026-03-25 15:50
ledger         | 0            | 0        | NULL      | (none)
asset_mapping  | 0            | 0        | NULL      | (none)
```

**Critical issues requiring immediate attention:**
```sql
SELECT
  c.id,
  c.check_type,
  c.entity_type || ':' || c.entity_ref as entity,
  c.drift_percentage,
  c.created_at,
  COUNT(r.id) as repair_attempts,
  COUNT(r.id) FILTER (WHERE r.success) as successful
FROM critical_issues c
LEFT JOIN repair_actions r ON c.id = r.check_id
GROUP BY c.id, c.check_type, c.entity_type, c.entity_ref, c.drift_percentage, c.created_at
ORDER BY c.created_at DESC;
```

---

## Conflict Resolution Strategy

### Decision Matrix

The engine uses a severity-based decision tree:

```
                              Consistency Check Result
                                      |
                    ┌───────────────┬─┴─┬───────────────┐
                    |               |   |               |
                   INFO          WARNING           CRITICAL
                    |               |               |
         ┌──────────┴────┐  ┌──────┴──────┐  ┌────┴────────┐
         |               |  |             |  |             |
      Log Only      Log + Emit    Alert + Refetch   Repair +
     (No Action)   Metrics      Operator        Alert +
                   (No Repair)                Invalidate
```

### Severity Assignment Rules

**Info** (no action):
- Data passes all consistency checks
- Within configured thresholds

**Warning** (log & monitor):
- Data drift detected but within acceptable bounds
- Examples:
  - Prices diverge 2.5-5% (arbitrage bands)
  - Reserves change 15-30% (active trading)
  - Offers stale 5-10 minutes (normal indexing jitter)

**Critical** (auto-repair):
- Data quality at risk
- Examples:
  - Missing asset references (corruption)
  - Prices diverge >5% (possible data error)
  - Reserves drain >30% (drain event)
  - Ledger lag >200 blocks (indexer failure)
  - Data stale >10 minutes (service failure)

### Repair Actions

| Action Type | Trigger | Mechanism | Rollback |
|---|---|---|---|
| `refetch_soroban` | Stale pools | Touch `updated_at` timestamp | N/A (refetch on next cycle) |
| `refetch_horizon` | Stale offers | Touch `updated_at` timestamp | N/A (refetch on next cycle) |
| `invalidate_record` | Data corruption | Log in `repair_actions` | Manual intervention required |
| `alert_operator` | Warning or too-complex issue | Database entry (for monitoring) | Operator reviews and decides |
| `auto_reconcile` | (future) | Automatic fix when safe | Case-by-case |

---

## Reconciliation Run Workflow

### Typical Execution Flow

```
┌─ Reconciliation Cycle Start (every 60 seconds)
│
├─ Load thresholds from database
│
├─ Run all consistency checks:
│  ├─ Asset Mapping Check → [check_id_1, check_id_2, ...]
│  ├─ Data Staleness Check → [results...]
│  ├─ Price Divergence Check → [results...]
│  ├─ Liquidity Anomalies Check → [results...]
│  └─ Ledger Alignment Check → [results...]
│
├─ For each check result:
│  ├─ Save to `reconciliation_checks` table
│  ├─ Create drift event in `drift_events` table
│  ├─ Update `ReconciliationMetrics` counters
│  │
│  └─ If Severity == Critical:
│     ├─ Determine repair action type
│     ├─ Execute repair (refetch/invalidate/alert)
│     └─ Save repair action to `repair_actions` table
│
├─ Aggregate results:
│  ├─ Checks passed/failed/critical
│  ├─ Total repairs attempted/successful
│  └─ Cycle duration
│
└─ Save summary to `reconciliation_runs`
   └─ Emit metrics to monitoring system
```

### Metrics Emitted

Every reconciliation run produces:

1. **Check Metrics:**
   - Total checks run
   - Pass/fail counts per check type
   - Critical count

2. **Drift Metrics:**
   - Time-series: price_divergence_pct, staleness_secs, etc.
   - Breach events

3. **Repair Metrics:**
   - Repairs attempted
   - Success rate per action type
   - Affected row counts

4. **System Metrics:**
   - Cycle duration (ms)
   - Database query times
   - Error rates

---

## Configuration & Tuning

### Threshold Configuration

All thresholds are stored in `reconciliation_thresholds` and can be adjusted without restart:

```sql
-- View current thresholds
SELECT * FROM reconciliation_thresholds WHERE enabled = true;

-- Make thresholds more lenient (useful during market volatility)
UPDATE reconciliation_thresholds
SET price_divergence_pct = 5.0
WHERE check_type = 'price_divergence';

-- Make thresholds stricter (for production stability)
UPDATE reconciliation_thresholds
SET liquidity_change_pct = 10.0
WHERE check_type = 'liquidity_anomaly';
```

### Recommended Threshold Values

| Parameter | Default | Production | Testing |
|-----------|---------|------------|---------|
| `staleness_threshold_secs` | 300 | 300 | 60 |
| `price_divergence_pct` | 2.5 | 2.5 | 1.0 |
| `liquidity_change_pct` | 15.0 | 15.0 | 10.0 |
| `ledger_lag_threshold` | 100 | 100 | 50 |
| Reconciliation cycle period | 60s | 60s | 10s |

---

## Operational Procedures

### Responding to Critical Drift

**Scenario: Price divergence detected (5% gap)**

1. **Immediate** (Ops Team):
   - Check `critical_issues` view
   - Review divergence metrics and context
   - Verify no data corruption

2. **Investigation** (Engineering):
   - Check Horizon and Soroban RPC API health
   - Review recent data ingestion logs
   - Compare reserve states between venues

3. **Resolution Options**:
   - If data error: Refetch affected records
   - If market condition: Update pricing thresholds
   - If API issue: Restart indexers, increase retry logic

**Scenario: Asset mapping corruption (missing references)**

1. **Immediate** (Ops Team):
   - CRITICAL - data integrity issue
   - Alert engineering and database team

2. **Investigation** (Engineering):
   - Identify which assets are orphaned
   - Check migration history for data loss
   - Review recent schema changes

3. **Resolution**:
   - Restore from backup if necessary
   - Re-sync asset table from Horizon
   - Audit offer/pool tables for consistency

---

## Monitoring & Dashboards

### Key Metrics to Track

**Reconciliation Health:**
- Check pass rate (%)
- Critical drift events (count/hour)
- Repair success rate (%)

**Data Quality:**
- Price divergence (%)
- Liquidity volatility (%)
- Ledger alignment gap (blocks)

**System Performance:**
- Reconciliation cycle time (ms)
- Database query latency
- False positive rate

### Example Prometheus Metrics

```prometheus
# Counter: Total checks performed
stellarroute_reconciliation_checks_total{status="pass|fail|critical"}

# Gauge: Current drift in each category
stellarroute_drift_price_divergence_pct
stellarroute_drift_staleness_secs
stellarroute_drift_liquidity_change_pct

# Counter: Repair attempts and success
stellarroute_repairs_total{type="refetch|invalidate|alert", status="success|failure"}

# Histogram: Reconciliation cycle duration
stellarroute_reconciliation_cycle_duration_ms
```

---

## Testing Approach

### Unit Tests
- Severity ordering and display
- Check type formatting
- Metrics aggregation logic

### Integration Tests
- All five consistency checks in realistic scenarios
- Repair workflow execution and tracking
- Threshold configuration reloading
- Concurrent reconciliation cycles
- Metric accumulation over multiple runs

### Failure Scenarios

1. **Horizon Outage:**
   - Stop SDEX updates for 10+ minutes
   - Verify staleness detection
   - Confirm repair refetch is queued
   - Resume updates, verify recovery

2. **Soroban RPC Outage:**
   - Stop AMM updates for 10+ minutes
   - Verify critical staleness for pools
   - Confirm refetch is queued
   - Resume, verify recovery

3. **Data Corruption:**
   - Delete asset, leave orphaned offers
   - Verify asset_mapping check catches it
   - Confirm operator alert is raised
   - Manually fix, verify reconciliation passes

4. **Market Volatility:**
   - Sudden large trades
   - Verify liquidity anomaly within 30 seconds
   - Confirm operator is notified
   - Verify operational thresholds can be adjusted

---

## Future Enhancements

- [ ] **Auto-reconciliation for price divergence**: Automatically redirect flow to better venue
- [ ] **Machine learning for anomaly detection**: Learn normal patterns, flag unusual events
- [ ] **Cross-venue routing optimization**: Use reconciliation data to improve routing
- [ ] **Liquidity prediction**: Forecast when pools will need rebalancing
- [ ] **Webhook alerts**: Integrate with Slack/PagerDuty/CloudWatch
- [ ] **GraphQL API for historical queries**: Operations dashboard

---

## Appendix: SQL Examples

### Check Repair Effectiveness Over Time

```sql
SELECT
  DATE(executed_at) as date,
  action_type,
  COUNT(*) as total,
  COUNT(*) FILTER (WHERE success) as successful,
  ROUND(100.0 * COUNT(*) FILTER (WHERE success) / COUNT(*), 1) as success_rate
FROM repair_actions
WHERE executed_at > NOW() - INTERVAL '7 days'
GROUP BY 1, 2
ORDER BY 1 DESC, success_rate DESC;
```

### Monthly Drift Summary

```sql
SELECT
  DATE_TRUNC('month', recorded_at) as month,
  drift_category,
  COUNT(*) as total_events,
  COUNT(*) FILTER (WHERE breach) as breach_events,
  ROUND(AVG(metric_value), 2) as avg_metric,
  MAX(metric_value) as max_metric
FROM drift_events
GROUP BY 1, 2
ORDER BY 1 DESC, 2;
```

### Stale Data Timeline

```sql
SELECT
  entity_type,
  COUNT(*) as items,
  MAX(NOW() - updated_at) as stalest_age,
  ROUND(AVG(EXTRACT(EPOCH FROM (NOW() - updated_at)) / 60), 1) as avg_age_minidx
FROM (
  SELECT 'sdex_offer' as entity_type, updated_at FROM sdex_offers
  UNION ALL
  SELECT 'amm_pool', updated_at FROM amm_pool_reserves
)
GROUP BY entity_type;
```
