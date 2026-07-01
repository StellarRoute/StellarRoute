# StellarRoute Security Audit Package

This directory contains all materials required for an external security audit of the StellarRoute smart contracts.

## Contentslk

| Document | Description |
|----------|-------------|
| [architecture.md](architecture.md) | Contract architecture, data flow, and trust model |
| [scope.md](scope.md) | Files and functions in scope for audit |
| [assumptions.md](assumptions.md) | Security assumptions and trust boundaries |
| [known-issues.md](known-issues.md) | Known limitations and accepted risks |

## Quick Start for Auditors

1. Clone the repository and follow setup in `docs/development/SETUP.md`.
2. Review the architecture overview in `architecture.md`.
3. Focus on files listed in `scope.md`.
4. Build and run tests:
   ```bash
   cd crates/contracts
   cargo build --release --target wasm32-unknown-unknown
   cargo test
   ```
5. Generate coverage report (requires `cargo-tarpaulin`):
   ```bash
   cargo install cargo-tarpaulin
   cargo tarpaulin -p stellarroute-contracts --out Html
   ```

## Contract Version

- Soroban SDK: `21.0.0`
- Rust edition: `2021`
- Target: `wasm32-unknown-unknown`

## External Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `soroban-sdk` | 21.0 | Soroban smart contract framework |
| `soroban-token-sdk` | 21.0.0 | Token interface utilities |

## Self-Assessment Checklist

- [x] All public functions validate inputs (fee_rate bounds, empty route, hop count)
- [x] Arithmetic uses checked operations (`overflow-checks = true` in release profile)
- [x] Access control on all admin functions (`require_auth()`)
- [x] No funds can be stuck in the contract (router is stateless for swaps)
- [x] Emergency pause covers `get_quote` path (paused state is queryable)
- [x] Events emitted for all state changes (init, admin change, pool register, pause/unpause)
- [x] No reentrancy risk (Soroban execution model is single-threaded per invocation)
- [x] Storage TTLs managed (instance: 7 days bump, persistent pools: 30 days)
- [x] Error paths return typed `ContractError` variants
- [x] WASM size optimized (`opt-level = "z"`, LTO enabled, symbols stripped)

## Contact

For questions during the audit, open a GitHub issue or contact the maintainers.
