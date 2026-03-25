//! Multi-signature governance for the StellarRoute router contract.
//!
//! Critical operations (fee changes, pool registration, pause, upgrade, signer
//! management) require M-of-N approvals from an authorized signer set. A single
//! compromised key cannot execute any privileged action unilaterally.
//!
//! Flow:
//!   1. Any signer calls `propose()` → receives a proposal ID.
//!      The proposer's signature counts as the first approval.
//!   2. Other signers call `approve(proposal_id)`.
//!      When the threshold is reached the action auto-executes.
//!   3. Alternatively, anyone can call `execute(proposal_id)` once the
//!      threshold is met (useful when the last approver is a hardware wallet
//!      that cannot trigger a follow-up tx in the same call).
//!   4. The original proposer (or a majority of signers) can `cancel()`.
//!
//! Guardian:
//!   A single trusted address (e.g. a hot key in a multi-sig cold-wallet
//!   setup) that may call `guardian_pause()`. Unpausing still requires a
//!   full multi-sig proposal.

use crate::errors::ContractError;
use crate::storage::{self, extend_instance_ttl};
use crate::types::{GovernanceConfig, Proposal, ProposalAction};
use crate::{events, storage::StorageKey};
use soroban_sdk::{Address, Env, Vec};

// Maximum number of authorized signers.
const MAX_SIGNERS: u32 = 10;

// ─── Internal helpers ─────────────────────────────────────────────────────────

/// Returns `true` if `addr` is in the governance signer list.
fn is_signer(config: &GovernanceConfig, addr: &Address) -> bool {
    for i in 0..config.signers.len() {
        if config.signers.get(i).unwrap() == *addr {
            return true;
        }
    }
    false
}

/// Returns `true` if `addr` has already approved proposal `p`.
fn has_approved(p: &Proposal, addr: &Address) -> bool {
    for i in 0..p.approvals.len() {
        if p.approvals.get(i).unwrap() == *addr {
            return true;
        }
    }
    false
}

/// Execute the privileged action encoded in a proposal.
fn dispatch_action(e: &Env, action: ProposalAction) -> Result<(), ContractError> {
    match action {
        ProposalAction::SetFeeRate(rate) => {
            if rate > 1000 {
                return Err(ContractError::InvalidAmount);
            }
            storage::set_fee_rate(e, rate);
        }
        ProposalAction::SetFeeTo(addr) => {
            e.storage().instance().set(&StorageKey::FeeTo, &addr);
        }
        ProposalAction::SetFeeConfig(config) => {
            // Validate max 5 recipients
            if config.recipients.len() > 5 {
                return Err(ContractError::InvalidFeeConfig);
            }

            // Validate shares sum to exactly 10000 bps (100%)
            let mut total_bps: u32 = 0;
            for recipient in config.recipients.iter() {
                total_bps = total_bps
                    .checked_add(recipient.share_bps)
                    .ok_or(ContractError::InvalidFeeConfig)?;
            }
            if total_bps != 10000 {
                return Err(ContractError::InvalidFeeConfig);
            }

            storage::set_fee_config(e, &config);
        }
        ProposalAction::RegisterPool(pool, _pool_type) => {
            let key = StorageKey::SupportedPool(pool.clone());
            e.storage().persistent().set(&key, &true);
            e.storage().persistent().extend_ttl(&key, 17280, 17280 * 30);
            let new_count = storage::get_pool_count(e)
                .checked_add(1)
                .unwrap_or_else(|| panic!("pool count overflow"));
            storage::set_pool_count(e, new_count);
            events::pool_registered(e, pool);
        }
        ProposalAction::DeregisterPool(pool) => {
            let key = StorageKey::SupportedPool(pool);
            e.storage().persistent().remove(&key);
            let count = storage::get_pool_count(e);
            if count > 0 {
                storage::set_pool_count(e, count.saturating_sub(1));
            }
        }
        ProposalAction::Pause => {
            e.storage().instance().set(&StorageKey::Paused, &true);
            events::paused(e);
        }
        ProposalAction::Unpause => {
            e.storage().instance().set(&StorageKey::Paused, &false);
            events::unpaused(e);
        }
        ProposalAction::Upgrade(wasm_hash) => {
            // Delegate to the upgrade module.
            crate::upgrade::execute_wasm_upgrade(e, wasm_hash)?;
        }
        ProposalAction::AddSigner(new_signer) => {
            let mut config = storage::get_governance(e);
            if config.signers.len() >= MAX_SIGNERS {
                return Err(ContractError::SignerLimitReached);
            }
            config.signers.push_back(new_signer);
            storage::set_governance(e, &config);
        }
        ProposalAction::RemoveSigner(signer) => {
            let mut config = storage::get_governance(e);
            // Prevent dropping below threshold.
            if config.signers.len() <= config.threshold {
                return Err(ContractError::ThresholdNotMet);
            }
            let mut updated = Vec::new(e);
            for i in 0..config.signers.len() {
                let s = config.signers.get(i).unwrap();
                if s != signer {
                    updated.push_back(s);
                }
            }
            config.signers = updated;
            storage::set_governance(e, &config);
        }
        ProposalAction::ChangeThreshold(new_threshold) => {
            let mut config = storage::get_governance(e);
            if new_threshold == 0 || new_threshold > config.signers.len() {
                return Err(ContractError::InvalidAmount);
            }
            config.threshold = new_threshold;
            storage::set_governance(e, &config);
        }
    }
    Ok(())
}

// ─── Public API ───────────────────────────────────────────────────────────────

/// Initialize the multi-sig governance system.
///
/// Called from `router::initialize()`. Records the governance config and sets
/// the optional guardian address.
pub fn init_governance(
    e: &Env,
    signers: Vec<Address>,
    threshold: u32,
    proposal_ttl: u64,
    guardian: Option<Address>,
) -> Result<(), ContractError> {
    if signers.is_empty() || threshold == 0 || threshold > signers.len() {
        return Err(ContractError::InvalidAmount);
    }
    if signers.len() > MAX_SIGNERS {
        return Err(ContractError::SignerLimitReached);
    }
    // A zero TTL makes every proposal immediately expired on creation.
    if proposal_ttl == 0 {
        return Err(ContractError::InvalidAmount);
    }
    // Reject duplicate signers.
    for i in 0..signers.len() {
        for j in (i + 1)..signers.len() {
            if signers.get(i).unwrap() == signers.get(j).unwrap() {
                return Err(ContractError::InvalidAmount);
            }
        }
    }
    // Reject the contract itself as a signer.
    let contract = e.current_contract_address();
    for i in 0..signers.len() {
        if signers.get(i).unwrap() == contract {
            return Err(ContractError::InvalidRecipient);
        }
    }

    let config = GovernanceConfig {
        signers,
        threshold,
        proposal_ttl,
    };
    storage::set_governance(e, &config);

    if let Some(g) = guardian {
        storage::set_guardian(e, &g);
        events::guardian_set(e, g);
    }

    Ok(())
}

/// Migrate from single-admin to multi-sig governance. One-way and irreversible.
pub fn migrate_to_multisig(
    e: &Env,
    admin: Address,
    signers: Vec<Address>,
    threshold: u32,
    proposal_ttl: u64,
    guardian: Option<Address>,
) -> Result<(), ContractError> {
    // Only callable by the current single admin.
    admin.require_auth();
    let current_admin = storage::get_admin(e);
    if current_admin != admin {
        return Err(ContractError::Unauthorized);
    }
    if storage::is_multisig(e) {
        return Err(ContractError::AlreadyInitialized);
    }

    init_governance(e, signers.clone(), threshold, proposal_ttl, guardian)?;
    storage::set_multisig(e);

    events::governance_migrated(e, admin, signers.len(), threshold);
    extend_instance_ttl(e);
    Ok(())
}

/// Create a new governance proposal. Returns the proposal ID.
pub fn propose(e: &Env, signer: Address, action: ProposalAction) -> Result<u64, ContractError> {
    signer.require_auth();
    let config = storage::get_governance(e);
    if !is_signer(&config, &signer) {
        return Err(ContractError::Unauthorized);
    }

    let id = storage::next_proposal_id(e);
    let now = e.ledger().sequence() as u64;
    let mut approvals = Vec::new(e);
    approvals.push_back(signer.clone());

    let proposal = Proposal {
        id,
        action: action.clone(),
        proposer: signer.clone(),
        approvals,
        created_at: now,
        expires_at: now.saturating_add(config.proposal_ttl),
        executed: false,
    };
    storage::save_proposal(e, &proposal);

    events::proposal_created(e, id, signer, action);

    // Auto-execute if threshold is 1.
    if config.threshold == 1 {
        execute_proposal(e, id)?;
    }

    extend_instance_ttl(e);
    Ok(id)
}

/// Approve an existing proposal. Auto-executes when approval count meets threshold.
pub fn approve(e: &Env, signer: Address, proposal_id: u64) -> Result<(), ContractError> {
    signer.require_auth();
    let config = storage::get_governance(e);
    if !is_signer(&config, &signer) {
        return Err(ContractError::Unauthorized);
    }

    let mut proposal =
        storage::get_proposal(e, proposal_id).ok_or(ContractError::ProposalNotFound)?;

    if proposal.executed {
        return Err(ContractError::ProposalAlreadyExecuted);
    }
    if e.ledger().sequence() as u64 > proposal.expires_at {
        return Err(ContractError::ProposalExpired);
    }
    if has_approved(&proposal, &signer) {
        return Err(ContractError::AlreadyApproved);
    }

    proposal.approvals.push_back(signer.clone());
    let approval_count = proposal.approvals.len();
    storage::save_proposal(e, &proposal);

    events::proposal_approved(e, proposal_id, signer, approval_count);

    if approval_count >= config.threshold {
        execute_proposal(e, proposal_id)?;
    }

    extend_instance_ttl(e);
    Ok(())
}

/// Manually trigger execution of a proposal that has met the approval threshold.
pub fn execute_proposal(e: &Env, proposal_id: u64) -> Result<(), ContractError> {
    let config = storage::get_governance(e);
    let mut proposal =
        storage::get_proposal(e, proposal_id).ok_or(ContractError::ProposalNotFound)?;

    if proposal.executed {
        return Err(ContractError::ProposalAlreadyExecuted);
    }
    if e.ledger().sequence() as u64 > proposal.expires_at {
        return Err(ContractError::ProposalExpired);
    }
    if proposal.approvals.len() < config.threshold {
        return Err(ContractError::ThresholdNotMet);
    }

    proposal.executed = true;
    storage::save_proposal(e, &proposal);

    dispatch_action(e, proposal.action)?;

    events::proposal_executed(e, proposal_id);
    extend_instance_ttl(e);
    Ok(())
}

/// Cancel a proposal. Callable by the original proposer or by any signer when
/// a majority wishes to cancel (approvals of cancel intent are not tracked —
/// for simplicity the contract accepts a single signer cancel and relies on
/// social consensus; on-chain majority-cancel can be implemented as a
/// CancelProposal proposal action in a future iteration).
pub fn cancel(e: &Env, signer: Address, proposal_id: u64) -> Result<(), ContractError> {
    signer.require_auth();
    let config = storage::get_governance(e);

    let mut proposal =
        storage::get_proposal(e, proposal_id).ok_or(ContractError::ProposalNotFound)?;

    if proposal.executed {
        return Err(ContractError::ProposalAlreadyExecuted);
    }

    // Allow: original proposer OR any authorized signer.
    if proposal.proposer != signer && !is_signer(&config, &signer) {
        return Err(ContractError::Unauthorized);
    }

    proposal.executed = true; // Mark done so it cannot be executed later.
    storage::save_proposal(e, &proposal);

    events::proposal_cancelled(e, proposal_id, signer);
    extend_instance_ttl(e);
    Ok(())
}

/// Emergency pause callable by the guardian only. Unpausing requires multi-sig.
pub fn guardian_pause(e: &Env, guardian: Address) -> Result<(), ContractError> {
    guardian.require_auth();
    let stored = storage::get_guardian(e).ok_or(ContractError::Unauthorized)?;
    if stored != guardian {
        return Err(ContractError::Unauthorized);
    }

    e.storage().instance().set(&StorageKey::Paused, &true);
    events::guardian_paused(e, guardian);
    extend_instance_ttl(e);
    Ok(())
}

/// Read-only: return the current governance config.
pub fn get_governance_config(e: &Env) -> Result<GovernanceConfig, ContractError> {
    if !storage::is_multisig(e) {
        return Err(ContractError::NotMultiSig);
    }
    Ok(storage::get_governance(e))
}

/// Read-only: return a proposal by ID.
pub fn get_proposal(e: &Env, proposal_id: u64) -> Result<Proposal, ContractError> {
    storage::get_proposal(e, proposal_id).ok_or(ContractError::ProposalNotFound)
}
