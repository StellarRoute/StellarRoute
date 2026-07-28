# Contract Fuzzing Guide

Fuzzing router entrypoints reduces mainnet exploit risk from malformed routes and amounts.

## Targets

| Target | Entrypoint | Coverage |
|--------|------------|----------|
| `fuzz_validate_route_*` | `validate_route` | Hop bounds, expiry, amount consistency, hop continuity, no-panic |
| `fuzz_execute_swap_*` | `execute_swap` | Amount/bps guards, hop bounds, deadline window, invalid recipient, no-panic |

Implementation: `crates/contracts/src/fuzz_targets.rs` (proptest structured fuzzing).

Soroban entrypoints need `Env`, registered contracts, and mocked auth, so we use **proptest** rather than raw `cargo-fuzz` byte corpora. The oracle for every target is: **no panic / host abort**; malformed inputs return typed `ContractError`.

## Quick run (CI / local)

```bash
cargo test -p stellarroute-contracts fuzz_ -- --nocapture
```

Default case count is 64 per property (suitable for CI).

## Overnight fuzz

Raise the case count and run only the fuzz module:

```bash
# Linux / macOS / Git Bash
PROPTEST_CASES=500000 cargo test -p stellarroute-contracts fuzz_ -- --nocapture

# Windows PowerShell
$env:PROPTEST_CASES = "500000"
cargo test -p stellarroute-contracts fuzz_ -- --nocapture
```

Or use the helper script (8-hour-friendly default of 500k cases):

```bash
./scripts/fuzz-contracts-overnight.sh
# optional override:
PROPTEST_CASES=1000000 ./scripts/fuzz-contracts-overnight.sh
```

### Suggested overnight report

After a long run, record under `audit/fuzz-runs/YYYY-MM-DD.md`:

- Date, commit SHA, runner
- `PROPTEST_CASES` value and wall-clock duration
- Targets exercised (`fuzz_validate_route_*`, `fuzz_execute_swap_*`)
- Crashes / panics found (with minimized repro if any)
- Pass / fail verdict and follow-up issues

## Acceptance

- [x] Fuzz targets for `execute_swap` / `validate_route`
- [x] Overnight run documented (this file + script)
- [x] No crashes on malformed inputs (no-panic targets + typed error oracles)
