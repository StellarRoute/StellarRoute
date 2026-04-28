#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::scorer::{HealthRecord, ScoredVenue, VenueType};
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

    fn default_policy() -> ExclusionPolicy {
        ExclusionPolicy {
            thresholds: ExclusionThresholds {
                sdex: 0.5,
                amm: 0.5,
            },
            overrides: OverrideRegistry::default(),
            circuit_breaker: None,
        }
    }

    #[test]
    fn threshold_boundary_not_excluded() {
        let policy = default_policy();
        let scored = vec![make_scored("venue:A", VenueType::Sdex, 0.5)];
        let (excluded, _) = policy.apply(&scored);

        assert!(!excluded.contains("venue:A"));
    }

    #[test]
    fn below_threshold_excluded() {
        let policy = default_policy();
        let scored = vec![make_scored("venue:B", VenueType::Sdex, 0.49)];
        let (excluded, diagnostics) = policy.apply(&scored);

        assert!(excluded.contains("venue:B"));
        assert_eq!(diagnostics.excluded_venues.len(), 1);
    }

    #[test]
    fn force_include_overrides_low_score() {
        let policy = ExclusionPolicy {
            thresholds: ExclusionThresholds::default(),
            overrides: OverrideRegistry::from_entries(vec![OverrideEntry {
                venue_ref: "venue:C".to_string(),
                directive: OverrideDirective::ForceInclude,
            }]),
            circuit_breaker: None,
        };

        let scored = vec![make_scored("venue:C", VenueType::Sdex, 0.0)];
        let (excluded, _) = policy.apply(&scored);

        assert!(!excluded.contains("venue:C"));
    }

    #[test]
    fn force_exclude_overrides_high_score() {
        let policy = ExclusionPolicy {
            thresholds: ExclusionThresholds::default(),
            overrides: OverrideRegistry::from_entries(vec![OverrideEntry {
                venue_ref: "venue:D".to_string(),
                directive: OverrideDirective::ForceExclude,
            }]),
            circuit_breaker: None,
        };

        let scored = vec![make_scored("venue:D", VenueType::Sdex, 1.0)];
        let (excluded, diagnostics) = policy.apply(&scored);

        assert!(excluded.contains("venue:D"));
        assert!(matches!(
            diagnostics.excluded_venues[0].reason,
            ExclusionReason::Override
        ));
    }

    #[test]
    fn unrecognized_override_key_no_error() {
        let policy = ExclusionPolicy {
            thresholds: ExclusionThresholds::default(),
            overrides: OverrideRegistry::from_entries(vec![OverrideEntry {
                venue_ref: "venue:UNKNOWN".to_string(),
                directive: OverrideDirective::ForceExclude,
            }]),
            circuit_breaker: None,
        };

        let scored = vec![make_scored("venue:E", VenueType::Sdex, 0.8)];
        let (excluded, _) = policy.apply(&scored);

        assert!(!excluded.contains("venue:E"));
    }

    // ----------------------------------------------------------------------
    // ✅ STEP 4 REQUIRED TESTS (MUTATION + ROLLBACK BEHAVIOR)
    // ----------------------------------------------------------------------

    #[test]
    fn test_policy_mutation_updates_threshold() {
        let mut policy = default_policy();

        // mutate policy
        policy.thresholds.sdex = 0.9;

        let scored = vec![make_scored("venue:X", VenueType::Sdex, 0.85)];
        let (excluded, _) = policy.apply(&scored);

        // should now be excluded due to higher threshold
        assert!(excluded.contains("venue:X"));
    }

    #[test]
    fn test_policy_override_mutation() {
        let mut policy = default_policy();

        // add override AFTER creation
        policy.overrides = OverrideRegistry::from_entries(vec![OverrideEntry {
            venue_ref: "venue:Y".to_string(),
            directive: OverrideDirective::ForceInclude,
        }]);

        let scored = vec![make_scored("venue:Y", VenueType::Sdex, 0.0)];
        let (excluded, _) = policy.apply(&scored);

        assert!(!excluded.contains("venue:Y"));
    }

    #[test]
    fn test_policy_rollback_behavior() {
        let original = ExclusionThresholds::default();

        let mut policy = ExclusionPolicy {
            thresholds: original.clone(),
            overrides: OverrideRegistry::default(),
            circuit_breaker: None,
        };

        // mutate
        policy.thresholds.sdex = 0.95;

        // rollback
        policy.thresholds = original;

        let scored = vec![make_scored("venue:Z", VenueType::Sdex, 0.6)];
        let (excluded, _) = policy.apply(&scored);

        // after rollback, should NOT be excluded
        assert!(!excluded.contains("venue:Z"));
    }

    #[test]
    fn test_policy_applies_after_multiple_mutations() {
        let mut policy = default_policy();

        // step 1: tighten threshold
        policy.thresholds.sdex = 0.8;

        // step 2: add override
        policy.overrides = OverrideRegistry::from_entries(vec![OverrideEntry {
            venue_ref: "venue:M".to_string(),
            directive: OverrideDirective::ForceExclude,
        }]);

        let scored = vec![make_scored("venue:M", VenueType::Sdex, 0.99)];
        let (excluded, _) = policy.apply(&scored);

        // override wins even after mutation
        assert!(excluded.contains("venue:M"));
    }
}