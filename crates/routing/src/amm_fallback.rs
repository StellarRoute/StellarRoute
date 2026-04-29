//! Tiered fallback strategy for degraded AMM data availability.
//!
//! # Problem
//! AMM pool state (reserves, fees) can be partially unavailable: a single pool
//! may be stale, an entire data source may be down, or reserve data may be
//! missing altogether.  The router must not hard-fail in these situations if
//! viable routes still exist at a lower tier.
//!
//! # Tiers (highest → lowest preference)
//!
//! | Tier | Name | Description |
//! |------|------|-------------|
//! | 0 | `LiveAmm` | Fresh AMM data within the staleness threshold. |
//! | 1 | `CachedAmm` | Slightly stale AMM data (beyond threshold but within `max_cache_age_secs`). |
//! | 2 | `SdexOnly` | No AMM data at all; route only through the SDEX orderbook. |
//! | 3 | `Empty` | No viable data source. |
//!
//! Tiers are tried in order until one succeeds.  The tier actually used is
//! recorded in [`FallbackResult`] so quote responses can surface it to clients
//! (e.g. `"data_tier": "cached_amm"`).
//!
//! # Configuration
//! All tier thresholds are configurable via [`AmmFallbackConfig`].

use crate::pathfinder::LiquidityEdge;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

// ── Tier enumeration ──────────────────────────────────────────────────────────

/// The data availability tier used for a given quote.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmmFallbackTier {
    /// Fresh AMM reserves within the normal staleness threshold.
    LiveAmm = 0,
    /// AMM reserves that have exceeded the freshness threshold but are still
    /// within the configurable `max_cache_age_secs` window.
    CachedAmm = 1,
    /// No AMM data available; routes are assembled from SDEX orderbook only.
    SdexOnly = 2,
    /// No viable edges remain.  Quote must fail.
    Empty = 3,
}

impl std::fmt::Display for AmmFallbackTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::LiveAmm => "live_amm",
            Self::CachedAmm => "cached_amm",
            Self::SdexOnly => "sdex_only",
            Self::Empty => "empty",
        };
        f.write_str(s)
    }
}

// ── Configuration ─────────────────────────────────────────────────────────────

/// Configuration for the tiered AMM fallback strategy.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AmmFallbackConfig {
    /// Maximum age of live AMM data (edges are considered fresh below this).
    pub live_threshold: Duration,
    /// Maximum age before an edge is no longer usable even as cached data.
    pub max_cache_age: Duration,
    /// When `true` (default), fall through to SDEX-only if all AMM tiers fail.
    pub allow_sdex_fallback: bool,
}

impl Default for AmmFallbackConfig {
    fn default() -> Self {
        Self {
            live_threshold: Duration::from_secs(60),
            max_cache_age: Duration::from_secs(300),
            allow_sdex_fallback: true,
        }
    }
}

// ── Edge metadata ─────────────────────────────────────────────────────────────

/// A liquidity edge annotated with the age of its underlying pool state.
#[derive(Clone, Debug)]
pub struct AmmEdgeWithAge {
    pub edge: LiquidityEdge,
    /// How long ago the pool state was captured.
    pub data_age: Duration,
}

// ── Result ────────────────────────────────────────────────────────────────────

/// Outcome of a fallback resolution attempt.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FallbackResult {
    /// The tier that was ultimately used to assemble the edge set.
    pub tier: AmmFallbackTier,
    /// Edges selected for routing (filtered to the chosen tier).
    pub edges: Vec<LiquidityEdge>,
    /// Informational message describing why this tier was chosen.
    pub reason: String,
    /// Number of edges dropped due to data staleness.
    pub dropped_edge_count: usize,
}

// ── Errors ────────────────────────────────────────────────────────────────────

/// Returned when no tier produces viable edges.
#[derive(Debug, Error)]
pub enum FallbackError {
    #[error("no viable edges in any fallback tier")]
    NoViableEdges,
}

// ── Strategy ──────────────────────────────────────────────────────────────────

/// Tiered fallback resolver.  Accepts a mixed set of age-annotated edges and
/// returns the best available subset together with the tier label.
pub struct TieredAmmFallback {
    config: AmmFallbackConfig,
}

impl TieredAmmFallback {
    /// Create a new strategy with the given config.
    pub fn new(config: AmmFallbackConfig) -> Self {
        Self { config }
    }

    /// Resolve the best available edge set given the annotated input.
    ///
    /// # Algorithm
    /// 1. Try **Tier 0** (LiveAmm): all edges whose `data_age < live_threshold`.
    /// 2. Try **Tier 1** (CachedAmm): all AMM edges whose `data_age < max_cache_age`.
    /// 3. Try **Tier 2** (SdexOnly): all edges whose `venue_type == "sdex"`.
    /// 4. Return `Err(FallbackError::NoViableEdges)`.
    pub fn resolve(&self, candidates: &[AmmEdgeWithAge]) -> Result<FallbackResult, FallbackError> {
        // Tier 0 – live AMM (only AMM-typed edges within the freshness window)
        let live: Vec<LiquidityEdge> = candidates
            .iter()
            .filter(|c| c.edge.venue_type == "amm" && c.data_age < self.config.live_threshold)
            .map(|c| c.edge.clone())
            .collect();

        if !live.is_empty() {
            let dropped = candidates.len() - live.len();
            tracing::debug!(tier = "live_amm", kept = live.len(), dropped, "fallback resolved");
            return Ok(FallbackResult {
                tier: AmmFallbackTier::LiveAmm,
                edges: live,
                reason: "fresh AMM data available".to_string(),
                dropped_edge_count: dropped,
            });
        }

        // Tier 1 – cached AMM (AMM edges beyond live threshold but within max_cache_age)
        let cached: Vec<LiquidityEdge> = candidates
            .iter()
            .filter(|c| {
                c.edge.venue_type == "amm" && c.data_age < self.config.max_cache_age
            })
            .map(|c| c.edge.clone())
            .collect();

        if !cached.is_empty() {
            let dropped = candidates.len() - cached.len();
            tracing::warn!(
                tier = "cached_amm",
                kept = cached.len(),
                dropped,
                "falling back to cached AMM data"
            );
            return Ok(FallbackResult {
                tier: AmmFallbackTier::CachedAmm,
                edges: cached,
                reason: "AMM data is stale but within cache window".to_string(),
                dropped_edge_count: dropped,
            });
        }

        // Tier 2 – SDEX only
        if self.config.allow_sdex_fallback {
            let sdex: Vec<LiquidityEdge> = candidates
                .iter()
                .filter(|c| c.edge.venue_type == "sdex")
                .map(|c| c.edge.clone())
                .collect();

            if !sdex.is_empty() {
                let dropped = candidates.len() - sdex.len();
                tracing::warn!(
                    tier = "sdex_only",
                    kept = sdex.len(),
                    dropped,
                    "falling back to SDEX-only routing"
                );
                return Ok(FallbackResult {
                    tier: AmmFallbackTier::SdexOnly,
                    edges: sdex,
                    reason: "AMM data unavailable; routing via SDEX only".to_string(),
                    dropped_edge_count: dropped,
                });
            }
        }

        // Tier 3 – nothing viable
        tracing::error!(tier = "empty", "all fallback tiers exhausted; no viable edges");
        Err(FallbackError::NoViableEdges)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn amm_edge(venue_ref: &str) -> LiquidityEdge {
        LiquidityEdge {
            from: "XLM".to_string(),
            to: "USDC".to_string(),
            venue_type: "amm".to_string(),
            venue_ref: venue_ref.to_string(),
            liquidity: 1_000_000_000,
            price: 1.0,
            fee_bps: 30,
            anomaly_score: 0.0,
            anomaly_reasons: vec![],
        }
    }

    fn sdex_edge(venue_ref: &str) -> LiquidityEdge {
        LiquidityEdge {
            from: "XLM".to_string(),
            to: "USDC".to_string(),
            venue_type: "sdex".to_string(),
            venue_ref: venue_ref.to_string(),
            liquidity: 500_000_000,
            price: 1.0,
            fee_bps: 10,
            anomaly_score: 0.0,
            anomaly_reasons: vec![],
        }
    }

    fn default_strategy() -> TieredAmmFallback {
        TieredAmmFallback::new(AmmFallbackConfig {
            live_threshold: Duration::from_secs(60),
            max_cache_age: Duration::from_secs(300),
            allow_sdex_fallback: true,
        })
    }

    #[test]
    fn test_tier0_live_amm_selected_when_fresh() {
        let strategy = default_strategy();
        let candidates = vec![AmmEdgeWithAge {
            edge: amm_edge("pool_fresh"),
            data_age: Duration::from_secs(10),
        }];
        let result = strategy.resolve(&candidates).unwrap();
        assert_eq!(result.tier, AmmFallbackTier::LiveAmm);
        assert_eq!(result.edges.len(), 1);
    }

    #[test]
    fn test_tier1_cached_amm_when_stale_but_within_cache() {
        let strategy = default_strategy();
        let candidates = vec![AmmEdgeWithAge {
            edge: amm_edge("pool_stale"),
            data_age: Duration::from_secs(120), // >60s live threshold, <300s max_cache
        }];
        let result = strategy.resolve(&candidates).unwrap();
        assert_eq!(result.tier, AmmFallbackTier::CachedAmm);
    }

    #[test]
    fn test_tier2_sdex_only_when_amm_expired() {
        let strategy = default_strategy();
        let candidates = vec![
            AmmEdgeWithAge {
                edge: amm_edge("pool_expired"),
                data_age: Duration::from_secs(400), // beyond max_cache_age
            },
            AmmEdgeWithAge {
                edge: sdex_edge("offer_1"),
                data_age: Duration::from_secs(5),
            },
        ];
        let result = strategy.resolve(&candidates).unwrap();
        assert_eq!(result.tier, AmmFallbackTier::SdexOnly);
        assert!(result.edges.iter().all(|e| e.venue_type == "sdex"));
    }

    #[test]
    fn test_no_viable_edges_returns_error() {
        let strategy = default_strategy();
        let candidates = vec![AmmEdgeWithAge {
            edge: amm_edge("pool_expired"),
            data_age: Duration::from_secs(9999),
        }];
        // SDEX-only fallback won't help since there are no SDEX edges.
        assert!(matches!(strategy.resolve(&candidates), Err(FallbackError::NoViableEdges)));
    }

    #[test]
    fn test_sdex_fallback_disabled() {
        let strategy = TieredAmmFallback::new(AmmFallbackConfig {
            live_threshold: Duration::from_secs(60),
            max_cache_age: Duration::from_secs(300),
            allow_sdex_fallback: false,
        });
        let candidates = vec![
            AmmEdgeWithAge {
                edge: amm_edge("pool_expired"),
                data_age: Duration::from_secs(400),
            },
            AmmEdgeWithAge {
                edge: sdex_edge("offer_1"),
                data_age: Duration::from_secs(5),
            },
        ];
        assert!(matches!(strategy.resolve(&candidates), Err(FallbackError::NoViableEdges)));
    }

    #[test]
    fn test_tier0_preferred_over_cached() {
        let strategy = default_strategy();
        let candidates = vec![
            AmmEdgeWithAge {
                edge: amm_edge("pool_fresh"),
                data_age: Duration::from_secs(10),
            },
            AmmEdgeWithAge {
                edge: amm_edge("pool_stale"),
                data_age: Duration::from_secs(150),
            },
        ];
        let result = strategy.resolve(&candidates).unwrap();
        assert_eq!(result.tier, AmmFallbackTier::LiveAmm);
        // Only the fresh pool is in the result
        assert_eq!(result.edges.len(), 1);
        assert_eq!(result.edges[0].venue_ref, "pool_fresh");
        assert_eq!(result.dropped_edge_count, 1);
    }

    #[test]
    fn test_fallback_result_serializable() {
        let result = FallbackResult {
            tier: AmmFallbackTier::CachedAmm,
            edges: vec![],
            reason: "test".to_string(),
            dropped_edge_count: 2,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("cached_amm"));
    }

    #[test]
    fn test_tier_ordering() {
        assert!(AmmFallbackTier::LiveAmm < AmmFallbackTier::CachedAmm);
        assert!(AmmFallbackTier::CachedAmm < AmmFallbackTier::SdexOnly);
        assert!(AmmFallbackTier::SdexOnly < AmmFallbackTier::Empty);
    }

    #[test]
    fn test_empty_candidates_returns_error() {
        let strategy = default_strategy();
        assert!(matches!(strategy.resolve(&[]), Err(FallbackError::NoViableEdges)));
    }
}
