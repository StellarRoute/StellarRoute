//! Deterministic replay engine.
//!
//! Re-executes the route selection logic from a stored `ReplayArtifact`
//! without touching the live database. The engine is a pure synchronous
//! function — no I/O, no async, no randomness — guaranteeing idempotent
//! determinism.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ApiError, Result};
use crate::models::{AssetInfo, PathStep, VenueEvaluation};
use crate::replay::artifact::{LiquidityCandidate, ReplayArtifact, CURRENT_SCHEMA_VERSION};

// ---------------------------------------------------------------------------
// Output type
// ---------------------------------------------------------------------------

/// The result produced by the replay engine for a given artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayOutput {
    /// The artifact this output was produced from.
    pub artifact_id: Uuid,
    /// The venue that was selected (e.g. "sdex:offer1" or "amm:pool1").
    pub selected_source: String,
    /// Best price as a 7-decimal string.
    pub price: String,
    /// Execution path (single hop for current implementation).
    pub path: Vec<PathStep>,
    /// Deterministically ordered venues considered for selection.
    pub compared_venues: Vec<VenueEvaluation>,
    /// `true` when `selected_source` matches the value in `original_output`.
    pub is_deterministic: bool,
    /// Wall-clock time when the replay was executed.
    pub replayed_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// Pure deterministic replay of route selection from a stored artifact.
pub struct ReplayEngine;

impl ReplayEngine {
    /// Re-execute route selection from a stored artifact.
    ///
    /// # Errors
    ///
    /// - `ApiError::BadRequest` if `schema_version` is incompatible.
    /// - `ApiError::BadRequest` if `liquidity_snapshot` is empty.
    /// - `ApiError::NoRouteFound` if no candidate has sufficient liquidity.
    pub fn run(artifact: &ReplayArtifact) -> Result<ReplayOutput> {
        // Schema version guard
        if artifact.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(ApiError::BadRequest(format!(
                "Incompatible artifact schema version: found {}, expected {}",
                artifact.schema_version, CURRENT_SCHEMA_VERSION
            )));
        }

        if artifact.liquidity_snapshot.is_empty() {
            return Err(ApiError::BadRequest(
                "Artifact has an empty liquidity snapshot; cannot replay".to_string(),
            ));
        }

        // Parse amount
        let amount: f64 = artifact
            .amount
            .parse()
            .map_err(|_| ApiError::BadRequest("Invalid amount in artifact".to_string()))?;

        // Reconstruct candidates from decision graph stage input when present.
        // Fallback to liquidity_snapshot for older artifacts.
        let candidates: Vec<ReplayCandidate> = selection_input_candidates(artifact)
            .or_else(|| {
                Some(
                    artifact
                        .liquidity_snapshot
                        .iter()
                        .filter_map(parse_candidate)
                        .collect(),
                )
            })
            .unwrap_or_default();

        // Run the same deterministic selection as the live pipeline:
        // sort by price ASC, venue_type ASC, venue_ref ASC
        let (selected, sorted_candidates) = select_best_venue(candidates, amount)?;

        let selected_source = format!("{}:{}", selected.venue_type, selected.venue_ref);
        let compared_venues = sorted_candidates
            .iter()
            .map(|candidate| VenueEvaluation {
                source: format!("{}:{}", candidate.venue_type, candidate.venue_ref),
                price: format!("{:.7}", candidate.price),
                available_amount: format!("{:.7}", candidate.available_amount),
                executable: candidate.available_amount >= amount && candidate.price > 0.0,
            })
            .collect::<Vec<_>>();

        // Determine is_deterministic by comparing with original_output
        let original_source = artifact
            .original_output
            .get("selected_source")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let is_deterministic = selected_source == original_source;

        // Build path using asset strings from the artifact
        let base_info = parse_asset_info(&artifact.base);
        let quote_info = parse_asset_info(&artifact.quote);

        let path = vec![PathStep {
            from_asset: base_info,
            to_asset: quote_info,
            price: format!("{:.7}", selected.price),
            source: if selected.venue_type == "amm" {
                format!("amm:{}", selected.venue_ref)
            } else {
                "sdex".to_string()
            },
            liquidity_depth: Some(format!("{:.7}", selected.available_amount)),
            fee_bps: Some(selected.fee_bps),
        }];

        Ok(ReplayOutput {
            artifact_id: artifact.id,
            selected_source,
            price: format!("{:.7}", selected.price),
            path,
            compared_venues,
            is_deterministic,
            replayed_at: Utc::now(),
        })
    }
}

// ---------------------------------------------------------------------------
// Internal helpers (mirrors quote.rs logic without importing it)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct ReplayCandidate {
    venue_type: String,
    venue_ref: String,
    price: f64,
    available_amount: f64,
    fee_bps: u32,
}

fn parse_candidate(row: &LiquidityCandidate) -> Option<ReplayCandidate> {
    let price: f64 = row.price.parse().ok()?;
    let available_amount: f64 = row.available_amount.parse().ok()?;
    Some(ReplayCandidate {
        venue_type: row.venue_type.clone(),
        venue_ref: row.venue_ref.clone(),
        price,
        available_amount,
        fee_bps: row.fee_bps.unwrap_or(0),
    })
}

fn selection_input_candidates(artifact: &ReplayArtifact) -> Option<Vec<ReplayCandidate>> {
    let node = artifact
        .decision_graph
        .nodes
        .iter()
        .find(|n| n.stage == "venue_selection_input")?;

    let arr = node.payload.as_array()?;
    let candidates = arr
        .iter()
        .filter_map(|row| {
            Some(ReplayCandidate {
                venue_type: row.get("venue_type")?.as_str()?.to_string(),
                venue_ref: row.get("venue_ref")?.as_str()?.to_string(),
                price: row.get("price")?.as_f64()?,
                available_amount: row.get("available_amount")?.as_f64()?,
                fee_bps: row.get("fee_bps")?.as_u64()? as u32,
            })
        })
        .collect::<Vec<_>>();

    Some(candidates)
}

/// Deterministic venue selection: sort price ASC → venue_type ASC → venue_ref ASC,
/// then pick the first candidate with sufficient liquidity and positive price.
fn select_best_venue(
    mut candidates: Vec<ReplayCandidate>,
    amount: f64,
) -> Result<(ReplayCandidate, Vec<ReplayCandidate>)> {
    if candidates.is_empty() {
        return Err(ApiError::NoRouteFound);
    }

    candidates.sort_by(|a, b| {
        a.price
            .partial_cmp(&b.price)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.venue_type.cmp(&b.venue_type))
            .then_with(|| a.venue_ref.cmp(&b.venue_ref))
    });

    let selected = candidates
        .iter()
        .find(|c| c.available_amount >= amount && c.price > 0.0)
        .cloned()
        .ok_or(ApiError::NoRouteFound)?;

    Ok((selected, candidates))
}

/// Parse a canonical asset string into `AssetInfo`.
fn parse_asset_info(s: &str) -> AssetInfo {
    if s == "native" {
        return AssetInfo::native();
    }
    let parts: Vec<&str> = s.splitn(2, ':').collect();
    match parts.as_slice() {
        [code, issuer] => AssetInfo::credit(code.to_string(), Some(issuer.to_string())),
        [code] => AssetInfo::credit(code.to_string(), None),
        _ => AssetInfo::native(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::artifact::{
        DecisionGraphSnapshot, HealthConfigSnapshot, CURRENT_SCHEMA_VERSION,
    };
    use proptest::prelude::*;

    fn make_artifact(candidates: Vec<LiquidityCandidate>, amount: &str) -> ReplayArtifact {
        let first = candidates.first().cloned().unwrap_or(LiquidityCandidate {
            venue_type: "sdex".to_string(),
            venue_ref: "offer1".to_string(),
            price: "1.0000000".to_string(),
            available_amount: "100.0000000".to_string(),
            fee_bps: Some(0),
        });
        let source = format!("{}:{}", first.venue_type, first.venue_ref);
        ReplayArtifact {
            id: Uuid::new_v4(),
            schema_version: CURRENT_SCHEMA_VERSION,
            incident_id: None,
            captured_at: Utc::now(),
            base: "native".to_string(),
            quote: "USDC:[REDACTED]".to_string(),
            amount: amount.to_string(),
            slippage_bps: 50,
            quote_type: "sell".to_string(),
            liquidity_snapshot: candidates,
            decision_graph: DecisionGraphSnapshot::default(),
            health_config_snapshot: HealthConfigSnapshot {
                freshness_threshold_secs_sdex: 30,
                freshness_threshold_secs_amm: 60,
                staleness_threshold_secs: 30,
                min_tvl_threshold_e7: 1_000_000_000,
            },
            original_output: serde_json::json!({
                "price": first.price,
                "selected_source": source,
            }),
        }
    }

    fn candidate(
        venue_type: &str,
        venue_ref: &str,
        price: &str,
        amount: &str,
        fee_bps: Option<u32>,
    ) -> LiquidityCandidate {
        LiquidityCandidate {
            venue_type: venue_type.to_string(),
            venue_ref: venue_ref.to_string(),
            price: price.to_string(),
            available_amount: amount.to_string(),
            fee_bps,
        }
    }

    // ── Unit tests ──────────────────────────────────────────────────────────

    #[test]
    fn selects_lower_priced_candidate() {
        let artifact = make_artifact(
            vec![
                candidate("amm", "pool1", "1.0200000", "100.0000000", Some(30)),
                candidate("sdex", "offer1", "1.0000000", "100.0000000", Some(0)),
            ],
            "50.0000000",
        );
        let output = ReplayEngine::run(&artifact).expect("should succeed");
        assert_eq!(output.selected_source, "sdex:offer1");
        assert_eq!(output.price, "1.0000000");
    }

    #[test]
    fn schema_version_mismatch_returns_bad_request() {
        let mut artifact = make_artifact(
            vec![candidate("sdex", "offer1", "1.0000000", "100.0000000", Some(0))],
            "1.0000000",
        );
        artifact.schema_version = 99;
        let err = ReplayEngine::run(&artifact).unwrap_err();
        assert!(matches!(err, ApiError::BadRequest(_)));
    }

    #[test]
    fn empty_snapshot_returns_bad_request() {
        let mut artifact = make_artifact(vec![], "1.0000000");
        artifact.liquidity_snapshot = vec![];
        let err = ReplayEngine::run(&artifact).unwrap_err();
        assert!(matches!(err, ApiError::BadRequest(_)));
    }

    #[test]
    fn insufficient_liquidity_returns_no_route() {
        let artifact = make_artifact(
            vec![candidate("sdex", "offer1", "1.0000000", "5.0000000", Some(0))],
            "100.0000000",
        );
        let err = ReplayEngine::run(&artifact).unwrap_err();
        assert!(matches!(err, ApiError::NoRouteFound));
    }

    #[test]
    fn is_deterministic_true_when_source_matches() {
        let artifact = make_artifact(
            vec![candidate("sdex", "offer1", "1.0000000", "100.0000000", Some(0))],
            "50.0000000",
        );
        let output = ReplayEngine::run(&artifact).expect("should succeed");
        assert!(output.is_deterministic);
    }

    // ── Property-based tests ────────────────────────────────────────────────

    prop_compose! {
        fn arb_candidate()(
            venue_type in prop::sample::select(vec!["sdex", "amm"]),
            venue_ref in "[a-z0-9]{4,12}",
            price_int in 1u64..1_000_000u64,
        ) -> LiquidityCandidate {
            let vt = venue_type.to_string();
            LiquidityCandidate {
                venue_type: vt.clone(),
                venue_ref,
                price: format!("{:.7}", price_int as f64 / 1_000_000.0),
                available_amount: "1000.0000000".to_string(),
                fee_bps: if vt == "amm" { Some(30) } else { Some(0) },
            }
        }
    }

    proptest! {
        /// Property 2: Replay determinism — running the engine twice on the same
        /// artifact produces identical selected_source and price.
        ///
        /// Feature: quote-replay-system, Property 2: replay determinism
        #[test]
        fn prop_replay_is_deterministic(
            candidates in prop::collection::vec(arb_candidate(), 1..8)
        ) {
            let artifact = make_artifact(candidates, "1.0000000");
            let out1 = ReplayEngine::run(&artifact);
            let out2 = ReplayEngine::run(&artifact);
            match (out1, out2) {
                (Ok(a), Ok(b)) => {
                    prop_assert_eq!(&a.selected_source, &b.selected_source);
                    prop_assert_eq!(&a.price, &b.price);
                }
                (Err(_), Err(_)) => {} // both fail consistently — still deterministic
                _ => prop_assert!(false, "inconsistent results between two runs"),
            }
        }
    }
}
