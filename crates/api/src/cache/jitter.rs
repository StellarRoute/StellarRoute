//! Jittered TTL helpers for cache stampede prevention.
//!
//! When many cache entries for hot pairs expire at the same time, all
//! concurrent readers race to recompute the value — a "thundering herd".
//! Adding a small random jitter to each TTL spreads expiry times across a
//! window, dramatically reducing the probability of a synchronized storm.
//!
//! # Algorithm
//!
//! Given a base TTL `T` and a jitter fraction `f` (0.0–1.0):
//!
//! ```text
//! jitter_range = T * f
//! actual_ttl   = T + random(-jitter_range/2, +jitter_range/2)
//! ```
//!
//! The result is clamped to `[min_ttl, max_ttl]` so we never produce a
//! zero or negative TTL.
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::cache::jitter::JitteredTtl;
//!
//! let jitter = JitteredTtl::default();          // ±15 % jitter
//! let ttl = jitter.apply(Duration::from_secs(5));
//! cache.set(key, value, ttl).await?;
//! ```

use std::time::Duration;

/// Configuration for jittered TTL.
#[derive(Debug, Clone)]
pub struct JitteredTtl {
    /// Fraction of the base TTL to use as the jitter window (0.0–1.0).
    /// Default: 0.15 (±15 %).
    pub jitter_fraction: f64,
    /// Minimum TTL floor — the result is never shorter than this.
    pub min_ttl: Duration,
    /// Maximum TTL ceiling — the result is never longer than this.
    pub max_ttl: Duration,
}

impl Default for JitteredTtl {
    fn default() -> Self {
        Self {
            jitter_fraction: 0.15,
            min_ttl: Duration::from_millis(200),
            max_ttl: Duration::from_secs(300),
        }
    }
}

impl JitteredTtl {
    /// Create a new `JitteredTtl` with the given jitter fraction.
    pub fn with_fraction(jitter_fraction: f64) -> Self {
        Self {
            jitter_fraction: jitter_fraction.clamp(0.0, 1.0),
            ..Default::default()
        }
    }

    /// Apply jitter to `base_ttl` and return the adjusted duration.
    ///
    /// The jitter is uniformly distributed in `[-jitter_range/2, +jitter_range/2]`
    /// where `jitter_range = base_ttl * jitter_fraction`.
    pub fn apply(&self, base_ttl: Duration) -> Duration {
        let base_ms = base_ttl.as_millis() as f64;
        let range_ms = base_ms * self.jitter_fraction;

        // Uniform random in [-range/2, +range/2]
        let offset_ms = (rand::random::<f64>() - 0.5) * range_ms;
        let adjusted_ms = (base_ms + offset_ms).max(0.0) as u64;

        let adjusted = Duration::from_millis(adjusted_ms);
        adjusted.clamp(self.min_ttl, self.max_ttl)
    }

    /// Apply jitter and return the result as whole seconds (minimum 1).
    pub fn apply_secs(&self, base_ttl: Duration) -> u64 {
        self.apply(base_ttl).as_secs().max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jitter_within_bounds() {
        let jitter = JitteredTtl::default();
        let base = Duration::from_secs(10);

        for _ in 0..1000 {
            let ttl = jitter.apply(base);
            assert!(
                ttl >= jitter.min_ttl,
                "TTL {:?} below minimum {:?}",
                ttl,
                jitter.min_ttl
            );
            assert!(
                ttl <= jitter.max_ttl,
                "TTL {:?} above maximum {:?}",
                ttl,
                jitter.max_ttl
            );
        }
    }

    #[test]
    fn test_jitter_spreads_values() {
        let jitter = JitteredTtl::with_fraction(0.5);
        let base = Duration::from_secs(10);

        let mut values: std::collections::HashSet<u64> = std::collections::HashSet::new();
        for _ in 0..200 {
            let ttl = jitter.apply(base);
            values.insert(ttl.as_millis() as u64);
        }

        // With 50 % jitter over 200 samples we expect many distinct values.
        assert!(
            values.len() > 10,
            "Expected spread of TTL values, got {} distinct values",
            values.len()
        );
    }

    #[test]
    fn test_zero_jitter_returns_base() {
        let jitter = JitteredTtl {
            jitter_fraction: 0.0,
            min_ttl: Duration::from_millis(1),
            max_ttl: Duration::from_secs(3600),
        };
        let base = Duration::from_secs(5);
        let ttl = jitter.apply(base);
        assert_eq!(ttl, base);
    }

    #[test]
    fn test_apply_secs_minimum_one() {
        let jitter = JitteredTtl {
            jitter_fraction: 0.0,
            min_ttl: Duration::from_millis(1),
            max_ttl: Duration::from_secs(3600),
        };
        // Even a sub-second base should return at least 1 second.
        let secs = jitter.apply_secs(Duration::from_millis(100));
        assert!(secs >= 1);
    }

    #[test]
    fn test_jitter_fraction_clamped() {
        let jitter = JitteredTtl::with_fraction(2.0); // > 1.0 should be clamped
        assert!(jitter.jitter_fraction <= 1.0);
    }

    // ── Clamp + default-fraction guards (issue #1301) ────────────────────

    /// A zero base TTL must not survive as a zero TTL: the floor takes over.
    /// A TTL of 0 would expire every entry immediately and stampede Redis.
    #[test]
    fn test_zero_base_ttl_clamps_to_min() {
        let jitter = JitteredTtl::default();
        assert_eq!(jitter.apply(Duration::ZERO), jitter.min_ttl);
    }

    /// Same guard one step up: a base below the floor is raised to the floor.
    #[test]
    fn test_base_below_min_clamps_to_min() {
        let jitter = JitteredTtl::default();
        let ttl = jitter.apply(Duration::from_millis(10));
        assert_eq!(ttl, jitter.min_ttl);
    }

    /// The ceiling is enforced too, so a stray large base cannot pin an entry
    /// in cache for longer than `max_ttl`.
    #[test]
    fn test_base_above_max_clamps_to_max() {
        let jitter = JitteredTtl::default();
        let ttl = jitter.apply(Duration::from_secs(600));
        assert_eq!(ttl, jitter.max_ttl);
    }

    /// Even at the widest allowed jitter and the smallest possible floor, the
    /// result is never zero — this is the regression the floor exists for.
    #[test]
    fn test_never_returns_zero_ttl_at_max_jitter() {
        let jitter = JitteredTtl {
            jitter_fraction: 1.0,
            min_ttl: Duration::from_millis(1),
            max_ttl: Duration::from_secs(3600),
        };
        let base = Duration::from_millis(20);
        for _ in 0..1000 {
            let ttl = jitter.apply(base);
            assert!(!ttl.is_zero(), "TTL must never be zero, got {:?}", ttl);
        }
    }

    /// `apply_secs` floors at 1 second even when the clamped duration rounds
    /// down to zero whole seconds.
    #[test]
    fn test_apply_secs_never_zero_for_zero_base() {
        let jitter = JitteredTtl::default();
        assert!(jitter.apply_secs(Duration::ZERO) >= 1);
    }

    /// A negative fraction is clamped to 0.0 rather than shrinking the TTL.
    #[test]
    fn test_negative_jitter_fraction_clamped_to_zero() {
        let jitter = JitteredTtl::with_fraction(-0.5);
        assert_eq!(jitter.jitter_fraction, 0.0);
        let base = Duration::from_secs(5);
        assert_eq!(jitter.apply(base), base);
    }

    /// Pin the production defaults. Changing these changes cache behaviour for
    /// live users, so it should be a deliberate edit that updates this test.
    #[test]
    fn test_default_config_is_15_percent_with_200ms_floor() {
        let jitter = JitteredTtl::default();
        assert_eq!(jitter.jitter_fraction, 0.15);
        assert_eq!(jitter.min_ttl, Duration::from_millis(200));
        assert_eq!(jitter.max_ttl, Duration::from_secs(300));
    }

    /// `with_fraction` overrides the fraction but keeps the default clamps.
    #[test]
    fn test_with_fraction_keeps_default_clamps() {
        let jitter = JitteredTtl::with_fraction(0.5);
        let default = JitteredTtl::default();
        assert_eq!(jitter.min_ttl, default.min_ttl);
        assert_eq!(jitter.max_ttl, default.max_ttl);
    }

    /// With the default ±15 % window, a 10 s base stays inside
    /// [9.25 s, 10.75 s] — half the 15 % range either side.
    #[test]
    fn test_default_jitter_stays_within_15_percent_window() {
        let jitter = JitteredTtl::default();
        let base = Duration::from_secs(10);
        let lower = Duration::from_millis(9_250);
        let upper = Duration::from_millis(10_750);

        for _ in 0..1000 {
            let ttl = jitter.apply(base);
            assert!(ttl >= lower, "TTL {:?} below -7.5 % bound {:?}", ttl, lower);
            assert!(ttl <= upper, "TTL {:?} above +7.5 % bound {:?}", ttl, upper);
        }
    }

    /// Verify that jitter reduces the probability of synchronized expiry.
    ///
    /// We simulate 1000 cache entries all set with the same base TTL and
    /// check that the resulting TTLs are spread across a window rather than
    /// all landing on the same millisecond.
    #[test]
    fn test_stampede_reduction() {
        let jitter = JitteredTtl::with_fraction(0.2); // ±20 %
        let base = Duration::from_secs(5); // 5 000 ms

        let ttls: Vec<u64> = (0..1000)
            .map(|_| jitter.apply(base).as_millis() as u64)
            .collect();

        let min = *ttls.iter().min().unwrap();
        let max = *ttls.iter().max().unwrap();
        let spread = max - min;

        // With ±20 % jitter on 5 000 ms the spread should be at least 500 ms.
        assert!(
            spread >= 500,
            "Expected spread >= 500 ms, got {} ms (min={}, max={})",
            spread,
            min,
            max
        );
    }
}
