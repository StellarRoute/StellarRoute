use crate::health::policy::{ExclusionDiagnostics, ExclusionPolicy};
use crate::health::scorer::ScoredVenue;
use crate::pathfinder::LiquidityEdge;

pub struct GraphFilter<'a> {
    pub policy: &'a ExclusionPolicy,
}

impl<'a> GraphFilter<'a> {
    pub fn new(policy: &'a ExclusionPolicy) -> Self {
        Self { policy }
    }

    /// Filters edges and returns `(filtered_edges, diagnostics)`.
    pub fn filter_edges(
        &self,
        edges: &[LiquidityEdge],
        scored: &[ScoredVenue],
    ) -> (Vec<LiquidityEdge>, ExclusionDiagnostics) {
        let (excluded, diagnostics) = self.policy.apply(scored);
        let filtered = edges
            .iter()
            .filter(|e| !excluded.contains(&e.venue_ref))
            .cloned()
            .collect();
        (filtered, diagnostics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::policy::{ExclusionPolicy, ExclusionThresholds, OverrideRegistry};
    use crate::health::scorer::{HealthRecord, ScoredVenue, VenueType};
    use crate::pathfinder::LiquidityEdge;
    use chrono::Utc;

    fn make_scored(venue_ref: &str, venue_type: VenueType, score: f64) -> ScoredVenue {
        ScoredVenue {
            venue_ref: venue_ref.to_string(),
            venue_type: venue_type.clone(),
            record: HealthRecord {
                venue_ref: venue_ref.to_string(),
                venue_type,
                score,
                signals: serde_json::json!({}),
                computed_at: Utc::now(),
            },
        }
    }

    fn make_edge(venue_ref: &str) -> LiquidityEdge {
        LiquidityEdge {
            from: "XLM".to_string(),
            to: "USDC".to_string(),
            venue_type: "sdex".to_string(),
            venue_ref: venue_ref.to_string(),
            liquidity: 1_000_000_000,
            price: 1.0,
            fee_bps: 30,
        }
    }

    fn policy_with_threshold(threshold: f64) -> ExclusionPolicy {
        ExclusionPolicy {
            thresholds: ExclusionThresholds {
                sdex: threshold,
                amm: threshold,
            },
            overrides: OverrideRegistry::default(),
            circuit_breaker: None,
        }
    }

    #[test]
    fn all_excluded() {
        // All edges excluded when all venues below threshold
        let policy = policy_with_threshold(0.5);
        let filter = GraphFilter::new(&policy);

        let scored = vec![
            make_scored("venue:A", VenueType::Sdex, 0.1),
            make_scored("venue:B", VenueType::Sdex, 0.2),
        ];
        let edges = vec![make_edge("venue:A"), make_edge("venue:B")];

        let (filtered, diagnostics) = filter.filter_edges(&edges, &scored);
        assert!(
            filtered.is_empty(),
            "all edges should be excluded when all venues are below threshold"
        );
        assert_eq!(diagnostics.excluded_venues.len(), 2);
    }

    #[test]
    fn none_excluded() {
        // All edges kept when all venues above threshold
        let policy = policy_with_threshold(0.5);
        let filter = GraphFilter::new(&policy);

        let scored = vec![
            make_scored("venue:A", VenueType::Sdex, 0.8),
            make_scored("venue:B", VenueType::Sdex, 0.9),
        ];
        let edges = vec![make_edge("venue:A"), make_edge("venue:B")];

        let (filtered, diagnostics) = filter.filter_edges(&edges, &scored);
        assert_eq!(
            filtered.len(),
            2,
            "no edges should be excluded when all venues are above threshold"
        );
        assert!(diagnostics.excluded_venues.is_empty());
    }

    #[test]
    fn partial_exclusion() {
        // Only degraded venue edges removed
        let policy = policy_with_threshold(0.5);
        let filter = GraphFilter::new(&policy);

        let scored = vec![
            make_scored("venue:good", VenueType::Sdex, 0.8),
            make_scored("venue:bad", VenueType::Sdex, 0.1),
        ];
        let edges = vec![make_edge("venue:good"), make_edge("venue:bad")];

        let (filtered, diagnostics) = filter.filter_edges(&edges, &scored);
        assert_eq!(
            filtered.len(),
            1,
            "only the healthy venue edge should remain"
        );
        assert_eq!(filtered[0].venue_ref, "venue:good");
        assert_eq!(diagnostics.excluded_venues.len(), 1);
        assert_eq!(diagnostics.excluded_venues[0].venue_ref, "venue:bad");
    }
}
