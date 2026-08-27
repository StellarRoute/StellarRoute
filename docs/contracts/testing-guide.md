# Soroban Contracts Testing Guide

This guide covers testing the StellarRoute Soroban router contracts, with a focus on the multi-sig migration edge cases.

---

## Migration Edge-Case Tests

We've added a comprehensive suite of tests specifically for the `migrate_to_multisig` function and the pre/post migration state. These tests are located in `crates/contracts/src/test.rs` under the `// ── Migration Edge Case Tests ──────────────────────────────────────────────────` section.

### What they cover

1. **Admin functions blocked before migration**
   - Verifies that governance functions like `propose` cannot be called until `migrate_to_multisig` completes
   - Asserts error `ContractError::NotMultiSig`

2. **Threshold boundary conditions**
   - Valid threshold at exactly signer count (unanimous)
   - Valid threshold at 1 (minimum)
   - Rejects threshold 0
   - Rejects threshold exceeding signer count
   - Uses `ContractError::InvalidAmount` for all invalid threshold cases

3. **Duplicate signers rejected**
   - If duplicate addresses are in the signers list, migration fails
   - Uses `ContractError::InvalidAmount`

4. **Empty signers list rejected**
   - Migration fails if no signers are provided
   - Uses `ContractError::InvalidAmount`

5. **Non-admin cannot migrate**
   - Migration must be called by the initial admin set at contract initialization
   - Uses `ContractError::Unauthorized`

6. **Double migration rejected**
   - After `migrate_to_multisig` succeeds, calling it again fails
   - Uses `ContractError::AlreadyInitialized`

7. **Non-signer cannot propose post-migration**
   - After migration, only authorized signers can call governance functions
   - Uses `ContractError::Unauthorized`

---

## Running Migration Tests

To run just the migration edge case tests:

```bash
cargo test -p stellarroute-contracts migration
```

To run all contract tests:

```bash
cargo test -p stellarroute-contracts
```

---

## Input-Validation Fuzz Targets

Fuzz coverage for `validate_route` and `execute_swap` lives in
`crates/contracts/src/fuzz_targets.rs` (proptest). Full overnight instructions are in
[`audit/fuzzing.md`](../../audit/fuzzing.md).

```bash
# CI / local (default 64 cases per property)
cargo test -p stellarroute-contracts fuzz_ -- --nocapture

# Overnight
PROPTEST_CASES=500000 cargo test -p stellarroute-contracts fuzz_ -- --nocapture
# or:
./scripts/fuzz-contracts-overnight.sh
```

---

## Example Test Function

Here's an example of the pattern used in the migration tests, showing the `// ARRANGE // ACT // ASSERT` structure:

```rust
#[test]
fn test_double_migration_rejected() {
    // ARRANGE
    let env = setup_env();
    let (admin, _, client) = deploy_multisig_router(&env); // already migrated

    let s1 = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(s1);

    // ACT
    let result = client.try_migrate_to_multisig(
        &admin,
        &signers,
        &1_u32,
        &10000_u64,
        &None,
    );

    // ASSERT
    assert_eq!(result, Err(Ok(ContractError::AlreadyInitialized)));
}
```

---

## E2E Harness Guide

This section explains how to run, read, and extend the contract end-to-end harness in `crates/contracts/src/e2e_harness.rs`. No live Stellar network, wallet, or token contract is required.

---

### Quick Start

Run all E2E harness tests:

```bash
cargo test -p stellarroute-contracts e2e
```

Run a single named test:

```bash
cargo test -p stellarroute-contracts e2e::<test_name>
# example:
cargo test -p stellarroute-contracts e2e::e2e_direct_swap_single_pool_success
```

Surface `println!` and event output during a test run:

```bash
cargo test -p stellarroute-contracts e2e -- --nocapture
```

**No external dependencies required.** The harness runs entirely in-process using the Soroban SDK test environment. You do not need Postgres, Redis, Stellar Horizon, Soroban RPC, or a wallet.

---

### Test Environment

All harness tests use `Env::default()` from the Soroban SDK, which provides a fully sandboxed in-process Soroban execution environment. Calling `env.mock_all_auths()` removes authorization checks so tests can focus on swap logic.

```rust
let env = Env::default();
env.mock_all_auths();
```

No real ledger is created. No XLM balance is needed. Pool state is entirely defined by the mock pool contracts registered at test time.

---

### Mock Pool Reference

The harness defines three mock pool contracts inside `e2e_harness.rs`. Each is a minimal `#[contract]` that implements `adapter_quote`, `swap`, and `get_rsrvs`.

| Mock pool | Return behavior | Use case |
|---|---|---|
| `MockPool99` | Returns `amount_in * 99 / 100` | Happy-path single and multi-hop tests |
| `MockPool98` | Returns `amount_in * 98 / 100` | Multi-hop compounding, mixed-rate routes |
| `MockPoolFail` | Always panics | Failure, rollback, and error-path tests |

Deploy a mock pool into the test environment using the corresponding helper:

```rust
let pool = deploy_pool_99(&env);   // or deploy_pool_98, deploy_pool_fail
client.register_pool(&pool);       // must be registered before use
```

---

### Helper Functions Reference

Helper functions are defined in `crates/contracts/src/e2e_harness.rs` and re-exported by `crates/contracts/src/e2e_helpers.rs`.

| Function | Description |
|---|---|
| `setup()` | Creates `Env::default()` with `mock_all_auths()` |
| `deploy_router(env)` | Registers the `StellarRoute` contract; initializes with `fee_bps = 30`, randomly generated `admin` and `fee_to` addresses |
| `deploy_pool_99(env)` | Registers `MockPool99` |
| `deploy_pool_98(env)` | Registers `MockPool98` |
| `deploy_pool_fail(env)` | Registers `MockPoolFail` |
| `multi_pool_route(env, pools)` | Builds a `Route` with one `RouteHop` per pool address; all hops use `Asset::Native` and `PoolType::AmmConstProd` |
| `swap_params(env, route, amount_in, min_out)` | Builds `SwapParams` with a random recipient, `deadline = seq + 200`, `not_before = 0` |

---

### Test Section Map

The harness is organized into five sections. Use these names to filter with `cargo test -p stellarroute-contracts e2e::<prefix>`.

**Direct (single-hop) swap E2E tests** (`e2e_direct_*`)
Tests covering the happy path, output-less-than-input invariant, slippage guard, deadline enforcement, `not_before` enforcement, pause/unpause, and resume-after-unpause for single-pool routes.

**Multi-hop swap E2E tests** (`e2e_multi_hop_*`)
Tests covering 2-hop, 3-hop, and 4-hop routes using distinct pool contracts per hop. Verifies compounded output, the more-hops-less-output invariant, the `MAX_HOPS = 4` limit (a 5-hop route returns `ContractError::InvalidRoute`), and that `get_quote` matches the actual swap output.

**Event assertion tests** (`e2e_event_*`)
Tests verifying that `swap_executed`, `execution_requested`, `execution_failed`, `pause`, `unpause`, and `pool_registered` events are emitted at the right moments. Confirms that a multi-hop swap emits a single `swap_executed` event, not one per hop.

**Failure rollback / error recovery tests** (`e2e_failure_*`)
Tests verifying that broken pools, unregistered pools, zero/negative amounts, empty routes, contract recipients, and mid-route failures all return the expected `ContractError` and leave nonce and swap volume unchanged.

**MEV protection E2E tests** (`e2e_mev_*`)
Tests covering the commit-reveal flow, rate limiting, and whitelist bypass. Uses `configure_mev()` to set `commitment_required_above = 100_000`, `rate_limit_max_swaps = 3`, and other MEV parameters.

---

### Error Variants Reference

The harness exercises the following `ContractError` variants. Use `try_execute_swap` (or `try_execute`) to receive the error rather than panicking.

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
| `InvalidRoute` | Empty route or hop count > `MAX_HOPS` (4) |
| `RateLimitExceeded` | Sender exceeds `rate_limit_max_swaps` within the window |
| `CommitmentRequired` | `amount_in >= commitment_required_above` with no prior commitment |

Idiomatic error assertion pattern:

```rust
let result = client.try_execute_swap(&sender, &params);
assert_eq!(result, Err(Ok(ContractError::SlippageExceeded)));
```

---

### Rollback Guarantee

When a swap fails with any `ContractError`, the Soroban environment reverts all state changes atomically. This means:

- The sender's **nonce is not incremented**.
- The contract's **total swap volume counter is not updated**.

Use `MockPoolFail` to trigger a `ContractError::AmmSwapCallFailed` and verify rollback in tests:

```rust
let vol_before = client.get_total_swap_volume();
let _ = client.try_execute_swap(&sender, &params); // pool panics → AmmSwapCallFailed
assert_eq!(client.get_total_swap_volume(), vol_before);
```

---

### MEV Protection

The harness covers three MEV protection modes, enabled via `client.configure_mev(&mev_config)`.

**Commit-reveal**

Swaps where `amount_in >= commitment_required_above` (set to `100_000` in harness tests) require a prior commitment. Without one, execution returns `ContractError::CommitmentRequired`. The full flow is:

1. Call `client.commit_swap(&sender, &commitment_hash, &amount_in)` — submit the hash of `(amount_in, min_out, deadline, salt)`.
2. Call `client.execute_swap` (or the `execute` alias) with the matching parameters — the contract verifies the hash before executing.

`ContractError::ExecutionTooEarly` is returned when execution is attempted before the commit window opens.

**Rate limiting**

When `rate_limit_max_swaps` is set (e.g. `3`), a sender who exceeds that count within `rate_limit_window_ledgers` receives `ContractError::RateLimitExceeded`. Subsequent swaps in the same window are all rejected until the window rolls over.

**Whitelist bypass**

Addresses added via `client.set_whitelist(&addr, &true)` are exempt from rate limiting and can swap beyond `rate_limit_max_swaps` without restriction.

---

### Copy-Paste Skeleton

Use this pattern to add a new harness test. The skeleton follows the `// ARRANGE / // ACT / // ASSERT` convention used throughout the test suite.

```rust
#[cfg(test)]
mod my_new_tests {
    use crate::e2e_harness::{deploy_pool_99, deploy_pool_fail};
    use crate::e2e_helpers::{deploy_router, multi_pool_route, setup, swap_params};
    use crate::errors::ContractError;

    #[test]
    fn e2e_my_new_scenario() {
        // ARRANGE
        let env = setup();
        let (_admin, client) = deploy_router(&env);
        let pool = deploy_pool_99(&env);
        client.register_pool(&pool);

        let route = multi_pool_route(&env, &[pool]);
        let params = swap_params(&env, route, 10_000, 0);
        let sender = soroban_sdk::testutils::Address::generate(&env);

        // ACT
        let result = client.try_execute_swap(&sender, &params);

        // ASSERT
        assert!(result.is_ok(), "expected swap to succeed");
        let outcome = result.unwrap();
        assert!(outcome.amount_out > 0);
        assert!(outcome.amount_out < 10_000); // fees always apply
    }
}
```

For failure-path tests, swap `deploy_pool_99` for `deploy_pool_fail` and use `assert_eq!(result, Err(Ok(ContractError::AmmSwapCallFailed)))`.

---

Do NOT modify any existing content above this section. Do NOT touch any file under `crates/` (other than the two test files referenced in other tasks), `frontend/`, `sdk-js/`, or any OpenAPI spec.
