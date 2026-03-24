use crate::types::{Asset, ProposalAction, Route};
use soroban_sdk::{symbol_short, Address, BytesN, Env, Symbol};

pub fn initialized(e: &Env, admin: Address, fee_rate: u32) {
    let topics = (Symbol::new(e, "StellarRoute"), symbol_short!("init"));
    e.events().publish(topics, (admin, fee_rate));
}

pub fn admin_changed(e: &Env, old_admin: Address, new_admin: Address) {
    let topics = (Symbol::new(e, "StellarRoute"), symbol_short!("adm_chg"));
    e.events().publish(topics, (old_admin, new_admin));
}

pub fn pool_registered(e: &Env, pool_address: Address) {
    let topics = (Symbol::new(e, "StellarRoute"), symbol_short!("reg_pol"));
    e.events().publish(topics, pool_address);
}

pub fn paused(e: &Env) {
    let topics = (Symbol::new(e, "StellarRoute"), symbol_short!("paused"));
    e.events().publish(topics, ());
}

pub fn unpaused(e: &Env) {
    let topics = (Symbol::new(e, "StellarRoute"), symbol_short!("unpaused"));
    e.events().publish(topics, ());
}

pub fn swap_executed(
    e: &Env,
    sender: Address,
    amount_in: i128,
    amount_out: i128,
    fee: i128,
    route: Route,
) {
    let topics = (
        Symbol::new(e, "StellarRoute"),
        symbol_short!("swap"),
        sender,
    );
    e.events().publish(
        topics,
        (amount_in, amount_out, fee, route, e.ledger().sequence()),
    );
}

pub fn route_validated(e: &Env, hop_count: u32, expires_at: u64) {
    let topics = (Symbol::new(e, "StellarRoute"), symbol_short!("rt_val"));
    e.events()
        .publish(topics, (hop_count, expires_at, e.ledger().sequence()));
}

pub fn quote_generated(
    e: &Env,
    amount_in: i128,
    expected_output: i128,
    fee_amount: i128,
    price_impact_bps: u32,
    hop_count: u32,
    valid_until: u64,
) {
    let topics = (Symbol::new(e, "StellarRoute"), symbol_short!("quote"));
    e.events().publish(
        topics,
        (
            amount_in,
            expected_output,
            fee_amount,
            price_impact_bps,
            hop_count,
            valid_until,
            e.ledger().sequence(),
        ),
    );
}

pub fn execution_requested(
    e: &Env,
    sender: Address,
    amount_in: i128,
    hop_count: u32,
    deadline: u64,
) {
    let topics = (
        Symbol::new(e, "StellarRoute"),
        symbol_short!("exe_req"),
        sender,
    );
    e.events().publish(
        topics,
        (amount_in, hop_count, deadline, e.ledger().sequence()),
    );
}

pub fn execution_failed(e: &Env, sender: Address, error_code: u32) {
    let topics = (
        Symbol::new(e, "StellarRoute"),
        symbol_short!("exe_fail"),
        sender,
    );
    e.events()
        .publish(topics, (error_code, e.ledger().sequence()));
}

// ─── Multi-sig governance events ─────────────────────────────────────────────

pub fn governance_migrated(e: &Env, old_admin: Address, signer_count: u32, threshold: u32) {
    let topics = (Symbol::new(e, "StellarRoute"), symbol_short!("gov_mgr"));
    e.events()
        .publish(topics, (old_admin, signer_count, threshold));
}

pub fn proposal_created(e: &Env, id: u64, proposer: Address, action: ProposalAction) {
    let topics = (Symbol::new(e, "StellarRoute"), symbol_short!("prop_new"));
    e.events().publish(topics, (id, proposer, action));
}

pub fn proposal_approved(e: &Env, id: u64, signer: Address, approvals: u32) {
    let topics = (Symbol::new(e, "StellarRoute"), symbol_short!("prop_apr"));
    e.events().publish(topics, (id, signer, approvals));
}

pub fn proposal_executed(e: &Env, id: u64) {
    let topics = (Symbol::new(e, "StellarRoute"), symbol_short!("prop_exe"));
    e.events().publish(topics, id);
}

pub fn proposal_cancelled(e: &Env, id: u64, by: Address) {
    let topics = (Symbol::new(e, "StellarRoute"), symbol_short!("prop_can"));
    e.events().publish(topics, (id, by));
}

pub fn guardian_set(e: &Env, guardian: Address) {
    let topics = (Symbol::new(e, "StellarRoute"), symbol_short!("grd_set"));
    e.events().publish(topics, guardian);
}

pub fn guardian_paused(e: &Env, guardian: Address) {
    let topics = (Symbol::new(e, "StellarRoute"), symbol_short!("grd_pse"));
    e.events().publish(topics, guardian);
}

// ─── Upgrade events ──────────────────────────────────────────────────────────

pub fn upgrade_proposed(
    e: &Env,
    proposer: Address,
    old_hash: BytesN<32>,
    new_hash: BytesN<32>,
    execute_after: u64,
) {
    let topics = (Symbol::new(e, "StellarRoute"), symbol_short!("upg_prp"));
    e.events()
        .publish(topics, (proposer, old_hash, new_hash, execute_after));
}

pub fn upgrade_completed(e: &Env, old_hash: BytesN<32>, new_hash: BytesN<32>, ledger: u64) {
    let topics = (Symbol::new(e, "StellarRoute"), symbol_short!("upg_done"));
    e.events().publish(topics, (old_hash, new_hash, ledger));
}

pub fn upgrade_cancelled(e: &Env, by: Address) {
    let topics = (Symbol::new(e, "StellarRoute"), symbol_short!("upg_can"));
    e.events().publish(topics, by);
}

pub fn migration_completed(e: &Env, major: u32, minor: u32, patch: u32) {
    let topics = (Symbol::new(e, "StellarRoute"), symbol_short!("mig_done"));
    e.events().publish(topics, (major, minor, patch));
}

// ─── Token allowlist events ───────────────────────────────────────────────────

pub fn token_added(e: &Env, asset: crate::types::Asset, added_by: Address) {
    let topics = (Symbol::new(e, "StellarRoute"), symbol_short!("tok_add"));
    e.events().publish(topics, (asset, added_by));
}

pub fn token_removed(e: &Env, asset: crate::types::Asset, removed_by: Address) {
    let topics = (Symbol::new(e, "StellarRoute"), symbol_short!("tok_rm"));
    e.events().publish(topics, (asset, removed_by));
}

pub fn token_updated(e: &Env, asset: crate::types::Asset, updated_by: Address) {
    let topics = (Symbol::new(e, "StellarRoute"), symbol_short!("tok_upd"));
    e.events().publish(topics, (asset, updated_by));
}

// --- MEV Protection Events ---

pub fn high_impact_swap(e: &Env, sender: Address, impact_bps: u32, amount_in: i128) {
    let topics = (
        Symbol::new(e, "StellarRoute"),
        symbol_short!("hi_imp"),
        sender,
    );
    e.events().publish(topics, (impact_bps, amount_in));
}

pub fn rate_limit_hit(e: &Env, sender: Address, swap_count: u32, window: u32) {
    let topics = (
        Symbol::new(e, "StellarRoute"),
        symbol_short!("rl_hit"),
        sender,
    );
    e.events().publish(topics, (swap_count, window));
}

pub fn commitment_created(
    e: &Env,
    sender: Address,
    commitment_hash: BytesN<32>,
    deposit_amount: i128,
) {
    let topics = (
        Symbol::new(e, "StellarRoute"),
        symbol_short!("cmt_new"),
        sender,
    );
    e.events()
        .publish(topics, (commitment_hash, deposit_amount));
}

pub fn commitment_revealed(e: &Env, sender: Address, commitment_hash: BytesN<32>) {
    let topics = (
        Symbol::new(e, "StellarRoute"),
        symbol_short!("cmt_rvl"),
        sender,
    );
    e.events()
        .publish(topics, (commitment_hash, e.ledger().sequence()));
}

pub fn ttl_extended(e: &Env, pools_extended: u32, ledger: u32) {
    let topics = (Symbol::new(e, "StellarRoute"), symbol_short!("ttl_ext"));
    e.events().publish(topics, (pools_extended, ledger));
}

pub fn ttl_warning(e: &Env, estimated_remaining: u64, threshold: u32) {
    let topics = (Symbol::new(e, "StellarRoute"), symbol_short!("ttl_wrn"));
    e.events().publish(topics, (estimated_remaining, threshold));
}

// ─── Fee Distribution Events ─────────────────────────────────────────────────

pub fn fee_collected(e: &Env, asset: Asset, amount: i128) {
    let topics = (Symbol::new(e, "StellarRoute"), symbol_short!("fee_col"));
    e.events()
        .publish(topics, (asset, amount, e.ledger().sequence()));
}

pub fn fees_distributed(e: &Env, asset: Asset, total_distributed: i128) {
    let topics = (Symbol::new(e, "StellarRoute"), symbol_short!("fee_dist"));
    e.events()
        .publish(topics, (asset, total_distributed, e.ledger().sequence()));
}

pub fn fees_burned(e: &Env, asset: Asset, amount: i128) {
    let topics = (Symbol::new(e, "StellarRoute"), symbol_short!("fee_brn"));
    e.events()
        .publish(topics, (asset, amount, e.ledger().sequence()));
}
