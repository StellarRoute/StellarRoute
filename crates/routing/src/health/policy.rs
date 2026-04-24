use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::health::scorer::{ScoredVenue, VenueType};

// ---------------------------------------------------------------------------
// ExclusionThresholds
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExclusionThresholds {
    pub sdex: f64,
    pub amm: f64,
}

impl Default for ExclusionThresholds {
    fn default() -> Self {
        Self {
            sdex: 0.5,
            amm: 0.5,
        }
    }
}

// ---------------------------------------------------------------------------
// OverrideDirective / OverrideEntry / OverrideRegistry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverrideDirective {
    ForceInclude,
    ForceExclude,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OverrideEntry {
    pub venue_ref: String,
    pub directive: OverrideDirective,
}

#[derive(Debug, Clone, Default)]
pub struct OverrideRegistry {
    pub venue_entries: HashMap<String, OverrideDirective>,
    pub source_entries: HashMap<VenueType, OverrideDirective>,
}

impl OverrideRegistry {
    pub fn from_entries(entries: Vec<OverrideEntry>) -> Self {
        Self {
            venue_entries: entries
                .into_iter()
                .map(|e| (e.venue_ref, e.directive))
                .collect(),
            source_entries: HashMap::new(),
        }
    }

    pub fn with_source_overrides(mut self, sources: HashMap<VenueType, OverrideDirective>) -> Self {
        self.source_entries = sources;
        self
    }
}

// ---------------------------------------------------------------------------
// ExclusionPolicy
// ---------------------------------------------------------------------------

use crate::health::circuit_breaker::CircuitBreakerRegistry;
use std::sync::Arc;

pub struct ExclusionPolicy {
    pub thresholds: ExclusionThresholds,
    pub overrides: OverrideRegistry,
    pub circuit_breaker: Option<Arc<CircuitBreakerRegistry>>,
}

impl ExclusionPolicy {
    /// Returns `(excluded_refs, ExclusionDiagnostics)`.
    pub fn apply(&self, scored: &[ScoredVenue]) -> (HashSet<String>, ExclusionDiagnostics) {
        // Build a set of venue_refs present in the scored list for override validation.
        let scored_refs: HashSet<&str> = scored.iter().map(|v| v.venue_ref.as_str()).collect();

        // Warn about override entries that don't match any known venue.
        for venue_ref in self.overrides.venue_entries.keys() {
            if !scored_refs.contains(venue_ref.as_str()) {
                tracing::warn!(
                    venue_ref = %venue_ref,
                    "OverrideRegistry entry does not match any known venue"
                );
            }
        }

        let mut excluded = HashSet::new();
        let mut excluded_venues = Vec::new();

        for venue in scored {
            // 1. Check Source Override first
            let source_directive = self.overrides.source_entries.get(&venue.venue_type);
            if let Some(OverrideDirective::ForceExclude) = source_directive {
                excluded.insert(venue.venue_ref.clone());
                excluded_venues.push(ExcludedVenueInfo {
                    venue_ref: venue.venue_ref.clone(),
                    score: venue.record.score,
                    signals: venue.record.signals.clone(),
                    reason: ExclusionReason::Override,
                });
                continue;
            }

            // 2. Check Venue Override
            let venue_directive = self.overrides.venue_entries.get(&venue.venue_ref);

            match (source_directive, venue_directive) {
                (_, Some(OverrideDirective::ForceInclude)) | (Some(OverrideDirective::ForceInclude), _) => {
                    // Skip threshold check entirely — always included.
                }
                (_, Some(OverrideDirective::ForceExclude)) => {
                    excluded.insert(venue.venue_ref.clone());
                    excluded_venues.push(ExcludedVenueInfo {
                        venue_ref: venue.venue_ref.clone(),
                        score: venue.record.score,
                        signals: venue.record.signals.clone(),
                        reason: ExclusionReason::Override,
                    });
                }
                (None, None) => {
                    // 1. Check Circuit Breaker first
                    if let Some(registry) = &self.circuit_breaker {
                        if registry.is_venue_excluded(&venue.venue_ref) {
                            excluded.insert(venue.venue_ref.clone());
                            excluded_venues.push(ExcludedVenueInfo {
                                venue_ref: venue.venue_ref.clone(),
                                score: venue.record.score,
                                signals: venue.record.signals.clone(),
                                reason: ExclusionReason::CircuitBreakerOpen,
                            });
                            continue;
                        }
                    }

                    // 2. Check Static Threshold
                    let threshold = match venue.venue_type {
                        VenueType::Sdex => self.thresholds.sdex,
                        VenueType::Amm => self.thresholds.amm,
                    };
                    if venue.record.score < threshold {
                        excluded.insert(venue.venue_ref.clone());
                        excluded_venues.push(ExcludedVenueInfo {
                            venue_ref: venue.venue_ref.clone(),
                            score: venue.record.score,
                            signals: venue.record.signals.clone(),
                            reason: ExclusionReason::PolicyThreshold { threshold },
                        });
                    }
                }
            }
        }

        (excluded, ExclusionDiagnostics { excluded_venues })
    }

    /// Quick check if a venue/source is explicitly excluded (overrides + circuit breaker)
    pub fn is_excluded(&self, venue_ref: &str, venue_type: &VenueType) -> bool {
        // 1. Check Source Override
        let source_directive = self.overrides.source_entries.get(venue_type);
        if let Some(OverrideDirective::ForceExclude) = source_directive {
            return true;
        }

        // 2. Check Venue Override
        let venue_directive = self.overrides.venue_entries.get(venue_ref);
        match (source_directive, venue_directive) {
            (_, Some(OverrideDirective::ForceInclude))
            | (Some(OverrideDirective::ForceInclude), _) => false,
            (_, Some(OverrideDirective::ForceExclude)) => true,
            (None, None) => {
                if let Some(registry) = &self.circuit_breaker {
                    if registry.is_venue_excluded(venue_ref) {
                        return true;
                    }
                }
                false
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ExclusionDiagnostics / ExcludedVenueInfo / ExclusionReason
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExclusionDiagnostics {
    pub excluded_venues: Vec<ExcludedVenueInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcludedVenueInfo {
    pub venue_ref: String,
    pub score: f64,
    pub signals: serde_json::Value,
    pub reason: ExclusionReason,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ExclusionReason {
    PolicyThreshold { threshold: f64 },
    Override,
    StaleData,
    CircuitBreakerOpen,
    LiquidityAnomaly { score: f64, reasons: Vec<String> },
}

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
        // score == threshold (0.5) should NOT be excluded
        let policy = default_policy();
        let scored = vec![make_scored("venue:A", VenueType::Sdex, 0.5)];
        let (excluded, _) = policy.apply(&scored);
        assert!(
            !excluded.contains("venue:A"),
            "score == threshold should not be excluded"
        );
    }

    #[test]
    fn below_threshold_excluded() {
        // score < threshold should be excluded with PolicyThreshold reason
        let policy = default_policy();
        let scored = vec![make_scored("venue:B", VenueType::Sdex, 0.49)];
        let (excluded, diagnostics) = policy.apply(&scored);
        assert!(
            excluded.contains("venue:B"),
            "score below threshold should be excluded"
        );
        assert_eq!(diagnostics.excluded_venues.len(), 1);
        let info = &diagnostics.excluded_venues[0];
        assert_eq!(info.venue_ref, "venue:B");
        assert!(
            matches!(info.reason, ExclusionReason::PolicyThreshold { threshold } if (threshold - 0.5).abs() < f64::EPSILON),
            "reason should be PolicyThreshold with threshold 0.5"
        );
    }

    #[test]
    fn force_include_overrides_low_score() {
        // venue with score 0.0 but force_include should NOT be excluded
        let policy = ExclusionPolicy {
            thresholds: ExclusionThresholds {
                sdex: 0.5,
                amm: 0.5,
            },
            overrides: OverrideRegistry::from_entries(vec![OverrideEntry {
                venue_ref: "venue:C".to_string(),
                directive: OverrideDirective::ForceInclude,
            }]),
            circuit_breaker: None,
        };
        let scored = vec![make_scored("venue:C", VenueType::Sdex, 0.0)];
        let (excluded, _) = policy.apply(&scored);
        assert!(
            !excluded.contains("venue:C"),
            "force_include should prevent exclusion even at score 0.0"
        );
    }

    #[test]
    fn force_exclude_overrides_high_score() {
        // venue with score 1.0 but force_exclude should be excluded with Override reason
        let policy = ExclusionPolicy {
            thresholds: ExclusionThresholds {
                sdex: 0.5,
                amm: 0.5,
            },
            overrides: OverrideRegistry::from_entries(vec![OverrideEntry {
                venue_ref: "venue:D".to_string(),
                directive: OverrideDirective::ForceExclude,
            }]),
            circuit_breaker: None,
        };
        let scored = vec![make_scored("venue:D", VenueType::Sdex, 1.0)];
        let (excluded, diagnostics) = policy.apply(&scored);
        assert!(
            excluded.contains("venue:D"),
            "force_exclude should exclude even at score 1.0"
        );
        assert_eq!(diagnostics.excluded_venues.len(), 1);
        assert!(
            matches!(
                diagnostics.excluded_venues[0].reason,
                ExclusionReason::Override
            ),
            "reason should be Override"
        );
    }

    #[test]
    fn unrecognized_override_key_no_error() {
        // override entry for unknown venue_ref should not panic or error
        let policy = ExclusionPolicy {
            thresholds: ExclusionThresholds {
                sdex: 0.5,
                amm: 0.5,
            },
            overrides: OverrideRegistry::from_entries(vec![OverrideEntry {
                venue_ref: "venue:UNKNOWN".to_string(),
                directive: OverrideDirective::ForceExclude,
            }]),
            circuit_breaker: None,
        };
        // scored list does not contain "venue:UNKNOWN"
        let scored = vec![make_scored("venue:E", VenueType::Sdex, 0.8)];
        // Should not panic; the unrecognized key just triggers a warn log
        let (excluded, _) = policy.apply(&scored);
        assert!(
            !excluded.contains("venue:E"),
            "venue:E should not be excluded"
        );
        assert!(
            !excluded.contains("venue:UNKNOWN"),
            "unknown override key should not cause exclusion of absent venue"
        );
    }
}
