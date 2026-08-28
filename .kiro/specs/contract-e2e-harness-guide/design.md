# Design Document: Contract E2E Harness Guide

## Overview

This is a docs-only feature. The deliverable is an additive section appended to `docs/contracts/testing-guide.md` that explains how to run, read, and extend the contract end-to-end harness in `crates/contracts/src/e2e_harness.rs`.

No production code is modified. The new section targets Wave contributors who may be unfamiliar with the Soroban test harness, mock pool architecture, and MEV protection test patterns.

## Architecture

The guide lives entirely in documentation. The implementation is:

- **Primary file modified**: `docs/contracts/testing-guide.md` (additive append only)
- **No new files created** under `crates/`, `frontend/`, `sdk-js/`, or any OpenAPI spec

The single-file approach (appending to the existing guide) is chosen over a separate `e2e-harness.md` to keep all contract testing documentation in one place and avoid navigation overhead for contributors.

The section order mirrors how a contributor would actually approach the harness: run it first, understand the environment, then understand what each section tests, then look up errors and extend with their own tests.

## Components and Interfaces

The guide section is composed of eight subsections:

1. **Quick Start** — run commands with copy-paste examples
2. **Test Environment** — `Env::default()`, `mock_all_auths()`, no-network guarantee
3. **Mock Pool Reference** — 3-row table describing MockPool99, MockPool98, MockPoolFail
4. **Helper Functions Reference** — descriptions of all 7 harness helpers
5. **Test Section Map** — one-paragraph description of each of the 5 test groups
6. **Error Variants Reference** — 2-column table of ContractError variants used in the harness
7. **Rollback Guarantee** — explanation of transactional atomicity on failure
8. **MEV Protection** — commit-reveal, rate limiting, whitelist coverage
9. **Copy-Paste Skeleton** — a minimal `#[test]` function using ARRANGE/ACT/ASSERT

There are no interfaces in the traditional sense — the guide is static markdown.

## Data Models

The guide documents these data structures as they appear to test authors:

**Mock pools** (defined in `e2e_harness.rs`):

| Name | Return behavior | Use case |
|---|---|---|
| `MockPool99` | Returns `amount_in * 99 / 100` | Happy-path single and multi-hop tests |
| `MockPool98` | Returns `amount_in * 98 / 100` | Multi-hop compounding, mixed-rate routes |
| `MockPoolFail` | Always panics | Failure, rollback, and error-path tests |

**Helper functions** (re-exported from `e2e_helpers.rs`, also defined in `e2e_harness.rs`):

| Function | Description |
|---|---|
| `setup()` | Creates `Env::default()` with `mock_all_auths()` |
| `deploy_router(env)` | Registers the `StellarRoute` contract; initializes with `fee_bps=30`, random admin and `fee_to` |
| `deploy_pool_99(env)` | Registers `MockPool99` |
| `deploy_pool_98(env)` | Registers `MockPool98` |
| `deploy_pool_fail(env)` | Registers `MockPoolFail` |
| `multi_pool_route(env, pools)` | Builds a `Route` with one `RouteHop` per pool address; all hops use `Asset::Native` and `PoolType::AmmConstProd` |
| `swap_params(env, route, amount_in, min_out)` | Builds a `SwapParams` with a random recipient, `deadline = seq + 200`, and `not_before = 0` |

**ContractError variants exercised in the harness** (from `errors.rs`):

| Variant | When triggered |
|---|---|
| `SlippageExceeded` | `amount_out < min_amount_out` |
| `DeadlineExceeded` | `deadline < current_ledger_sequence` |
| `ExecutionTooEarly` | `not_before > current_ledger_sequence` |
| `Paused` | Router is paused via `pause()` |
| `AmmSwapCallFailed` | Pool's `swap()` panics (e.g. `MockPoolFail`) |
| `PoolNotSupported` | Pool address not registered via `register_pool()` |
| `InvalidRecipient` | Recipient is a contract address |
| `InvalidAmount` | `amount_in <= 0` |
| `InvalidRoute` | Empty route or hops > `MAX_HOPS` (4) |
| `RateLimitExceeded` | Sender exceeds `rate_limit_max_swaps` within the window |
| `CommitmentRequired` | `amount_in >= commitment_required_above` with no prior commitment |

**Router initialization** (from `deploy_router` in both `e2e_harness.rs` and `e2e_helpers.rs`):
- `fee_bps = 30` (30 basis points protocol fee)
- Admin and `fee_to` are randomly generated `Address` values
- All optional MEV fields initialized to `None` by default; `configure_mev()` is called separately in MEV tests

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system — essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

The testable acceptance criteria for this docs feature fall into two categories:

- **Examples**: specific string or structural facts that must be present in the output file (e.g., the run command, ARRANGE/ACT/ASSERT markers)
- **Properties**: universal claims over a collection (e.g., every documented helper name must exist in source; every documented error variant must exist in `errors.rs`)

After prework reflection, two properties emerge as genuinely universal (applying to all members of a defined set), while the rest are specific examples handled by the Testing Strategy.

---

### Property 1: Run command presence

The string `cargo test -p stellarroute-contracts e2e` must appear in `docs/contracts/testing-guide.md` after the guide is written.

*For any* reader of the guide, the primary run command they would need to execute must be present verbatim in the document, matching the actual command that exercises `crates/contracts/src/e2e_harness.rs`.

**Validates: Requirements 1.1**

---

### Property 2: All documented ContractError variants exist in source

*For any* `ContractError` variant name documented in the guide's error reference table, that variant must also be declared in `crates/contracts/src/errors.rs`.

This prevents documentation drift where the guide lists a variant that was renamed or removed from the actual enum. The 11 variants listed (SlippageExceeded, DeadlineExceeded, ExecutionTooEarly, Paused, AmmSwapCallFailed, PoolNotSupported, InvalidRecipient, InvalidAmount, InvalidRoute, RateLimitExceeded, CommitmentRequired) must all be verifiable against the source enum.

**Validates: Requirements 4.1**

---

### Property 3: All documented helper function names exist in source

*For any* helper function name documented in the guide's helper reference table, that function must appear in `crates/contracts/src/e2e_helpers.rs` or `crates/contracts/src/e2e_harness.rs`.

The 7 helpers are: `setup`, `deploy_router`, `deploy_pool_99`, `deploy_pool_98`, `deploy_pool_fail`, `multi_pool_route`, `swap_params`. If any is renamed or removed in source, this property catches the documentation being stale.

**Validates: Requirements 2.3**

---

### Property 4: MAX_HOPS value matches source

The value `4` documented as `MAX_HOPS` in the guide must match the actual behavior exhibited by the harness — specifically that a 5-hop route returns `ContractError::InvalidRoute` (proven by `e2e_multi_hop_five_hops_rejected`) while a 4-hop route succeeds (proven by `e2e_multi_hop_four_distinct_pools_max_hops`).

*For any* documented constant value in the guide, it must match the observable behavior of the contract tests.

**Validates: Requirements 3.2**

## Error Handling

This is a documentation feature; there are no runtime errors. The relevant error-handling design decisions are:

- If the guide section is appended to the wrong file, a PR reviewer will catch it — the additive constraint is enforced by the PR description listing all touched files.
- Stale documentation (variants or helper names that drift from source) is caught by Property 2 and Property 3 above, which are written as verifiable checks rather than prose claims.
- The guide explicitly avoids claiming behaviors not proven by actual test names — every behavior described maps to a named test function in `e2e_harness.rs`.

## Testing Strategy

### Dual approach

Both unit/example checks and property checks apply here, even for a docs feature.

**Example checks** (specific facts that must be present):

These are verified manually during PR review and can be automated as grep/string checks:

- `cargo test -p stellarroute-contracts e2e` appears in the file
- `e2e::<test_name>` (single-test pattern) appears in the file
- "no Postgres" / "no Redis" / "no network" statement appears
- `--nocapture` appears
- `Env::default()` and `mock_all_auths()` both appear
- All 3 mock pool names (MockPool99, MockPool98, MockPoolFail) appear
- `fee_bps` and `30` appear together in router initialization context
- All 5 test section names appear (Direct, Multi-hop, Event, Failure, MEV)
- `MAX_HOPS` and `4` appear
- `Err(Ok(ContractError::` pattern appears (error assertion idiom)
- Nonce and volume rollback text appears
- `100_000` and `commitment_required_above` appear in MEV section
- `ExecutionTooEarly` and `CommitmentRequired` appear
- `ARRANGE`, `ACT`, `ASSERT` comment markers appear in the skeleton
- `use` statements referencing `e2e_helpers` or `e2e_harness` appear in the skeleton

**Property-based checks** (universal claims over a set):

These are well-suited for a small script or CI check that parses the markdown and cross-references source:

- **Feature: contract-e2e-harness-guide, Property 2**: For each variant name in the error table, `grep` for it in `crates/contracts/src/errors.rs` must succeed.
- **Feature: contract-e2e-harness-guide, Property 3**: For each helper name in the helper table, `grep` for it in `crates/contracts/src/e2e_helpers.rs` or `e2e_harness.rs` must succeed.

Minimum iterations for property checks: each set has a small fixed cardinality (11 variants, 7 helpers), so exhaustive checking is appropriate — no randomization needed.

### What does not need testing

- The prose quality, formatting, or readability of the guide (human review)
- The ARRANGE/ACT/ASSERT skeleton being runnable (it is illustrative, not a real test file)
- Structural constraints like "does not touch crates/" (enforced by git diff in PR review)

### PR review checklist

The PR description for the implementation task must list:
- Every file touched (expected: only `docs/contracts/testing-guide.md`)
- Confirmation that no file under `crates/`, `frontend/`, or `sdk-js/` was modified
- Confirmation that the new section is appended after the existing content
