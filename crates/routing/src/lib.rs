//! StellarRoute Routing Engine
//!
//! Provides pathfinding algorithms for optimal swap routing across SDEX and Soroban AMM pools.
//! Supports N-hop paths with safety bounds, cycle prevention, and price impact calculation.

pub mod adaptive_routing;
pub mod adaptive_timeout;
pub mod amm_fallback;
pub mod canary;
pub mod compaction;
pub mod consensus;
pub mod error;
pub mod execution_quality;
pub mod fixtures;
pub mod health;
pub mod impact;
pub mod normalization;
pub mod optimizer;
pub mod pathfinder;
pub mod policy;
pub mod regression;
pub mod risk;
pub mod scorer;
pub mod simulator;
pub mod snapshot;

pub use adaptive_routing::{AdaptiveError, AdaptivePolicy, AdaptiveRouter, QualityMetrics};
pub use adaptive_timeout::{TimeoutConfig, TimeoutController};
pub use amm_fallback::{AmmFallbackConfig, AmmFallbackTier, FallbackResult, TieredAmmFallback};
pub use canary::{CanaryConfig, CanaryEvaluation, CanaryEvaluator};
pub use compaction::{CompactedEdge, CompactedGraph};
pub use consensus::{
    ConsensusDiagnostics, ConsensusEngine, ConsensusError, ConsensusPolicy, RouteCandidate,
};
pub use execution_quality::{
    ExecutionQualityTracker, QualityObservation, QualityTrackerConfig, SourceState, WeightBounds,
};
pub use impact::{AmmQuoteCalculator, OrderbookImpactCalculator};
pub use optimizer::{
    HybridOptimizer, OptimizerDiagnostics, OptimizerPolicy, PolicyPresets, RouteMetrics,
};
pub use pathfinder::{LiquidityEdge, Pathfinder, PathfinderConfig, SwapPath};
pub use policy::RoutingPolicy;
pub use regression::{
    BaselineStore, BenchmarkFixture, RegressionReport, RegressionRunner, RegressionRunnerConfig,
    RouteRegressionEntry,
};
pub use risk::{AssetRiskLimit, ExclusionReason, RiskLimitConfig, RiskValidator, RouteExclusion};
pub use scorer::{
    BenchmarkHarness, BenchmarkReport, DefaultScorer, FeeMinimizingScorer, OutputMaximizingScorer,
    RouteScorer, ScorerInput, ScorerOutput, ScorerRegistry, ScorerResult,
};
pub use snapshot::{
    SnapshotId, SnapshotIsolationError, SnapshotIsolationMetrics, SnapshotIsolationValidator,
    SnapshotValidatorConfig, ValidatedHop,
};

// ─── Asset Canonicalization ──────────────────────────────────────────

/// Normalize an asset identifier to its canonical form.
///
/// - `"XLM"`, `"xlm"`, and `"native"` all become `"native"`.
/// - Everything else is uppercased (e.g. `"usdc"` → `"USDC"`).
pub fn normalize_asset(asset: &str) -> String {
    let lower = asset.to_lowercase();
    if lower == "xlm" || lower == "native" {
        "native".to_string()
    } else {
        lower.to_uppercase()
    }
}

/// Normalize both assets individually, then return them in canonical pair
/// order via [`normalize_pair`].
///
/// This is the one-stop function for all crates that need a consistent
/// base/quote ordering — API, cache, routing, etc.
pub fn normalize_pair_owned(a: &str, b: &str) -> (String, String) {
    let na = normalize_asset(a);
    let nb = normalize_asset(b);
    let (base, quote) = normalize_pair(&na, &nb);
    (base.to_owned(), quote.to_owned())
}

// ─── Pair Ordering ───────────────────────────────────────────────────

/// Canonical ordering for asset pairs.
///
/// Returns `(canonical_base, canonical_quote)` by sorting the two assets
/// lexicographically by their canonical string representation.
///
/// This ensures consistent pair ordering across API endpoints, cache keys,
/// and internal graph lookups regardless of the order supplied by the caller.
///
/// # Examples
///
/// ```
/// use stellarroute_routing::normalize_pair;
///
/// // "USDC:GA5ZSEJ" < "native" lexicographically ('U' = 85, 'n' = 110 in ASCII)
/// let (base, quote) = normalize_pair("USDC:GA5ZSEJ", "native");
/// assert_eq!(base, "USDC:GA5ZSEJ");
/// assert_eq!(quote, "native");
///
/// // Idempotent: same result regardless of input order
/// let (base, quote) = normalize_pair("native", "USDC:GA5ZSEJ");
/// assert_eq!(base, "USDC:GA5ZSEJ");
/// assert_eq!(quote, "native");
///
/// // Issued assets sort by canonical string
/// let (base, quote) = normalize_pair("USDC", "BTC");
/// assert_eq!(base, "BTC");
/// assert_eq!(quote, "USDC");
/// ```
pub fn normalize_pair<'a>(a: &'a str, b: &'a str) -> (&'a str, &'a str) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Routing engine with integrated pathfinding and impact calculations
pub struct RoutingEngine {
    pathfinder: Pathfinder,
    amm_calculator: AmmQuoteCalculator,
    orderbook_calculator: OrderbookImpactCalculator,
    hybrid_optimizer: HybridOptimizer,
    routing_policy: RoutingPolicy,
}

impl RoutingEngine {
    /// Create a new routing engine instance with default config
    pub fn new() -> Self {
        Self::with_config(PathfinderConfig::default())
    }

    /// Create a new routing engine with custom config
    pub fn with_config(config: PathfinderConfig) -> Self {
        Self::with_config_and_policy(config, RoutingPolicy::default())
    }

    /// Create a new routing engine with custom config and routing policy
    pub fn with_config_and_policy(config: PathfinderConfig, policy: RoutingPolicy) -> Self {
        Self {
            pathfinder: Pathfinder::new(config.clone()),
            amm_calculator: AmmQuoteCalculator,
            orderbook_calculator: OrderbookImpactCalculator,
            hybrid_optimizer: HybridOptimizer::new(config),
            routing_policy: policy,
        }
    }

    /// Get reference to pathfinder
    pub fn pathfinder(&self) -> &Pathfinder {
        &self.pathfinder
    }

    /// Get reference to AMM calculator
    pub fn amm_calculator(&self) -> &AmmQuoteCalculator {
        &self.amm_calculator
    }

    /// Get reference to orderbook calculator
    pub fn orderbook_calculator(&self) -> &OrderbookImpactCalculator {
        &self.orderbook_calculator
    }

    /// Get reference to hybrid optimizer
    pub fn hybrid_optimizer(&self) -> &HybridOptimizer {
        &self.hybrid_optimizer
    }

    /// Get mutable reference to hybrid optimizer
    pub fn hybrid_optimizer_mut(&mut self) -> &mut HybridOptimizer {
        &mut self.hybrid_optimizer
    }

    /// Get reference to routing policy
    pub fn routing_policy(&self) -> &RoutingPolicy {
        &self.routing_policy
    }
}

impl Default for RoutingEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_pair_native_with_issued() {
        // "USDC:GA5ZSEJ" < "native" lexicographically ('U' = 85, 'n' = 110 in ASCII)
        let (base, quote) = normalize_pair("native", "USDC:GA5ZSEJ");
        assert_eq!(base, "USDC:GA5ZSEJ");
        assert_eq!(quote, "native");

        let (base, quote) = normalize_pair("USDC:GA5ZSEJ", "native");
        assert_eq!(base, "USDC:GA5ZSEJ");
        assert_eq!(quote, "native");
    }

    #[test]
    fn test_normalize_pair_issued_assets() {
        let (base, quote) = normalize_pair("BTC:GA1", "USDC:GA2");
        assert_eq!(base, "BTC:GA1");
        assert_eq!(quote, "USDC:GA2");

        let (base, quote) = normalize_pair("USDC:GA2", "BTC:GA1");
        assert_eq!(base, "BTC:GA1");
        assert_eq!(quote, "USDC:GA2");
    }

    #[test]
    fn test_normalize_pair_equal_assets() {
        let (base, quote) = normalize_pair("native", "native");
        assert_eq!(base, "native");
        assert_eq!(quote, "native");
    }

    #[test]
    fn test_normalize_pair_code_only() {
        let (base, quote) = normalize_pair("USDC", "BTC");
        assert_eq!(base, "BTC");
        assert_eq!(quote, "USDC");
    }

    #[test]
    fn test_normalize_pair_mixed_formats() {
        // 'U' (85) < 'X' (88) in ASCII
        let (base, quote) = normalize_pair("XLM", "USDC:GA5ZSEJ");
        assert_eq!(base, "USDC:GA5ZSEJ");
        assert_eq!(quote, "XLM");

        // 'X' (88) < 'n' (110) in ASCII
        let (base, quote) = normalize_pair("native", "XLM");
        assert_eq!(base, "XLM");
        assert_eq!(quote, "native");
    }

    #[test]
    fn test_normalize_pair_identity() {
        // normalize_pair is a pure function: applying it twice is idempotent
        let a = "USDC:GA5ZSEJ";
        let b = "native";
        let (base, quote) = normalize_pair(a, b);
        let (base2, quote2) = normalize_pair(base, quote);
        assert_eq!(base, base2);
        assert_eq!(quote, quote2);
    }

    // ── normalize_asset tests ─────────────────────────────────────────

    #[test]
    fn test_normalize_asset_native_forms() {
        assert_eq!(normalize_asset("native"), "native");
        assert_eq!(normalize_asset("XLM"), "native");
        assert_eq!(normalize_asset("xlm"), "native");
        assert_eq!(normalize_asset("Xlm"), "native");
    }

    #[test]
    fn test_normalize_asset_issued_uppercased() {
        assert_eq!(normalize_asset("usdc"), "USDC");
        assert_eq!(normalize_asset("USDC"), "USDC");
        assert_eq!(normalize_asset("Usdc"), "USDC");
        assert_eq!(normalize_asset("btc"), "BTC");
    }

    #[test]
    fn test_normalize_asset_with_issuer() {
        // The colon and issuer part is preserved as-is (uppercased as a whole)
        assert_eq!(
            normalize_asset("usdc:GA5ZSEJ"),
            "USDC:GA5ZSEJ"
        );
        assert_eq!(
            normalize_asset("USDC:ga5zsej"),
            "USDC:GA5ZSEJ"
        );
    }

    // ── normalize_pair_owned tests ────────────────────────────────────

    #[test]
    fn test_normalize_pair_owned_native_with_issued() {
        // normalize_asset maps XLM→native, then native > USDC lexicographically
        let (base, quote) = normalize_pair_owned("XLM", "USDC:GA5ZSEJ");
        assert_eq!(base, "USDC:GA5ZSEJ");
        assert_eq!(quote, "native");

        // Idempotent regardless of input order
        let (base, quote) = normalize_pair_owned("USDC:GA5ZSEJ", "xlm");
        assert_eq!(base, "USDC:GA5ZSEJ");
        assert_eq!(quote, "native");
    }

    #[test]
    fn test_normalize_pair_owned_both_issued() {
        let (base, quote) = normalize_pair_owned("BTC", "USDC");
        assert_eq!(base, "BTC");
        assert_eq!(quote, "USDC");

        let (base, quote) = normalize_pair_owned("usdc", "btc");
        assert_eq!(base, "BTC");
        assert_eq!(quote, "USDC");
    }

    #[test]
    fn test_normalize_pair_owned_equal_assets() {
        let (base, quote) = normalize_pair_owned("XLM", "xlm");
        assert_eq!(base, "native");
        assert_eq!(quote, "native");
    }

    #[test]
    fn test_normalize_pair_owned_round_trip() {
        // Applying twice is idempotent
        let (b1, q1) = normalize_pair_owned("XLM", "USDC");
        let (b2, q2) = normalize_pair_owned(&b1, &q1);
        assert_eq!(b1, b2);
        assert_eq!(q1, q2);
    }

    #[test]
    fn test_normalize_pair_owned_mixed_case() {
        let (base, quote) = normalize_pair_owned("xlm", "usdc:GA5ZSEJ");
        assert_eq!(base, "USDC:GA5ZSEJ");
        assert_eq!(quote, "native");
    }
}
