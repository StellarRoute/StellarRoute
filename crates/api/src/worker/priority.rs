//! Adaptive queue prioritization for quote requests.
//!
//! # Design
//!
//! Requests are classified into four priority bands:
//!
//! | Level    | Value | Criteria (configurable)                          |
//! |----------|-------|--------------------------------------------------|
//! | Critical | 0     | amount ≥ `critical_amount_threshold`             |
//! | High     | 1     | amount ≥ `high_amount_threshold`                 |
//! | Normal   | 2     | everything else (default)                        |
//! | Low      | 3     | batch requests or explicitly deprioritised calls |
//!
//! ## Starvation prevention
//!
//! The scheduler uses a **weighted virtual clock** (inspired by WFQ / STFQ).
//! Each priority band has a weight; when a job is enqueued its `virtual_time`
//! is set to `max(global_virtual_time, last_virtual_time_for_band) + cost/weight`.
//! The dequeue query orders by `(virtual_time ASC, created_at ASC)`, so
//! lower-priority jobs are never starved indefinitely — they simply advance
//! their virtual clock more slowly than higher-priority ones.
//!
//! ## Configuration
//!
//! All thresholds and weights are exposed through [`PriorityConfig`] so they
//! can be tuned at startup (or later via an admin endpoint) without recompiling.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

// ─── Priority level ──────────────────────────────────────────────────────────

/// Priority band for a route-computation job.
///
/// The integer value is stored directly in the database `priority` column.
/// Lower numbers are processed first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(i16)]
pub enum RequestPriority {
    /// Large-amount or explicitly elevated requests — processed first.
    Critical = 0,
    /// Medium-amount requests.
    High = 1,
    /// Standard requests (default).
    Normal = 2,
    /// Batch / background requests — processed last.
    Low = 3,
}

impl RequestPriority {
    /// Convert the stored `i16` back to a [`RequestPriority`].
    pub fn from_i16(v: i16) -> Self {
        match v {
            0 => Self::Critical,
            1 => Self::High,
            3 => Self::Low,
            _ => Self::Normal,
        }
    }

    /// Human-readable label used in metrics.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::High => "high",
            Self::Normal => "normal",
            Self::Low => "low",
        }
    }
}

impl std::fmt::Display for RequestPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ─── Configuration ───────────────────────────────────────────────────────────

/// Tunable parameters for the priority classifier and starvation-prevention
/// scheduler.  All fields have sensible defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityConfig {
    // ── Classification thresholds ──────────────────────────────────────────
    /// Requests with `amount ≥ critical_amount_threshold` are classified as
    /// [`RequestPriority::Critical`].  Default: 100 000.
    pub critical_amount_threshold: f64,

    /// Requests with `amount ≥ high_amount_threshold` (but below critical)
    /// are classified as [`RequestPriority::High`].  Default: 1 000.
    pub high_amount_threshold: f64,

    // ── WFQ weights (higher weight = faster virtual-clock advance = lower
    //    effective priority relative to other bands) ────────────────────────
    /// Virtual-clock weight for [`RequestPriority::Critical`].  Default: 1.
    pub weight_critical: u32,
    /// Virtual-clock weight for [`RequestPriority::High`].  Default: 2.
    pub weight_high: u32,
    /// Virtual-clock weight for [`RequestPriority::Normal`].  Default: 4.
    pub weight_normal: u32,
    /// Virtual-clock weight for [`RequestPriority::Low`].  Default: 8.
    pub weight_low: u32,

    // ── Starvation cap ────────────────────────────────────────────────────
    /// Maximum number of consecutive high-priority jobs that can be dequeued
    /// before the scheduler is forced to serve at least one lower-priority
    /// job.  Set to 0 to disable the cap (pure WFQ).  Default: 50.
    pub starvation_cap: u32,
}

impl Default for PriorityConfig {
    fn default() -> Self {
        Self {
            critical_amount_threshold: 100_000.0,
            high_amount_threshold: 1_000.0,
            weight_critical: 1,
            weight_high: 2,
            weight_normal: 4,
            weight_low: 8,
            starvation_cap: 50,
        }
    }
}

impl PriorityConfig {
    /// Return the WFQ weight for a given priority band.
    pub fn weight_for(&self, priority: RequestPriority) -> u32 {
        match priority {
            RequestPriority::Critical => self.weight_critical,
            RequestPriority::High => self.weight_high,
            RequestPriority::Normal => self.weight_normal,
            RequestPriority::Low => self.weight_low,
        }
    }
}

// ─── Classifier ──────────────────────────────────────────────────────────────

/// Classifies incoming quote requests into priority bands and computes the
/// WFQ virtual time that should be stored with each job.
///
/// The classifier is cheap to clone (it wraps an `Arc`) and is intended to
/// live in [`AppState`](crate::state::AppState).
#[derive(Clone, Debug)]
pub struct PriorityClassifier {
    config: PriorityConfig,
    /// Shared virtual clock — a monotonically increasing counter measured in
    /// "cost units".  Stored as `i64` to match the PostgreSQL `BIGINT` column.
    virtual_clock: Arc<AtomicI64>,
    /// Per-band finish times (last virtual_time assigned to each band).
    finish_critical: Arc<AtomicI64>,
    finish_high: Arc<AtomicI64>,
    finish_normal: Arc<AtomicI64>,
    finish_low: Arc<AtomicI64>,
}

impl PriorityClassifier {
    /// Create a new classifier with the given configuration.
    pub fn new(config: PriorityConfig) -> Self {
        Self {
            config,
            virtual_clock: Arc::new(AtomicI64::new(0)),
            finish_critical: Arc::new(AtomicI64::new(0)),
            finish_high: Arc::new(AtomicI64::new(0)),
            finish_normal: Arc::new(AtomicI64::new(0)),
            finish_low: Arc::new(AtomicI64::new(0)),
        }
    }

    /// Classify a request and return its priority band.
    ///
    /// Rules (evaluated in order):
    /// 1. `is_batch = true`  → [`RequestPriority::Low`]
    /// 2. `amount ≥ critical_amount_threshold` → [`RequestPriority::Critical`]
    /// 3. `amount ≥ high_amount_threshold`     → [`RequestPriority::High`]
    /// 4. otherwise                            → [`RequestPriority::Normal`]
    pub fn classify(&self, amount: f64, is_batch: bool) -> RequestPriority {
        if is_batch {
            return RequestPriority::Low;
        }
        if amount >= self.config.critical_amount_threshold {
            RequestPriority::Critical
        } else if amount >= self.config.high_amount_threshold {
            RequestPriority::High
        } else {
            RequestPriority::Normal
        }
    }

    /// Compute the WFQ virtual time for a new job with the given priority.
    ///
    /// The virtual time is:
    ///   `max(global_virtual_clock, band_finish_time) + weight`
    ///
    /// After computing, both the global clock and the band finish time are
    /// advanced to the new value.
    pub fn next_virtual_time(&self, priority: RequestPriority) -> i64 {
        let weight = self.config.weight_for(priority) as i64;
        let band_finish = self.band_finish(priority);

        let global = self.virtual_clock.load(Ordering::Relaxed);
        let start = global.max(band_finish.load(Ordering::Relaxed));
        let vt = start + weight;

        // Advance the band finish time (best-effort; slight races are harmless
        // because the DB ordering is the authoritative scheduler).
        band_finish.fetch_max(vt, Ordering::Relaxed);
        self.virtual_clock.fetch_max(vt, Ordering::Relaxed);

        vt
    }

    /// Borrow the atomic finish-time counter for a given band.
    fn band_finish(&self, priority: RequestPriority) -> &AtomicI64 {
        match priority {
            RequestPriority::Critical => &self.finish_critical,
            RequestPriority::High => &self.finish_high,
            RequestPriority::Normal => &self.finish_normal,
            RequestPriority::Low => &self.finish_low,
        }
    }

    /// Read-only access to the current global virtual clock value (for metrics).
    pub fn current_virtual_clock(&self) -> i64 {
        self.virtual_clock.load(Ordering::Relaxed)
    }

    /// Expose the underlying config (for admin/metrics endpoints).
    pub fn config(&self) -> &PriorityConfig {
        &self.config
    }
}

impl Default for PriorityClassifier {
    fn default() -> Self {
        Self::new(PriorityConfig::default())
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn classifier() -> PriorityClassifier {
        PriorityClassifier::new(PriorityConfig::default())
    }

    // ── Classification ────────────────────────────────────────────────────

    #[test]
    fn batch_is_always_low() {
        let c = classifier();
        assert_eq!(c.classify(999_999.0, true), RequestPriority::Low);
    }

    #[test]
    fn small_amount_is_normal() {
        let c = classifier();
        assert_eq!(c.classify(1.0, false), RequestPriority::Normal);
        assert_eq!(c.classify(999.99, false), RequestPriority::Normal);
    }

    #[test]
    fn medium_amount_is_high() {
        let c = classifier();
        assert_eq!(c.classify(1_000.0, false), RequestPriority::High);
        assert_eq!(c.classify(99_999.99, false), RequestPriority::High);
    }

    #[test]
    fn large_amount_is_critical() {
        let c = classifier();
        assert_eq!(c.classify(100_000.0, false), RequestPriority::Critical);
        assert_eq!(c.classify(1_000_000.0, false), RequestPriority::Critical);
    }

    // ── Virtual time / starvation prevention ─────────────────────────────

    #[test]
    fn virtual_time_increases_monotonically() {
        let c = classifier();
        let vt1 = c.next_virtual_time(RequestPriority::Normal);
        let vt2 = c.next_virtual_time(RequestPriority::Normal);
        assert!(vt2 > vt1, "virtual time must be strictly increasing");
    }

    #[test]
    fn critical_advances_slower_than_low() {
        let c = classifier();
        // Enqueue one critical and one low job from the same starting point.
        let vt_critical = c.next_virtual_time(RequestPriority::Critical);
        // Reset clock to simulate independent enqueue from same baseline.
        c.virtual_clock.store(0, Ordering::Relaxed);
        c.finish_low.store(0, Ordering::Relaxed);
        let vt_low = c.next_virtual_time(RequestPriority::Low);
        // Critical weight (1) < Low weight (8), so critical gets a smaller vt.
        assert!(
            vt_critical < vt_low,
            "critical vt ({}) should be less than low vt ({})",
            vt_critical,
            vt_low
        );
    }

    #[test]
    fn priority_ordering() {
        assert!(RequestPriority::Critical < RequestPriority::High);
        assert!(RequestPriority::High < RequestPriority::Normal);
        assert!(RequestPriority::Normal < RequestPriority::Low);
    }

    #[test]
    fn from_i16_roundtrip() {
        for &p in &[
            RequestPriority::Critical,
            RequestPriority::High,
            RequestPriority::Normal,
            RequestPriority::Low,
        ] {
            assert_eq!(RequestPriority::from_i16(p as i16), p);
        }
    }

    #[test]
    fn config_weights_are_ordered() {
        let cfg = PriorityConfig::default();
        assert!(cfg.weight_critical < cfg.weight_high);
        assert!(cfg.weight_high < cfg.weight_normal);
        assert!(cfg.weight_normal < cfg.weight_low);
    }
}
