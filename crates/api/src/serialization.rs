//! Deterministic serialization contract for route diagnostics payload
//!
//! This module provides deterministic serialization for route diagnostics
//! so clients and tests can compare outputs reliably.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Trait for types that support deterministic serialization
pub trait DeterministicSerialize: Serialize + for<'de> Deserialize<'de> + Sized {
    /// Serialize to a deterministic JSON byte representation
    fn to_deterministic_json(&self) -> Result<Vec<u8>, serde_json::Error> {
        let value = serde_json::to_value(self)?;
        let normalized = Self::normalize_value(value);
        let mut buf = Vec::new();
        let mut serializer = serde_json::Serializer::new(&mut buf);
        serde_json::to_writer(&mut serializer, &normalized)?;
        Ok(buf)
    }

    /// Normalize a JSON value for deterministic output
    fn normalize_value(value: Value) -> Value {
        match value {
            Value::Object(map) => {
                let mut sorted = BTreeMap::new();
                for (k, v) in map {
                    sorted.insert(k, Self::normalize_value(v));
                }
                Value::Object(sorted.into_iter().collect())
            }
            Value::Array(arr) => {
                Value::Array(arr.into_iter().map(Self::normalize_value).collect())
            }
            Value::Number(n) => {
                if let Some(f) = n.as_f64() {
                    if f.is_nan() || f.is_infinite() {
                        Value::Null
                    } else {
                        Value::Number(n)
                    }
                } else {
                    Value::Number(n)
                }
            }
            other => other,
        }
    }
}

/// Configuration for deterministic serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializationConfig {
    /// Version of the serialization contract
    pub version: String,
    /// Whether to include non-deterministic fields
    pub include_non_deterministic: bool,
    /// Fields to exclude from serialization
    pub excluded_fields: Vec<String>,
}

impl Default for SerializationConfig {
    fn default() -> Self {
        Self {
            version: "1.0.0".to_string(),
            include_non_deterministic: true,
            excluded_fields: Vec::new(),
        }
    }
}

/// Normalized route diagnostics for deterministic serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedRouteDiagnostics {
    /// Selected route path (normalized)
    pub selected_path: NormalizedSwapPath,
    /// Route metrics (normalized)
    pub metrics: NormalizedRouteMetrics,
    /// Alternative routes (sorted by score descending)
    pub alternatives: Vec<NormalizedAlternative>,
    /// Policy used for optimization
    pub policy: NormalizedPolicy,
    /// Total computation time in milliseconds
    pub total_compute_time_ms: u64,
    /// Excluded routes (sorted by venue_ref)
    pub excluded_routes: Vec<NormalizedExclusion>,
    /// Flagged venues (sorted by venue_ref)
    pub flagged_venues: Vec<NormalizedAnomaly>,
    /// Serialization contract version
    pub serialization_version: String,
}

impl DeterministicSerialize for NormalizedRouteDiagnostics {}

/// Normalized swap path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedSwapPath {
    /// Hops in execution order
    pub hops: Vec<NormalizedHop>,
    /// Estimated output (string for precision)
    pub estimated_output: String,
}

/// Normalized hop in a swap path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedHop {
    pub source_asset: String,
    pub destination_asset: String,
    pub venue_type: String,
    pub venue_ref: String,
    pub price: String,
    pub fee_bps: u32,
    pub anomaly_score: String,
    pub anomaly_reasons: Vec<String>,
}

/// Normalized route metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedRouteMetrics {
    pub output_amount: String,
    pub impact_bps: u32,
    pub compute_time_us: u64,
    pub hop_count: usize,
    pub score: String,
    pub anomaly_score: String,
    pub anomaly_reasons: Vec<String>,
}

/// Normalized alternative route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedAlternative {
    pub path: NormalizedSwapPath,
    pub metrics: NormalizedRouteMetrics,
    /// Score for deterministic ordering
    pub sort_key: String,
}

/// Normalized policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedPolicy {
    pub output_weight: String,
    pub impact_weight: String,
    pub latency_weight: String,
    pub max_impact_bps: u32,
    pub max_compute_time_ms: u64,
    pub environment: String,
}

/// Normalized route exclusion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedExclusion {
    pub venue_ref: String,
    pub reason: String,
}

/// Normalized anomaly result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedAnomaly {
    pub venue_ref: String,
    pub score: String,
    pub reasons: Vec<String>,
}

/// Helper to format floating point numbers deterministically
pub fn format_float(f: f64, precision: usize) -> String {
    if f.is_nan() || f.is_infinite() {
        "0".to_string()
    } else {
        format!("{:.prec$}", f, prec = precision)
    }
}

/// Helper to format large integers as strings for JSON safety
pub fn format_int(i: i128) -> String {
    i.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalize_value_sorts_object_keys() {
        let value = json!({
            "z": 1,
            "a": 2,
            "m": 3
        });
        let normalized = NormalizedRouteDiagnostics::normalize_value(value);
        let obj = normalized.as_object().unwrap();
        let keys: Vec<_> = obj.keys().collect();
        assert_eq!(keys, vec!["a", "m", "z"]);
    }

    #[test]
    fn normalize_value_handles_nan() {
        let value = json!({
            "valid": 1.5,
            "nan": f64::NAN,
            "inf": f64::INFINITY
        });
        let normalized = NormalizedRouteDiagnostics::normalize_value(value);
        let obj = normalized.as_object().unwrap();
        assert!(obj["valid"].is_number());
        assert!(obj["nan"].is_null());
        assert!(obj["inf"].is_null());
    }

    #[test]
    fn format_float_handles_special_values() {
        assert_eq!(format_float(f64::NAN, 7), "0");
        assert_eq!(format_float(f64::INFINITY, 7), "0");
        assert_eq!(format_float(1.234567, 4), "1.2346");
    }

    #[test]
    fn deterministic_json_is_stable() {
        let diag = NormalizedRouteDiagnostics {
            selected_path: NormalizedSwapPath {
                hops: vec![],
                estimated_output: "1000000000".to_string(),
            },
            metrics: NormalizedRouteMetrics {
                output_amount: "1000000000".to_string(),
                impact_bps: 10,
                compute_time_us: 1000,
                hop_count: 1,
                score: "0.9500000".to_string(),
                anomaly_score: "0.0000000".to_string(),
                anomaly_reasons: vec![],
            },
            alternatives: vec![],
            policy: NormalizedPolicy {
                output_weight: "0.5000000".to_string(),
                impact_weight: "0.3000000".to_string(),
                latency_weight: "0.2000000".to_string(),
                max_impact_bps: 300,
                max_compute_time_ms: 500,
                environment: "production".to_string(),
            },
            total_compute_time_ms: 5,
            excluded_routes: vec![],
            flagged_venues: vec![],
            serialization_version: "1.0.0".to_string(),
        };

        let json1 = diag.to_deterministic_json().unwrap();
        let json2 = diag.to_deterministic_json().unwrap();
        assert_eq!(json1, json2);
    }
}
