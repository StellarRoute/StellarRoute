use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for liquidity anomaly detection.
///
/// Supports both global defaults and optional per-pool overrides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyConfig {
    /// Global default: threshold for AMM reserve delta spikes/drains (e.g. 0.5 means 50%).
    pub reserve_delta_threshold: f64,
    /// Global default: threshold for SDEX depth collapse (e.g. 0.8 means 80% decrease).
    pub depth_collapse_threshold: f64,
    /// Global default: minimum anomaly score to flag as suspicious.
    pub alert_threshold: f64,

    /// Optional per-pool overrides (keyed by venue_ref).
    #[serde(default)]
    pub per_pool: HashMap<String, AnomalyPoolThresholds>,

    /// How to treat missing/unknown reserve snapshot age.
    /// If true, missing reserve timestamps are treated as stale-read anomalies.
    #[serde(default)]
    pub stale_read_is_anomaly: bool,

    /// Global staleness threshold for AMM reserves. If `reserve_updated_at_secs_ago > max`, treat as stale.
    #[serde(default = "default_max_reserve_age_secs")]
    pub max_reserve_age_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyPoolThresholds {
    #[serde(default)]
    pub reserve_delta_threshold: Option<f64>,
    #[serde(default)]
    pub alert_threshold: Option<f64>,
}

fn default_max_reserve_age_secs() -> u64 {
    120
}

impl Default for AnomalyConfig {
    fn default() -> Self {
        Self {
            reserve_delta_threshold: 0.5,
            depth_collapse_threshold: 0.8,
            alert_threshold: 0.7,
            per_pool: HashMap::new(),
            stale_read_is_anomaly: true,
            max_reserve_age_secs: default_max_reserve_age_secs(),
        }
    }
}

/// Historical state for a venue to detect shifts
#[derive(Debug, Clone)]
pub struct VenueHistory {
    pub venue_ref: String,
    pub last_reserves: Option<(i128, i128)>,
    pub last_depth: Option<i128>,
    pub last_updated_at: DateTime<Utc>,
}

/// Result of anomaly detection for a specific venue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyResult {
    pub venue_ref: String,
    pub score: f64,
    pub reasons: Vec<String>,
}

/// Detector for identifying liquidity anomalies
pub struct LiquidityAnomalyDetector {
    config: AnomalyConfig,
    history: HashMap<String, VenueHistory>,
}

impl LiquidityAnomalyDetector {
    pub fn new(config: AnomalyConfig) -> Self {
        Self {
            config,
            history: HashMap::new(),
        }
    }

    fn thresholds_for(&self, venue_ref: &str) -> (f64, f64) {
        // returns (reserve_delta_threshold, alert_threshold)
        if let Some(pool) = self.config.per_pool.get(venue_ref) {
            let reserve = pool
                .reserve_delta_threshold
                .unwrap_or(self.config.reserve_delta_threshold);
            let alert = pool.alert_threshold.unwrap_or(self.config.alert_threshold);
            (reserve, alert)
        } else {
            (
                self.config.reserve_delta_threshold,
                self.config.alert_threshold,
            )
        }
    }

    /// Update the detector with new venue state and return any detected anomalies.
    ///
    /// `reserve_updated_at` is used to detect stale-read behavior.
    pub fn update_and_detect(
        &mut self,
        venue_ref: &str,
        current_reserves: Option<(i128, i128)>,
        current_depth: Option<i128>,
        reserve_updated_at: Option<DateTime<Utc>>,
    ) -> AnomalyResult {
        let now = Utc::now();
        let (reserve_delta_threshold, _) = self.thresholds_for(venue_ref);
        let history = self
            .history
            .entry(venue_ref.to_string())
            .or_insert_with(|| VenueHistory {
                venue_ref: venue_ref.to_string(),
                last_reserves: current_reserves,
                last_depth: current_depth,
                last_updated_at: now,
            });

        let mut score: f64 = 0.0;
        let mut reasons = Vec::new();

        // 0. Stale-read detection for AMM reserves
        // - If we can't determine age, optionally treat as anomaly.
        // - If we exceed max_reserve_age_secs, treat as anomaly.
        if self.config.stale_read_is_anomaly {
            match reserve_updated_at {
                None => {
                    score = score.max(0.9);
                    reasons.push("Stale read: missing reserve_updated_at".to_string());
                }
                Some(ts) => {
                    let age_secs = (now - ts).num_seconds().max(0) as u64;
                    if age_secs > self.config.max_reserve_age_secs {
                        score = score.max(0.9);
                        reasons.push(format!(
                            "Stale read: reserve_updated_at age {}s > {}s",
                            age_secs, self.config.max_reserve_age_secs
                        ));
                    }
                }
            }
        }

        // 1. Detect AMM reserve delta spikes/drains
        if let (Some((last_a, last_b)), Some((curr_a, curr_b))) =
            (history.last_reserves, current_reserves)
        {
            if last_a > 0 && last_b > 0 {
                // reserve delta as relative change per leg
                let delta_a = (curr_a as f64 - last_a as f64) / last_a as f64;
                let delta_b = (curr_b as f64 - last_b as f64) / last_b as f64;

                let abs_max_delta = delta_a.abs().max(delta_b.abs());
                if abs_max_delta > reserve_delta_threshold {
                    let dir = if delta_a < 0.0 && delta_b < 0.0 {
                        "drain"
                    } else {
                        "spike"
                    };
                    score = score.max((abs_max_delta / reserve_delta_threshold).min(1.0) * 0.9);
                    reasons.push(format!(
                        "AMM reserve {}: {:.1}% (threshold {:.1}%)",
                        dir,
                        abs_max_delta * 100.0,
                        reserve_delta_threshold * 100.0
                    ));
                }
            }
        }

        // 2. Detect SDEX depth collapse
        if let (Some(last_d), Some(curr_d)) = (history.last_depth, current_depth) {
            if last_d > 0 {
                let collapse = (last_d as f64 - curr_d as f64) / last_d as f64;
                if collapse > self.config.depth_collapse_threshold {
                    score =
                        score.max((collapse / self.config.depth_collapse_threshold).min(1.0) * 0.9);
                    reasons.push(format!(
                        "Significant depth collapse: {:.1}%",
                        collapse * 100.0
                    ));
                }
            }
        }

        // Update history
        history.last_reserves = current_reserves;
        history.last_depth = current_depth;
        history.last_updated_at = now;

        let score = score.clamp(0.0, 1.0);
        // If no specific reasons were triggered, ensure score doesn't accidentally carry over.
        // (We already start from 0.0, so this is mostly defensive.)
        let reasons = reasons;
        if reasons.is_empty() {
            // Keep behavior: score should remain 0.0 if nothing is detected.
            // This also aligns with existing unit tests.
            // Note: stale-read might have set score and reasons.
        }

        AnomalyResult {
            venue_ref: venue_ref.to_string(),
            score: score.min(1.0),
            reasons,
        }
    }

    pub fn is_anomalous(&self, result: &AnomalyResult) -> bool {
        let (_, alert_threshold) = self.thresholds_for(&result.venue_ref);
        result.score >= alert_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reserve_delta_spike_detection() {
        let config = AnomalyConfig {
            reserve_delta_threshold: 0.5,
            alert_threshold: 0.7,
            stale_read_is_anomaly: false,
            ..Default::default()
        };
        let mut detector = LiquidityAnomalyDetector::new(config.clone());

        // Initial update
        let _ = detector.update_and_detect("amm:1", Some((1000, 1000)), None, Some(Utc::now()));

        // Moderate delta (40%) - should not trigger
        let res = detector.update_and_detect("amm:1", Some((1400, 1400)), None, Some(Utc::now()));
        assert!(res.score < 0.7);
        assert!(res.reasons.is_empty());

        // Large delta (60%) - should trigger
        let res = detector.update_and_detect("amm:1", Some((400, 400)), None, Some(Utc::now()));
        assert!(res.score > 0.5);
        assert!(!res.reasons.is_empty());
        assert!(res.reasons[0].to_lowercase().contains("amm reserve"));
        assert!(
            res.reasons[0].to_lowercase().contains("spike")
                || res.reasons[0].to_lowercase().contains("drain")
        );
    }

    #[test]
    fn test_reserve_delta_drain_detection() {
        let config = AnomalyConfig {
            reserve_delta_threshold: 0.5,
            alert_threshold: 0.7,
            stale_read_is_anomaly: false,
            ..Default::default()
        };
        let mut detector = LiquidityAnomalyDetector::new(config.clone());

        // Initial update
        let _ = detector.update_and_detect("amm:drain", Some((1000, 1000)), None, Some(Utc::now()));

        // Large negative delta (drain 60%) - should trigger
        let res = detector.update_and_detect("amm:drain", Some((400, 400)), None, Some(Utc::now()));
        assert!(res.score >= config.alert_threshold);
        assert!(!res.reasons.is_empty());
        assert!(res.reasons[0].to_lowercase().contains("drain"));
    }

    #[test]
    fn test_stale_read_detection_missing_timestamp() {
        let config = AnomalyConfig {
            stale_read_is_anomaly: true,
            max_reserve_age_secs: 10,
            alert_threshold: 0.7,
            ..Default::default()
        };
        let mut detector = LiquidityAnomalyDetector::new(config.clone());

        // Initial update
        let _ = detector.update_and_detect(
            "amm:stale_missing",
            Some((1000, 1000)),
            None,
            Some(Utc::now()),
        );

        // Missing reserve_updated_at should yield anomaly
        let res = detector.update_and_detect("amm:stale_missing", Some((900, 900)), None, None);
        assert!(res.score >= config.alert_threshold);
        assert!(res.reasons.iter().any(|r| r.contains("Stale read")));
    }

    #[test]
    fn test_depth_collapse_detection() {
        let config = AnomalyConfig {
            depth_collapse_threshold: 0.8,
            alert_threshold: 0.7,
            stale_read_is_anomaly: false,
            ..Default::default()
        };
        let mut detector = LiquidityAnomalyDetector::new(config.clone());

        // Initial update
        let _ = detector.update_and_detect("sdex:1", None, Some(1000), Some(Utc::now()));

        // 90% collapse
        let res = detector.update_and_detect("sdex:1", None, Some(100), Some(Utc::now()));
        assert!(res.score > 0.8);
        assert!(!res.reasons.is_empty());
        assert!(res.reasons[0].to_lowercase().contains("depth collapse"));
    }
}
