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

## API Production Security (M5)

The `stellarroute-api` HTTP server has a hardened production posture gated
behind `STELLARROUTE_ENV=production`. Set this (plus the vars below) for any
internet-reachable deployment. Full details, defaults, and the break-glass
override are documented in
[`docs/development/environment-variables.md`](../development/environment-variables.md#deployment-profile--security-m5);
the full endpoint-by-endpoint exposure inventory is in
[`docs/api/production-exposure.md`](../api/production-exposure.md).

Required production environment:

```bash
STELLARROUTE_ENV=production

# Explicit allowlist of browser origins — no wildcard CORS in production.
# Include the deployed frontend's Vercel production origin:
CORS_ALLOWED_ORIGINS=https://stellarroute.vercel.app,https://app.stellarroute.io

# API key(s) for integrators; REQUIRE_AUTH defaults to true in production.
API_KEYS=<comma-separated integrator keys>

# If browser clients call quote/orderbook endpoints directly without a key,
# allowlist those specific GET routes rather than disabling auth globally:
PUBLIC_GET_ROUTES=/api/v1/quote,/api/v1/pairs,/api/v1/markets,/api/v1/orderbook,/api/v1/routes,/api/v1/price-history

# Required to reach /api/v1/admin/*, /api/v1/system/*, and (in production)
# /metrics + /api/v1/replay/*.
ADMIN_AUTH_TOKEN=<operator token>
```

The server refuses to start in production if `CORS_ALLOWED_ORIGINS` is empty,
or if auth ends up disabled (`REQUIRE_AUTH=false`) without the explicit
`ALLOW_INSECURE_PUBLIC_API=1` break-glass override — that override should
never be set in a real production deployment.

## Container images (API + Indexer)

### Build locally

```bash
docker build -f Dockerfile.api -t stellarroute-api:local .
docker build -f Dockerfile.indexer -t stellarroute-indexer:local .
```

### Registry (GHCR)

On every push to `main` (and on `v*` tags), GitHub Actions builds both images and
publishes them to GitHub Container Registry using `GITHUB_TOKEN` (OIDC / short-lived
credentials — **no** long-lived registry passwords):

| Image | Repository |
|-------|------------|
| API | `ghcr.io/<owner>/stellarroute-api` |
| Indexer | `ghcr.io/<owner>/stellarroute-indexer` |

Tags:

- Git SHA (short) on every publish
- `latest` on `main`
- Semver (`1.2.3`, `1.2`) when pushing a `v*` tag

PRs that touch Dockerfiles or the API/indexer crate graph run a **build-only**
job (no push) via `.github/workflows/docker-images.yml`.

### Pull and run examples

```bash
OWNER=stellarroute   # or your fork owner (lowercase)
SHA=<git-sha>

# API — requires reachable DATABASE_URL; PORT binds 0.0.0.0
docker pull ghcr.io/${OWNER}/stellarroute-api:latest
docker run --rm -p 8080:8080 \
  -e PORT=8080 \
  -e DATABASE_URL='postgresql://stellarroute:stellarroute_dev@host.docker.internal:5432/stellarroute' \
  ghcr.io/${OWNER}/stellarroute-api:latest

# Pin a specific SHA
docker pull ghcr.io/${OWNER}/stellarroute-api:${SHA}

# Indexer — required env vars
docker pull ghcr.io/${OWNER}/stellarroute-indexer:latest
docker run --rm \
  -e DATABASE_URL='postgresql://stellarroute:stellarroute_dev@host.docker.internal:5432/stellarroute' \
  -e STELLAR_HORIZON_URL='https://horizon-testnet.stellar.org' \
  -e SOROBAN_RPC_URL='https://soroban-testnet.stellar.org' \
  -e ROUTER_CONTRACT_ADDRESS='C...' \
  ghcr.io/${OWNER}/stellarroute-indexer:latest
```

### Health / env notes

- **API** `GET /health` is readiness (needs Postgres). Process exits if `DATABASE_URL` is unreachable.
- **API** `GET /health/deps` probes DB + Horizon + Soroban when configured.
- **Indexer** has no HTTP health endpoint; missing required env vars causes a non-zero exit.
- Indexer required: `DATABASE_URL`, `STELLAR_HORIZON_URL`, `SOROBAN_RPC_URL`, `ROUTER_CONTRACT_ADDRESS`.
- Images run as non-root UID 10001; do not bake `.env` into layers.

Compose:

```bash
docker compose -f docker-compose.yml -f docker-compose.app.yml up -d
docker compose -f docker-compose.yml -f docker-compose.app.yml --profile indexer up -d
```

## Unified Liquidity Migration and Rollback

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

### 0. Exact command order

This is the canonical sequence for a clean testnet deployment:

```bash
# 1. Deploy router contract (writes config/deployment-testnet.json)
./scripts/deploy.sh --network testnet

# 2. Set ROUTER_CONTRACT_ADDRESS in your environment
export ROUTER_CONTRACT_ADDRESS=$(jq -r '.contract_id' config/deployment-testnet.json)

# 3. Register pools from config/pools-testnet.json (idempotent — safe to re-run)
./scripts/register-pools.sh --network testnet

# 4. Verify all configured pools are registered (CI gate — exits non-zero on failure)
./scripts/verify-pools.sh --network testnet

# 5. Start the indexer (reads ROUTER_CONTRACT_ADDRESS from env or .env file)
cargo run -p stellarroute-indexer
# or in Docker:
# docker compose -f docker-compose.yml -f docker-compose.app.yml --profile indexer up -d
```

Re-running step 3 at any time is safe: pools that are already registered are
skipped automatically (idempotent).  Step 4 will fail CI if any configured
non-placeholder pool is missing from the live router state.

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
5. Save contract IDs to `config/deployment-testnet.json` (gitignored, local detail)
6. Save the reviewable, non-secret artifact to `config/deployments/testnet.json` (committed)
7. Verify router deployment by calling `get_admin()`

#### The committed deploy artifact

`config/deployments/testnet.json` is the repo's reviewable record of what is live.
It contains public data only — contract IDs, the public RPC URL, a UTC timestamp,
and the git SHA of the build:

```json
{
  "network": "testnet",
  "router_contract_id": "C...",
  "constant_product_adapter_contract_id": "C...",
  "deployed_at": "2026-01-01T00:00:00Z",
  "git_sha": "abc1234...",
  "rpc_url": "https://soroban-testnet.stellar.org:443"
}
```

Point the indexer at it:

```bash
export ROUTER_CONTRACT_ADDRESS="$(jq -r .router_contract_id config/deployments/testnet.json)"
```

Commit the updated artifact so the deployed address is reviewable in git history.
See [`config/deployments/README.md`](../../config/deployments/README.md) for the
full schema. Mainnet has no committed artifact: those deploys stay manually gated
and `config/pools-mainnet.json` is never auto-populated.

#### Keeping secrets out of artifacts and logs

- The artifact writer emits only the fields above. Secret keys, seed phrases, and
  deployer identities are never written to it.
- In CI the deployer secret is piped to `soroban keys add --secret-key stdin`, so it
  never appears in `argv` or the job log. GitHub additionally masks any value
  registered as a repository secret.
- `config/deployment-*.json` stays gitignored because it records local build paths.
- Before sharing a run log, confirm no step echoes `SOROBAN_DEPLOYER_SECRET`; scripts
  in `scripts/` log contract IDs and tx hashes only.

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

Before running the script you need real Soroban AMM pool contract addresses. Follow the steps below to discover them, then update `config/pools-testnet.json`.

#### Step 1 — Discover pool contract addresses on testnet

Soroban AMM pools are ordinary contracts deployed independently of StellarRoute. There are three ways to find their addresses:

**Option A — Stellar Expert (browser)**

1. Open [https://testnet.stellar.expert/explorer/testnet](https://testnet.stellar.expert/explorer/testnet).
2. Search for the AMM factory contract or a known pool token pair (e.g. `XLM/USDC`).
3. Copy the contract ID (starts with `C`, 56 characters).

**Option B — Soroban RPC query**

If you know the factory contract address, enumerate pools via its `get_pools` method:

```bash
soroban contract invoke \
  --id <FACTORY_CONTRACT_ID> \
  --network testnet \
  -- get_pools
```

Each returned entry is a pool contract ID you can use in `pools-testnet.json`.

**Option C — Stellar Horizon liquidity-pools endpoint**

Classic AMM pools (constant-product) are also discoverable via Horizon:

```bash
curl "https://horizon-testnet.stellar.org/liquidity_pools?limit=20" | jq '.._embedded.records[].id'
```

> **Note:** Horizon liquidity pool IDs are hex strings, not Soroban contract addresses. Use this only if the StellarRoute adapter contract accepts Horizon pool IDs. For Soroban-native pools, prefer Option A or B.

**Option D — Aquarius testnet API**

`config/pools-testnet.json` lists Aquarius Soroban AMM pools on testnet. To discover or refresh pool contract IDs:

```bash
curl -s "https://amm-api-testnet.aqua.network/api/external/v1/pools/?limit=100" | jq '.results[] | {address, tokens: .tokens_str, type: .pool_type}'
```

Prefer `constant_product` pools for XLM/USDC pairs and pools whose `tokens_str` includes `native` (XLM) plus the target asset. Cross-check the pool address on [Stellar Expert testnet](https://stellar.expert/explorer/testnet) before registering. The Aquarius router on testnet is documented at [Aquarius developer guides](https://docs.aqua.network/developers/code-examples/prerequisites-and-basics).

#### Step 2 — Edit `config/pools-testnet.json`

Replace each `PLACEHOLDER_POOL_ADDRESS_*` value with the contract ID you discovered. Keep the `name` and `notes` fields for operator reference.

**Example filled `config/pools-testnet.json`:**

```json
{
  "description": "Testnet liquidity pool addresses to register with the StellarRoute router contract.",
  "pools": [
    {
      "name": "XLM/USDC Testnet Pool",
      "address": "CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA",
      "notes": "Constant-product AMM pool — XLM base, USDC quote"
    },
    {
      "name": "XLM/BTC Testnet Pool",
      "address": "CBEZJWFMKJHPJ3YHPYUGJFMXM5VBXE3HMGHQRQNMFVNRQWKQ6RZXKWM",
      "notes": "Constant-product AMM pool — XLM base, BTC quote"
    }
  ]
}
```

> The addresses above are illustrative examples. Use real contract IDs obtained from Step 1.

Any entry whose `address` starts with `PLACEHOLDER` is automatically skipped by the registration script.

#### Step 3 — Run the registration script

```bash
./scripts/register-pools.sh --network testnet
```

The script reads `config/pools-testnet.json`, skips placeholder entries, and calls the router contract's `register_pool` function for each real address.

**The script is idempotent**: pools that are already registered are detected via `is_pool_registered` before each call and skipped without error. Re-running the script after a partial failure or a re-deploy is always safe.

**Expected log output (successful first run):**

```
[INFO]  Registering 2 pools on testnet (contract: C...ROUTER...)
[INFO]  [1/2] Checking: XLM/USDC Testnet Pool (CBIELTK6...)
[OK]    Registered and verified: XLM/USDC Testnet Pool
[INFO]  [2/2] Checking: XLM/BTC Testnet Pool (CBEZJWFM...)
[OK]    Registered and verified: XLM/BTC Testnet Pool

[OK]    ===== POOL REGISTRATION COMPLETE =====
[OK]    Registered (new):     2
[OK]    Already registered:   0
[OK]    Skipped (placeholder):0
[OK]    Failed:               0
[OK]    Total on-chain pools: 2
```

**Expected log output (re-run — idempotent):**

```
[INFO]  [1/2] Checking: XLM/USDC Testnet Pool (CBIELTK6...)
[OK]    Already registered (no-op): XLM/USDC Testnet Pool
[INFO]  [2/2] Checking: XLM/BTC Testnet Pool (CBEZJWFM...)
[OK]    Already registered (no-op): XLM/BTC Testnet Pool

[OK]    ===== POOL REGISTRATION COMPLETE =====
[OK]    Registered (new):     0
[OK]    Already registered:   2
[OK]    Skipped (placeholder):0
[OK]    Failed:               0
[OK]    Total on-chain pools: 2
```

**Expected log output (placeholder entries present):**

```
[WARN]  Skipping placeholder pool: XLM/USDC Testnet Pool
[WARN]  Skipping placeholder pool: XLM/BTC Testnet Pool

[OK]    ===== POOL REGISTRATION COMPLETE =====
[OK]    Registered (new):     0
[OK]    Already registered:   0
[OK]    Skipped (placeholder):2
[OK]    Failed:               0
```

The script also writes a machine-readable JSON summary to
`logs/<network>-register-summary.json`.

#### Step 4 — Verify pools (CI gate)

```bash
./scripts/verify-pools.sh --network testnet
```

This script queries `is_pool_registered` for every non-placeholder pool and
exits non-zero if any pool is missing from the live router.  Use it as a CI/CD
gate after `register-pools.sh`.  Output is also written as JSON to
`logs/<network>-verify-pools-summary.json`.

If `Registered: 0` is shown for a non-placeholder run, verify the router contract is deployed (`./scripts/deploy.sh` must have run first) and that `config/deployment-testnet.json` exists with a valid contract ID.

#### Relationship between pool config and `register_pool`

Each entry in `pools-testnet.json` maps directly to one `register_pool` invocation on the router contract:

```
pools-testnet.json entry.address
        │
        ▼
router.register_pool(pool = <address>)   ← on-chain call
        │
        ▼
router.is_pool_registered(pool = <address>)  ← verification call
```

Once registered, the StellarRoute indexer discovers pools via `get_pool_count` / `get_pools` at startup and includes their reserve data in the `amm_pool_reserves` table and `normalized_liquidity` view used by the quote and routing APIs.

For full details on the registration script internals see [`docs/contracts/deployment-runbook.md`](../contracts/deployment-runbook.md#pool-registration-with-scriptsregister-poolssh).

After registration, run `./scripts/smoke-test-testnet.sh --network testnet` to verify end-to-end connectivity.

### 4. Verify
```bash
./scripts/verify.sh --network testnet
```

### 5. Monitor
```bash
./scripts/monitor.sh --network testnet
```

## Load Testing

Before launch, run the public quote/routes load test and record the results in
the report template:

```bash
# Against a local dev server
k6 run scripts/load-test-quote-routes.k6.js

# Against a deployed environment
k6 run -e BASE_URL=https://api.stellarroute.io \
       -e VUS=250 \
       -e DURATION=5m \
       scripts/load-test-quote-routes.k6.js
```

See [`docs/deployment/load-test-report.md`](load-test-report.md) for the report
template and pass/fail criteria (quote/routes p95 < 500 ms, error rate < 1%).

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

## Database Migrations (Zero-Downtime)

Postgres schema changes for the API and indexer use the **expand/contract**
pattern so the live DEX never takes a long outage.  See
[`docs/deployment/migration-runbook.md`](migration-runbook.md) for the full
runbook, including:

- The five-phase expand → dual-write → backfill → flip-reads → contract flow.
- Production and CI runbooks.
- A complete backward-compatible migration example.
- Anti-patterns to avoid (non-concurrent indexes, same-deploy schema+code flips,
  dropping columns still read by the previous release).

Migrations are ordered: indexer migrations run first, then API migrations.

## Communication Checklist for Upgrades

Before deploying an upgrade to mainnet:

- [ ] All changes reviewed and merged to `main`
- [ ] Testnet deployment successful and verified
- [ ] Changelog written describing what changed and why
- [ ] Stakeholders notified (Discord, GitHub Discussions)
- [ ] Monitoring in place for post-upgrade health checks
- [ ] Previous WASM binary archived
- [ ] Deployment artifact backed up

## Mainnet Deployment

Mainnet deploys are manual-only and gated behind repository safeguards. Do not run until testnet verification and audit sign-off are complete.

### Prerequisites

- Funded mainnet deployer identity (separate from testnet)
- Repository variable `DEPLOY_MAINNET_ENABLED=true`
- Repository secret `SOROBAN_MAINNET_DEPLOYER_SECRET` (mainnet-only; never reuse testnet keys)

### Deploy via GitHub Actions

1. Open **Actions → Deploy to Mainnet → Run workflow**.
2. Use **dry run** first to build WASM and validate the pipeline without submitting transactions.
3. Re-run with dry run disabled after secrets and variables are confirmed.

On success, the workflow uploads `config/deployment-mainnet.json` as a GitHub Actions artifact (90-day retention).

### Deploy locally

```bash
./scripts/deploy.sh --network mainnet --dry-run
./scripts/deploy.sh --network mainnet
./scripts/verify.sh --network mainnet
```

### Mainnet rollback and upgrade

Soroban does not support native rollback. To revert a bad upgrade:

1. Stop routing traffic to the affected router contract.
2. Archive the last known-good WASM binary from the deployment artifact.
3. Run `./scripts/upgrade.sh --network mainnet` with the previous WASM version checked out, or deploy a fresh router if state is compromised.
4. Re-register pools from `config/pools-mainnet.json` after the router is healthy.
5. Run `./scripts/verify.sh --network mainnet` and `./scripts/monitor.sh --network mainnet` before restoring traffic.

For planned upgrades, follow the testnet upgrade flow in [Upgrade Process](#upgrade-process) on mainnet only after testnet verification passes.

## CI/CD Workflows

### Manual Deploy (`deploy-testnet.yml`)
- Trigger: GitHub Actions > "Deploy to Testnet" > Run workflow
- Set the `deploy_router` input to deploy a fresh router; the job writes
  `config/deployments/testnet.json`, smoke-checks the deployed ID with `get_admin`,
  and **fails the run** if the contract does not answer (so a bad ID is never published)
- The artifact is uploaded as `deployment-testnet` for review before committing
- Supports dry-run mode (build + hash only, no deploy)
- Requires `SOROBAN_DEPLOYER_SECRET` secret and `DEPLOY_ENABLED=true` variable
- Automatically registers pools from `config/pools-testnet.json` after deployment
- Fails if all pools are placeholders (no real pool addresses)
- Smoke tests verify at least one pool is registered and routable
- Runs testnet contract smoke tests against `vars.SOROBAN_CONTRACT_ID`

### Gated Mainnet Deploy (`deploy-mainnet.yml`)
- Trigger: GitHub Actions > "Deploy to Mainnet" > Run workflow (manual only)
- Requires `DEPLOY_MAINNET_ENABLED=true` repository variable
- Requires `SOROBAN_MAINNET_DEPLOYER_SECRET` repository secret (separate from testnet)
- Supports dry-run mode (build + simulate without on-chain deploy)
- Uploads `config/deployment-mainnet.json` artifact after a successful deploy

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


---

## Hosting Blueprint (Issue #1035)

The following sections document the concrete hosting blueprint that satisfies
M5 (Live hosting). A single-region Render deployment and a Docker Compose
production overlay are both provided.

### Files

| File | Purpose |
|---|---|
| `render.yaml` | Render Blueprint — managed Postgres, Redis, API web service, indexer worker |
| `deploy/docker-compose.prod.yml` | Compose production overlay (hardened, no host ports for DB/Redis) |
| `deploy/secrets.checklist.md` | Operator checklist — work through before first deploy |

### Dry-run / validate commands

**Render Blueprint:**
```bash
python3 -c "import yaml, sys; yaml.safe_load(open('render.yaml'))" && echo "render.yaml OK"
```
Use the Render dashboard → **Blueprint → Validate** for full schema validation.

**Docker Compose production overlay:**
```bash
docker compose -f docker-compose.yml -f deploy/docker-compose.prod.yml config
# Exits 0 and prints the merged config if the YAML is valid.
```

### Environment variable mapping

The table below maps every env var key used in `render.yaml` and
`deploy/docker-compose.prod.yml` to its purpose, source, and which service
requires it. It is kept 1:1 with the blueprint keys — if you add a variable
to the blueprint, add a row here.

| Key | Required by | Source in Render | Description |
|---|---|---|---|
| `DATABASE_URL` | API, Indexer | Auto-wired from `stellarroute-postgres` | Primary PostgreSQL connection string |
| `REDIS_URL` | API | Auto-wired from `stellarroute-redis` | Redis connection string for quote cache + rate limiting |
| `API_PORT` | API | Set to `3000` in blueprint | HTTP listen port |
| `RUST_LOG` | API, Indexer | Set to `info,warn` in blueprint | Log level directive |
| `SOROBAN_RPC_URL` | API (optional), Indexer (**required**) | Secret — set in Render dashboard | Soroban RPC endpoint (e.g. `https://soroban-rpc.testnet.stellar.org`) |
| `STELLAR_HORIZON_URL` | Indexer (**required**) | Secret — set in Render dashboard | Stellar Horizon endpoint |
| `ROUTER_CONTRACT_ADDRESS` | Indexer (**required**) | Secret — set in Render dashboard | Deployed router contract ID |
| `ENABLE_ADMIN_ROUTES` | API | Hardcoded `false` in blueprint | Enable/disable admin kill-switch routes (see §Security) |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | API, Indexer | Optional secret | OTLP collector URL; unset disables trace export |
| `POSTGRES_USER` | Compose only | `.env.prod` | PostgreSQL superuser (not used in Render managed DB) |
| `POSTGRES_PASSWORD` | Compose only | `.env.prod` | PostgreSQL password |
| `POSTGRES_DB` | Compose only | `.env.prod` | PostgreSQL database name |
| `REDIS_PASSWORD` | Compose only | `.env.prod` | Redis `requirepass` value |

### Health checks

| Endpoint | Type | Used by |
|---|---|---|
| `GET /health` | Liveness — is the process alive? | Render web service health check; Docker Compose healthcheck |
| `GET /health/deps` | Readiness — are Postgres and Redis reachable? | Post-deploy verification |

Both endpoints are wired in `render.yaml` via `healthCheckPath: /health`.
The production Compose overlay additionally runs a `curl -sf` healthcheck
against `/health` at 15 s intervals with 3 retries.

### Security

⚠️  **Admin routes are disabled by default.**

`ENABLE_ADMIN_ROUTES` is set to `"false"` in both blueprints. Do **not**
change this to `"true"` until the kill-switch security issues have been
reviewed and merged. Relevant tracking: see `docs/RUNBOOK_KILL_SWITCH.md`
and the issue tracker for open security issues tagged `[security]`.

The blueprint also:
- Removes host-port exposure for Postgres and Redis in the Compose overlay
  so those services are only reachable from within the Docker network.
- Uses `ipAllowList: []` in `render.yaml` so managed databases only accept
  connections from Render-internal services.

### Deploying to staging (no tribal knowledge required)

1. Fork/clone the repo.
2. Work through `deploy/secrets.checklist.md`.
3. Connect the repo to Render → **Blueprints → New Blueprint** → select `render.yaml`.
4. Render will create the Postgres database, Redis instance, API web service, and indexer worker.
5. In the Render dashboard, add the secret env vars listed in `deploy/secrets.checklist.md`.
6. Trigger a deploy and verify:
   ```bash
   curl -sf https://<your-render-url>/health && echo "liveness OK"
   curl -sf https://<your-render-url>/health/deps && echo "readiness OK"
   ```
