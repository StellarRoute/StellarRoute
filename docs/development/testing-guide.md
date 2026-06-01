# Testing Guide

This guide consolidates the test commands, conventions, and common fixes used across the Rust workspace, Soroban contracts, integration tests, and the frontend Vitest suite.

## 1. Quick start

Run these from the repository root:

```bash
cargo test
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

For API integration tests that hit Postgres, start the local services first:

```bash
docker-compose up -d
```

Then use the crate-specific commands below.

---

## 2. Rust workspace and crate-level tests

### Workspace-wide

```bash
cargo test
cargo test -- --include-ignored
```

Use `--include-ignored` only for tests that intentionally require live services such as PostgreSQL or Redis.

### Per-crate examples

```bash
# API
cargo test -p stellarroute-api
cargo test -p stellarroute-api --test validation_integration
cargo test -p stellarroute-api -- --include-ignored

# Routing engine
cargo test -p stellarroute-routing

# Indexer
cargo test -p stellarroute-indexer

# Soroban contracts
cargo test -p stellarroute-contracts
cargo test -p stellarroute-contracts e2e
```

If you only want one test file, pass the test name or file path after `--test` or `-p` as needed.

---

## 3. Soroban contract tests and snapshots

Contract tests live under `crates/contracts/` and are the primary place for router and AMM behavior validation.

### Recommended commands

```bash
cargo test -p stellarroute-contracts
cargo test -p stellarroute-contracts e2e
cargo clippy -p stellarroute-contracts --all-targets -- -D warnings
```

### Snapshot conventions

- Contract snapshot artifacts are stored in `crates/contracts/test_snapshots/`.
- The two main subtrees are:
  - `crates/contracts/test_snapshots/test/` for contract test snapshots
  - `crates/contracts/test_snapshots/e2e_harness/` and `crates/contracts/test_snapshots/benchmarks/` for larger scenario and benchmark outputs
- Treat snapshot changes as deliberate test outputs. Review them when a contract behavior or error path changes.

If a contract test fails because a snapshot changed, inspect the diffs and update the expected snapshot only when the behavior change is intentional.

---

## 4. API integration tests and live dependencies

The API integration suite under `crates/api/tests/` uses a mix of self-contained tests and ignored tests that require Postgres or Redis.

### Typical commands

```bash
cargo test -p stellarroute-api
cargo test -p stellarroute-api --test validation_integration
cargo test -p stellarroute-api -- --include-ignored
```

### Dependencies required for live tests

- `DATABASE_URL` for Postgres-backed integration tests
- Optional `REDIS_URL` for caching and rate-limit scenarios
- `docker-compose up -d` to start the local services listed in `docker-compose.yml`

The same database and Redis assumptions are used by the API server and the integration tests under `crates/api/tests/`.

### Common API integration failure modes

- `connection refused` or `database does not exist`:
  - Start the local stack with `docker-compose up -d`
  - Verify the expected Postgres/Redis ports in `docker-compose.yml`
- `DATABASE_URL not set` for an ignored integration test:
  - Export the value before running `cargo test -- --include-ignored`

---

## 5. Routing benchmarks

Routing performance benchmarks are located in `crates/routing/benches/` and are run with Criterion.

### Commands

```bash
cargo bench -p stellarroute-routing
cargo bench -p stellarroute-routing --bench routing_benchmarks
```

Use the benchmark suite when tuning pathfinding, optimizer logic, or route-selection heuristics. Results are also referenced by the performance budget documents in `docs/`.

---

## 6. Frontend Vitest suite

The frontend test suite uses Vitest and jsdom.

### Commands

```bash
npm --prefix frontend install
npm --prefix frontend test
npm --prefix frontend run test -- src/path/to/file.test.tsx -t "test name"
```

### Important Vitest setup details

The test environment is configured in `frontend/vitest.setup.ts`:

- `window.matchMedia` is polyfilled because jsdom does not implement it by default.
- `window.localStorage` is patched so components that read or write storage behave consistently in tests.

The icon mock in `frontend/__mocks__/lucide-react.tsx` is also important for tests that render UI components using `lucide-react`.

### Common frontend test failures

- `matchMedia is not a function`:
  - Ensure the test uses the shared setup in `frontend/vitest.setup.ts`.
- `lucide-react` import errors or missing icon exports:
  - Use the existing mock under `frontend/__mocks__/lucide-react.tsx`.
- `localStorage` is undefined:
  - Confirm the shared Vitest setup is active for the test run.

### Ladle / visual storybook CI command

```bash
npm --prefix frontend run storybook:ci
```

This is the CI-oriented Ladle build command used for the frontend story/snapshot path.

---

## 7. CI workflow mapping

The local commands above map to the GitHub Actions workflows in `.github/workflows/`:

| Workflow | What it covers | Local equivalent |
| --- | --- | --- |
| `.github/workflows/ci.yml` | Rust backend checks, frontend Vitest, visual regression, SDK checks | `cargo test`, `cargo fmt --check`, `cargo clippy`, `npm --prefix frontend test` |
| `.github/workflows/gas-benchmarks.yml` | Soroban gas/benchmark checks for contract changes | `cargo test -p stellarroute-contracts` and benchmark-oriented contract runs |
| `.github/workflows/verify-contracts.yml` | Contract bytecode verification and on-chain comparison | `cargo build --release --target wasm32-unknown-unknown` in `crates/contracts` |

Use this mapping when you need to explain which CI path a test failure is likely coming from.

---

## 8. Coverage expectations

The roadmap references these minimum expectations:

- Backend / Rust coverage target: at least 70%
- Contracts coverage target: at least 90%

These are planning targets for the wider test strategy; use them to judge whether a change is sufficiently exercised before opening a PR.

---

## 9. Common fixes for stale or broken local runs

- Lockfile drift or Cargo resolution issues:
  - Run `cargo generate-lockfile` if the lockfile is out of date, then rerun `cargo test`.
- Missing Docker services:
  - Run `docker-compose up -d` and confirm the containers are healthy.
- Frontend mocks missing in Vitest:
  - Reuse the shared setup and mock files under `frontend/vitest.setup.ts` and `frontend/__mocks__/lucide-react.tsx`.
- Ignored integration tests not running:
  - Add the required environment variables, then rerun with `-- --include-ignored`.
