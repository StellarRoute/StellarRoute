# AGENTS.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## What this repo is
StellarRoute is a Rust-first Stellar DEX aggregator with:
- an indexer (`crates/indexer`) that ingests SDEX + Soroban AMM state into Postgres,
- an API (`crates/api`) that serves quotes/orderbooks/routes and optional Redis-backed caching,
- a routing engine (`crates/routing`) used by API logic,
- Soroban contracts (`crates/contracts`),
- a Next.js frontend (`frontend`) and TypeScript SDK (`sdk-js`).

## Common commands
Use these commands from repo root unless noted.

### Local dependencies
- Start Postgres + Redis (deps only):
  - `docker-compose up -d`
- Start full stack (Postgres + Redis + API):
  - `docker compose -f docker-compose.yml -f docker-compose.app.yml up -d`
- Start full stack with indexer (requires `ROUTER_CONTRACT_ADDRESS` in `.env`):
  - `docker compose -f docker-compose.yml -f docker-compose.app.yml --profile indexer up -d`
- Start full stack with frontend UI:
  - `docker compose -f docker-compose.yml -f docker-compose.app.yml --profile ui up -d`
- Wait for service health (deps only):
  - `./scripts/wait-for-services.sh`
- Wait for service health (deps + API):
  - `./scripts/wait-for-services.sh --api`
- Wait for databases to be healthy:
  - `./scripts/wait-for-dbs.sh`
- Check service health:
  - `docker-compose ps`

### Rust workspace
- Build all crates:
  - `cargo build`
- Run formatting check (same as CI `Rust Format` job):
  - `cargo fmt --all -- --check`
- Lean CI Rust checks (same as CI job `Rust Lean Clippy + Lib Tests (excl. api/contracts)`).
  These are bootstrap gates — not full workspace/all-targets CI:
  - `cargo clippy --workspace --all-features --exclude stellarroute-contracts -- -D warnings`
  - `cargo clippy -p stellarroute-contracts -- -D warnings`
  - `cargo test --workspace --lib --exclude stellarroute-contracts --exclude stellarroute-api`
- Focused swap/OpenAPI contract CI (job `Rust API Swap + OpenAPI Contract Tests (no external DB)`):
  - `cargo test -p stellarroute-api --test swap_integration --test swap_submit_integration --test openapi_swap_contract`
  - `cargo test -p stellarroute-api --lib` (when green in that job)
  These use lazy pools / in-memory stores — no Postgres/Redis required. They cover
  AssetPath string+object wire hops, prepare `network_passphrase`, and OpenAPI honesty.
  Deferred vs full coverage: no `--all-targets` / `cfg(test)` clippy, no contracts lib
  tests, no broader API integration suite, no ignored DB tests. Known blockers:
  indexer `tests/amm_ingest.rs`, contracts `fuzz_targets`, other `crates/*/tests/*`
  that need live deps.
- Recommended full local commands (stricter than lean CI; may fail until deferred debt is fixed):
  - `cargo test`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo clippy -p stellarroute-contracts --all-targets -- -D warnings`
- Run a single test (example pattern):
  - `cargo test -p stellarroute-api quote::tests::selects_best_executable_direct_venue`
  - `cargo test -p stellarroute-routing pathfinder::tests::...`
- Run ignored/integration-style tests when needed:
  - `cargo test -- --include-ignored`

### Run services
- API server:
  - `cargo run -p stellarroute-api`
- Indexer:
  - `cargo run -p stellarroute-indexer`

### Frontend (`frontend/`)
- Install deps:
  - `npm --prefix frontend install`
- Dev server:
  - `npm --prefix frontend run dev`
- Build:
  - `npm --prefix frontend run build`
- Lint:
  - `npm --prefix frontend run lint`
- Unit tests:
  - `npm --prefix frontend run test`
- Single test file / test name:
  - `npm --prefix frontend run test -- src/path/to/file.test.tsx -t "test name"`
- E2E:
  - `npm --prefix frontend run test:e2e`
- Story snapshot build:
  - `npm --prefix frontend run storybook:ci`

### JS SDK (`sdk-js/`)
- Install deps:
  - `npm --prefix sdk-js install`
- Build:
  - `npm --prefix sdk-js run build`
- Test:
  - `npm --prefix sdk-js run test`
- Single test file / test name:
  - `npm --prefix sdk-js run test -- src/path/to/file.test.ts -t "test name"`
- Typecheck/lint:
  - `npm --prefix sdk-js run typecheck`

## Required runtime configuration
- API requires `DATABASE_URL`; optional `REDIS_URL`.
- Indexer requires `DATABASE_URL`, `STELLAR_HORIZON_URL`, `SOROBAN_RPC_URL`, and `ROUTER_CONTRACT_ADDRESS`.
- Typical local values are documented in `docs/development/SETUP.md`.

## Big-picture architecture and execution flow
Focus here first when debugging behavior across crates.

1. Data ingestion and normalization
- `crates/indexer/src/bin/stellarroute-indexer.rs` boots DB, runs migrations, then starts:
  - SDEX loop (`sdex.rs`) reading Horizon offers,
  - AMM loop (`amm.rs`) reading Soroban events/pool state,
  - maintenance loop (snapshot compaction, retention cleanup, materialized view refresh).
- Ingestion writes into `assets`, `sdex_offers`, `amm_pool_reserves`, and supporting tables/functions.
- Quote/routing read path is unified via `normalized_liquidity` (see `docs/architecture/database-schema.md`).

2. API request path
- `crates/api/src/bin/stellarroute-api.rs` configures DB pool guardrails, optional startup dependency checks, and launches `Server`.
- `crates/api/src/server.rs` wires middleware (request ID, versioning headers, rate limiting, tracing), routes, Swagger UI, and optional Redis cache.
- `crates/api/src/routes/mod.rs` exposes primary endpoints:
  - `/api/v1/pairs`, `/api/v1/orderbook/:base/:quote`, `/api/v1/quote/:base/:quote`, `/api/v1/routes/:base/:quote`, plus replay/admin/metrics.
- `crates/api/src/routes/quote.rs` is the key quote pipeline:
  - loads candidates from `normalized_liquidity`,
  - applies freshness/health/policy filters from `stellarroute-routing::health::*`,
  - chooses best executable venue,
  - records metrics/tracing and caches short-TTL quote results.

3. Routing engine role
- `crates/routing` is shared routing/health logic (pathfinder, optimizer, risk/policy, consensus, anomaly/freshness/health modules).
- API currently uses routing health + policy components directly for venue filtering/scoring in quote computation.

4. Contracts and SDKs
- `crates/contracts` contains Soroban router-related contracts and tests.
- `sdk-js` wraps API endpoints for external clients; examples in `sdk-js/examples/`.
- `crates/sdk-rust` is the Rust SDK workspace member.

## High-value files to open first
- `crates/indexer/src/bin/stellarroute-indexer.rs`
- `crates/indexer/src/sdex.rs`
- `crates/indexer/src/amm.rs`
- `crates/api/src/bin/stellarroute-api.rs`
- `crates/api/src/server.rs`
- `crates/api/src/routes/quote.rs`
- `crates/api/src/state.rs`
- `crates/routing/src/lib.rs`
- `docs/architecture/database-schema.md`

## Known project-specific testing details
- Frontend Vitest setup includes `matchMedia` and `localStorage` mocks in `frontend/vitest.setup.ts`.
- If icon imports break frontend tests, check `frontend/__mocks__/lucide-react.tsx`.

## Learned User Preferences
- Primary goal is a live non-custodial DEX with real users, expanding toward cross-chain bridge/aggregation and offramp; first offramp corridor is stablecoin→NGN (Naira) via Paycrest (EVM/Base deposit networks; Stellar users bridge-then-offramp); prioritize production deployability over docs-only or filler work.
- GitHub issues should be grounded in real codebase gaps (not placeholders), with hard/high-quality acceptance criteria and Wave-friendly labels.
- Stale or unfresh orderbook/market data must not hard-block swaps; return a degraded quote and notify the user instead of failing the swap.
- Frontend UI should feel unique and spacious; reject dense/jammed header, swap, and offramp chrome; polish wallet/error messaging rather than stacking warnings; for bridge/cross-chain waiting, visualize the journey (both Sepolia↔Stellar directions quote Fast attestation by default; Stellar-source Fast can still take ~15+ min because Circle often executes at Standard finality; Standard remains ~15–19 min if selected) and show a completion popup with the destination transaction hash (do not leave the primary CTA stuck disabled after success; prefer mint status over a local failed flag after a successful Freighter/MetaMask mint).
- When processing contributor/fork PRs, fix conflicts and CI and merge rather than closing; keep going until the open queue is empty unless a PR is explicitly unmergeable.
- For fork PRs far behind `main`, cherry-pick feature commits onto current `main` instead of merging the stale branch wholesale.
- Prefer lean CI that contributors can get green easily; remove or simplify unnecessary checks when CI is blocking merges.
- For large PR queues, prefer parallel per-PR workers over a single serial queue.
- When closing multiple related issues, prefer one PR that closes them together.
- Do not edit attached plan files during implementation.
- Prefer free always-on Wave 0 staging (Oracle Always Free + Cloudflare Tunnel) over paid Render when cost matters; a public HTTPS API is required for Freighter/Vercel (localhost alone is not enough).
- SCF application has advanced past the Interest Form to next-stage Q&A; materials should prefer Open Track (not Integration Track) and lead with non-custodial Stellar SDEX+AMM aggregation, with cross-chain bridge/swap/offramp as the longer-term vision.

## Learned Workspace Facts
- Canonical GitHub repo is `StellarRoute/StellarRoute`; local path is `/Users/daniel/Desktop/2026/StellarRoute`.
- Project participates in the Drips/Stellar Wave contributor program; issues commonly use Wave/`help wanted`/complexity labels.
- Frontend production is on Vercel (`stellarroute.app` and `www.stellarroute.app`); GitHub-linked auto-deploy from `main` with root directory `frontend`; API CORS and env allowlists should include both hosts; wiring to a public testnet API/indexer is an explicit product goal.
- Browser wallet support is Freighter, xBull, Albedo, and LOBSTR; Freighter detection should use `isConnected()`, not `isAllowed`. Destination EVM wallets use injected providers and WalletConnect (Stellar wallets cannot sign the mint/gas step).
- Wave 0 public testnet API path is Oracle Always Free ARM VM + `deploy/docker-compose.prod.yml` + Cloudflare Tunnel; staging API hostname is `https://34.224.110.144.sslip.io` (sslip.io); staging deploys via GitHub Actions `deploy-ec2-staging.yml` on `push` to `main` for `crates/**` (not gated on CI green; manual `gh workflow run` is the fallback); runbook is `docs/deployment/oracle-always-free.md` (paid Render blueprint remains optional later).
- Stellar CCTP source burn is two-step: USDC `approve`, then `deposit_for_burn`; `submit-burn` must classify on-chain function (never verify approval txs as burns). CCTP v2 API is under `/api/v2/bridge/cctp/`; quote TTL defaults to 5 minutes. Iris `pending_confirmations` returns `message: null` / `attestation: "PENDING"` — client must treat `message` as optional (required-String parse failures become 503 and freeze the UI). Both directions quote Fast (`minFinalityThreshold` 1000, Iris-priced); Stellar burn builder must pass transfer finality (never hardcode Standard). EVM→Stellar message header recipient is Stellar TokenMessenger (not MessageTransmitter). Iris tx-hash query is domain-specific: EVM wants `0x…`, Stellar domain 27 rejects `0x` (bare hex only). Freighter-signed mint `submit-mint` must hash the envelope with signatures stripped so it matches the unsigned prepare hash; Fast completion nets `fee_executed` from the forward amount; `prepare-mint` must be idempotent when status is already `mint_prepared`. Key paths: `crates/api/src/cctp/*`, `frontend/hooks/useCctpSaga.ts`, `frontend/lib/cctp/*`.
- Cross-chain CCTP swap is fail-closed behind `CCTP_ENABLED` (defaults false); enabling `/cross-chain` in an environment also requires `CCTP_ACCESS_TOKEN_HMAC_KEY` and a Sepolia RPC URL; local reverse/UI proof runs need CCTP enabled so the corridor appears in the swap flow.
- CCTP Stellar Testnet → Sepolia signed-live destination mint proven 2026-08-14: `0x713cc8b174d775bf7a3a97f33c53a37f698c93bc66b378dfa55ccfcc7f1cbed6` (25 USDC); narrative `docs/cctp/signed-live-stellar-to-sepolia.md`. Reverse Sepolia→Stellar signed-live proven 2026-08-14: burn `0x339b96ccb6c3bcc0eb4c37d70fb5b8e6f3ee4c6fd1e7c032e93827faab6a5e73`, mint `13d2025db39b461756954e1266864ea39c126cada55ddf24db9ec364138d16f2` (5 USDC Fast); narrative `docs/cctp/signed-live-sepolia-to-stellar.md`. Staging flip via `docs/deployment/cctp-staging-enablement-checklist.md`. Prefer staging enablement before mainnet or Solana (Solana is a separate CCTP destination stack, not a shortcut).
- Live swap execution today is classic one-hop SDEX `PathPaymentStrictSend`; multi-hop/AMM Soroban settlement and public CCTP remain gated milestones. Offramp UI is at `/offramp` (`frontend/app/offramp`, `frontend/lib/offramp`); Paycrest is the chosen NGN partner (`docs.paycrest.io`) but has no Stellar deposit network (EVM/Starknet/Tron only; 8-char institution codes, not 3-digit CBN bank codes). SCF reviewer architecture SoT: `docs/architecture/scf-technical-architecture.md`.
- Related sibling work under `~/Desktop/2026/` (separate from this repo) includes StellarHydra, WaveFlow, route-visualizer, and swap-agrregrator — do not commit StellarRoute changes into those trees by mistake.
- Frontend Vitest in CI is split by path (app/components/hooks/lib); flaky or heavy suites have been a recurring main-branch blocker.
- `gh` is the expected interface for GitHub issues, PRs, labels, and CI log inspection; `main` has classic branch protection blocking force pushes (`allow_force_pushes=false`, `enforce_admins=true`).
