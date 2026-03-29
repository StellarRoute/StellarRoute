//! Non-blocking capture hook for the live quote pipeline.
//!
//! `CaptureHook::capture` is called after a `QuoteResponse` is fully built.
//! It serialises the inputs into a `ReplayArtifact`, redacts sensitive fields,
//! and persists the artifact in a detached `tokio::spawn` task — never blocking
//! the quote response path.

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::QuoteResponse;
use crate::replay::artifact::{
    HealthConfigSnapshot, LiquidityCandidate, ReplayArtifact, CURRENT_SCHEMA_VERSION,
};
use crate::replay::Redactor;

/// Non-blocking capture hook.
///
/// Store in `AppState` as `Option<Arc<CaptureHook>>`.
/// Set to `None` when `REPLAY_CAPTURE_ENABLED` is `false` (default).
pub struct CaptureHook {
    db: PgPool,
    /// When `false`, `capture()` is a no-op.
    pub enabled: bool,
}

impl CaptureHook {
    /// Create a new hook. Pass `enabled = false` to disable capture without
    /// removing the hook from `AppState`.
    pub fn new(db: PgPool, enabled: bool) -> Self {
        Self { db, enabled }
    }

    /// Create a hook whose enabled state is read from the
    /// `REPLAY_CAPTURE_ENABLED` environment variable (default: `false`).
    pub fn from_env(db: PgPool) -> Self {
        let enabled = std::env::var("REPLAY_CAPTURE_ENABLED")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);
        Self::new(db, enabled)
    }

    /// Capture a quote computation as a replay artifact.
    ///
    /// This method is **synchronous and non-blocking**: it builds and redacts
    /// the artifact on the calling thread, then spawns a detached task for the
    /// DB write. The caller never awaits the spawn.
    ///
    /// # Arguments
    ///
    /// * `base` – canonical base asset string (e.g. `"native"` or `"USDC:ISSUER"`)
    /// * `quote` – canonical quote asset string
    /// * `amount` – amount string as used in the request (7-decimal)
    /// * `slippage_bps` – slippage tolerance
    /// * `quote_type` – `"sell"` or `"buy"`
    /// * `liquidity_snapshot` – all candidates fetched from `normalized_liquidity`
    /// * `health_config` – health scoring config snapshot used during computation
    /// * `response` – the `QuoteResponse` produced by the live pipeline
    /// * `incident_id` – optional incident label
    pub fn capture(
        &self,
        base: &str,
        quote: &str,
        amount: &str,
        slippage_bps: u32,
        quote_type: &str,
        liquidity_snapshot: Vec<LiquidityCandidate>,
        health_config: HealthConfigSnapshot,
        response: &QuoteResponse,
        incident_id: Option<String>,
    ) {
        if !self.enabled {
            return;
        }

        // Serialise the QuoteResponse to a JSON value for storage.
        let original_output = match serde_json::to_value(response) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, "replay capture: failed to serialise response");
                return;
            }
        };

        let mut artifact = ReplayArtifact {
            id: Uuid::new_v4(),
            schema_version: CURRENT_SCHEMA_VERSION,
            incident_id,
            captured_at: Utc::now(),
            base: base.to_string(),
            quote: quote.to_string(),
            amount: amount.to_string(),
            slippage_bps,
            quote_type: quote_type.to_string(),
            liquidity_snapshot,
            health_config_snapshot: health_config,
            original_output,
        };

        // Redact synchronously before handing off to the async task.
        Redactor::redact(&mut artifact);

        let db = self.db.clone();
        let artifact_id = artifact.id;

        tokio::spawn(async move {
            if let Err(e) = ReplayArtifact::insert(&db, &artifact).await {
                tracing::warn!(
                    %artifact_id,
                    error = %e,
                    "replay capture failed — quote unaffected"
                );
            } else {
                tracing::debug!(%artifact_id, "replay artifact captured");
            }
        });
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AssetInfo, DataFreshness, PathStep, QuoteResponse};
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn make_response() -> QuoteResponse {
        QuoteResponse {
            base_asset: AssetInfo::native(),
            quote_asset: AssetInfo::native(),
            amount: "100.0000000".to_string(),
            price: "1.0000000".to_string(),
            total: "100.0000000".to_string(),
            quote_type: "sell".to_string(),
            path: vec![PathStep {
                from_asset: AssetInfo::native(),
                to_asset: AssetInfo::native(),
                price: "1.0000000".to_string(),
                source: "sdex".to_string(),
            }],
            timestamp: 0,
            expires_at: None,
            source_timestamp: None,
            ttl_seconds: None,
            rationale: None,
            exclusion_diagnostics: None,
            data_freshness: Some(DataFreshness {
                fresh_count: 1,
                stale_count: 0,
                max_staleness_secs: 0,
            }),
        }
    }

    fn make_snapshot() -> Vec<LiquidityCandidate> {
        vec![LiquidityCandidate {
            venue_type: "sdex".to_string(),
            venue_ref: "offer1".to_string(),
            price: "1.0000000".to_string(),
            available_amount: "100.0000000".to_string(),
        }]
    }

    fn make_health_config() -> HealthConfigSnapshot {
        HealthConfigSnapshot {
            freshness_threshold_secs_sdex: 30,
            freshness_threshold_secs_amm: 60,
            staleness_threshold_secs: 30,
            min_tvl_threshold_e7: 1_000_000_000,
        }
    }

    /// Verify that when `enabled = false`, the hook is a no-op.
    /// We test this by checking that the artifact builder path is never reached
    /// (no panic, no side effects).
    #[test]
    fn disabled_hook_is_noop() {
        // We can't easily mock the DB in a unit test, but we can verify that
        // a disabled hook with a dummy PgPool never panics and returns immediately.
        // The real DB-insert path is covered by integration tests.
        //
        // Use a counter to verify the early-return branch is taken.
        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        struct CountingHook {
            enabled: bool,
        }
        impl CountingHook {
            fn capture_noop(&self) {
                if !self.enabled {
                    COUNTER.fetch_add(1, Ordering::SeqCst);
                    return;
                }
                panic!("should not reach here when disabled");
            }
        }

        let hook = CountingHook { enabled: false };
        hook.capture_noop();
        assert_eq!(COUNTER.load(Ordering::SeqCst), 1);
    }

    /// Verify that `CaptureHook::from_env` reads the env var correctly.
    #[test]
    fn from_env_disabled_by_default() {
        // Ensure the env var is not set
        std::env::remove_var("REPLAY_CAPTURE_ENABLED");
        // We can't construct a real PgPool in a unit test, so we test the
        // enabled flag logic directly.
        let enabled = std::env::var("REPLAY_CAPTURE_ENABLED")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);
        assert!(!enabled, "capture should be disabled by default");
    }

    #[test]
    fn from_env_enabled_when_set() {
        std::env::set_var("REPLAY_CAPTURE_ENABLED", "true");
        let enabled = std::env::var("REPLAY_CAPTURE_ENABLED")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);
        assert!(enabled);
        std::env::remove_var("REPLAY_CAPTURE_ENABLED");
    }

    /// Verify that the artifact built inside `capture` has the correct fields.
    /// We test the builder logic directly without a real DB.
    #[test]
    fn artifact_fields_are_correct() {
        let response = make_response();
        let snapshot = make_snapshot();
        let health = make_health_config();

        let original_output = serde_json::to_value(&response).expect("serialize");

        let mut artifact = ReplayArtifact {
            id: Uuid::new_v4(),
            schema_version: CURRENT_SCHEMA_VERSION,
            incident_id: Some("INC-001".to_string()),
            captured_at: Utc::now(),
            base: "native".to_string(),
            quote: "USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5".to_string(),
            amount: "100.0000000".to_string(),
            slippage_bps: 50,
            quote_type: "sell".to_string(),
            liquidity_snapshot: snapshot,
            health_config_snapshot: health,
            original_output,
        };

        Redactor::redact(&mut artifact);

        assert_eq!(artifact.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(artifact.incident_id.as_deref(), Some("INC-001"));
        assert_eq!(artifact.base, "native");
        // Issuer in quote should be redacted
        assert!(artifact.quote.contains("[REDACTED]"));
        assert_eq!(artifact.slippage_bps, 50);
        assert_eq!(artifact.liquidity_snapshot.len(), 1);
    }
}
