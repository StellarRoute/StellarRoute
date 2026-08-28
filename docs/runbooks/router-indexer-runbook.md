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
- [ ] SQL verification confirms that `amm_pool_reserves` is populated.
- [ ] SQL verification confirms that `normalized_liquidity` reflects the new AMM quotes.
- [ ] No testnet workflows were unexpectedly broken by the deployment.
