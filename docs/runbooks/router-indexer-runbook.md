# Operator Runbook: Testnet Router to Healthy AMM Indexer

This runbook provides the numbered, copy-pasteable steps required to deploy the StellarRoute router contract, register liquidity pools, and verify that the indexer is successfully ingesting AMM state into the database.

## Prerequisites

Before starting, ensure you have reviewed the following reference documents:
- [Deployment Overview](../deployment/README.md)
- [Contract Deployment Runbook](../contracts/deployment-runbook.md)
- Pool configuration files: `config/pools-testnet.json`

Ensure your environment is correctly configured (e.g., `DATABASE_URL`, Soroban CLI tools installed, and testnet deployer identity funded).

## Deployment and Verification Steps

### 1. Deploy the Router Contract
Deploy the router and constant-product adapter contracts to testnet. This will update the `config/deployment-testnet.json` artifact.

```bash
./scripts/deploy.sh --network testnet
```

### 2. Verify Contract Deployment
Ensure the local deployment matches the on-chain state.

```bash
./scripts/verify.sh --network testnet
```

### 3. Register Pools
Register the configured testnet pools with the deployed router contract. Make sure you have updated `config/pools-testnet.json` with real testnet pool contract addresses if necessary.

```bash
./scripts/register-pools.sh --network testnet
```

### 4. Start the Indexer
Run the indexer locally (or ensure it is running in your staging environment) so it can discover the newly registered pools and begin ingestion.

```bash
docker compose -f docker-compose.yml -f docker-compose.app.yml --profile indexer up -d
```
*Note: Wait for the services to become healthy using `./scripts/wait-for-services.sh --api`.*

### 5. Verify Indexer Ingestion (SQL)
Verify that the AMM aggregator has successfully discovered the pools and written the reserves to the database.

Connect to your database (e.g., `psql $DATABASE_URL`) and run the following verification queries:

**Check AMM Pool Reserves:**
```sql
SELECT pool_id, token_a, token_b, reserve_a, reserve_b, updated_at
FROM amm_pool_reserves
ORDER BY updated_at DESC
LIMIT 10;
```
*Expected: You should see recent rows matching the pools registered in step 3.*

**Check Normalized Liquidity:**
```sql
SELECT venue_id, base_asset, quote_asset, price, available_amount
FROM normalized_liquidity
WHERE venue_type = 'amm'
ORDER BY updated_at DESC
LIMIT 10;
```
*Expected: The unified view should now contain AMM-based liquidity derived from the router.*

## AMM Refresh Failure Metrics

The AMM aggregation loop tolerates individual failed cycles: it logs the error
and waits for the next tick. That is correct for transient Soroban RPC blips,
but it means a permanently broken refresh looks identical to a healthy one from
the outside. These two metrics make the difference visible.

| Metric | Type | Labels | Meaning |
|--------|------|--------|---------|
| `stellarroute_indexer_amm_consecutive_refresh_failures` | Gauge | `source` (`amm`) | Failed cycles back-to-back right now. Resets to `0` on the next success. |
| `stellarroute_indexer_amm_refresh_failure_streaks_total` | Counter | `source` (`amm`) | Incremented on every failed cycle once the consecutive count reaches **3**. Never resets. |

The threshold is `AMM_REFRESH_FAILURE_STREAK_THRESHOLD` in
[`crates/indexer/src/metrics.rs`](../../crates/indexer/src/metrics.rs). Cycles 1
and 2 of a streak move the gauge only; the counter starts at cycle 3, so a
single failed poll never fires an alert.

Both are observability-only. They do not change the poll interval, do not abort
the process, and do not affect the SDEX ingest path or the quote API.

**Is the AMM loop stuck?**

```promql
# Currently failing, and for how many cycles
stellarroute_indexer_amm_consecutive_refresh_failures{source="amm"}

# Sustained breakage in the last 15 minutes — suggested alert condition
increase(stellarroute_indexer_amm_refresh_failure_streaks_total{source="amm"}[15m]) > 0
```

A non-zero gauge that keeps climbing while
`stellarroute_indexer_lag_ledgers{source="amm"}` also grows means pool reserves
are going stale. Check the indexer logs for `AMM aggregation cycle failed`, then
work through the verification steps above. Recovery is logged as
`AMM aggregation cycle recovered` and drops the gauge back to `0`.

## Rollback Procedures

If the deployment or indexing fails, follow these steps to roll back:

1. **Stop the Indexer:**
   Prevent the indexer from ingesting faulty AMM data.
   ```bash
   docker compose -f docker-compose.yml -f docker-compose.app.yml --profile indexer stop
   ```

2. **Purge Corrupted AMM State (Database):**
   If the indexer ingested bad pool data, you can clear the specific tables. The indexer will reconstruct state upon next healthy startup.
   ```sql
   DELETE FROM amm_pool_reserves;
   ```

3. **Revert Router Configuration:**
   If the router contract is in a bad state, you can pause it (if supported by your admin identity) or redeploy the previous contract version using the upgrade script:
   ```bash
   # If a previous WASM is available:
   ./scripts/upgrade.sh --network testnet
   ```

## Operator Peer-Review Checklist

Before signing off on the deployment, complete this peer-review checklist:

- [ ] Router contract successfully deployed without errors.
- [ ] Contract verification (`verify.sh`) passes locally.
- [ ] `register-pools.sh` correctly processed all pools defined in the testnet config.
- [ ] Indexer starts up cleanly with no recurring AMM aggregator error logs.
- [ ] `stellarroute_indexer_amm_consecutive_refresh_failures{source="amm"}` reads `0` after the first successful cycle.
- [ ] SQL verification confirms that `amm_pool_reserves` is populated.
- [ ] SQL verification confirms that `normalized_liquidity` reflects the new AMM quotes.
- [ ] No testnet workflows were unexpectedly broken by the deployment.
