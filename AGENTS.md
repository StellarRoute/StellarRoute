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
- Start Postgres + Redis:
  - `docker-compose up -d`
- Check service health:
  - `docker-compose ps`

### Rust workspace
- Build all crates:
  - `cargo build`
- Run all tests:
  - `cargo test`
- Run formatting check (same as CI):
  - `cargo fmt --all -- --check`
- Run clippy (same as CI):
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
