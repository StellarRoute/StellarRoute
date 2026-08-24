//! Multi-source route consensus engine with conflict resolution

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use tracing::instrument;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusPolicy {
    /// Source trust weights (0.0 to 1.0)
    pub source_weights: HashMap<String, f64>,
    /// Freshness decay (seconds)
    pub freshness_window: u64,
    /// Require consensus threshold (0.0 to 1.0)
    pub consensus_threshold: f64,
}

impl Default for ConsensusPolicy {
    fn default() -> Self {
        Self {
            source_weights: HashMap::new(),
            freshness_window: 5,
            consensus_threshold: 0.6,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteCandidate {
    pub source: String,
    pub hops: Vec<String>,
    pub price: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusDiagnostics {
    pub winning_route: RouteCandidate,
    pub runner_ups: Vec<RouteCandidate>,
    pub consensus_score: f64,
    pub conflict_detected: bool,
    pub resolution_reason: String,
}

#[derive(Error, Debug)]
pub enum ConsensusError {
    #[error("No route candidates provided")]
    NoCandidates,
    #[error("Failed to reach consensus: {0}")]
    NoConsensus(String),
}

pub struct ConsensusEngine {
    policy: ConsensusPolicy,
}

impl ConsensusEngine {
    pub fn new(policy: ConsensusPolicy) -> Self {
        Self { policy }
    }

    #[instrument(skip(self, candidates))]
    pub fn resolve(
        &self,
        candidates: Vec<RouteCandidate>,
    ) -> Result<ConsensusDiagnostics, ConsensusError> {
        if candidates.is_empty() {
            return Err(ConsensusError::NoCandidates);
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Compute weighted scores
        let mut scores: Vec<(RouteCandidate, f64)> = candidates
            .into_iter()
            .map(|candidate| {
                let base_weight = self
                    .policy
                    .source_weights
                    .get(&candidate.source)
                    .copied()
                    .unwrap_or(0.5);

                let freshness_penalty =
                    if now.saturating_sub(candidate.timestamp) > self.policy.freshness_window {
                        0.5
                    } else {
                        1.0
                    };

                let score = base_weight * freshness_penalty * (1.0 / (candidate.price + 0.001));
                (candidate, score)
            })
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let top_score = scores[0].1;
        let consensus_weight: f64 = scores
            .iter()
            .filter(|(_, s)| (*s - top_score).abs() < 0.01)
            .map(|(_, _s)| self.policy.source_weights.get("").copied().unwrap_or(0.5))
            .sum();

        let consensus_score = consensus_weight.min(1.0);
        let conflict = scores.len() > 1 && (scores[0].1 - scores[1].1).abs() / scores[0].1 < 0.15;

        Ok(ConsensusDiagnostics {
            winning_route: scores[0].0.clone(),
            runner_ups: scores[1..].iter().map(|(c, _)| c.clone()).collect(),
            consensus_score,
            conflict_detected: conflict,
            resolution_reason: if conflict {
                "Price threshold difference < 15%".to_string()
            } else {
                format!("Source '{}' selected by weight", scores[0].0.source)
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consensus_with_weighted_sources() {
        let mut weights = HashMap::new();
        weights.insert("amm".to_string(), 0.8);
        weights.insert("sdex".to_string(), 0.6);

        let policy = ConsensusPolicy {
            source_weights: weights,
            freshness_window: 5,
            consensus_threshold: 0.6,
        };

        let engine = ConsensusEngine::new(policy);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let candidates = vec![
            RouteCandidate {
                source: "amm".to_string(),
                hops: vec!["XLM".to_string(), "USDC".to_string()],
                price: 0.5,
                timestamp: now,
            },
            RouteCandidate {
                source: "sdex".to_string(),
                hops: vec!["XLM".to_string(), "BTC".to_string(), "USDC".to_string()],
                price: 0.52,
                timestamp: now,
            },
        ];

        let result = engine.resolve(candidates).unwrap();
        assert_eq!(result.winning_route.source, "amm");
    }

    #[test]
    fn test_conflict_detection() {
        let policy = ConsensusPolicy::default();
        let engine = ConsensusEngine::new(policy);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let candidates = vec![
            RouteCandidate {
                source: "source1".to_string(),
                hops: vec!["A".to_string(), "B".to_string()],
                price: 1.0,
                timestamp: now,
            },
            RouteCandidate {
                source: "source2".to_string(),
                hops: vec!["A".to_string(), "C".to_string()],
                price: 1.08,
                timestamp: now,
            },
        ];

        let result = engine.resolve(candidates).unwrap();
        assert!(result.conflict_detected);
    }
}
