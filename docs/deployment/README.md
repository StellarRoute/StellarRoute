# StellarRoute Deployment Runbook

This guide covers everything needed to deploy, verify, upgrade, and monitor StellarRoute contracts on Stellar Testnet and Mainnet.

## Prerequisites

- Rust 1.75+ with `wasm32-unknown-unknown` target
- Soroban CLI (`cargo install --locked soroban-cli`)
- `jq` (for JSON parsing in scripts)
- A funded Stellar account (use Friendbot for testnet)

## Key Management

### Local Development
```bash
# Generate a new identity (stored in ~/.config/soroban/identity/)
soroban keys generate deployer --network testnet

# Fund on testnet via Friendbot
curl "https://friendbot.stellar.org/?addr=$(soroban keys address deployer)"
```

### CI/CD (GitHub Actions)
- Store the deployer secret key as a GitHub repository secret: `SOROBAN_DEPLOYER_SECRET`
- Store the deployed contract ID as a repository variable: `SOROBAN_CONTRACT_ID`
- Set `DEPLOY_ENABLED=true` as a repository variable to enable the deploy workflow.

### Security Rules
- **NEVER** commit private keys, seed phrases, or secret keys to the repository.
- **NEVER** share identity files across environments (testnet vs mainnet).
- Use separate deployer accounts for testnet and mainnet.
- Rotate keys if compromise is suspected.
- The `.gitignore` excludes `.soroban/`, `*.secret-key`, and `identity.toml`.

### Secret Rotation Checklist

Use this checklist when rotating database, Redis, or Soroban RPC credentials:

1. Add the new secret or credential alongside the old one in the target secret store.
2. Update the runtime environment to point at the new value, keeping the old value available for rollback.
3. Restart one service at a time and confirm `GET /health` and `GET /health/deps` remain healthy.
4. Remove the old credential only after the new one has been verified in production.
5. Confirm no startup logs or health checks print secret material.

Recommended order: database first, Redis second, Soroban RPC last.

### Unified Liquidity Migration and Rollback

The unified liquidity path reads from `normalized_liquidity`, which combines SDEX offers and AMM reserves.

Migration sequence:

1. Apply the new schema/migration that creates or updates `normalized_liquidity` and the AMM reserve tables.
2. Backfill existing SDEX data before switching quote or routing reads.
3. Verify quote responses and route selection on a staging environment.
4. Flip the API/query path to the unified model.

Rollback sequence:

1. Stop new writes into the unified path.
2. Switch reads back to the previous SDEX-only query path.
3. Preserve the backfill checkpoint tables so a later retry can resume safely.
4. Keep the last known-good schema migration file and deployment artifact together.
## Testnet Deployment (From Clean Machine)

### 1. Setup
```bash
# Clone and enter the repository
git clone https://github.com/StellarRoute/StellarRoute.git
cd StellarRoute

# Install Rust + WASM target
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown

# Install Soroban CLI
cargo install --locked soroban-cli

# Generate and fund deployer identity
soroban keys generate deployer --network testnet
curl "https://friendbot.stellar.org/?addr=$(soroban keys address deployer)"
```

### 2. Deploy
```bash
./scripts/deploy.sh --network testnet
```

This will:
1. Build contracts to WASM
2. Optimize the WASM binary
3. Deploy router + adapter contracts to testnet
4. Initialize router with deployer as admin, 30 bps fee rate
5. Save contract IDs to `config/deployment-testnet.json`
6. Verify router deployment by calling `get_admin()`

Environment and runtime options:
```bash
# optional defaults
export STELLAR_NETWORK=testnet

# simulate without writing on-chain transactions
./scripts/deploy.sh --dry-run

# use a non-default soroban identity name
./scripts/deploy.sh --network testnet --identity deployer
```

### 3. Register Pools
Edit `config/pools-testnet.json` with real pool addresses, then:
```bash
./scripts/register-pools.sh --network testnet
```

### 4. Verify
```bash
./scripts/verify.sh --network testnet
```

### 5. Monitor
```bash
./scripts/monitor.sh --network testnet
```

## Upgrade Process

### When to Upgrade
- Bug fixes in contract logic
- New features (e.g., additional getter functions)
- Performance improvements

### How to Upgrade
```bash
# Increment CONTRACT_VERSION in crates/contracts/src/router.rs
# Then run:
./scripts/upgrade.sh --network testnet
```

The upgrade script will:
1. Capture pre-upgrade state (admin, fee rate, paused status, pool count, version)
2. Build and optimize new WASM
3. Compare bytecode hashes (skip if identical)
4. Install new WASM on-chain
5. Propose a timelocked router upgrade using `propose_upgrade`
6. Redeploy adapter contract with the new WASM
7. Verify all critical invariants are preserved
8. Update the deployment artifact

### Post-Upgrade Verification
```bash
./scripts/verify.sh --network testnet
./scripts/monitor.sh --network testnet
```

### Rollback Limitations
Soroban does **not** support native rollback. Once a contract is upgraded:
- The old WASM code is replaced.
- Storage state is preserved (keys and values persist).
- To "rollback," you must deploy the previous WASM version as a new upgrade.

**Recommendation**: Always keep the last known-good WASM binary archived (the deploy workflow uploads it as a GitHub Actions artifact with 30-day retention).

## Data Migration Strategy

If a contract upgrade changes the storage schema (e.g., new `StorageKey` variants):

1. **Additive changes** (new keys): No migration needed. New keys will have default values (`unwrap_or` pattern).
2. **Renamed keys**: Requires a migration function that reads old keys and writes new ones. This must be called once after upgrade.
3. **Removed keys**: Old keys will remain in storage but become unused. They will naturally expire when their TTL runs out.
4. **Changed value types**: Not supported without migration. Deploy a one-time migration entrypoint, call it, then upgrade again to remove the migration code.

## Communication Checklist for Upgrades

Before deploying an upgrade to mainnet:

- [ ] All changes reviewed and merged to `main`
- [ ] Testnet deployment successful and verified
- [ ] Changelog written describing what changed and why
- [ ] Stakeholders notified (Discord, GitHub Discussions)
- [ ] Monitoring in place for post-upgrade health checks
- [ ] Previous WASM binary archived
- [ ] Deployment artifact backed up

## CI/CD Workflows

### Manual Deploy (`deploy-testnet.yml`)
- Trigger: GitHub Actions > "Deploy to Testnet" > Run workflow
- Supports dry-run mode (build + hash only, no deploy)
- Requires `SOROBAN_DEPLOYER_SECRET` secret and `DEPLOY_ENABLED=true` variable

### Nightly Verification (`verify-contracts.yml`)
- Runs automatically at 03:00 UTC daily
- Rebuilds contracts from source and compares bytecode hash against deployed contract
- Requires `SOROBAN_CONTRACT_ID` repository variable
- Fails the workflow if hashes mismatch

### CI Restoration Sequence

Restore the main CI gate in this order so regressions are easier to isolate:

1. Re-enable formatting and lint checks first (`cargo fmt --check`, `cargo clippy -- -D warnings`).
2. Re-enable unit tests next, starting with the crates touched most often.
3. Re-enable contract verification last, keeping the nightly verification workflow as the safety net.
4. Quarantine any flaky step in a separate workflow or scheduled job until it is stable.
5. Require the restored baseline to stay green for a full review window before tightening merge policy again.

Merge gating policy:

- Main branch merges should require the restored baseline checks to pass.
- Contract verification can remain advisory until the restore sequence is complete.
- Flaky checks should be documented with owner and next review date.

## Troubleshooting

### "No deployment artifact found"
Run `./scripts/deploy.sh --network testnet` first. The deployment artifact is generated at deploy time.

### "Soroban CLI not found"
```bash
cargo install --locked soroban-cli
# Ensure ~/.cargo/bin is in your PATH
```

### "Identity not found"
```bash
soroban keys generate deployer --network testnet
# Or import an existing key:
echo "S..." | soroban keys add deployer --secret-key stdin
```

### "Transaction failed: insufficient balance"
Fund the deployer account:
```bash
# Testnet
curl "https://friendbot.stellar.org/?addr=$(soroban keys address deployer)"
# Mainnet: transfer XLM from an exchange or wallet
```
