use crate::errors::ContractError;
use crate::events;
use crate::storage::{
    self, batch_check_pools, extend_instance_ttl, get_fee_rate, get_instance_config,
    increment_nonce, transfer_asset, StorageKey,
};
use crate::storage::{
    INSTANCE_TTL_EXTEND_TO, INSTANCE_TTL_THRESHOLD, POOL_TTL_EXTEND_TO, POOL_TTL_THRESHOLD,
};
use crate::types::{
    CommitmentData, ContractVersion, DistributionRecord, FeeConfig, GovernanceConfig, MevConfig,
    Proposal, ProposalAction, QuoteResult, Route, SwapParams, SwapResult, TTLStatus, TokenCategory,
    TokenInfo,
};
use crate::{governance, tokens, upgrade};
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, vec, Address, Bytes, BytesN, Env, IntoVal,
    Symbol, Vec,
};

const MAX_HOPS: u32 = 4;
const BASE_CPU_PER_HOP: u64 = 5_000_000;
const CCI_OVERHEAD: u64 = 1_000_000;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceEstimate {
    pub estimated_cpu: u64,
    pub storage_reads: u32,
    pub storage_writes: u32,
    pub events: u32,
    pub will_succeed: bool,
}

#[contract]
pub struct StellarRoute;

#[contractimpl]
impl StellarRoute {
    /// Initialize the contract.
    ///
    /// When `signers` is non-empty the contract starts in multi-sig mode
    /// immediately. Otherwise it starts in single-admin mode and can be
    /// migrated later via `migrate_to_multisig`.
    pub fn initialize(
        e: Env,
        admin: Address,
        fee_rate: u32,
        fee_to: Address,
        // ── Optional multi-sig bootstrap ─────────────────────────────────────
        signers: Option<Vec<Address>>,
        threshold: Option<u32>,
        proposal_ttl: Option<u64>,
        guardian: Option<Address>,
        // ── Optional initial WASM hash for version tracking ──────────────────
        initial_wasm_hash: Option<BytesN<32>>,
    ) -> Result<(), ContractError> {
        if e.storage().instance().has(&StorageKey::Admin) {
            return Err(ContractError::AlreadyInitialized);
        }
        if fee_rate > 1000 {
            return Err(ContractError::InvalidAmount);
        }

        e.storage().instance().set(&StorageKey::Admin, &admin);
        e.storage().instance().set(&StorageKey::FeeRate, &fee_rate);
        e.storage().instance().set(&StorageKey::FeeTo, &fee_to);
        e.storage().instance().set(&StorageKey::Paused, &false);

        storage::set_last_ttl_extension(&e, e.ledger().sequence());

        events::initialized(&e, admin, fee_rate);
        extend_instance_ttl(&e);
        Ok(())
    }

    /// Switch a single-admin contract to multi-sig governance (one-way).
    pub fn migrate_to_multisig(
        e: Env,
        admin: Address,
        signers: Vec<Address>,
        threshold: u32,
        proposal_ttl: u64,
        guardian: Option<Address>,
    ) -> Result<(), ContractError> {
        governance::migrate_to_multisig(&e, admin, signers, threshold, proposal_ttl, guardian)
    }

    // ── Single-admin operations (rejected in multi-sig mode) ──────────────────

    pub fn set_admin(e: Env, new_admin: Address) -> Result<(), ContractError> {
        if storage::is_multisig(&e) {
            return Err(ContractError::UseGovernance);
        }
        let admin = storage::get_admin(&e);
        admin.require_auth();

        e.storage().instance().set(&StorageKey::Admin, &new_admin);
        events::admin_changed(&e, admin, new_admin);
        extend_instance_ttl(&e);
        Ok(())
    }

    // ── Single-admin Fee Configuration ────────────────────────────────────

    pub fn set_fee_distribution_config(e: Env, config: FeeConfig) -> Result<(), ContractError> {
        if storage::is_multisig(&e) {
            return Err(ContractError::UseGovernance);
        }
        storage::get_admin(&e).require_auth();

        let mut total_bps: u32 = 0;
        for recipient in config.recipients.iter() {
            total_bps += recipient.share_bps;
        }
        if total_bps != 10000 || config.recipients.len() > 5 {
            return Err(ContractError::InvalidFeeConfig);
        }

        storage::set_fee_config(&e, &config);
        extend_instance_ttl(&e);
        Ok(())
    }

    // ── Fee Distribution Execution ────────────────────────────────────────

    pub fn distribute_fees(e: Env, asset: crate::types::Asset) -> Result<(), ContractError> {
        let config = storage::get_fee_config(&e).ok_or(ContractError::NotInitialized)?;
        Self::distribute_fees_internal(&e, &asset, &config);
        Ok(())
    }

    fn distribute_fees_internal(e: &Env, asset: &crate::types::Asset, config: &FeeConfig) {
        let total_balance = storage::get_fee_balance(e, asset);
        if total_balance == 0 {
            return;
        }

        // Reset balance immediately
        storage::set_fee_balance(e, asset, 0);

        let mut remaining_dust = total_balance;
        let mut treasury_idx = 0;
        let mut found_treasury = false;

        // Locate treasury for dust allocation
        for (i, rec) in config.recipients.iter().enumerate() {
            if rec.label == symbol_short!("treasury") {
                treasury_idx = i;
                found_treasury = true;
                break;
            }
        }

        let num_recipients = config.recipients.len() as usize;

        for (i, rec) in config.recipients.iter().enumerate() {
            let mut amount = (total_balance * rec.share_bps as i128) / 10000;

            // Add rounding dust to treasury or last recipient
            if (found_treasury && i == treasury_idx) || (!found_treasury && i == num_recipients - 1)
            {
                amount = remaining_dust;
            }

            if amount > 0 {
                remaining_dust -= amount;

                if rec.label == symbol_short!("burn") {
                    match asset {
                        crate::types::Asset::Soroban(ref token_addr) => {
                            let client = soroban_sdk::token::Client::new(e, token_addr);
                            client.burn(&e.current_contract_address(), &amount);
                            events::fees_burned(e, asset.clone(), amount);
                        }
                        crate::types::Asset::Issued(ref issuer, _) => {
                            storage::transfer_asset(
                                e,
                                asset,
                                &e.current_contract_address(),
                                issuer,
                                amount,
                            );
                            events::fees_burned(e, asset.clone(), amount);
                        }
                        crate::types::Asset::Native => { /* XLM has no standard burn, skip */ }
                    }
                    storage::add_total_burned(e, asset, amount);
                } else {
                    storage::transfer_asset(
                        e,
                        asset,
                        &e.current_contract_address(),
                        &rec.address,
                        amount,
                    );
                }
            }
        }

        storage::push_distribution_history(
            e,
            asset,
            DistributionRecord {
                timestamp: e.ledger().sequence() as u64,
                total_distributed: total_balance,
            },
        );

        events::fees_distributed(e, asset.clone(), total_balance);
    }

    pub fn register_pool(e: Env, pool: Address) -> Result<(), ContractError> {
        if storage::is_multisig(&e) {
            return Err(ContractError::UseGovernance);
        }
        storage::get_admin(&e).require_auth();

        let key = StorageKey::SupportedPool(pool.clone());
        if e.storage().persistent().has(&key) {
            return Err(ContractError::PoolNotSupported);
        }

        e.storage().persistent().set(&key, &true);
        storage::extend_pool_ttl(&e, &pool);

        let new_count = storage::get_pool_count(&e) + 1;
        storage::set_pool_count(&e, new_count);
        storage::add_to_pool_list(&e, &pool);

        events::pool_registered(&e, pool);
        extend_instance_ttl(&e);
        Ok(())
    }

    pub fn pause(e: Env) -> Result<(), ContractError> {
        if storage::is_multisig(&e) {
            return Err(ContractError::UseGovernance);
        }
        storage::get_admin(&e).require_auth();
        e.storage().instance().set(&StorageKey::Paused, &true);
        events::paused(&e);
        extend_instance_ttl(&e);
        Ok(())
    }

    pub fn unpause(e: Env) -> Result<(), ContractError> {
        if storage::is_multisig(&e) {
            return Err(ContractError::UseGovernance);
        }
        storage::get_admin(&e).require_auth();
        e.storage().instance().set(&StorageKey::Paused, &false);
        events::unpaused(&e);
        extend_instance_ttl(&e);
        Ok(())
    }

    // ── Multi-sig governance entrypoints ──────────────────────────────────────

    /// Create a governance proposal. Returns the proposal ID.
    pub fn propose(e: Env, signer: Address, action: ProposalAction) -> Result<u64, ContractError> {
        if !storage::is_multisig(&e) {
            return Err(ContractError::NotMultiSig);
        }
        governance::propose(&e, signer, action)
    }

    /// Approve a proposal. Auto-executes when threshold is reached.
    pub fn approve_proposal(
        e: Env,
        signer: Address,
        proposal_id: u64,
    ) -> Result<(), ContractError> {
        if !storage::is_multisig(&e) {
            return Err(ContractError::NotMultiSig);
        }
        governance::approve(&e, signer, proposal_id)
    }

    /// Manually execute a proposal once threshold has been met.
    pub fn execute_proposal(e: Env, proposal_id: u64) -> Result<(), ContractError> {
        if !storage::is_multisig(&e) {
            return Err(ContractError::NotMultiSig);
        }
        governance::execute_proposal(&e, proposal_id)
    }

    /// Cancel a proposal (proposer or any signer).
    pub fn cancel_proposal(e: Env, signer: Address, proposal_id: u64) -> Result<(), ContractError> {
        if !storage::is_multisig(&e) {
            return Err(ContractError::NotMultiSig);
        }
        governance::cancel(&e, signer, proposal_id)
    }

    /// Emergency pause callable by the guardian only (no multi-sig delay).
    pub fn guardian_pause(e: Env, guardian: Address) -> Result<(), ContractError> {
        governance::guardian_pause(&e, guardian)
    }

    /// Read-only: return the governance config.
    pub fn get_governance_config(e: Env) -> Result<GovernanceConfig, ContractError> {
        governance::get_governance_config(&e)
    }

    /// Read-only: return a proposal by ID.
    pub fn get_proposal(e: Env, proposal_id: u64) -> Result<Proposal, ContractError> {
        governance::get_proposal(&e, proposal_id)
    }

    // ── Upgrade entrypoints ───────────────────────────────────────────────────

    /// Propose a time-locked upgrade (single-admin mode only).
    pub fn propose_upgrade(
        e: Env,
        admin: Address,
        new_wasm_hash: BytesN<32>,
        execute_after: u64,
    ) -> Result<(), ContractError> {
        upgrade::propose_upgrade(&e, admin, new_wasm_hash, execute_after)
    }

    /// Execute a pending upgrade after the time-lock has elapsed.
    pub fn execute_upgrade(e: Env) -> Result<(), ContractError> {
        upgrade::execute_upgrade(&e)
    }

    /// Cancel a pending upgrade (proposer only).
    pub fn cancel_upgrade(e: Env, admin: Address) -> Result<(), ContractError> {
        upgrade::cancel_upgrade(&e, admin)
    }

    /// Return the current contract version.
    pub fn get_version(e: Env) -> ContractVersion {
        upgrade::get_version_for_query(&e)
    }

    // ── Token allowlist entrypoints ─────────────────────────────────────────────

    /// Add a single token to the allowlist (single-admin mode).
    pub fn add_token(e: Env, caller: Address, info: TokenInfo) -> Result<(), ContractError> {
        tokens::add_token(&e, caller, info)
    }

    /// Remove a token from the allowlist (single-admin mode).
    pub fn remove_token(
        e: Env,
        caller: Address,
        asset: crate::types::Asset,
    ) -> Result<(), ContractError> {
        tokens::remove_token(&e, caller, asset)
    }

    /// Update token metadata without re-adding (single-admin mode).
    pub fn update_token(
        e: Env,
        caller: Address,
        asset: crate::types::Asset,
        updated: TokenInfo,
    ) -> Result<(), ContractError> {
        tokens::update_token(&e, caller, asset, updated)
    }

    /// Batch-add up to 10 tokens in a single call (single-admin mode).
    pub fn add_tokens_batch(
        e: Env,
        caller: Address,
        token_list: Vec<TokenInfo>,
    ) -> Result<(), ContractError> {
        tokens::add_tokens_batch(&e, caller, token_list)
    }

    /// Read-only: return `true` if the asset is on the allowlist.
    pub fn is_token_allowed(e: Env, asset: crate::types::Asset) -> bool {
        tokens::is_token_allowed(&e, &asset)
    }

    /// Read-only: return token metadata.
    pub fn get_token_info(e: Env, asset: crate::types::Asset) -> Option<TokenInfo> {
        tokens::get_token_info(&e, &asset)
    }

    /// Read-only: total count of active allowlisted tokens.
    pub fn get_token_count(e: Env) -> u32 {
        tokens::get_token_count(&e)
    }

    /// Read-only: all active assets in a given category.
    pub fn get_tokens_by_category(e: Env, category: TokenCategory) -> Vec<crate::types::Asset> {
        tokens::get_tokens_by_category(&e, category)
    }

    // ── Read-only getters ─────────────────────────────────────────────────────

    pub fn get_admin(e: Env) -> Result<Address, ContractError> {
        if !storage::is_initialized(&e) {
            return Err(ContractError::NotInitialized);
        }
        Ok(storage::get_admin(&e))
    }

    pub fn get_fee_rate_value(e: Env) -> u32 {
        storage::get_fee_rate(&e)
    }

    pub fn get_fee_to_address(e: Env) -> Result<Address, ContractError> {
        storage::get_fee_to_optional(&e).ok_or(ContractError::NotInitialized)
    }

    // ── Fee Distribution Getters ─────────────────────────────────────────

    pub fn get_fee_distribution_config(e: Env) -> Option<FeeConfig> {
        storage::get_fee_config(&e)
    }

    pub fn get_fee_balance(e: Env, asset: crate::types::Asset) -> i128 {
        storage::get_fee_balance(&e, &asset)
    }

    pub fn get_total_fees_collected(e: Env, asset: crate::types::Asset) -> i128 {
        storage::get_total_fees_collected(&e, &asset)
    }

    pub fn get_total_fees_burned(e: Env, asset: crate::types::Asset) -> i128 {
        storage::get_total_burned(&e, &asset)
    }

    pub fn get_distribution_history(e: Env, asset: crate::types::Asset) -> Vec<DistributionRecord> {
        storage::get_distribution_history(&e, &asset)
    }

    pub fn is_paused(e: Env) -> bool {
        storage::get_paused(&e)
    }

    pub fn get_pool_count(e: Env) -> u32 {
        storage::get_pool_count(&e)
    }

    pub fn is_pool_registered(e: Env, pool: Address) -> bool {
        storage::is_supported_pool(&e, pool)
    }

    // --- Admin MEV Configuration ---

    pub fn configure_mev(e: Env, config: MevConfig) -> Result<(), ContractError> {
        storage::get_admin(&e).require_auth();
        storage::set_mev_config(&e, &config);
        extend_instance_ttl(&e);
        Ok(())
    }

    pub fn set_whitelist(e: Env, address: Address, whitelisted: bool) -> Result<(), ContractError> {
        storage::get_admin(&e).require_auth();
        storage::set_whitelisted(&e, &address, whitelisted);
        extend_instance_ttl(&e);
        Ok(())
    }

    pub fn update_known_price(
        e: Env,
        token_a: Address,
        token_b: Address,
        price: i128,
    ) -> Result<(), ContractError> {
        storage::get_admin(&e).require_auth();
        storage::set_latest_known_price(&e, &token_a, &token_b, price);
        extend_instance_ttl(&e);
        Ok(())
    }

    pub fn get_mev_config(e: Env) -> Result<MevConfig, ContractError> {
        storage::get_mev_config(&e).ok_or(ContractError::NotInitialized)
    }

    // --- Commit-Reveal Pattern ---

    pub fn commit_swap(
        e: Env,
        sender: Address,
        commitment_hash: BytesN<32>,
        deposit_amount: i128,
    ) -> Result<(), ContractError> {
        sender.require_auth();
        StellarRoute::require_not_paused(&e)?;

        if deposit_amount <= 0 {
            return Err(ContractError::InvalidAmount);
        }

        let mev_config = storage::get_mev_config(&e).ok_or(ContractError::NotInitialized)?;

        let current_ledger = e.ledger().sequence();
        let expires_at = current_ledger + mev_config.commit_window_ledgers;

        let commitment = CommitmentData {
            sender: sender.clone(),
            deposit_amount,
            created_at: current_ledger,
            expires_at,
        };

        storage::set_commitment(
            &e,
            &commitment_hash,
            &commitment,
            mev_config.commit_window_ledgers,
        );

        events::commitment_created(&e, sender, commitment_hash, deposit_amount);
        extend_instance_ttl(&e);
        Ok(())
    }

    pub fn reveal_and_execute(
        e: Env,
        sender: Address,
        params: SwapParams,
        salt: BytesN<32>,
    ) -> Result<SwapResult, ContractError> {
        sender.require_auth();
        StellarRoute::require_not_paused(&e)?;

        // Recompute hash from params + salt
        let mut payload = Bytes::new(&e);
        payload.append(&Bytes::from_slice(&e, &params.amount_in.to_be_bytes()));
        payload.append(&Bytes::from_slice(&e, &params.min_amount_out.to_be_bytes()));
        payload.append(&Bytes::from_slice(&e, &params.deadline.to_be_bytes()));
        let salt_bytes: Bytes = salt.into();
        payload.append(&salt_bytes);
        let computed_hash: BytesN<32> = e.crypto().sha256(&payload).into();

        // Verify commitment exists
        let commitment =
            storage::get_commitment(&e, &computed_hash).ok_or(ContractError::CommitmentNotFound)?;

        // Verify sender matches
        if commitment.sender != sender {
            return Err(ContractError::InvalidReveal);
        }

        // Verify not expired
        if e.ledger().sequence() > commitment.expires_at {
            return Err(ContractError::CommitmentExpired);
        }

        // Remove commitment
        storage::remove_commitment(&e, &computed_hash);

        events::commitment_revealed(&e, sender.clone(), computed_hash);

        // Execute the swap using the internal logic
        Self::execute_swap_internal(&e, &sender, &params)
    }

    // --- Core operations ---

    pub fn require_not_paused(e: &Env) -> Result<(), ContractError> {
        let paused: bool = e
            .storage()
            .instance()
            .get(&StorageKey::Paused)
            .unwrap_or(false);
        if paused {
            return Err(ContractError::Paused);
        }
        Ok(())
    }

    /// Estimate resource consumption for a swap operation
    pub fn estimate_resources(
        _e: Env,
        amount_in: i128,
        route: Route,
    ) -> Result<ResourceEstimate, ContractError> {
        if amount_in <= 0 || route.hops.is_empty() {
            return Err(ContractError::InvalidRoute);
        }

        let num_hops = route.hops.len() as u32;
        if num_hops > MAX_HOPS {
            return Err(ContractError::InvalidRoute);
        }

        // Estimate CPU: base + per-hop + CCI overhead
        let estimated_cpu = (BASE_CPU_PER_HOP * num_hops as u64) + (CCI_OVERHEAD * num_hops as u64);

        // Storage reads: 1 instance config + num_hops pool checks + 1 nonce
        let storage_reads = 1 + num_hops + 1;

        // Storage writes: 1 nonce update
        let storage_writes = 1;

        // Events: 1 swap event
        let events = 1;

        // Will succeed if under 100M instructions
        let will_succeed = estimated_cpu < 100_000_000;

        Ok(ResourceEstimate {
            estimated_cpu,
            storage_reads,
            storage_writes,
            events,
            will_succeed,
        })
    }

    /// Public entry point for users to get quotes
    pub fn get_quote(e: Env, amount_in: i128, route: Route) -> Result<QuoteResult, ContractError> {
        if amount_in <= 0 {
            return Err(ContractError::InvalidRoute);
        }
        Self::validate_route_internal(&e, &route)?;

        let mut current_amount = amount_in;
        let mut total_impact_bps: u32 = 0;

        for i in 0..route.hops.len() {
            let hop = route.hops.get(i).unwrap();

            let call_result = e.try_invoke_contract::<i128, soroban_sdk::Error>(
                &hop.pool,
                &Symbol::new(&e, "adapter_quote"),
                vec![
                    &e,
                    hop.source.into_val(&e),
                    hop.destination.into_val(&e),
                    current_amount.into_val(&e),
                ],
            );

            current_amount = match call_result {
                Ok(Ok(val)) => val,
                _ => return Err(ContractError::PoolCallFailed),
            };
            total_impact_bps += 5;
        }

        let fee_rate = get_fee_rate(&e);
        let fee_amount = (current_amount * fee_rate as i128) / 10000;
        let final_output = current_amount - fee_amount;

        let quote = QuoteResult {
            expected_output: final_output,
            price_impact_bps: total_impact_bps,
            fee_amount,
            route: route.clone(),
            valid_until: (e.ledger().sequence() + 120) as u64,
        };

        events::quote_generated(
            &e,
            amount_in,
            quote.expected_output,
            quote.fee_amount,
            quote.price_impact_bps,
            route.hops.len(),
            quote.valid_until,
        );

        Ok(quote)
    }

    /// Interface alias for external integrators: returns the best-effort quote.
    pub fn quote(e: Env, amount_in: i128, route: Route) -> Result<QuoteResult, ContractError> {
        Self::get_quote(e, amount_in, route)
    }

    /// Preflight route validation used by indexers, relayers, and debuggers.
    pub fn validate(e: Env, route: Route) -> Result<(), ContractError> {
        Self::validate_route_internal(&e, &route)?;
        events::route_validated(&e, route.hops.len(), route.expires_at);
        Ok(())
    }

    pub fn execute_swap(
        e: Env,
        sender: Address,
        params: SwapParams,
    ) -> Result<SwapResult, ContractError> {
        sender.require_auth();
        StellarRoute::require_not_paused(&e)?;
        // Validate every asset in the route is on the allowlist.
        tokens::validate_route_assets(&e, &params.route)?;

        // Check commit-reveal requirement for large swaps
        if let Some(mev_config) = storage::get_mev_config(&e) {
            if params.amount_in >= mev_config.commit_threshold {
                return Err(ContractError::CommitmentRequired);
            }
        }

        Self::execute_swap_internal(&e, &sender, &params)
    }

    /// Interface alias for external integrators: execute a validated route.
    pub fn execute(e: Env, sender: Address, params: SwapParams) -> Result<SwapResult, ContractError> {
        events::execution_requested(
            &e,
            sender.clone(),
            params.amount_in,
            params.route.hops.len(),
            params.deadline,
        );

        let result = Self::execute_swap(e.clone(), sender.clone(), params);
        if let Err(err) = result {
            events::execution_failed(&e, sender, err as u32);
            return Err(err);
        }
        result
    }

    // --- Internal swap execution (shared by execute_swap and reveal_and_execute) ---

    fn execute_swap_internal(
        e: &Env,
        sender: &Address,
        params: &SwapParams,
    ) -> Result<SwapResult, ContractError> {
        // 1. Deadline check
        if e.ledger().sequence() as u64 > params.deadline {
            return Err(ContractError::DeadlineExceeded);
        }

        // 2. Not-before check
        if (e.ledger().sequence() as u64) < params.not_before {
            return Err(ContractError::ExecutionTooEarly);
        }

        // 3. Route validation
        if params.route.hops.is_empty() || params.route.hops.len() > 4 {
            return Err(ContractError::InvalidRoute);
        }

        // 4. Rate limiting (if MEV config is set)
        if let Some(mev_config) = storage::get_mev_config(e) {
            if !storage::is_whitelisted(e, sender) {
                let current_ledger = e.ledger().sequence();
                let window_start = storage::get_account_swap_window_start(e, sender);
                let swap_count = storage::get_account_swap_count(e, sender);

                if swap_count > 0 && current_ledger < window_start + mev_config.rate_limit_window {
                    // Still within the window
                    if swap_count >= mev_config.max_swaps_per_window {
                        events::rate_limit_hit(
                            e,
                            sender.clone(),
                            swap_count,
                            mev_config.rate_limit_window,
                        );
                        return Err(ContractError::RateLimitExceeded);
                    }
                    storage::set_account_swap_count(
                        e,
                        sender,
                        swap_count + 1,
                        mev_config.rate_limit_window,
                    );
                } else {
                    // Window expired or first swap — reset
                    storage::set_account_swap_window_start(
                        e,
                        sender,
                        current_ledger,
                        mev_config.rate_limit_window,
                    );
                    storage::set_account_swap_count(e, sender, 1, mev_config.rate_limit_window);
                }
            }
        }

        // 5. Snapshot pool reserves before swap (for sandwich detection)
        let mut pre_reserves: soroban_sdk::Vec<(i128, i128)> = soroban_sdk::Vec::new(e);
        for i in 0..params.route.hops.len() {
            let hop = params.route.hops.get(i).unwrap();
            let reserves_result = e.try_invoke_contract::<(i128, i128), soroban_sdk::Error>(
                &hop.pool,
                &symbol_short!("get_rsrvs"),
                vec![e],
            );
            let reserves = match reserves_result {
                Ok(Ok(val)) => val,
                _ => (0_i128, 0_i128), // If pool doesn't support reserves, skip check
            };
            pre_reserves.push_back(reserves);
        }

        // 6. Transfer input to first pool
        let mut current_input_amount = params.amount_in;
        let first_hop = params.route.hops.get(0).unwrap();
        transfer_asset(
            e,
            &first_hop.source,
            sender,
            &first_hop.pool,
            params.amount_in,
        );

        // 7. Execute swap hops
        let mut total_impact_bps: u32 = 0;
        for i in 0..params.route.hops.len() {
            let hop = params.route.hops.get(i).unwrap();

            if !storage::is_supported_pool(e, hop.pool.clone()) {
                return Err(ContractError::PoolNotSupported);
            }

            let call_result = e.try_invoke_contract::<i128, soroban_sdk::Error>(
                &hop.pool,
                &symbol_short!("swap"),
                vec![
                    e,
                    hop.source.into_val(e),
                    hop.destination.into_val(e),
                    current_input_amount.into_val(e),
                    0_i128.into_val(e),
                ],
            );

            current_input_amount = match call_result {
                Ok(Ok(val)) => val,
                _ => return Err(ContractError::PoolCallFailed),
            };
            total_impact_bps += 5;
        }

        // 8. Calculate fees
        let fee_rate = get_fee_rate(e);
        let fee_amount = (current_input_amount * fee_rate as i128) / 10000;
        let final_output = current_input_amount - fee_amount;

        // 9. Enhanced slippage guards
        // max_price_impact_bps check
        if params.max_price_impact_bps > 0 && total_impact_bps > params.max_price_impact_bps {
            return Err(ContractError::PriceImpactTooHigh);
        }

        // max_execution_spread_bps check (compare actual output vs expected)
        if params.max_execution_spread_bps > 0 && params.route.estimated_output > 0 {
            let spread = if final_output < params.route.estimated_output {
                ((params.route.estimated_output - final_output) * 10000)
                    / params.route.estimated_output
            } else {
                0
            };
            if spread > params.max_execution_spread_bps as i128 {
                return Err(ContractError::SpreadTooHigh);
            }
        }

        // Standard slippage check
        if final_output < params.min_amount_out {
            return Err(ContractError::SlippageExceeded);
        }

        // 10. Post-swap reserve validation (sandwich detection)
        for i in 0..params.route.hops.len() {
            let hop = params.route.hops.get(i).unwrap();
            let pre = pre_reserves.get(i).unwrap();
            if pre.0 == 0 && pre.1 == 0 {
                continue; // Skip if pre-snapshot wasn't available
            }

            let post_result = e.try_invoke_contract::<(i128, i128), soroban_sdk::Error>(
                &hop.pool,
                &symbol_short!("get_rsrvs"),
                vec![e],
            );
            if let Ok(Ok(post)) = post_result {
                // Check that reserves changed in the expected direction
                // For a swap: one reserve goes up, one goes down
                let delta_0 = post.0 - pre.0;
                let delta_1 = post.1 - pre.1;
                // If both reserves moved in the same direction, something is wrong
                if delta_0 > 0 && delta_1 > 0 {
                    return Err(ContractError::ReserveManipulationDetected);
                }
                if delta_0 < 0 && delta_1 < 0 {
                    return Err(ContractError::ReserveManipulationDetected);
                }
            }
        }

        // 11. Emit high impact event if configured
        if let Some(mev_config) = storage::get_mev_config(e) {
            if total_impact_bps > mev_config.high_impact_threshold_bps {
                events::high_impact_swap(e, sender.clone(), total_impact_bps, params.amount_in);
            }
        }

        // 12. Transfer output to recipient
        let last_hop = params.route.hops.get(params.route.hops.len() - 1).unwrap();

        transfer_asset(
            e,
            &last_hop.destination,
            &e.current_contract_address(),
            &params.recipient,
            final_output,
        );

        // ── Collect and Handle Distribution ──────────────────────────────
        if fee_amount > 0 {
            storage::add_fee_balance(e, &last_hop.destination, fee_amount);
            events::fee_collected(e, last_hop.destination.clone(), fee_amount);

            if let Some(config) = storage::get_fee_config(e) {
                if config.auto_distribute {
                    let current_balance = storage::get_fee_balance(e, &last_hop.destination);
                    if current_balance >= config.min_distribution {
                        Self::distribute_fees_internal(e, &last_hop.destination, &config);
                    }
                }
            }
        }
        // ──────────────────────────────────────────────────────────────────────

        increment_nonce(&e, sender.clone());
        storage::add_swap_volume(&e, params.amount_in);

        // Extend TTLs for pools used in this route
        for i in 0..params.route.hops.len() {
            let hop = params.route.hops.get(i).unwrap();
            storage::extend_pool_ttl(&e, &hop.pool);
        }
        extend_instance_ttl(&e);

        // Check TTL health and emit warning if needed
        Self::check_ttl_health(&e);

        // Emit compact event (use IDs instead of full structs where possible)
        events::swap_executed(
            e,
            sender.clone(),
            params.amount_in,
            final_output,
            fee_amount,
            params.route.clone(),
        );

        Ok(SwapResult {
            amount_in: params.amount_in,
            amount_out: final_output,
            route: params.route.clone(),
            executed_at: e.ledger().sequence() as u64,
        })
    }

    fn validate_route_internal(e: &Env, route: &Route) -> Result<(), ContractError> {
        if route.hops.is_empty() || route.hops.len() > MAX_HOPS {
            return Err(ContractError::InvalidRoute);
        }
        if route.expires_at > 0 && (e.ledger().sequence() as u64) > route.expires_at {
            return Err(ContractError::RouteExpired);
        }

        // Validate every asset in the route is on the allowlist.
        tokens::validate_route_assets(e, route)?;

        let mut pools = Vec::new(e);
        for i in 0..route.hops.len() {
            pools.push_back(route.hops.get(i).unwrap().pool.clone());
        }
        if !batch_check_pools(e, &pools) {
            return Err(ContractError::PoolNotSupported);
        }
        Ok(())
    }

    // --- TTL Management ---

    /// Public function anyone can call to extend all storage TTLs.
    /// No authorization required — keeping the contract alive is a public good.
    pub fn extend_storage_ttl(e: Env) {
        // Extend instance TTL (Admin, FeeRate, FeeTo, Paused, PoolCount, PoolList)
        extend_instance_ttl(&e);

        // Extend all registered pool TTLs
        let pool_list = storage::get_pool_list(&e);
        let pools_extended = pool_list.len();
        for i in 0..pool_list.len() {
            let pool = pool_list.get(i).unwrap();
            storage::extend_pool_ttl(&e, &pool);
        }

        // Extend TotalSwapVolume TTL
        storage::extend_volume_ttl(&e);

        // Record when this extension was performed
        storage::set_last_ttl_extension(&e, e.ledger().sequence());

        events::ttl_extended(&e, pools_extended, e.ledger().sequence());
    }

    /// Returns estimated TTL status for monitoring. Values are estimates
    /// based on when extend_storage_ttl was last called.
    pub fn get_ttl_status(e: Env) -> TTLStatus {
        let current_ledger = e.ledger().sequence();
        let last_extended = storage::get_last_ttl_extension(&e);

        let elapsed = current_ledger.saturating_sub(last_extended);

        let instance_remaining = INSTANCE_TTL_EXTEND_TO.saturating_sub(elapsed) as u64;
        let pools_remaining = POOL_TTL_EXTEND_TO.saturating_sub(elapsed) as u64;

        let needs_extension = instance_remaining < INSTANCE_TTL_THRESHOLD as u64
            || pools_remaining < POOL_TTL_THRESHOLD as u64;

        TTLStatus {
            instance_ttl_remaining: instance_remaining,
            pools_min_ttl: pools_remaining,
            needs_extension,
            last_extended_ledger: last_extended,
        }
    }

    /// Returns the total swap volume tracked by the contract.
    pub fn get_total_swap_volume(e: Env) -> i128 {
        storage::get_total_swap_volume(&e)
    }

    /// Internal: check TTL health and emit warning if below threshold.
    fn check_ttl_health(e: &Env) {
        let last_extended = storage::get_last_ttl_extension(e);
        if last_extended == 0 {
            return;
        }

        let elapsed = e.ledger().sequence().saturating_sub(last_extended);
        let pools_remaining = POOL_TTL_EXTEND_TO.saturating_sub(elapsed);

        if pools_remaining < POOL_TTL_THRESHOLD {
            events::ttl_warning(e, pools_remaining as u64, POOL_TTL_THRESHOLD);
        }
    }
}
