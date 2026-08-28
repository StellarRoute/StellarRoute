# Implementation Plan: Contract E2E Harness Guide

## Overview

Append a 9-subsection E2E harness how-to guide to `docs/contracts/testing-guide.md`, then add a `#[cfg(test)]` module `e2e_harness_doc_props.rs` in `crates/contracts/src/` with four grep-based correctness checks. Only these two files (plus the `lib.rs` mod declaration) are touched.

## Tasks

- [x] 1. Append the E2E harness guide section to `docs/contracts/testing-guide.md`
  - Add a top-level `## E2E Harness Guide` section after the existing content
  - Subsection 1 — **Quick Start**: document `cargo test -p stellarroute-contracts e2e` as primary command, single-test pattern `e2e::<test_name>`, `--nocapture` tip, and the no-network statement (no Postgres, Redis, Horizon, Soroban RPC, or wallet required)
  - Subsection 2 — **Test Environment**: explain `Env::default()` with `mock_all_auths()`, fully in-process sandbox, no real network
  - Subsection 3 — **Mock Pool Reference**: 3-row table for `MockPool99` (99% of amount_in), `MockPool98` (98% of amount_in), `MockPoolFail` (always panics, use for rollback tests)
  - Subsection 4 — **Helper Functions Reference**: table of all 7 helpers (`setup`, `deploy_router`, `deploy_pool_99`, `deploy_pool_98`, `deploy_pool_fail`, `multi_pool_route`, `swap_params`) with descriptions; note `fee_bps = 30`, random admin and `fee_to`
  - Subsection 5 — **Test Section Map**: one-paragraph description of each of the 5 test groups (Direct, Multi-hop, Event assertions, Failure/rollback, MEV protection); note `MAX_HOPS = 4` and that a 5-hop route returns `ContractError::InvalidRoute`
  - Subsection 6 — **Error Variants Reference**: 2-column table of all 11 `ContractError` variants (`SlippageExceeded`, `DeadlineExceeded`, `ExecutionTooEarly`, `Paused`, `AmmSwapCallFailed`, `PoolNotSupported`, `InvalidRecipient`, `InvalidAmount`, `InvalidRoute`, `RateLimitExceeded`, `CommitmentRequired`) with trigger condition; show the idiomatic assertion pattern `assert_eq!(result, Err(Ok(ContractError::SlippageExceeded)))`
  - Subsection 7 — **Rollback Guarantee**: state that on any `ContractError` neither the nonce nor swap volume counter is mutated; reference `MockPoolFail` as the tool for triggering rollback scenarios
  - Subsection 8 — **MEV Protection**: explain commit-reveal flow (`commitment_required_above = 100_000`), `ExecutionTooEarly` when execution precedes commit window, `CommitmentRequired` when no commitment exists, rate-limiting and whitelist modes
  - Subsection 9 — **Copy-Paste Skeleton**: minimal `#[test]` fn with `// ARRANGE / // ACT / // ASSERT` markers, showing `Env::default()`, `mock_all_auths()`, pool deployment, router deployment, `swap_params()`, and a result assertion; include required `use` statements referencing `e2e_helpers` and `e2e_harness` module paths
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3, 2.4, 3.1, 3.2, 4.1, 4.2, 5.1, 5.2, 6.1, 6.2, 6.3, 7.1, 8.1, 8.2, 8.3_

- [x] 2. Create `crates/contracts/src/e2e_harness_doc_props.rs` with four correctness checks
  - [x] 2.1 Implement Property 1 — run command presence check
    - `#[test] fn prop_run_command_present()`: read `docs/contracts/testing-guide.md` and assert it contains `cargo test -p stellarroute-contracts e2e`
    - **Property 1: Run command presence**
    - **Validates: Requirements 1.1**
  - [ ]* 2.2 Confirm Property 1 test passes
    - Run `cargo test -p stellarroute-contracts e2e_harness_doc_props::prop_run_command_present`
  - [x] 2.3 Implement Property 2 — all documented ContractError variants exist in source
    - `#[test] fn prop_error_variants_in_source()`: for each of the 11 variant name strings, assert the string appears in `crates/contracts/src/errors.rs`
    - **Property 2: All documented ContractError variants exist in source**
    - **Validates: Requirements 4.1**
  - [ ]* 2.4 Confirm Property 2 test passes
    - Run `cargo test -p stellarroute-contracts e2e_harness_doc_props::prop_error_variants_in_source`
  - [x] 2.5 Implement Property 3 — all documented helper function names exist in source
    - `#[test] fn prop_helpers_in_source()`: for each of the 7 helper name strings, assert the name appears in `e2e_helpers.rs` or `e2e_harness.rs`
    - **Property 3: All documented helper function names exist in source**
    - **Validates: Requirements 2.3**
  - [ ]* 2.6 Confirm Property 3 test passes
    - Run `cargo test -p stellarroute-contracts e2e_harness_doc_props::prop_helpers_in_source`
  - [x] 2.7 Implement Property 4 — MAX_HOPS value documented correctly
    - `#[test] fn prop_max_hops_value_correct()`: read `docs/contracts/testing-guide.md` and assert it contains both `MAX_HOPS` and `4` in proximity; also assert the guide mentions `InvalidRoute`
    - **Property 4: MAX_HOPS value matches source**
    - **Validates: Requirements 3.2**
  - [ ]* 2.8 Confirm Property 4 test passes
    - Run `cargo test -p stellarroute-contracts e2e_harness_doc_props::prop_max_hops_value_correct`
  - _Requirements: 1.1, 2.3, 3.2, 4.1_

- [x] 3. Add `#[cfg(test)] mod e2e_harness_doc_props;` to `crates/contracts/src/lib.rs`
  - Append the module declaration after the existing `#[cfg(test)]` mod declarations
  - _Requirements: 7.3_

- [x] 4. Checkpoint — Ensure all tests pass
  - Run `cargo test -p stellarroute-contracts` and confirm the four new doc-prop tests pass
  - Ensure no existing tests are broken
  - Verify `docs/contracts/testing-guide.md` is the only docs file changed and no file under `crates/`, `frontend/`, or `sdk-js/` (other than the two new/modified test files) was modified
  - Ask the user if any questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- The only files that should be touched are: `docs/contracts/testing-guide.md`, `crates/contracts/src/e2e_harness_doc_props.rs` (new), and `crates/contracts/src/lib.rs` (mod declaration only)
- The copy-paste skeleton in the guide is illustrative — it does not need to be a compilable test in the docs file
- Property tests use `include_str!` macro and `contains()` string checks — no proptest or randomization needed
- All four properties verify documentation accuracy against source truth, preventing drift as the codebase evolves
