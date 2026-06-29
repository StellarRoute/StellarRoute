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
