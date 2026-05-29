# Contract Deployment Runbook

This document describes the end-to-end Soroban contract lifecycle for StellarRoute, including build, deploy, verify, upgrade, storage TTL maintenance, and pool registration.

## Purpose

Use this runbook for operators managing StellarRoute contracts on Testnet or Mainnet.
It is aligned with the repository's existing deployment scripts in `scripts/`.

## Prerequisites

- Rust 1.75+ with the `wasm32-unknown-unknown` target installed
- Soroban CLI installed (`cargo install --locked soroban-cli`)
- `jq` installed for JSON parsing
- A funded Stellar account for the target network
- Repository checkout with the latest `main` branch

### Required tools

```bash
rustup target add wasm32-unknown-unknown
cargo install --locked soroban-cli
sudo apt-get install -y jq
```

### Network identities

Use separate deployer identities for Testnet and Mainnet.
For Testnet, fund the deployer via Friendbot:

```bash
soroban keys generate deployer --network testnet
curl "https://friendbot.stellar.org/?addr=$(soroban keys address deployer)"
```

For Mainnet, the identity must be funded via a real Stellar wallet or on-chain transfer.

## Build and test workflow

From the repository root:

```bash
cargo test -p stellarroute-contracts
```

To build the contract WASM artifact directly:

```bash
cargo build --manifest-path crates/contracts/Cargo.toml --target wasm32-unknown-unknown --release
```

The deployment scripts rely on the compiled artifact at:

- `crates/contracts/target/wasm32-unknown-unknown/release/stellarroute_contracts.wasm`

## Configuration differences: Testnet vs Mainnet

The scripts use `--network` to select the target configuration.
Valid values are:

- `testnet`
- `mainnet`

Configuration is defined in `config/networks.json`:

- `testnet.rpc_url` → `https://soroban-testnet.stellar.org:443`
- `mainnet.rpc_url` → `https://soroban-rpc.mainnet.stellar.org:443`
- `testnet.network_passphrase` → Testnet passphrase
- `mainnet.network_passphrase` → Public mainnet passphrase

Pool registration data is separated by network:

- `config/pools-testnet.json`
- `config/pools-mainnet.json`

### Key differences

- Testnet supports Friendbot; Mainnet does not.
- Use different deployer accounts and deployment artifacts for each network.
- The `--network` flag selects both RPC URL and network passphrase.
- Mainnet operations incur real XLM costs.

## Deploying contracts with `scripts/deploy.sh`

### Usage

```bash
./scripts/deploy.sh --network testnet
```

Optional flags:

- `--identity <name>`: Soroban identity name (default: `deployer`)
- `--dry-run`: build and simulate without on-chain transactions

### What this script does

1. Runs `cargo build` for `crates/contracts` targeting `wasm32-unknown-unknown`
2. Optimizes the WASM using `soroban contract optimize`
3. Deploys two contracts on-chain:
   - `router`
   - `constant_product_adapter`
4. Initializes the router with:
   - `admin` set to the deployer identity address
   - `fee_rate` set to `30` (bps)
   - `fee_to` set to the deployer address
5. Saves `config/deployment-<network>.json`
6. Verifies the deployed router admin via `get_admin()`

### Deployment artifact

The deploy script writes the artifact to:

- `config/deployment-testnet.json`
- `config/deployment-mainnet.json`

This file includes contract IDs, network RPC metadata, and the current Git commit.

## Verifying deployed contracts with `scripts/verify.sh`

### Usage

```bash
./scripts/verify.sh --network testnet
```

The script:

1. Builds and optimizes local WASM
2. Fetches on-chain deployed bytecode for the router contract
3. Compares local and deployed SHA-256 hashes
4. Invokes read-only router methods to confirm state:
   - `get_admin`
   - `get_fee_rate_value`
   - `get_fee_to`
   - `is_paused`
   - `get_pool_count`
   - `version`

### Notes

- The script uses `config/deployment-<network>.json` to resolve the router contract ID.
- If no deployment artifact exists, the verification step will fail.

## Upgrade flow with `scripts/upgrade.sh`

### Usage

```bash
./scripts/upgrade.sh --network testnet
```

Optional flags:

- `--identity <name>`: Soroban identity name (default: `deployer`)
- `--dry-run`: build and compare without submitting on-chain transactions

### What this script does

1. Reads the current router contract ID from `config/deployment-<network>.json`
2. Captures pre-upgrade state from the router
3. Builds and optimizes the new WASM
4. Compares the new WASM hash against the deployed router code
5. Installs the new WASM on-chain if different
6. Submits a router upgrade proposal via `propose_upgrade`
   - uses `execute_after=4320`
7. Redeploys the adapter contract with the updated WASM
8. Verifies key invariants after upgrade
9. Updates the deployment artifact

### Governance assumptions

- The deployer identity is assumed to be the router admin.
- The router upgrade path is assumed to allow the admin to propose upgrades.
- The script does not execute a full governance vote; it submits a timed upgrade proposal.
- If your deployment uses a separate multi-sig or governance quorum, ensure the proposal is accepted and executed per your process.

### Post-upgrade checks

After the script completes, run:

```bash
./scripts/verify.sh --network testnet
```

And confirm that the router continues to report expected state.

## TTL extension with `scripts/extend-ttl.sh`

### Usage

```bash
./scripts/extend-ttl.sh --network testnet
```

Optional flags:

- `--watch`: run continuously on a schedule
- `--dry-run`: check TTL status without extending
- `--interval <seconds>`: poll interval when watching
- `--identity <name>`: Soroban identity name (default: `deployer`)

### Purpose

This script monitors storage TTL and calls `extend_storage_ttl()` before contract storage keys expire.
It is intended to keep long-lived router state healthy on Soroban.

### Recommended cadence

- Weekly for active deployments
- More frequently if the contract has many registered pools

### When TTL extension is needed

The script uses the router's `get_ttl_status` response:

- If `needs_extension` is `true`, it will call `extend_storage_ttl`
- If the status check fails, the script may still attempt an extension as a safety measure

### Notes

- Mainnet extension costs real XLM; monitor spend carefully.
- Testnet extension is a low-cost operation, but still requires a funded account.

## Pool registration with `scripts/register-pools.sh`

### Usage

```bash
./scripts/register-pools.sh --network testnet
```

The script reads the pool list from:

- `config/pools-testnet.json`
- `config/pools-mainnet.json`

### Pool registration workflow

1. Ensure the target `pools-<network>.json` file exists.
2. Confirm each pool entry contains a valid Stellar address.
3. Run the script to invoke `register_pool` for each configured pool.
4. The script verifies registration using `is_pool_registered`.
5. It prints the deployed on-chain pool count.

### Common failure modes

- placeholder pool addresses in config
- invalid or malformed pool contract IDs
- router contract not deployed or incorrect deployment artifact

## Common operational examples

```bash
./scripts/deploy.sh --network testnet
./scripts/verify.sh --network testnet
./scripts/register-pools.sh --network testnet
./scripts/extend-ttl.sh --network testnet
./scripts/upgrade.sh --network testnet
```

For Mainnet, replace `testnet` with `mainnet` and use a funded mainnet deployer identity.

## Troubleshooting

### `Soroban CLI (soroban or stellar) is not installed.`

Install the Soroban CLI:

```bash
cargo install --locked soroban-cli
```

If you have the `stellar` command instead, the scripts will use it automatically.

### `No deployment artifact found at ...` or missing contract ID

Run deployment first:

```bash
./scripts/deploy.sh --network testnet
```

Or confirm the deployment artifact path exists:

- `config/deployment-testnet.json`
- `config/deployment-mainnet.json`

### `Invalid network '...'` error

Use only `testnet` or `mainnet` with `--network`.

### `Failed to fetch deployed bytecode`

Possible causes:

- wrong contract ID in the deployment artifact
- network RPC or passphrase mismatch
- contract not deployed to the selected network

### TTL extension failures

If `get_ttl_status` fails, verify the router contract is deployed and reachable.
If gas is insufficient, fund the deployer account and retry.

### Upgrade invariant failures

If invariants break after upgrade, stop and inspect the contract state.
Verify `admin`, `fee_rate`, `is_paused`, and `get_pool_count` before continuing.

## References

- `scripts/deploy.sh`
- `scripts/verify.sh`
- `scripts/upgrade.sh`
- `scripts/extend-ttl.sh`
- `scripts/register-pools.sh`
- `docs/contracts/router-interface.md`
- `docs/contracts/gas-benchmarks.md`
- `docs/deployment/README.md`
