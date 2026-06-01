//! Snapshot isolation validator for multi-hop quote assembly.
//!
//! # Problem
//! In a multi-hop swap (e.g. XLM → USDC → BTC) each hop reads pool/orderbook
//! state.  If two hops read from *different* market snapshots (different ledger
//! sequences), the assembled quote is internally inconsistent: the price used
//! for hop 1 may no longer hold by the time hop 2 executes.
//!
//! # Solution
//! Every [`LiquidityEdge`] that enters a multi-hop path must carry the same
//! `snapshot_id` (an opaque monotonic counter derived from the ledger sequence
//! at which pool state was captured).  This module validates that invariant and
//! returns a machine-readable [`SnapshotIsolationError`] when it is violated.
//!
//! # Metrics
//! A [`SnapshotIsolationMetrics`] counter is updated on every validation call
//! so Prometheus scrapers can alert on isolation violations.

use crate::pathfinder::SwapPath;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use thiserror::Error;

// ── Snapshot identifier ───────────────────────────────────────────────────────

/// Opaque monotonic snapshot identifier derived from a ledger sequence number.
///
/// Two hops are considered *snapshot-compatible* if and only if their
/// `SnapshotId`s are equal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SnapshotId(pub u64);

impl fmt::Display for SnapshotId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "snapshot#{}", self.0)
    }
}

// ── Errors ────────────────────────────────────────────────────────────────────

/// Machine-readable error returned when snapshot isolation is violated.
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum SnapshotIsolationError {
    /// Two or more hops in the path used state from different snapshots.
    #[error("mixed snapshot ids in path: hop {hop_index} uses {hop_snapshot}, expected {expected_snapshot}")]
    MixedSnapshots {
        /// Zero-based index of the offending hop.
        hop_index: usize,
        /// The snapshot id on the first hop (the expected baseline).
        expected_snapshot: SnapshotId,
        /// The snapshot id found on the offending hop.
        hop_snapshot: SnapshotId,
        /// Human-readable venue reference of the offending hop.
        venue_ref: String,
    },
    /// The path carries no hops and therefore cannot be validated.
    #[error("path contains no hops")]
    EmptyPath,
}

// ── Validated hop ─────────────────────────────────────────────────────────────

/// A path hop that has been stamped with its market snapshot id.
///
/// Callers that assemble multi-hop paths should attach a `snapshot_id` to each
/// hop at the time pool state is read.  The validator then checks consistency.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatedHop {
    /// The snapshot at which this hop's pool/orderbook state was captured.
    pub snapshot_id: SnapshotId,
    /// The venue reference (e.g. AMM pool address or SDEX offer-book key).
    pub venue_ref: String,
    /// Source asset key.
    pub source_asset: String,
    /// Destination asset key.
    pub destination_asset: String,
}

// ── Metrics ───────────────────────────────────────────────────────────────────

/// Shared atomic counters tracking validator outcomes.
///
/// Mount these in your Prometheus registry by reading them in the metrics
/// scrape handler.
#[derive(Clone, Default)]
pub struct SnapshotIsolationMetrics {
    inner: Arc<SnapshotIsolationMetricsInner>,
}

#[derive(Default)]
struct SnapshotIsolationMetricsInner {
    total_validations: AtomicU64,
    violations: AtomicU64,
    empty_paths: AtomicU64,
}

impl SnapshotIsolationMetrics {
    /// Number of times `validate` has been called.
    pub fn total_validations(&self) -> u64 {
        self.inner.total_validations.load(Ordering::Relaxed)
    }

    /// Number of validation calls that detected a snapshot mismatch.
    pub fn violations(&self) -> u64 {
        self.inner.violations.load(Ordering::Relaxed)
    }

    /// Number of validation calls that received an empty path.
    pub fn empty_paths(&self) -> u64 {
        self.inner.empty_paths.load(Ordering::Relaxed)
    }
}

// ── Config ────────────────────────────────────────────────────────────────────

/// Configuration for the snapshot isolation validator.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SnapshotValidatorConfig {
    /// When `true` the validator strictly rejects any mixed-snapshot path.
    /// When `false` it records the violation in metrics but returns `Ok`.
    /// Defaults to `true` (strict).
    pub strict: bool,
}

impl Default for SnapshotValidatorConfig {
    fn default() -> Self {
        Self { strict: true }
    }
}

// ── Validator ─────────────────────────────────────────────────────────────────

/// Validates that all hops in a multi-hop path share the same snapshot id.
pub struct SnapshotIsolationValidator {
    config: SnapshotValidatorConfig,
    metrics: SnapshotIsolationMetrics,
}

impl SnapshotIsolationValidator {
    /// Create a new validator with default (strict) config and fresh metrics.
    pub fn new(config: SnapshotValidatorConfig) -> Self {
        Self {
            config,
            metrics: SnapshotIsolationMetrics::default(),
        }
    }

    /// Shared read access to the metrics counters.
    pub fn metrics(&self) -> &SnapshotIsolationMetrics {
        &self.metrics
    }

    /// Validate snapshot consistency across a slice of stamped hops.
    ///
    /// Returns `Err(SnapshotIsolationError::MixedSnapshots)` if any hop
    /// deviates from the snapshot of the first hop.
    pub fn validate_hops(
        &self,
        hops: &[ValidatedHop],
    ) -> Result<(), SnapshotIsolationError> {
        self.metrics
            .inner
            .total_validations
            .fetch_add(1, Ordering::Relaxed);

        if hops.is_empty() {
            self.metrics.inner.empty_paths.fetch_add(1, Ordering::Relaxed);
            if self.config.strict {
                return Err(SnapshotIsolationError::EmptyPath);
            }
            return Ok(());
        }

        let baseline = hops[0].snapshot_id;
        for (idx, hop) in hops.iter().enumerate().skip(1) {
            if hop.snapshot_id != baseline {
                self.metrics.inner.violations.fetch_add(1, Ordering::Relaxed);
                tracing::warn!(
                    hop_index = idx,
                    expected = %baseline,
                    found = %hop.snapshot_id,
                    venue_ref = %hop.venue_ref,
                    "snapshot isolation violation detected"
                );
                if self.config.strict {
                    return Err(SnapshotIsolationError::MixedSnapshots {
                        hop_index: idx,
                        expected_snapshot: baseline,
                        hop_snapshot: hop.snapshot_id,
                        venue_ref: hop.venue_ref.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Convenience: extract `ValidatedHop`s from a [`SwapPath`] using a
    /// uniform `snapshot_id` (i.e. the snapshot captured when the path was
    /// assembled).  This is the common case where the caller already knows
    /// the snapshot under which the whole path was built.
    ///
    /// For paths where each hop may have been read from a different snapshot
    /// use `validate_hops` directly with individually stamped hops.
    pub fn validate_path_uniform(
        &self,
        path: &SwapPath,
        snapshot_id: SnapshotId,
    ) -> Result<(), SnapshotIsolationError> {
        let hops: Vec<ValidatedHop> = path
            .hops
            .iter()
            .map(|h| ValidatedHop {
                snapshot_id,
                venue_ref: h.venue_ref.clone(),
                source_asset: h.source_asset.clone(),
                destination_asset: h.destination_asset.clone(),
            })
            .collect();
        self.validate_hops(&hops)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn hop(snapshot: u64, venue: &str) -> ValidatedHop {
        ValidatedHop {
            snapshot_id: SnapshotId(snapshot),
            venue_ref: venue.to_string(),
            source_asset: "XLM".to_string(),
            destination_asset: "USDC".to_string(),
        }
    }

    fn strict_validator() -> SnapshotIsolationValidator {
        SnapshotIsolationValidator::new(SnapshotValidatorConfig { strict: true })
    }

    fn lenient_validator() -> SnapshotIsolationValidator {
        SnapshotIsolationValidator::new(SnapshotValidatorConfig { strict: false })
    }

    #[test]
    fn test_consistent_snapshots_pass() {
        let v = strict_validator();
        let hops = vec![hop(42, "pool_a"), hop(42, "pool_b"), hop(42, "pool_c")];
        assert!(v.validate_hops(&hops).is_ok());
    }

    #[test]
    fn test_mixed_snapshot_rejected_strict() {
        let v = strict_validator();
        let hops = vec![hop(42, "pool_a"), hop(43, "pool_b")];
        let err = v.validate_hops(&hops).unwrap_err();
        match err {
            SnapshotIsolationError::MixedSnapshots {
                hop_index,
                expected_snapshot,
                hop_snapshot,
                venue_ref,
            } => {
                assert_eq!(hop_index, 1);
                assert_eq!(expected_snapshot, SnapshotId(42));
                assert_eq!(hop_snapshot, SnapshotId(43));
                assert_eq!(venue_ref, "pool_b");
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn test_mixed_snapshot_allowed_lenient() {
        let v = lenient_validator();
        let hops = vec![hop(42, "pool_a"), hop(99, "pool_b")];
        assert!(v.validate_hops(&hops).is_ok());
        assert_eq!(v.metrics().violations(), 1);
    }

    #[test]
    fn test_empty_path_rejected_strict() {
        let v = strict_validator();
        assert!(matches!(
            v.validate_hops(&[]),
            Err(SnapshotIsolationError::EmptyPath)
        ));
        assert_eq!(v.metrics().empty_paths(), 1);
    }

    #[test]
    fn test_empty_path_allowed_lenient() {
        let v = lenient_validator();
        assert!(v.validate_hops(&[]).is_ok());
    }

    #[test]
    fn test_metrics_total_validations_increment() {
        let v = strict_validator();
        for _ in 0..5 {
            let _ = v.validate_hops(&[hop(1, "pool_a"), hop(1, "pool_b")]);
        }
        assert_eq!(v.metrics().total_validations(), 5);
    }

    #[test]
    fn test_metrics_violation_count() {
        let v = strict_validator();
        let _ = v.validate_hops(&[hop(1, "pool_a"), hop(2, "pool_b")]);
        let _ = v.validate_hops(&[hop(1, "pool_a"), hop(2, "pool_b")]);
        let _ = v.validate_hops(&[hop(1, "pool_a"), hop(1, "pool_b")]); // ok
        assert_eq!(v.metrics().violations(), 2);
    }

    #[test]
    fn test_single_hop_always_passes() {
        let v = strict_validator();
        assert!(v.validate_hops(&[hop(77, "pool_x")]).is_ok());
    }

    #[test]
    fn test_error_is_serializable() {
        let err = SnapshotIsolationError::MixedSnapshots {
            hop_index: 2,
            expected_snapshot: SnapshotId(10),
            hop_snapshot: SnapshotId(11),
            venue_ref: "pool_z".to_string(),
        };
        let json = serde_json::to_string(&err).expect("should serialize");
        assert!(json.contains("mixed_snapshots") || json.contains("MixedSnapshots") || json.contains("hop_index"));
    }

    #[test]
    fn test_concurrent_update_detection() {
        // Simulates two concurrent indexer updates producing different ledger seqs.
        // Hop 0 was read at ledger 1000, hop 1 at ledger 1001 due to a concurrent update.
        let v = strict_validator();
        let hops = vec![
            ValidatedHop {
                snapshot_id: SnapshotId(1000),
                venue_ref: "xlm_usdc_amm".into(),
                source_asset: "XLM".into(),
                destination_asset: "USDC".into(),
            },
            ValidatedHop {
                snapshot_id: SnapshotId(1001), // concurrent update bumped this
                venue_ref: "usdc_btc_sdex".into(),
                source_asset: "USDC".into(),
                destination_asset: "BTC".into(),
            },
        ];
        assert!(v.validate_hops(&hops).is_err(), "concurrent-update mixed snapshot must be rejected");
    }
}
