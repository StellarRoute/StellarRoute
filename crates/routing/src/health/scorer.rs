use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::health::policy::{ExclusionThresholds, OverrideEntry};

// ---------------------------------------------------------------------------
// VenueType
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VenueType {
    Sdex,
    Amm,
}

// ---------------------------------------------------------------------------
// VenueScorerInput
// ---------------------------------------------------------------------------

pub struct VenueScorerInput {
    pub venue_ref: String,
    pub venue_type: VenueType,
    // SDEX signals
    pub best_bid_e7: Option<i128>,
    pub best_ask_e7: Option<i128>,
    pub depth_top_n_e7: Option<i128>,
    // AMM signals
    pub reserve_a_e7: Option<i128>,
    pub reserve_b_e7: Option<i128>,
    pub tvl_e7: Option<i128>,
    // Shared
    pub last_updated_at: Option<DateTime<Utc>>,
}

// ---------------------------------------------------------------------------
// HealthRecord
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct HealthRecord {
    pub venue_ref: String,
    pub venue_type: VenueType,
    /// Score in [0.0, 1.0]
    pub score: f64,
    /// JSONB-compatible signal map
    pub signals: serde_json::Value,
    pub computed_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// ScoredVenue
// ---------------------------------------------------------------------------

pub struct ScoredVenue {
    pub venue_ref: String,
    pub venue_type: VenueType,
    pub record: HealthRecord,
}

// ---------------------------------------------------------------------------
// Scorer structs
// ---------------------------------------------------------------------------

pub struct SdexScorer {
    pub staleness_threshold_secs: u64,
    /// Maximum spread before score bottoms out (default 0.05 = 5%)
    pub max_spread: f64,
    /// Target depth in e7 units (default 10_000_000_000 = 1000 units)
    pub target_depth_e7: i128,
    /// Number of depth levels (default 5)
    pub depth_levels: usize,
}

pub struct AmmScorer {
    pub staleness_threshold_secs: u64,
    pub min_tvl_threshold_e7: i128,
}

pub struct HealthScorer {
    pub sdex: SdexScorer,
    pub amm: AmmScorer,
}

impl HealthScorer {
    pub fn score_venues(&self, inputs: &[VenueScorerInput]) -> Vec<ScoredVenue> {
        inputs
            .iter()
            .map(|input| {
                let record = match input.venue_type {
                    VenueType::Sdex => self.sdex.score(input),
                    VenueType::Amm => self.amm.score(input),
                };
                ScoredVenue {
                    venue_ref: input.venue_ref.clone(),
                    venue_type: input.venue_type.clone(),
                    record,
                }
            })
            .collect()
    }
}

pub trait VenueScorer: Send + Sync {
    fn score(&self, input: &VenueScorerInput) -> HealthRecord;
}

impl VenueScorer for SdexScorer {
    fn score(&self, input: &VenueScorerInput) -> HealthRecord {
        let now = Utc::now();
        let staleness_secs = input
            .last_updated_at
            .map(|last_updated_at| (now - last_updated_at).num_seconds().max(0) as u64)
            .unwrap_or(u64::MAX);

        // Stale or missing bids/asks → 0.0
        if staleness_secs > self.staleness_threshold_secs
            || input.best_bid_e7.is_none()
            || input.best_ask_e7.is_none()
        {
            return HealthRecord {
                venue_ref: input.venue_ref.clone(),
                venue_type: VenueType::Sdex,
                score: 0.0,
                signals: serde_json::json!({
                    "spread_ratio": null,
                    "depth_top_n_e7": input.depth_top_n_e7,
                    "staleness_secs": staleness_secs,
                }),
                computed_at: now,
            };
        }

        let bid = input.best_bid_e7.unwrap() as f64;
        let ask = input.best_ask_e7.unwrap() as f64;
        let mid = (bid + ask) / 2.0;

        let spread_ratio = if mid > 0.0 {
            (ask - bid) / mid
        } else {
            f64::INFINITY
        };
        let spread_score = (1.0 - spread_ratio / self.max_spread).clamp(0.0, 1.0);

        let depth = input.depth_top_n_e7.unwrap_or(0) as f64;
        let depth_score = (depth / self.target_depth_e7 as f64).clamp(0.0, 1.0);

        let staleness_score: f64 =
            1.0_f64 - (staleness_secs as f64 / self.staleness_threshold_secs as f64);
        let staleness_score = staleness_score.clamp(0.0, 1.0);

        let score =
            (0.4 * spread_score + 0.4 * depth_score + 0.2 * staleness_score).clamp(0.0, 1.0);

        HealthRecord {
            venue_ref: input.venue_ref.clone(),
            venue_type: VenueType::Sdex,
            score,
            signals: serde_json::json!({
                "spread_ratio": spread_ratio,
                "depth_top_n_e7": input.depth_top_n_e7,
                "staleness_secs": staleness_secs,
            }),
            computed_at: now,
        }
    }
}

impl VenueScorer for AmmScorer {
    fn score(&self, input: &VenueScorerInput) -> HealthRecord {
        let now = Utc::now();
        let staleness_secs = input
            .last_updated_at
            .map(|last_updated_at| (now - last_updated_at).num_seconds().max(0) as u64)
            .unwrap_or(u64::MAX);

        let reserve_a = input.reserve_a_e7.unwrap_or(0);
        let reserve_b = input.reserve_b_e7.unwrap_or(0);

        // Stale or zero reserve → 0.0
        if staleness_secs > self.staleness_threshold_secs || reserve_a == 0 || reserve_b == 0 {
            return HealthRecord {
                venue_ref: input.venue_ref.clone(),
                venue_type: VenueType::Amm,
                score: 0.0,
                signals: serde_json::json!({
                    "reserve_ratio_dev": null,
                    "tvl_e7": input.tvl_e7,
                    "staleness_secs": staleness_secs,
                }),
                computed_at: now,
            };
        }

        // Invariant score: deviation from constant-product (treat as 0 on first observation)
        let invariant_score = 1.0_f64.clamp(0.0, 1.0); // first observation → no deviation

        let tvl = input.tvl_e7.unwrap_or(0) as f64;
        let tvl_score = (tvl / self.min_tvl_threshold_e7 as f64).clamp(0.0, 1.0);

        let staleness_score: f64 =
            1.0_f64 - (staleness_secs as f64 / self.staleness_threshold_secs as f64);
        let staleness_score = staleness_score.clamp(0.0, 1.0);

        let score =
            (0.4 * invariant_score + 0.4 * tvl_score + 0.2 * staleness_score).clamp(0.0, 1.0);

        if !(0.0..=1.0).contains(&score) {
            tracing::debug!(score, "AMM score out of bounds, clamping");
        }

        HealthRecord {
            venue_ref: input.venue_ref.clone(),
            venue_type: VenueType::Amm,
            score,
            signals: serde_json::json!({
                "reserve_ratio_dev": 0.0,
                "tvl_e7": input.tvl_e7,
                "staleness_secs": staleness_secs,
            }),
            computed_at: now,
        }
    }
}

// ---------------------------------------------------------------------------
// FreshnessThresholds
// ---------------------------------------------------------------------------

fn default_sdex_freshness_secs() -> u64 {
    30
}

fn default_amm_freshness_secs() -> u64 {
    60
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct FreshnessThresholds {
    #[serde(default = "default_sdex_freshness_secs")]
    pub sdex: u64,
    #[serde(default = "default_amm_freshness_secs")]
    pub amm: u64,
}

impl Default for FreshnessThresholds {
    fn default() -> Self {
        Self {
            sdex: default_sdex_freshness_secs(),
            amm: default_amm_freshness_secs(),
        }
    }
}

impl FreshnessThresholds {
    /// Validates that both thresholds are positive (> 0).
    /// Returns an error string identifying the invalid field(s).
    pub fn validate(&self) -> Result<(), String> {
        if self.sdex == 0 {
            return Err(
                "freshness_threshold_secs.sdex must be a positive integer greater than zero"
                    .to_string(),
            );
        }
        if self.amm == 0 {
            return Err(
                "freshness_threshold_secs.amm must be a positive integer greater than zero"
                    .to_string(),
            );
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// HealthScoringConfig
// ---------------------------------------------------------------------------

fn default_staleness_secs() -> u64 {
    60
}
fn default_min_tvl() -> i128 {
    1_000_000_000
}
fn default_depth_levels() -> usize {
    5
}

#[derive(Debug, Clone, Deserialize)]
pub struct HealthScoringConfig {
    #[serde(default)]
    pub thresholds: ExclusionThresholds,
    #[serde(default)]
    pub overrides: Vec<OverrideEntry>,
    #[serde(default = "default_staleness_secs")]
    pub staleness_threshold_secs: u64,
    #[serde(default = "default_min_tvl")]
    pub min_tvl_threshold_e7: i128,
    #[serde(default = "default_depth_levels")]
    pub depth_levels: usize,
    #[serde(default)]
    pub freshness_threshold_secs: FreshnessThresholds,
    #[serde(default)]
    pub anomaly: crate::health::anomaly::AnomalyConfig,
}

impl Default for HealthScoringConfig {
    fn default() -> Self {
        Self {
            thresholds: ExclusionThresholds::default(),
            overrides: Vec::new(),
            staleness_threshold_secs: default_staleness_secs(),
            min_tvl_threshold_e7: default_min_tvl(),
            depth_levels: default_depth_levels(),
            freshness_threshold_secs: FreshnessThresholds::default(),
            anomaly: crate::health::anomaly::AnomalyConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sdex_scorer() -> SdexScorer {
        SdexScorer {
            staleness_threshold_secs: 60,
            max_spread: 0.05,
            target_depth_e7: 10_000_000_000,
            depth_levels: 5,
        }
    }

    fn amm_scorer() -> AmmScorer {
        AmmScorer {
            staleness_threshold_secs: 60,
            min_tvl_threshold_e7: 1_000_000_000,
        }
    }

    fn fresh_ts() -> Option<DateTime<Utc>> {
        Some(Utc::now())
    }

    fn stale_ts() -> Option<DateTime<Utc>> {
        Some(Utc::now() - chrono::Duration::seconds(120))
    }

    // --- SdexScorer tests ---

    #[test]
    fn sdex_zero_bid_returns_zero() {
        let scorer = sdex_scorer();
        let input = VenueScorerInput {
            venue_ref: "sdex:XLM/USDC".to_string(),
            venue_type: VenueType::Sdex,
            best_bid_e7: None,
            best_ask_e7: Some(10_000_000),
            depth_top_n_e7: Some(5_000_000_000),
            reserve_a_e7: None,
            reserve_b_e7: None,
            tvl_e7: None,
            last_updated_at: fresh_ts(),
        };
        let record = scorer.score(&input);
        assert_eq!(record.score, 0.0, "zero bid should produce score 0.0");
    }

    #[test]
    fn sdex_zero_ask_returns_zero() {
        let scorer = sdex_scorer();
        let input = VenueScorerInput {
            venue_ref: "sdex:XLM/USDC".to_string(),
            venue_type: VenueType::Sdex,
            best_bid_e7: Some(9_900_000),
            best_ask_e7: None,
            depth_top_n_e7: Some(5_000_000_000),
            reserve_a_e7: None,
            reserve_b_e7: None,
            tvl_e7: None,
            last_updated_at: fresh_ts(),
        };
        let record = scorer.score(&input);
        assert_eq!(record.score, 0.0, "zero ask should produce score 0.0");
    }

    #[test]
    fn sdex_stale_timestamp_returns_zero() {
        let scorer = sdex_scorer();
        let input = VenueScorerInput {
            venue_ref: "sdex:XLM/USDC".to_string(),
            venue_type: VenueType::Sdex,
            best_bid_e7: Some(9_900_000),
            best_ask_e7: Some(10_000_000),
            depth_top_n_e7: Some(5_000_000_000),
            reserve_a_e7: None,
            reserve_b_e7: None,
            tvl_e7: None,
            last_updated_at: stale_ts(),
        };
        let record = scorer.score(&input);
        assert_eq!(
            record.score, 0.0,
            "stale timestamp should produce score 0.0"
        );
    }

    #[test]
    fn sdex_healthy_input_returns_positive_score() {
        let scorer = sdex_scorer();
        let input = VenueScorerInput {
            venue_ref: "sdex:XLM/USDC".to_string(),
            venue_type: VenueType::Sdex,
            best_bid_e7: Some(9_990_000),
            best_ask_e7: Some(10_010_000),
            depth_top_n_e7: Some(10_000_000_000),
            reserve_a_e7: None,
            reserve_b_e7: None,
            tvl_e7: None,
            last_updated_at: fresh_ts(),
        };
        let record = scorer.score(&input);
        assert!(
            record.score > 0.0,
            "healthy input should produce positive score"
        );
        assert!(record.score <= 1.0, "score must not exceed 1.0");
    }

    // --- AmmScorer tests ---

    #[test]
    fn amm_zero_reserve_a_returns_zero() {
        let scorer = amm_scorer();
        let input = VenueScorerInput {
            venue_ref: "amm:XLM/USDC".to_string(),
            venue_type: VenueType::Amm,
            best_bid_e7: None,
            best_ask_e7: None,
            depth_top_n_e7: None,
            reserve_a_e7: Some(0),
            reserve_b_e7: Some(1_000_000_000),
            tvl_e7: Some(2_000_000_000),
            last_updated_at: fresh_ts(),
        };
        let record = scorer.score(&input);
        assert_eq!(record.score, 0.0, "zero reserve_a should produce score 0.0");
    }

    #[test]
    fn amm_zero_reserve_b_returns_zero() {
        let scorer = amm_scorer();
        let input = VenueScorerInput {
            venue_ref: "amm:XLM/USDC".to_string(),
            venue_type: VenueType::Amm,
            best_bid_e7: None,
            best_ask_e7: None,
            depth_top_n_e7: None,
            reserve_a_e7: Some(1_000_000_000),
            reserve_b_e7: Some(0),
            tvl_e7: Some(2_000_000_000),
            last_updated_at: fresh_ts(),
        };
        let record = scorer.score(&input);
        assert_eq!(record.score, 0.0, "zero reserve_b should produce score 0.0");
    }

    #[test]
    fn amm_stale_timestamp_returns_zero() {
        let scorer = amm_scorer();
        let input = VenueScorerInput {
            venue_ref: "amm:XLM/USDC".to_string(),
            venue_type: VenueType::Amm,
            best_bid_e7: None,
            best_ask_e7: None,
            depth_top_n_e7: None,
            reserve_a_e7: Some(1_000_000_000),
            reserve_b_e7: Some(1_000_000_000),
            tvl_e7: Some(2_000_000_000),
            last_updated_at: stale_ts(),
        };
        let record = scorer.score(&input);
        assert_eq!(
            record.score, 0.0,
            "stale timestamp should produce score 0.0"
        );
    }

    #[test]
    fn amm_tvl_below_threshold_reduces_score() {
        let scorer = amm_scorer();
        // tvl_e7 = 100_000_000 which is 10% of min_tvl_threshold_e7 (1_000_000_000)
        let input = VenueScorerInput {
            venue_ref: "amm:XLM/USDC".to_string(),
            venue_type: VenueType::Amm,
            best_bid_e7: None,
            best_ask_e7: None,
            depth_top_n_e7: None,
            reserve_a_e7: Some(1_000_000_000),
            reserve_b_e7: Some(1_000_000_000),
            tvl_e7: Some(100_000_000),
            last_updated_at: fresh_ts(),
        };
        let record = scorer.score(&input);
        // tvl_score = 0.1, invariant_score = 1.0, staleness_score ≈ 1.0
        // score ≈ 0.4*1.0 + 0.4*0.1 + 0.2*1.0 = 0.64
        assert!(
            record.score > 0.0,
            "low TVL should still produce non-zero score"
        );
        assert!(record.score < 1.0, "low TVL should reduce score below 1.0");
        // Verify it's less than a full-TVL score
        let full_tvl_input = VenueScorerInput {
            venue_ref: "amm:XLM/USDC".to_string(),
            venue_type: VenueType::Amm,
            best_bid_e7: None,
            best_ask_e7: None,
            depth_top_n_e7: None,
            reserve_a_e7: Some(1_000_000_000),
            reserve_b_e7: Some(1_000_000_000),
            tvl_e7: Some(1_000_000_000),
            last_updated_at: fresh_ts(),
        };
        let full_record = scorer.score(&full_tvl_input);
        assert!(
            record.score < full_record.score,
            "low TVL score should be less than full TVL score"
        );
    }

    #[test]
    fn amm_healthy_input_returns_positive_score() {
        let scorer = amm_scorer();
        let input = VenueScorerInput {
            venue_ref: "amm:XLM/USDC".to_string(),
            venue_type: VenueType::Amm,
            best_bid_e7: None,
            best_ask_e7: None,
            depth_top_n_e7: None,
            reserve_a_e7: Some(1_000_000_000),
            reserve_b_e7: Some(1_000_000_000),
            tvl_e7: Some(2_000_000_000),
            last_updated_at: fresh_ts(),
        };
        let record = scorer.score(&input);
        assert!(
            record.score > 0.0,
            "healthy AMM input should produce positive score"
        );
        assert!(record.score <= 1.0, "score must not exceed 1.0");
    }

    // --- HealthRecord serialization tests ---

    #[test]
    fn health_record_round_trip() {
        let record = HealthRecord {
            venue_ref: "sdex:XLM/USDC".to_string(),
            venue_type: VenueType::Sdex,
            score: 0.75,
            signals: serde_json::json!({
                "spread_ratio": 0.002,
                "depth_top_n_e7": 5_000_000_000i64,
                "staleness_secs": 10
            }),
            computed_at: chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        };
        let json = serde_json::to_string(&record).expect("serialize");
        let deserialized: HealthRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(record, deserialized);
    }

    #[test]
    fn health_record_unknown_fields_ignored() {
        let json = r#"{
            "venue_ref": "amm:XLM/USDC",
            "venue_type": "amm",
            "score": 0.9,
            "signals": {},
            "computed_at": "2024-01-01T00:00:00Z",
            "unknown_extra_field": "should be ignored"
        }"#;
        let result: Result<HealthRecord, _> = serde_json::from_str(json);
        assert!(
            result.is_ok(),
            "unknown fields should be ignored during deserialization"
        );
    }

    // --- FreshnessThresholds tests (Requirements 5.2, 5.5) ---

    #[test]
    fn freshness_thresholds_defaults_when_absent() {
        // When freshness_threshold_secs is absent from config, defaults apply
        let config: HealthScoringConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(
            config.freshness_threshold_secs.sdex, 30,
            "default SDEX freshness threshold should be 30s"
        );
        assert_eq!(
            config.freshness_threshold_secs.amm, 60,
            "default AMM freshness threshold should be 60s"
        );
    }

    #[test]
    fn freshness_thresholds_partial_defaults() {
        // Only sdex specified — amm should default to 60
        let config: HealthScoringConfig =
            serde_json::from_str(r#"{"freshness_threshold_secs": {"sdex": 15}}"#).unwrap();
        assert_eq!(config.freshness_threshold_secs.sdex, 15);
        assert_eq!(config.freshness_threshold_secs.amm, 60);
    }

    #[test]
    fn freshness_thresholds_explicit_values() {
        let config: HealthScoringConfig =
            serde_json::from_str(r#"{"freshness_threshold_secs": {"sdex": 10, "amm": 20}}"#)
                .unwrap();
        assert_eq!(config.freshness_threshold_secs.sdex, 10);
        assert_eq!(config.freshness_threshold_secs.amm, 20);
    }

    #[test]
    fn freshness_thresholds_zero_sdex_fails_validation() {
        let thresholds = FreshnessThresholds { sdex: 0, amm: 60 };
        let err = thresholds.validate().unwrap_err();
        assert!(
            err.contains("sdex"),
            "error message should identify the sdex field: {err}"
        );
    }

    #[test]
    fn freshness_thresholds_zero_amm_fails_validation() {
        let thresholds = FreshnessThresholds { sdex: 30, amm: 0 };
        let err = thresholds.validate().unwrap_err();
        assert!(
            err.contains("amm"),
            "error message should identify the amm field: {err}"
        );
    }

    #[test]
    fn freshness_thresholds_positive_values_pass_validation() {
        let thresholds = FreshnessThresholds { sdex: 1, amm: 1 };
        assert!(thresholds.validate().is_ok());
    }

    #[test]
    fn health_scoring_config_default_populates_freshness() {
        let config = HealthScoringConfig::default();
        assert_eq!(config.freshness_threshold_secs.sdex, 30);
        assert_eq!(config.freshness_threshold_secs.amm, 60);
    }
}
