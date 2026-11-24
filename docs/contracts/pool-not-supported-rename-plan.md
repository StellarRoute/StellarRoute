# `PoolNotSupported` Rename Plan

## Status

Proposal only. No code change is included in this document's PR; it records the
rename rationale and the steps a follow-up PR must take.

## Why rename

`ContractError::PoolNotSupported` (`crates/contracts/src/errors.rs`) is returned
only when a pool address was never registered via `register_pool()` (see the
Error Variants Reference in `docs/contracts/testing-guide.md`). "Not supported"
reads as a permanent capability limit, which is misleading — the same pool
becomes valid the moment it is registered. The clearer name is
`PoolNotRegistered`, matching the actual precondition and the registration
terminology already used by `register_pool()` and the `pool_registered` event.

## Frozen-surface constraint

This rename must not touch any frozen surface:

- **OpenAPI/HTTP error codes** — unaffected. `PoolNotSupported` is a Soroban
  contract error (`#[contracterror]`, `u32` repr), not an HTTP API error code.
  It does not appear in `docs/api/error_taxonomy.md`'s error catalog and has
  no corresponding `snake_case` API code to rename.
- **Swap prepare/submit** and **quote ranking** — unaffected; this variant is
  surfaced only through contract call results, not through prepare/submit
  response bodies or ranking logic.
- **CCTP default-off** — unrelated to this error path.

## Migration steps (for the implementing PR)

1. Add the new variant `PoolNotRegistered = 30` to `ContractError` alongside
   the existing `PoolNotSupported = 30` for exactly one release, marking
   `PoolNotSupported` with a `#[deprecated]` doc comment that points to
   `PoolNotRegistered`. Both names share discriminant `30` during the overlap
   window so no caller observes a value change.
2. Update all contract call sites that return the error (e.g.
   `crates/contracts/src/router.rs`) to return `PoolNotRegistered`.
3. Update references in docs and tests to use the new name:
   - `docs/contracts/testing-guide.md` (Error Variants Reference table)
   - `docs/contracts/gas-optimization-usage.md` (code samples)
   - `crates/contracts/src/e2e_harness.rs`, `e2e_harness_doc_props.rs`,
     `test.rs`
4. After one release with both variants present, remove the deprecated
   `PoolNotSupported` variant in a subsequent, separately-tracked PR.

## Acceptance for the implementing PR

- `ContractError` compiles with both variants during the overlap window.
- All non-test call sites return `PoolNotRegistered`; no test asserts on the
  deprecated variant.
- `docs/contracts/testing-guide.md` and `gas-optimization-usage.md` reflect the
  new name.
- No change to `docs/api/error_taxonomy.md`, swap prepare/submit behavior,
  quote ranking, or CCTP defaults.
