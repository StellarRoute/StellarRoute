# AGENTS.md

Guidance for AI agents working in the StellarRoute repository.

## Overview
StellarRoute is a Rust-first Stellar DEX aggregator routing trades across the Stellar Decentralized Exchange (SDEX) and Soroban AMMs, with cross-chain CCTP bridge capabilities and a Next.js web application.

### Monorepo Structure
- `crates/indexer`: Ingests SDEX offers and Soroban AMM state into PostgreSQL.
- `crates/api`: HTTP API serving quotes, routes, and orderbook data.
- `crates/routing`: Core pathfinding, venue scoring, and health evaluation engine.
- `crates/contracts`: Soroban smart contracts.
- `crates/sdk-rust`: Rust client SDK.
- `frontend`: Next.js web UI (Tailwind CSS, Vitest, Freighter/EVM wallet connectors).
- `sdk-js`: TypeScript client SDK.

---

## Development & Verification Commands

### Rust Workspace
```bash
# Build & format
cargo build
cargo fmt --all -- --check

# Clippy verification
cargo clippy --workspace --all-features --exclude stellarroute-contracts -- -D warnings
cargo clippy -p stellarroute-contracts -- -D warnings

# Tests (fast workspace / unit tests)
cargo test --workspace --lib --exclude stellarroute-contracts --exclude stellarroute-api
cargo test -p stellarroute-api --lib
cargo test -p stellarroute-api --test swap_integration --test openapi_swap_contract

# Full test suite
cargo test
```

### Frontend (`frontend/`)
```bash
npm --prefix frontend install
npm --prefix frontend run dev
npm --prefix frontend run build
npm --prefix frontend run lint
npm --prefix frontend run test
```

### JS SDK (`sdk-js/`)
```bash
npm --prefix sdk-js install
npm --prefix sdk-js run build
npm --prefix sdk-js run test
npm --prefix sdk-js run typecheck
```

### Local Services & Runtime
```bash
# Local databases (Postgres + Redis)
docker compose up -d

# Run API & Indexer
cargo run -p stellarroute-api
cargo run -p stellarroute-indexer
```

---

## Key Conventions & Guardrails
- **Fault-Tolerant Quotes**: Stale or unfresh market data must return degraded quotes with warnings rather than hard-failing user swap requests.
- **Fail-Closed Features**: Cross-chain CCTP and multi-hop execution features are gated behind configuration flags (e.g. `CCTP_ENABLED`).
- **Wallet Compatibility**: Freighter wallet detection should use `isConnected()` instead of `isAllowed()`.

---

## Documentation Pointers (Progressive Disclosure)
Refer to detailed documentation only when relevant to the task:
- Local setup & env configuration: `docs/development/SETUP.md`
- Database schema & unified liquidity: `docs/architecture/database-schema.md`
- Technical architecture: `docs/architecture/scf-technical-architecture.md`
- CCTP cross-chain bridge flows: `docs/cctp/`
- Deployment runbooks: `docs/deployment/oracle-always-free.md`

