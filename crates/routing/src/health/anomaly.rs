use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for liquidity anomaly detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyConfig {
    /// Threshold for AMM reserve shifts (e.g., 0.5 means 50% change)
    pub reserve_shift_threshold: f64,
    /// Threshold for SDEX depth collapse (e.g., 0.8 means 80% decrease)
    pub depth_collapse_threshold: f64,
    /// Minimum anomaly score to flag as suspicious
    pub alert_threshold: f64,
}

impl Default for AnomalyConfig {
    fn default() -> Self {
        Self {
            reserve_shift_threshold: 0.5,
            depth_collapse_threshold: 0.8,
            alert_threshold: 0.7,
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

    /// Update the detector with new venue state and return any detected anomalies
    pub fn update_and_detect(
        &mut self,
        venue_ref: &str,
        current_reserves: Option<(i128, i128)>,
        current_depth: Option<i128>,
    ) -> AnomalyResult {
        let now = Utc::now();
        let history = self
            .history
            .entry(venue_ref.to_string())
            .or_insert_with(|| VenueHistory {
                venue_ref: venue_ref.to_string(),
                last_reserves: current_reserves,
                last_depth: current_depth,
                last_updated_at: now,
            });

        let mut score = 0.0;
        let mut reasons = Vec::new();

        // 1. Detect AMM reserve shifts
        if let (Some((last_a, last_b)), Some((curr_a, curr_b))) =
            (history.last_reserves, current_reserves)
        {
            if last_a > 0 && last_b > 0 {
                let shift_a = (curr_a as f64 - last_a as f64).abs() / last_a as f64;
                let shift_b = (curr_b as f64 - last_b as f64).abs() / last_b as f64;
                let max_shift = shift_a.max(shift_b);

                if max_shift > self.config.reserve_shift_threshold {
                    score += (max_shift / self.config.reserve_shift_threshold).min(1.0) * 0.8;
                    reasons.push(format!("Sudden reserve shift: {:.1}%", max_shift * 100.0));
                }
            }
        }

        // 2. Detect SDEX depth collapse
        if let (Some(last_d), Some(curr_d)) = (history.last_depth, current_depth) {
            if last_d > 0 {
                let collapse = (last_d as f64 - curr_d as f64) / last_d as f64;
                if collapse > self.config.depth_collapse_threshold {
                    score += (collapse / self.config.depth_collapse_threshold).min(1.0) * 0.9;
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

        AnomalyResult {
            venue_ref: venue_ref.to_string(),
            score: score.clamp(0.0, 1.0),
            reasons,
        }
    }

    pub fn is_anomalous(&self, result: &AnomalyResult) -> bool {
        result.score >= self.config.alert_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reserve_shift_detection() {
        let config = AnomalyConfig {
            reserve_shift_threshold: 0.5,
            ..Default::default()
        };
        let mut detector = LiquidityAnomalyDetector::new(config);

        // Initial update
        let _ = detector.update_and_detect("amm:1", Some((1000, 1000)), None);

        // Moderate shift (40%) - should not trigger
        let res = detector.update_and_detect("amm:1", Some((1400, 1400)), None);
        assert!(res.score < 0.7);
        assert!(res.reasons.is_empty());

        // Large shift (60%) - should trigger
        let res = detector.update_and_detect("amm:1", Some((560, 560)), None);
        assert!(res.score > 0.5);
        assert!(!res.reasons.is_empty());
        assert!(res.reasons[0].contains("reserve shift"));
    }

    #[test]
    fn test_depth_collapse_detection() {
        let config = AnomalyConfig {
            depth_collapse_threshold: 0.8,
            ..Default::default()
        };
        let mut detector = LiquidityAnomalyDetector::new(config);

        // Initial update
        let _ = detector.update_and_detect("sdex:1", None, Some(1000));

        // 90% collapse
        let res = detector.update_and_detect("sdex:1", None, Some(100));
        assert!(res.score > 0.8);
        assert!(!res.reasons.is_empty());
        assert!(res.reasons[0].contains("depth collapse"));
    }
}
