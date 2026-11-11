use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ContractError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    Paused = 4,
    InvalidRoute = 10,
    RouteExpired = 11,
    EmptyRoute = 12,
    TooManyHops = 13,
    InsufficientInput = 20,
    InsufficientOutput = 21,
    SlippageExceeded = 22,
    DeadlineExceeded = 23,
    ExecutionTooEarly = 24,
    PriceImpactTooHigh = 25,
    SpreadTooHigh = 26,
    PoolNotSupported = 30,
    PoolCallFailed = 31,
    ReserveManipulationDetected = 32,
    AmmQuoteCallFailed = 33,
    AmmSwapCallFailed = 34,
    AmmReservesCallFailed = 35,
    InvalidAmount = 40,
    Overflow = 41,
    InvalidRecipient = 42,
    // ── Multi-sig governance ─────────────────────────────────────────────────
    /// Contract is in multi-sig mode; use governance proposals instead.
    UseGovernance = 50,
    /// Contract is not yet in multi-sig mode; call migrate_to_multisig first.
    NotMultiSig = 51,
    ProposalNotFound = 52,
    ProposalExpired = 53,
    ProposalAlreadyExecuted = 54,
    /// Signer already approved this proposal.
    AlreadyApproved = 55,
    /// Approval count below the configured threshold.
    ThresholdNotMet = 56,
    /// Signer list has reached the 10-signer maximum.
    SignerLimitReached = 57,
    // ── Upgrade ──────────────────────────────────────────────────────────────
    /// Time-lock delay has not elapsed yet.
    UpgradeLocked = 60,
    /// An upgrade is already pending; cancel it before proposing a new one.
    UpgradePending = 61,
    /// No upgrade is pending.
    NoUpgradePending = 62,
    /// The proposed WASM hash is identical to the current one.
    SameWasmHash = 63,
    /// Post-upgrade migration hook has already been executed for this version.
    MigrationAlreadyDone = 70,
    // ── Token allowlist ──────────────────────────────────────────────────────
    /// Asset is not on the allowlist and cannot be routed.
    TokenNotAllowed = 80,
    /// Token is already in the allowlist.
    TokenAlreadyAdded = 81,
    /// Attempting to remove a token that is referenced by an active pool.
    TokenInUse = 82,
    /// Batch add exceeds the 10-token-per-call limit.
    BatchTooLarge = 83,
    // ── MEV protection ───────────────────────────────────────────────────
    /// Swap exceeds commit threshold; must use commit-reveal flow.
    CommitmentRequired = 90,
    /// Commitment hash not found in storage.
    CommitmentNotFound = 91,
    /// Commitment window has passed.
    CommitmentExpired = 92,
    /// Revealed params do not match committed hash.
    InvalidReveal = 93,
    /// Sender has exceeded the per-window swap rate limit.
    RateLimitExceeded = 94,
    // ── Fee Distribution ─────────────────────────────────────────────────
    /// Fee configuration is invalid (e.g., shares don't sum to 10000 bps or too many recipients).
    InvalidFeeConfig = 100,
    /// Pool address is already registered on the router.
    PoolAlreadyRegistered = 101,
}
