# StellarRoute: Project Overview & Instructional Context

StellarRoute is an open-source DEX aggregation engine and UI for the Stellar ecosystem. It provides unified price discovery and optimal routing across both traditional Stellar DEX (SDEX) orderbooks and modern Soroban-based AMM pools.

## 🚀 Project Overview

- **Purpose:** Solve the gap in price discovery and routing left by the deprecation of the SDEX Explorer.
- **Main Technologies:**
    - **Backend (Rust):** Tokio, Axum, SQLx (PostgreSQL), Redis.
    - **Smart Contracts (Rust):** Soroban SDK.
    - **Frontend (TypeScript):** Next.js 16, React 19, Tailwind CSS 4, shadcn/ui.
    - **SDKs:** Rust and TypeScript/JavaScript.
- **Architecture:** Modular Rust workspace with separate crates for indexing, API, routing, and contracts.

## 🏗️ Core Components

1.  **`crates/indexer`:** Syncs SDEX orderbooks and Soroban AMM states to PostgreSQL. Supports polling and streaming (SSE) from Horizon.
2.  **`crates/api`:** Axum-based REST and WebSocket API providing quotes, orderbook snapshots, and trading pairs.
    - Entry point: `crates/api/src/bin/stellarroute-api.rs`.
3.  **`crates/routing`:** Multi-hop pathfinding algorithm for discovering optimal trade routes.
4.  **`crates/contracts`:** Soroban smart contracts for on-chain swap execution and router interfaces.
5.  **`frontend/`:** Modern Next.js web interface for traders with real-time price updates and wallet integration.
6.  **`sdk-js/`:** TypeScript SDK for easy integration into web applications.
7.  **`crates/sdk-rust/`:** Rust SDK for backend integrations.

## 🛠️ Building and Running

### Prerequisites
- Rust 1.75+
- Docker & Docker Compose
- Node.js 18+ (for frontend/SDK development)
- [Soroban CLI](https://developers.stellar.org/docs/smart-contracts/getting-started/setup#install-the-soroban-cli)

### Environment Setup
Create a `.env` file in the root directory:
```env
DATABASE_URL=postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute
REDIS_URL=redis://localhost:6379
STELLAR_HORIZON_URL=https://horizon.stellar.org
SOROBAN_RPC_URL=https://soroban-rpc.testnet.stellar.org
```

### Local Services
Start PostgreSQL and Redis:
```bash
docker-compose up -d
```

### Backend (Rust)
- **Build all:** `cargo build`
- **Run API:** `cargo run -p stellarroute-api`
- **Run Indexer:** `cargo run -p stellarroute-indexer`
- **Run Tests:** `cargo test` (unit tests)
- **Integration Tests:** `DATABASE_URL=... cargo test -- --include-ignored`

### Frontend (Next.js)
Located in `frontend/`:
- **Install:** `npm install`
- **Dev:** `npm run dev`
- **Build:** `npm run build`
- **Test:** `npm run test` (Vitest)
- **E2E Test:** `npm run test:e2e` (Playwright)

### JS SDK
Located in `sdk-js/`:
- **Install:** `npm install`
- **Build:** `npm run build`
- **Test:** `npm run test`

## 📝 Development Conventions

- **Code Style:**
    - **Rust:** Follow idiomatic patterns; use `cargo fmt` and `cargo clippy`. Unsafe code is strictly forbidden (enforced by workspace lint).
    - **TypeScript:** Use ESLint and Prettier. Follow existing patterns for hooks and components.
- **Testing:**
    - **Backend:** Use Rust's built-in testing framework (`cargo test`). Integration tests that require a database should be marked with `#[ignore]`.
    - **Frontend/SDK:** Use `vitest`.
- **Workflow:**
    - **Conventional Commits:** Use `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`.
    - **Branching:** `main` (stable), `feature/*`, `fix/*`, `docs/*`.
    - **CI/CD:** GitHub Actions for builds, tests, and linting. Always ensure CI is green before merging.
- **Technical Integrity:**
    - Use `thiserror` for library errors and `anyhow` for application-level errors in Rust.
    - Prefer `tracing` over `println!`.
    - Handle `window.matchMedia` missing in JSDOM by providing a mock in `frontend/vitest.setup.ts`.
    - Use `frontend/__mocks__/lucide-react.tsx` for icon mocking in tests.

## 📂 Key Directories
- `crates/`: Rust workspace members.
- `frontend/`: Next.js application.
- `sdk-js/`: TypeScript SDK.
- `docs/`: Comprehensive architecture, API, and development guides.
- `scripts/`: Deployment and utility scripts.
- `migrations/`: SQL migrations for PostgreSQL (managed by `sqlx`).

## 🧠 Project Specific Knowledge
- **Asset Format:** Stellar assets are typically represented as `native` or `CODE:ISSUER`.
- **API Performance:** Target <500ms for quote requests.
- **Caching:** Redis is used for pairs (10s), orderbooks (5s), and quotes (2s).
- **Icon Mocking:** If tests fail due to missing icons, check `frontend/__mocks__/lucide-react.tsx`.
- **URL State:** The frontend uses URL query parameters (`base`, `quote`) for persisting trading pair selection.
