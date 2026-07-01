//! Secure, auditable contract upgrade mechanism for StellarRoute.
//!
//! Supports two modes:
//!
//! **Single-admin (pre-migration) mode** — Two-step time-locked upgrade:
//!   1. Admin calls `propose_upgrade(new_wasm_hash, execute_after)`.
//!   2. After `execute_after` ledger sequences, the admin calls `execute_upgrade()`.
//!   3. Admin may `cancel_upgrade()` at any time before execution.
//!
//! **Multi-sig mode** — Upgrade is encoded as a `ProposalAction::Upgrade` and
//! routes through the governance approval flow. The low-level
//! `execute_wasm_upgrade()` function is the shared path called by both modes.
//!
//! Post each WASM swap a `migrate()` hook runs exactly once to handle any
//! storage schema changes introduced by the new version.

use crate::errors::ContractError;
use crate::storage::{self, extend_instance_ttl};
use crate::types::{ContractVersion, PendingUpgrade};
use crate::{events, storage::StorageKey};
use soroban_sdk::{Address, BytesN, Env};

/// Minimum time-lock delay in ledger sequences (~6 hours at ~5 s/ledger).
pub const MIN_DELAY_LEDGERS: u64 = 4320;

// ─── Internal ────────────────────────────────────────────────────────────────

/// Zero-filled 32-byte hash used as a sentinel for "no previous version".
fn zero_hash(e: &Env) -> BytesN<32> {
    BytesN::from_array(e, &[0u8; 32])
}

// ─── Public API ───────────────────────────────────────────────────────────────

/// Return the current contract version. Returns a zeroed default if not yet set.
pub fn get_version(e: &Env) -> ContractVersion {
    storage::get_contract_version(e).unwrap_or(ContractVersion {
        major: 1,
        minor: 0,
        patch: 0,
        wasm_hash: zero_hash(e),
        upgraded_at: 0,
    })
}

/// Set the initial version record during `initialize()`.
pub fn set_initial_version(e: &Env, wasm_hash: BytesN<32>) {
    let v = ContractVersion {
        major: 1,
        minor: 0,
        patch: 0,
        wasm_hash,
        upgraded_at: e.ledger().sequence() as u64,
    };
    storage::set_contract_version(e, &v);
}

/// Propose a time-locked upgrade (single-admin mode only).
///
/// * `execute_after` is the minimum ledger sequence at which the upgrade may
///   be executed. Callers should set this to at least `MIN_DELAY_LEDGERS`
///   sequences in the future.
pub fn propose_upgrade(
    e: &Env,
    admin: soroban_sdk::Address,
    new_wasm_hash: BytesN<32>,
    execute_after: u64,
) -> Result<(), ContractError> {
    admin.require_auth();
    if storage::get_admin(e) != admin {
        return Err(ContractError::Unauthorized);
    }
    // Must not be in multi-sig mode.
    if storage::is_multisig(e) {
        return Err(ContractError::UseGovernance);
    }
    // Contract must not be paused (prevent upgrading into a locked state).
    if storage::get_paused(e) {
        return Err(ContractError::Paused);
    }
    // Reject a no-op upgrade.
    let current = get_version(e);
    if current.wasm_hash == new_wasm_hash {
        return Err(ContractError::SameWasmHash);
    }
    // Reject zero hash.
    if new_wasm_hash == zero_hash(e) {
        return Err(ContractError::InvalidAmount);
    }
    // Only one pending upgrade at a time.
    if storage::get_pending_upgrade(e).is_some() {
        return Err(ContractError::UpgradePending);
    }
    // Enforce minimum delay.
    let now = e.ledger().sequence() as u64;
    let effective_after = if execute_after < now + MIN_DELAY_LEDGERS {
        now + MIN_DELAY_LEDGERS
    } else {
        execute_after
    };

    let pending = PendingUpgrade {
        new_wasm_hash: new_wasm_hash.clone(),
        proposed_at: now,
        execute_after: effective_after,
        proposer: admin.clone(),
    };
    storage::set_pending_upgrade(e, &pending);

    events::upgrade_proposed(e, admin, current.wasm_hash, new_wasm_hash, effective_after);
    extend_instance_ttl(e);
    Ok(())
}

/// Execute a pending time-locked upgrade once the delay has elapsed.
/// Callable only by the active single-admin identity.
pub fn execute_upgrade(e: &Env, admin: Address) -> Result<(), ContractError> {
    if storage::get_admin(e) != admin {
        return Err(ContractError::Unauthorized);
    }
    admin.require_auth();

    let pending = storage::get_pending_upgrade(e).ok_or(ContractError::NoUpgradePending)?;

    let now = e.ledger().sequence() as u64;
    if now < pending.execute_after {
        return Err(ContractError::UpgradeLocked);
    }
    if storage::get_paused(e) {
        return Err(ContractError::Paused);
    }

    storage::clear_pending_upgrade(e);
    execute_wasm_upgrade(e, pending.new_wasm_hash)?;
    extend_instance_ttl(e);
    Ok(())
}

/// Cancel a pending time-locked upgrade. Callable by the original proposer.
pub fn cancel_upgrade(e: &Env, admin: soroban_sdk::Address) -> Result<(), ContractError> {
    admin.require_auth();
    let pending = storage::get_pending_upgrade(e).ok_or(ContractError::NoUpgradePending)?;

    if pending.proposer != admin {
        return Err(ContractError::Unauthorized);
    }

    storage::clear_pending_upgrade(e);
    events::upgrade_cancelled(e, admin);
    extend_instance_ttl(e);
    Ok(())
}

/// Core WASM replacement. Called by both the time-locked path and the
/// governance `ProposalAction::Upgrade` dispatch.
///
/// Performs the actual `update_current_contract_wasm` call, records the new
/// version, and emits the UpgradeCompleted event.
pub(crate) fn execute_wasm_upgrade(
    e: &Env,
    new_wasm_hash: BytesN<32>,
) -> Result<(), ContractError> {
    let old_version = get_version(e);
    let old_hash = old_version.wasm_hash.clone();

    if old_hash == new_wasm_hash {
        return Err(ContractError::SameWasmHash);
    }

    // Replace the WASM bytecode.
    e.deployer()
        .update_current_contract_wasm(new_wasm_hash.clone());

    // Record the new version (patch bump; callers can set major/minor via migrate).
    let new_version = ContractVersion {
        major: old_version.major,
        minor: old_version.minor,
        patch: old_version.patch + 1,
        wasm_hash: new_wasm_hash.clone(),
        upgraded_at: e.ledger().sequence() as u64,
    };
    storage::set_contract_version(e, &new_version);

    events::upgrade_completed(e, old_hash, new_wasm_hash, e.ledger().sequence() as u64);

    // Run migration hook for the new version.
    migrate(e, &new_version)?;

    Ok(())
}

/// Post-upgrade migration hook. Runs exactly once per (major, minor, patch)
/// version triple. Add storage schema migrations here for each new version.
pub fn migrate(e: &Env, version: &ContractVersion) -> Result<(), ContractError> {
    if storage::is_migration_done(e, version.major, version.minor, version.patch) {
        return Err(ContractError::MigrationAlreadyDone);
    }

    // ── Version-specific migration logic ──────────────────────────────────
    // Example: v1.0.1 — no schema changes in this release, only WASM swap.
    // Future versions add migration branches here:
    //
    //   if version.major == 1 && version.minor == 1 && version.patch == 0 {
    //       // initialise new GovernanceConfig defaults, etc.
    //   }
    // ──────────────────────────────────────────────────────────────────────

    storage::set_migration_done(e, version.major, version.minor, version.patch);
    events::migration_completed(e, version.major, version.minor, version.patch);
    Ok(())
}

/// Read-only: return the current version (for on-chain queries and health checks).
pub fn get_version_for_query(e: &Env) -> ContractVersion {
    get_version(e)
}

/// Read-only: return a historical version by the ledger sequence it was
/// activated at, or None if no snapshot exists for that ledger.
pub fn get_version_at(e: &Env, ledger: u64) -> Option<ContractVersion> {
    e.storage()
        .persistent()
        .get(&StorageKey::VersionHistory(ledger))
}
