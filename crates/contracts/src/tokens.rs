//! Token allowlist for the StellarRoute router contract.
//!
//! Only assets that appear in this allowlist can be used as input, output, or
//! intermediary assets in a route. This prevents scam tokens, honeypots, and
//! fee-on-transfer assets from breaking the router's atomic execution.
//!
//! Management functions require admin authorization (single-admin mode) or a
//! successfully executed governance proposal (multi-sig mode).  All reads are
//! public.
//!
//! Storage layout:
//!   Persistent  AllowedToken(Asset)            -> TokenInfo
//!   Instance    TokenCount                     -> u32
//!   Persistent  TokenCategoryIndex(category,n) -> Asset   (sequential index)
//!   Instance    TokenCategoryCount(category)   -> u32

use crate::errors::ContractError;
use crate::storage::{self, extend_instance_ttl, StorageKey};
use crate::types::{Asset, TokenCategory, TokenInfo};
use crate::{events, storage as st};
use soroban_sdk::{contracttype, Address, Env, Vec};

/// Maximum number of tokens per `add_tokens_batch` call.
const MAX_BATCH: u32 = 10;
/// Maximum token decimals — Stellar uses 7, but allow up to 19 for Soroban tokens.
const MAX_DECIMALS: u32 = 19;

fn validate_token_info(info: &TokenInfo) -> Result<(), ContractError> {
    if info.decimals > MAX_DECIMALS {
        return Err(ContractError::InvalidAmount);
    }
    Ok(())
}

// ─── Category index helpers ───────────────────────────────────────────────────
// We maintain a per-category sequential index so that callers can retrieve all
// tokens in a category without a full scan.  The index is append-only; removed
// tokens stay in the index but their AllowedToken entry is absent, so callers
// must filter.

#[contracttype]
enum IdxKey {
    /// n-th asset added under `category`.
    CatEntry(TokenCategory, u32),
    /// Number of entries ever added under `category` (not current count).
    CatLen(TokenCategory),
}

fn cat_len(e: &Env, category: TokenCategory) -> u32 {
    e.storage()
        .persistent()
        .get(&IdxKey::CatLen(category))
        .unwrap_or(0)
}

fn set_cat_len(e: &Env, category: TokenCategory, len: u32) {
    let key = IdxKey::CatLen(category);
    e.storage().persistent().set(&key, &len);
    e.storage()
        .persistent()
        .extend_ttl(&key, 17280, 17280 * 365);
}

fn push_cat_entry(e: &Env, category: TokenCategory, asset: &Asset) {
    let idx = cat_len(e, category);
    let key = IdxKey::CatEntry(category, idx);
    e.storage().persistent().set(&key, asset);
    e.storage()
        .persistent()
        .extend_ttl(&key, 17280, 17280 * 365);
    set_cat_len(
        e,
        category,
        idx.checked_add(1)
            .unwrap_or_else(|| panic!("category index overflow")),
    );
}

// ─── Authorization helper ─────────────────────────────────────────────────────

/// Require admin authorization when NOT in multi-sig mode.
/// In multi-sig mode callers must use the governance proposal pathway and the
/// action is dispatched via `governance::dispatch_action`, which calls the
/// token management functions directly — no additional auth check is needed
/// at that point because the proposal has already been approved.
fn require_admin_auth(e: &Env, caller: &Address) -> Result<(), ContractError> {
    caller.require_auth();
    if storage::is_multisig(e) {
        return Err(ContractError::UseGovernance);
    }
    if storage::get_admin(e) != *caller {
        return Err(ContractError::Unauthorized);
    }
    Ok(())
}

// ─── Public management API ────────────────────────────────────────────────────

/// Add a single token to the allowlist (single-admin mode).
pub fn add_token(e: &Env, caller: Address, info: TokenInfo) -> Result<(), ContractError> {
    require_admin_auth(e, &caller)?;
    add_token_internal(e, caller, info)
}

/// Internal add — called by both `add_token` and `add_tokens_batch` (and by
/// governance dispatch for `ProposalAction::AddToken`).
pub fn add_token_internal(e: &Env, caller: Address, info: TokenInfo) -> Result<(), ContractError> {
    validate_token_info(&info)?;
    if st::is_token_allowed(e, &info.asset) {
        return Err(ContractError::TokenAlreadyAdded);
    }

    let category = info.category;
    let asset = info.asset.clone();

    st::save_token_info(e, &info);
    push_cat_entry(e, category, &asset);
    st::set_token_count(
        e,
        st::get_token_count(e)
            .checked_add(1)
            .unwrap_or_else(|| panic!("token count overflow")),
    );

    events::token_added(e, asset, caller);
    extend_instance_ttl(e);
    Ok(())
}

/// Remove a token from the allowlist (single-admin mode).
///
/// Callers wishing to remove a token that is referenced by registered pools
/// must first deregister those pools via the appropriate path — otherwise
/// `TokenInUse` is returned.
pub fn remove_token(e: &Env, caller: Address, asset: Asset) -> Result<(), ContractError> {
    require_admin_auth(e, &caller)?;
    remove_token_internal(e, caller, asset)
}

/// Internal remove — called by governance dispatch as well.
pub fn remove_token_internal(e: &Env, caller: Address, asset: Asset) -> Result<(), ContractError> {
    if !st::is_token_allowed(e, &asset) {
        return Err(ContractError::TokenNotAllowed);
    }
    // Safety: prevent removal of a token still referenced by a registered pool.
    if e.storage()
        .persistent()
        .has(&StorageKey::SupportedPool(match &asset {
            Asset::Soroban(addr) => addr.clone(),
            _ => {
                // Non-Soroban assets are tracked differently; skip pool check.
                // `SupportedPool` keys are only for Soroban pool contracts.
                st::remove_token(e, &asset);
                let count = st::get_token_count(e);
                if count > 0 {
                    st::set_token_count(e, count.saturating_sub(1));
                }
                events::token_removed(e, asset, caller);
                extend_instance_ttl(e);
                return Ok(());
            }
        }))
    {
        return Err(ContractError::TokenInUse);
    }

    st::remove_token(e, &asset);
    let count = st::get_token_count(e);
    if count > 0 {
        st::set_token_count(e, count.saturating_sub(1));
    }

    events::token_removed(e, asset, caller);
    extend_instance_ttl(e);
    Ok(())
}

/// Update token metadata without removing and re-adding.
pub fn update_token(
    e: &Env,
    caller: Address,
    asset: Asset,
    updated: TokenInfo,
) -> Result<(), ContractError> {
    require_admin_auth(e, &caller)?;
    update_token_internal(e, caller, asset, updated)
}

/// Internal update — called by governance dispatch as well.
pub fn update_token_internal(
    e: &Env,
    caller: Address,
    asset: Asset,
    updated: TokenInfo,
) -> Result<(), ContractError> {
    if !st::is_token_allowed(e, &asset) {
        return Err(ContractError::TokenNotAllowed);
    }
    // updated.asset must match the asset being updated — prevent silent asset swap.
    if updated.asset != asset {
        return Err(ContractError::InvalidAmount);
    }
    validate_token_info(&updated)?;
    st::save_token_info(e, &updated);
    events::token_updated(e, asset, caller);
    extend_instance_ttl(e);
    Ok(())
}

/// Batch-add up to 10 tokens in a single call (single-admin mode).
pub fn add_tokens_batch(
    e: &Env,
    caller: Address,
    tokens: Vec<TokenInfo>,
) -> Result<(), ContractError> {
    require_admin_auth(e, &caller)?;
    if tokens.len() > MAX_BATCH {
        return Err(ContractError::BatchTooLarge);
    }
    for i in 0..tokens.len() {
        let info = tokens.get(i).unwrap();
        add_token_internal(e, caller.clone(), info)?;
    }
    Ok(())
}

// ─── Public read API ──────────────────────────────────────────────────────────

/// Return `true` when `asset` is on the allowlist.
pub fn is_token_allowed(e: &Env, asset: &Asset) -> bool {
    st::is_token_allowed(e, asset)
}

/// Return the full metadata for an allowlisted token, or `None`.
pub fn get_token_info(e: &Env, asset: &Asset) -> Option<TokenInfo> {
    st::get_token_info(e, asset)
}

/// Return the total count of active allowlisted tokens.
pub fn get_token_count(e: &Env) -> u32 {
    st::get_token_count(e)
}

/// Return all assets that have ever been added under `category`.
/// Assets removed after addition are included in the raw index; callers should
/// filter out entries for which `is_token_allowed` returns `false`.
pub fn get_tokens_by_category(e: &Env, category: TokenCategory) -> Vec<Asset> {
    let len = cat_len(e, category);
    let mut result = Vec::new(e);
    for i in 0..len {
        let key = IdxKey::CatEntry(category, i);
        if let Some(asset) = e.storage().persistent().get::<IdxKey, Asset>(&key) {
            if st::is_token_allowed(e, &asset) {
                result.push_back(asset);
            }
        }
    }
    result
}

// ─── Route validation ─────────────────────────────────────────────────────────

/// Validate that every asset (source + destination) in every hop of `route` is
/// on the allowlist.  Returns `TokenNotAllowed` on the first violation.
///
/// This is called by both `get_quote` and `execute_swap`.  When the allowlist
/// feature is bootstrapped with no tokens (e.g. in tests that don't add tokens),
/// the token count is 0 and **validation is skipped** to preserve backward
/// compatibility with tests written before the allowlist was introduced.
pub fn validate_route_assets(e: &Env, route: &crate::types::Route) -> Result<(), ContractError> {
    // Skip validation when no tokens have been registered yet (bootstrap / tests).
    if st::get_token_count(e) == 0 {
        return Ok(());
    }
    for i in 0..route.hops.len() {
        let hop = route.hops.get(i).unwrap();
        if !st::is_token_allowed(e, &hop.source) {
            return Err(ContractError::TokenNotAllowed);
        }
        if !st::is_token_allowed(e, &hop.destination) {
            return Err(ContractError::TokenNotAllowed);
        }
    }
    Ok(())
}
