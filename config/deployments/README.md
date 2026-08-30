# Deployment artifacts (committed, non-secret)

This directory holds the **reviewable** record of what is deployed on each network.
Unlike `config/deployment-*.json` (gitignored, written locally by `scripts/deploy.sh`
and containing local build paths), everything here is public and safe to commit.

## Schema

| Field | Type | Description |
|-------|------|-------------|
| `network` | string | `testnet` or `mainnet` |
| `router_contract_id` | string | Deployed router contract ID (56 chars, starts with `C`). Empty means not yet deployed |
| `constant_product_adapter_contract_id` | string | Deployed adapter contract ID |
| `deployed_at` | string (RFC 3339, UTC) | When the deploy completed |
| `git_sha` | string | Commit that produced the deployed WASM |
| `rpc_url` | string | Public Soroban RPC endpoint for `network` |

## Producing it

`scripts/deploy.sh --network testnet` writes this file via `save_public_deployment`,
after which the deploy is smoke-checked against the live contract. The
`Deploy to Testnet` GitHub Actions workflow runs the same path and uploads the
result as a build artifact for review before it is committed.

## Consuming it

```bash
export ROUTER_CONTRACT_ADDRESS="$(jq -r .router_contract_id config/deployments/testnet.json)"
```

The indexer validates this value at startup and refuses to run on an empty or
malformed ID — see [`docs/development/indexer-guide.md`](../../docs/development/indexer-guide.md).

## Rules

- **Never** add secret keys, seed phrases, deployer identities, or local paths here.
- `mainnet.json` is intentionally absent: mainnet deploys stay manually gated, and
  `config/pools-mainnet.json` is never auto-populated by a workflow.
