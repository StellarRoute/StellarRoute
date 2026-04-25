# M2 Readiness Troubleshooting Guide

This document explains how to interpret the results of the `m2-readiness` tool and what actions to take when checks fail.

## Dimensions

### 1. Infrastructure
These checks ensure that the core dependencies for Milestone 2 are accessible.

- **Database Connectivity**: Fails if `DATABASE_URL` is missing or the PostgreSQL instance is down.
  - *Action*: Ensure Postgres is running via `docker-compose up -d` and check your `.env` file.
- **Redis Connectivity**: Fails if Redis is unreachable.
  - *Action*: Ensure Redis is running via `docker-compose up -d`.

### 2. Test Health
Ensures that all M2-related code is stable and regression-free.

- **Tests: crates/routing**: Core pathfinding logic tests.
- **Tests: crates/api**: API endpoint and quote logic tests.
- **Tests: crates/indexer**: Data ingestion and AMM sync tests.
  - *Action*: Run `cargo test -p <package>` locally to debug specific failures.

### 3. Route Quality
Validates the performance and capability of the routing engine.

- **Pathfinding Latency**: Fails if initialization or basic pathfinding takes > 100ms.
  - *Action*: Check for inefficient loops in `Pathfinder` or large graph initialization overhead.
- **Multi-hop Support**: Fails if `max_hops` is configured to less than 2.
  - *Action*: Update `PathfinderConfig` in `crates/routing/src/pathfinder.rs`.

### 4. Data & AMM Readiness
Ensures the system is correctly indexing Soroban AMM pools.

- **Indexer Operational**: Fails if the indexer binary cannot be run.
  - *Action*: Check for compilation errors in `crates/indexer`.
- **AMM Data Model**: Fails if the required Soroban AMM pool models are missing.
  - *Action*: Ensure `crates/indexer/src/models/pool.rs` is implemented and exports the necessary structures.

## Running in CI
The `m2-readiness` tool returns a non-zero exit code on failure, making it suitable for use as a CI release gate.

```yaml
- name: M2 Readiness Check
  run: cargo run -p stellarroute-api --bin m2-readiness
```

## JSON Output
For automated reporting or dashboard integration, use the `--format json` flag:
```bash
cargo run -p stellarroute-api --bin m2-readiness -- --format json
```
