use crate::types::{
    Asset, CommitmentData, ContractVersion, DistributionRecord, FeeConfig, GovernanceConfig,
    MevConfig, PendingUpgrade, Proposal, TokenCategory, TokenInfo,
};
use soroban_sdk::{contracttype, Address, BytesN, Env, Vec};

#[contracttype]
pub enum StorageKey {
    Admin,
    FeeRate,
    FeeTo,
    Paused,
    SupportedPool(Address),
    PoolCount,
    SwapNonce(Address),
    // ── Persistent ─────────────────────────────────────────────────────
    TotalSwapVolume,
    // ── Instance — TTL tracking ────────────────────────────────────────
    PoolList,
    LastTtlExtension,
    // ── Temporary (auto-expiring) ──────────────────────────────────────
    PendingUpgrade,
    Commitment(BytesN<32>),
    AccountSwapCount(Address),
    AccountSwapWindowStart(Address),
    Whitelisted(Address),
    LatestKnownPrice(Address, Address),
    // ── Fee Distribution Keys ────────────────────────────────────────────────
    FeeConfig,
    FeeBalance(Asset),
    TotalFeesCollected(Asset),
    TotalFeesBurned(Asset),
    DistributionHistory(Asset),
    // ── Governance and upgrade keys ──────────────────────────────────────────
    IsMultiSig,
    Governance,
    Guardian,
    ProposalCounter,
    ProposalEntry(u64),
    ContractVersionKey,
    PendingUpgradeKey,
    VersionHistory(u64),
    MigrationDone(u32, u32, u32),
    // ── Token + MEV keys ─────────────────────────────────────────────────────
    AllowedToken(Asset),
    TokenCount,
    MevConfig,
}

#[derive(Clone, Debug)]
pub struct InstanceConfig {
    pub admin: Address,
    pub fee_rate: u32,
    pub fee_to: Option<Address>,
    pub paused: bool,
}

// ── TTL Constants (in ledger sequences, ~5s per ledger) ──────────────────

pub const DAY_IN_LEDGERS: u32 = 17_280;

/// Instance storage: extend +30 days, threshold at 25% (~7 days)
pub const INSTANCE_TTL_EXTEND_TO: u32 = 30 * DAY_IN_LEDGERS;
pub const INSTANCE_TTL_THRESHOLD: u32 = 7 * DAY_IN_LEDGERS;

/// Persistent pool keys: extend +90 days, threshold at 25% (~22 days)
pub const POOL_TTL_EXTEND_TO: u32 = 90 * DAY_IN_LEDGERS;
pub const POOL_TTL_THRESHOLD: u32 = 22 * DAY_IN_LEDGERS;

/// Persistent swap volume: extend +30 days, threshold at 25% (~7 days)
pub const VOLUME_TTL_EXTEND_TO: u32 = 30 * DAY_IN_LEDGERS;
pub const VOLUME_TTL_THRESHOLD: u32 = 7 * DAY_IN_LEDGERS;

/// Persistent swap nonce: extend +30 days, threshold at 25% (~7 days)
pub const NONCE_TTL_EXTEND_TO: u32 = 30 * DAY_IN_LEDGERS;
pub const NONCE_TTL_THRESHOLD: u32 = 7 * DAY_IN_LEDGERS;

/// Temporary storage TTLs
pub const PENDING_UPGRADE_TTL: u32 = 6 * 720; // ~6 hours
pub const COMMITMENT_TTL: u32 = 720; // ~1 hour
pub const RATE_LIMIT_TTL: u32 = 120; // ~10 minutes

// ── TTL Extension Helpers ────────────────────────────────────────────────

/// Extend instance TTL after any write. Only extends if remaining < threshold.
pub fn extend_instance_ttl(e: &Env) {
    e.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
}

/// Extend a persistent storage key's TTL using the threshold pattern.
pub fn extend_persistent_ttl(e: &Env, key: &StorageKey, threshold: u32, extend_to: u32) {
    if e.storage().persistent().has(key) {
        e.storage()
            .persistent()
            .extend_ttl(key, threshold, extend_to);
    }
}

/// Extend a specific pool's persistent TTL (+90 days).
pub fn extend_pool_ttl(e: &Env, pool: &Address) {
    let key = StorageKey::SupportedPool(pool.clone());
    extend_persistent_ttl(e, &key, POOL_TTL_THRESHOLD, POOL_TTL_EXTEND_TO);
}

/// Extend the swap nonce TTL for a specific user (+30 days).
pub fn extend_nonce_ttl(e: &Env, address: &Address) {
    let key = StorageKey::SwapNonce(address.clone());
    extend_persistent_ttl(e, &key, NONCE_TTL_THRESHOLD, NONCE_TTL_EXTEND_TO);
}

/// Extend the total swap volume TTL (+30 days).
pub fn extend_volume_ttl(e: &Env) {
    extend_persistent_ttl(
        e,
        &StorageKey::TotalSwapVolume,
        VOLUME_TTL_THRESHOLD,
        VOLUME_TTL_EXTEND_TO,
    );
}

// --- Core storage helpers ---

pub fn get_admin(e: &Env) -> Address {
    e.storage().instance().get(&StorageKey::Admin).unwrap()
}

pub fn set_admin(e: &Env, admin: &Address) {
    e.storage().instance().set(&StorageKey::Admin, admin);
}

pub fn get_fee_rate(e: &Env) -> u32 {
    e.storage()
        .instance()
        .get(&StorageKey::FeeRate)
        .unwrap_or(0)
}

pub fn set_fee_rate(e: &Env, rate: u32) {
    e.storage().instance().set(&StorageKey::FeeRate, &rate);
}

pub fn get_fee_to(e: &Env) -> Address {
    e.storage().instance().get(&StorageKey::FeeTo).unwrap()
}

pub fn get_fee_to_optional(e: &Env) -> Option<Address> {
    e.storage().instance().get(&StorageKey::FeeTo)
}

pub fn get_pool_count(e: &Env) -> u32 {
    e.storage()
        .instance()
        .get(&StorageKey::PoolCount)
        .unwrap_or(0)
}

pub fn set_pool_count(e: &Env, count: u32) {
    e.storage().instance().set(&StorageKey::PoolCount, &count);
}

pub fn get_paused(e: &Env) -> bool {
    e.storage()
        .instance()
        .get(&StorageKey::Paused)
        .unwrap_or(false)
}

pub fn is_initialized(e: &Env) -> bool {
    e.storage().instance().has(&StorageKey::Admin)
}

pub fn is_supported_pool(e: &Env, pool: Address) -> bool {
    e.storage()
        .persistent()
        .has(&StorageKey::SupportedPool(pool))
}

pub fn batch_check_pools(e: &Env, pools: &Vec<Address>) -> bool {
    for i in 0..pools.len() {
        if !is_supported_pool(e, pools.get(i).unwrap()) {
            return false;
        }
    }
    true
}

pub fn get_instance_config(e: &Env) -> InstanceConfig {
    InstanceConfig {
        admin: get_admin(e),
        fee_rate: get_fee_rate(e),
        fee_to: get_fee_to_optional(e),
        paused: get_paused(e),
    }
}

/// Get the list of all registered pool addresses (for TTL enumeration).
pub fn get_pool_list(e: &Env) -> Vec<Address> {
    e.storage()
        .instance()
        .get(&StorageKey::PoolList)
        .unwrap_or_else(|| Vec::new(e))
}

/// Add a pool address to the enumerable pool list.
pub fn add_to_pool_list(e: &Env, pool: &Address) {
    let mut list = get_pool_list(e);
    list.push_back(pool.clone());
    e.storage().instance().set(&StorageKey::PoolList, &list);
}

// ── Nonces ───────────────────────────────────────────────────────────────

pub fn get_nonce(e: &Env, address: Address) -> i128 {
    let key = StorageKey::SwapNonce(address);
    e.storage().persistent().get(&key).unwrap_or(0)
}

pub fn increment_nonce(e: &Env, address: Address) {
    let key = StorageKey::SwapNonce(address.clone());
    let current = get_nonce(e, address.clone());
    e.storage().persistent().set(&key, &(current + 1));
    extend_nonce_ttl(e, &address);
}

// ── Swap Volume ──────────────────────────────────────────────────────────

pub fn get_total_swap_volume(e: &Env) -> i128 {
    e.storage()
        .persistent()
        .get(&StorageKey::TotalSwapVolume)
        .unwrap_or(0)
}

pub fn add_swap_volume(e: &Env, amount: i128) {
    let current = get_total_swap_volume(e);
    e.storage()
        .persistent()
        .set(&StorageKey::TotalSwapVolume, &(current + amount));
    extend_volume_ttl(e);
}

// ── TTL Extension Tracking ───────────────────────────────────────────────

pub fn get_last_ttl_extension(e: &Env) -> u32 {
    e.storage()
        .instance()
        .get(&StorageKey::LastTtlExtension)
        .unwrap_or(0)
}

pub fn set_last_ttl_extension(e: &Env, ledger: u32) {
    e.storage()
        .instance()
        .set(&StorageKey::LastTtlExtension, &ledger);
}

// ── Token Transfer ───────────────────────────────────────────────────────

pub fn transfer_asset(e: &Env, asset: &Asset, from: &Address, to: &Address, amount: i128) {
    if let Asset::Soroban(address) = asset {
        let client = soroban_sdk::token::Client::new(e, address);
        client.transfer(from, to, &amount);
    }
}

// ─── Multi-sig governance helpers ────────────────────────────────────────────

pub fn is_multisig(e: &Env) -> bool {
    e.storage()
        .instance()
        .get(&StorageKey::IsMultiSig)
        .unwrap_or(false)
}

pub fn set_multisig(e: &Env) {
    e.storage().instance().set(&StorageKey::IsMultiSig, &true);
}

pub fn get_governance(e: &Env) -> GovernanceConfig {
    e.storage().instance().get(&StorageKey::Governance).unwrap()
}

pub fn set_governance(e: &Env, config: &GovernanceConfig) {
    e.storage().instance().set(&StorageKey::Governance, config);
}

pub fn has_guardian(e: &Env) -> bool {
    e.storage().instance().has(&StorageKey::Guardian)
}

pub fn get_guardian(e: &Env) -> Option<Address> {
    e.storage().instance().get(&StorageKey::Guardian)
}

pub fn set_guardian(e: &Env, guardian: &Address) {
    e.storage().instance().set(&StorageKey::Guardian, guardian);
}

pub fn next_proposal_id(e: &Env) -> u64 {
    let id: u64 = e
        .storage()
        .instance()
        .get(&StorageKey::ProposalCounter)
        .unwrap_or(0);
    let next = id + 1;
    e.storage()
        .instance()
        .set(&StorageKey::ProposalCounter, &next);
    next
}

pub fn get_proposal(e: &Env, id: u64) -> Option<Proposal> {
    e.storage().persistent().get(&StorageKey::ProposalEntry(id))
}

pub fn save_proposal(e: &Env, proposal: &Proposal) {
    let key = StorageKey::ProposalEntry(proposal.id);
    e.storage().persistent().set(&key, proposal);
    e.storage().persistent().extend_ttl(&key, 17280, 17280 * 30);
}

// ─── Upgrade helpers ─────────────────────────────────────────────────────────

pub fn get_contract_version(e: &Env) -> Option<ContractVersion> {
    e.storage().instance().get(&StorageKey::ContractVersionKey)
}

pub fn set_contract_version(e: &Env, version: &ContractVersion) {
    e.storage()
        .instance()
        .set(&StorageKey::ContractVersionKey, version);
    // Also snapshot in persistent storage for audit trail
    let key = StorageKey::VersionHistory(version.upgraded_at);
    e.storage().persistent().set(&key, version);
    e.storage()
        .persistent()
        .extend_ttl(&key, 17280, 17280 * 365);
}

pub fn get_pending_upgrade(e: &Env) -> Option<PendingUpgrade> {
    e.storage().instance().get(&StorageKey::PendingUpgradeKey)
}

pub fn set_pending_upgrade(e: &Env, pending: &PendingUpgrade) {
    e.storage()
        .instance()
        .set(&StorageKey::PendingUpgradeKey, pending);
}

pub fn clear_pending_upgrade(e: &Env) {
    e.storage()
        .instance()
        .remove(&StorageKey::PendingUpgradeKey);
}

pub fn is_migration_done(e: &Env, major: u32, minor: u32, patch: u32) -> bool {
    e.storage()
        .persistent()
        .has(&StorageKey::MigrationDone(major, minor, patch))
}

pub fn set_migration_done(e: &Env, major: u32, minor: u32, patch: u32) {
    let key = StorageKey::MigrationDone(major, minor, patch);
    e.storage().persistent().set(&key, &true);
    e.storage()
        .persistent()
        .extend_ttl(&key, 17280, 17280 * 365);
}

// ─── Token allowlist helpers ──────────────────────────────────────────────────

pub fn get_token_info(e: &Env, asset: &Asset) -> Option<TokenInfo> {
    e.storage()
        .persistent()
        .get(&StorageKey::AllowedToken(asset.clone()))
}

pub fn is_token_allowed(e: &Env, asset: &Asset) -> bool {
    e.storage()
        .persistent()
        .has(&StorageKey::AllowedToken(asset.clone()))
}

pub fn save_token_info(e: &Env, info: &TokenInfo) {
    let key = StorageKey::AllowedToken(info.asset.clone());
    e.storage().persistent().set(&key, info);
    e.storage()
        .persistent()
        .extend_ttl(&key, 17280, 17280 * 365);
}

pub fn remove_token(e: &Env, asset: &Asset) {
    e.storage()
        .persistent()
        .remove(&StorageKey::AllowedToken(asset.clone()));
}

pub fn get_token_count(e: &Env) -> u32 {
    e.storage()
        .instance()
        .get(&StorageKey::TokenCount)
        .unwrap_or(0)
}

pub fn set_token_count(e: &Env, count: u32) {
    e.storage().instance().set(&StorageKey::TokenCount, &count);
}

pub fn get_tokens_by_category_key(e: &Env, asset: &Asset) -> Option<TokenCategory> {
    get_token_info(e, asset).map(|i| i.category)
}
// --- MEV Config ---

pub fn get_mev_config(e: &Env) -> Option<MevConfig> {
    e.storage().instance().get(&StorageKey::MevConfig)
}

pub fn set_mev_config(e: &Env, config: &MevConfig) {
    e.storage().instance().set(&StorageKey::MevConfig, config);
}

// --- Commitment storage (Temporary) ---

pub fn get_commitment(e: &Env, hash: &BytesN<32>) -> Option<CommitmentData> {
    let key = StorageKey::Commitment(hash.clone());
    e.storage().temporary().get(&key)
}

pub fn set_commitment(e: &Env, hash: &BytesN<32>, data: &CommitmentData, ttl_ledgers: u32) {
    let key = StorageKey::Commitment(hash.clone());
    e.storage().temporary().set(&key, data);
    e.storage()
        .temporary()
        .extend_ttl(&key, ttl_ledgers, ttl_ledgers);
}

pub fn remove_commitment(e: &Env, hash: &BytesN<32>) {
    let key = StorageKey::Commitment(hash.clone());
    e.storage().temporary().remove(&key);
}

// --- Rate limiting (Temporary) ---

pub fn get_account_swap_count(e: &Env, address: &Address) -> u32 {
    let key = StorageKey::AccountSwapCount(address.clone());
    e.storage().temporary().get(&key).unwrap_or(0)
}

pub fn set_account_swap_count(e: &Env, address: &Address, count: u32, ttl_ledgers: u32) {
    let key = StorageKey::AccountSwapCount(address.clone());
    e.storage().temporary().set(&key, &count);
    e.storage()
        .temporary()
        .extend_ttl(&key, ttl_ledgers, ttl_ledgers);
}

pub fn get_account_swap_window_start(e: &Env, address: &Address) -> u32 {
    let key = StorageKey::AccountSwapWindowStart(address.clone());
    e.storage().temporary().get(&key).unwrap_or(0)
}

pub fn set_account_swap_window_start(e: &Env, address: &Address, start: u32, ttl_ledgers: u32) {
    let key = StorageKey::AccountSwapWindowStart(address.clone());
    e.storage().temporary().set(&key, &start);
    e.storage()
        .temporary()
        .extend_ttl(&key, ttl_ledgers, ttl_ledgers);
}

// --- Whitelist (Persistent) ---

pub fn is_whitelisted(e: &Env, address: &Address) -> bool {
    let key = StorageKey::Whitelisted(address.clone());
    e.storage().persistent().get(&key).unwrap_or(false)
}

pub fn set_whitelisted(e: &Env, address: &Address, whitelisted: bool) {
    let key = StorageKey::Whitelisted(address.clone());
    e.storage().persistent().set(&key, &whitelisted);
    if whitelisted {
        e.storage()
            .persistent()
            .extend_ttl(&key, DAY_IN_LEDGERS, DAY_IN_LEDGERS * 30);
    }
}

// --- Latest known price (Instance) ---

pub fn get_latest_known_price(e: &Env, token_a: &Address, token_b: &Address) -> Option<i128> {
    let key = StorageKey::LatestKnownPrice(token_a.clone(), token_b.clone());
    e.storage().instance().get(&key)
}

pub fn set_latest_known_price(e: &Env, token_a: &Address, token_b: &Address, price: i128) {
    let key = StorageKey::LatestKnownPrice(token_a.clone(), token_b.clone());
    e.storage().instance().set(&key, &price);
}

// ─── Fee Distribution Storage Helpers ────────────────────────────────────────

pub fn get_fee_config(e: &Env) -> Option<FeeConfig> {
    e.storage().instance().get(&StorageKey::FeeConfig)
}

pub fn set_fee_config(e: &Env, config: &FeeConfig) {
    e.storage().instance().set(&StorageKey::FeeConfig, config);
}

pub fn get_fee_balance(e: &Env, asset: &Asset) -> i128 {
    e.storage()
        .persistent()
        .get(&StorageKey::FeeBalance(asset.clone()))
        .unwrap_or(0)
}

pub fn set_fee_balance(e: &Env, asset: &Asset, amount: i128) {
    let key = StorageKey::FeeBalance(asset.clone());
    e.storage().persistent().set(&key, &amount);
    e.storage().persistent().extend_ttl(&key, 17280, 17280 * 30);
}

pub fn add_fee_balance(e: &Env, asset: &Asset, amount: i128) {
    let new_balance = get_fee_balance(e, asset) + amount;
    set_fee_balance(e, asset, new_balance);

    let total_key = StorageKey::TotalFeesCollected(asset.clone());
    let current_total: i128 = e.storage().persistent().get(&total_key).unwrap_or(0);
    e.storage()
        .persistent()
        .set(&total_key, &(current_total + amount));
    e.storage()
        .persistent()
        .extend_ttl(&total_key, 17280, 17280 * 365);
}

pub fn get_total_fees_collected(e: &Env, asset: &Asset) -> i128 {
    e.storage()
        .persistent()
        .get(&StorageKey::TotalFeesCollected(asset.clone()))
        .unwrap_or(0)
}

pub fn add_total_burned(e: &Env, asset: &Asset, amount: i128) {
    let key = StorageKey::TotalFeesBurned(asset.clone());
    let current_total: i128 = e.storage().persistent().get(&key).unwrap_or(0);
    e.storage()
        .persistent()
        .set(&key, &(current_total + amount));
    e.storage()
        .persistent()
        .extend_ttl(&key, 17280, 17280 * 365);
}

pub fn get_total_burned(e: &Env, asset: &Asset) -> i128 {
    e.storage()
        .persistent()
        .get(&StorageKey::TotalFeesBurned(asset.clone()))
        .unwrap_or(0)
}

pub fn push_distribution_history(e: &Env, asset: &Asset, record: DistributionRecord) {
    let key = StorageKey::DistributionHistory(asset.clone());
    let mut history: Vec<DistributionRecord> = e
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| Vec::new(e));

    history.push_front(record);

    // Keep only the last 10 records to save state size
    if history.len() > 10 {
        history.pop_back();
    }

    e.storage().persistent().set(&key, &history);
    e.storage().persistent().extend_ttl(&key, 17280, 17280 * 30);
}

pub fn get_distribution_history(e: &Env, asset: &Asset) -> Vec<DistributionRecord> {
    e.storage()
        .persistent()
        .get(&StorageKey::DistributionHistory(asset.clone()))
        .unwrap_or_else(|| Vec::new(e))
}
