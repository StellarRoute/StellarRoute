use soroban_sdk::{contracttype, Address, BytesN, Env, Symbol, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Asset {
    Native,                  // XLM
    Issued(Address, Symbol), // (issuer, code)
    Soroban(Address),        // Soroban token contract address
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PoolType {
    Sdex,
    AmmConstProd,
    AmmStable,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct RouteHop {
    pub source: Asset,
    pub destination: Asset,
    pub pool: Address,
    pub pool_type: PoolType,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Route {
    pub hops: Vec<RouteHop>,
    pub estimated_output: i128,
    pub min_output: i128,
    pub expires_at: u64,
}

#[contracttype]
pub struct SwapParams {
    pub route: Route,
    pub amount_in: i128,
    pub min_amount_out: i128,
    pub recipient: Address,
    pub deadline: u64,
    pub not_before: u64,
    pub max_price_impact_bps: u32,
    pub max_execution_spread_bps: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct QuoteResult {
    pub expected_output: i128,
    pub price_impact_bps: u32, // 100 = 1%
    pub fee_amount: i128,
    pub route: Route,
    pub valid_until: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SwapResult {
    pub amount_in: i128,
    pub amount_out: i128,
    pub route: Route,
    pub executed_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct TTLStatus {
    pub instance_ttl_remaining: u64,
    pub pools_min_ttl: u64,
    pub needs_extension: bool,
    pub last_extended_ledger: u32,
}

// --- Fee Distribution Types ---

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeRecipient {
    pub address: Address,
    pub share_bps: u32, // Share in basis points (5000 = 50%)
    pub label: Symbol,  // "treasury", "stakers", "burn", etc.
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct FeeConfig {
    pub recipients: Vec<FeeRecipient>,
    pub min_distribution: i128,
    pub auto_distribute: bool,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct DistributionRecord {
    pub timestamp: u64,
    pub total_distributed: i128,
}

// Interface for AMM pools (SEP-like standard)
pub trait LiquidityPoolInterface {
    fn get_rsrvs(e: Env) -> (i128, i128);
    fn swap_out(e: Env, in_asset: Asset, out_asset: Asset, amount_in: i128) -> i128;
}

// ─── Token Allowlist ────────────────────────────────────────────────────────────

/// Category classification for allowlisted tokens.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenCategory {
    Native,     // XLM
    Stablecoin, // USDC, USDT, etc.
    Wrapped,    // Wrapped assets (wBTC, wETH)
    Ecosystem,  // Stellar ecosystem tokens
    Community,  // Community-added tokens
}

/// On-chain metadata for a whitelisted token.
#[contracttype]
#[derive(Clone, Debug)]
pub struct TokenInfo {
    pub asset: Asset,
    /// Human-readable name, e.g. "USD Coin"
    pub name: Symbol,
    /// Asset code, e.g. "USDC"
    pub code: Symbol,
    pub decimals: u32,
    /// True if the issuer account has been verified by governance.
    pub issuer_verified: bool,
    pub category: TokenCategory,
    /// Ledger sequence when the token was added.
    pub added_at: u64,
    /// The address that submitted the addition (admin or governance executor).
    pub added_by: Address,
}

// ─── Multi-sig Governance ─────────────────────────────────────────────────────

/// Authorized signer set and quorum configuration.
/// Stored in Instance storage.
#[contracttype]
#[derive(Clone, Debug)]
pub struct GovernanceConfig {
    /// Authorized signers (max 10).
    pub signers: Vec<Address>,
    /// Required approvals for a proposal to execute (M of N).
    pub threshold: u32,
    /// Ledger sequences a proposal stays valid before expiring.
    pub proposal_ttl: u64,
}

/// A governance action that can be proposed, approved, and executed.
#[contracttype]
#[derive(Clone, Debug)]
pub enum ProposalAction {
    SetFeeRate(u32),
    SetFeeTo(Address),
    SetFeeConfig(FeeConfig),
    RegisterPool(Address, PoolType),
    DeregisterPool(Address),
    Pause,
    Unpause,
    Upgrade(BytesN<32>),
    AddSigner(Address),
    RemoveSigner(Address),
    ChangeThreshold(u32),
}

/// On-chain governance proposal.
/// Stored in Persistent storage keyed by proposal ID.
#[contracttype]
#[derive(Clone, Debug)]
pub struct Proposal {
    pub id: u64,
    pub action: ProposalAction,
    pub proposer: Address,
    /// Addresses that have approved (first entry is always proposer).
    pub approvals: Vec<Address>,
    pub created_at: u64,
    pub expires_at: u64,
    /// True after the proposal has been executed or cancelled.
    pub executed: bool,
}

// ─── Contract Version + Upgrade ──────────────────────────────────────────────

/// Tracks the deployed contract version.
/// Stored in Instance storage; history snapshots in Persistent storage.
#[contracttype]
#[derive(Clone, Debug)]
pub struct ContractVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub wasm_hash: BytesN<32>,
    /// Ledger sequence at which this version became active.
    pub upgraded_at: u64,
}

/// Pending time-locked upgrade (single-admin mode).
/// Stored in Instance storage.
#[contracttype]
#[derive(Clone, Debug)]
pub struct PendingUpgrade {
    pub new_wasm_hash: BytesN<32>,
    pub proposed_at: u64,
    /// Minimum ledger sequence before `execute_upgrade` is callable.
    pub execute_after: u64,
    pub proposer: Address,
}

// ─── MEV Protection ──────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct MevConfig {
    pub commit_threshold: i128,
    pub commit_window_ledgers: u32,
    pub max_swaps_per_window: u32,
    pub rate_limit_window: u32,
    pub high_impact_threshold_bps: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct CommitmentData {
    pub sender: Address,
    pub deposit_amount: i128,
    pub created_at: u32,
    pub expires_at: u32,
}
