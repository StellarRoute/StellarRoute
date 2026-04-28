//! ReplayArtifact data types and PostgreSQL persistence.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::{ApiError, Result};

/// Current schema version. Bump when the artifact format changes in a breaking way.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Core data types
// ---------------------------------------------------------------------------

/// A single liquidity candidate captured from `normalized_liquidity` at quote time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiquidityCandidate {
    /// "sdex" or "amm"
    pub venue_type: String,
    /// Venue reference (offer ID or pool address). May be redacted.
    pub venue_ref: String,
    /// Price as a 7-decimal string, e.g. "1.0050000"
    pub price: String,
    /// Available amount as a 7-decimal string
    pub available_amount: String,
}

/// Snapshot of the `HealthScoringConfig` values used during the original quote.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthConfigSnapshot {
    pub freshness_threshold_secs_sdex: u64,
    pub freshness_threshold_secs_amm: u64,
    pub staleness_threshold_secs: u64,
    pub min_tvl_threshold_e7: i128,
}

/// A stored, redacted snapshot of a single quote computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayArtifact {
    /// Unique artifact identifier (UUID v4).
    pub id: Uuid,
    /// Schema version for forward-compatibility checks.
    pub schema_version: u32,
    /// Optional incident label for grouping related artifacts.
    pub incident_id: Option<String>,
    /// Wall-clock time when the artifact was captured.
    pub captured_at: DateTime<Utc>,

    // ── Request inputs ──────────────────────────────────────────────────────
    /// Canonical base asset string ("native" or "CODE:ISSUER", issuer redacted).
    pub base: String,
    /// Canonical quote asset string ("native" or "CODE:ISSUER", issuer redacted).
    pub quote: String,
    /// Amount as a 7-decimal string.
    pub amount: String,
    pub slippage_bps: u32,
    /// "sell" or "buy"
    pub quote_type: String,

    // ── Snapshots ───────────────────────────────────────────────────────────
    /// All liquidity candidates queried from `normalized_liquidity` at capture time.
    pub liquidity_snapshot: Vec<LiquidityCandidate>,
    /// Health scoring configuration used during the original computation.
    pub health_config_snapshot: HealthConfigSnapshot,
    /// The full `QuoteResponse` produced by the live pipeline (asset_issuer redacted).
    pub original_output: serde_json::Value,
}

/// Lightweight summary returned by list queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactSummary {
    pub id: Uuid,
    pub incident_id: Option<String>,
    pub captured_at: DateTime<Utc>,
    pub base_asset: String,
    pub quote_asset: String,
}

// ---------------------------------------------------------------------------
// Database operations
// ---------------------------------------------------------------------------

impl ReplayArtifact {
    /// Persist a new artifact. Returns the assigned `id`.
    pub async fn insert(db: &PgPool, artifact: &ReplayArtifact) -> Result<Uuid> {
        let payload = serde_json::to_value(artifact).map_err(|e| {
            ApiError::Internal(anyhow::anyhow!("Failed to serialize artifact: {}", e).into())
        })?;

        let row = sqlx::query(
            r#"
            INSERT INTO replay_artifacts (
                id, schema_version, incident_id, captured_at,
                base_asset, quote_asset, amount, slippage_bps, quote_type,
                artifact
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id
            "#,
        )
        .bind(artifact.id)
        .bind(artifact.schema_version as i32)
        .bind(&artifact.incident_id)
        .bind(artifact.captured_at)
        .bind(&artifact.base)
        .bind(&artifact.quote)
        .bind(&artifact.amount)
        .bind(artifact.slippage_bps as i32)
        .bind(&artifact.quote_type)
        .bind(payload)
        .fetch_one(db)
        .await
        .map_err(|e| {
            ApiError::Internal(anyhow::anyhow!("Failed to insert artifact: {}", e).into())
        })?;

        Ok(row.get("id"))
    }

    /// Fetch a single artifact by ID. Returns `ApiError::NotFound` if absent.
    pub async fn fetch(db: &PgPool, id: Uuid) -> Result<ReplayArtifact> {
        let row = sqlx::query(r#"SELECT artifact FROM replay_artifacts WHERE id = $1"#)
            .bind(id)
            .fetch_optional(db)
            .await
            .map_err(|e| {
                ApiError::Internal(anyhow::anyhow!("Failed to fetch artifact: {}", e).into())
            })?;

        match row {
            None => Err(ApiError::NotFound(format!(
                "Replay artifact not found: {}",
                id
            ))),
            Some(r) => {
                let json: serde_json::Value = r.get("artifact");
                serde_json::from_value(json).map_err(|e| {
                    ApiError::Internal(
                        anyhow::anyhow!("Failed to deserialize artifact: {}", e).into(),
                    )
                })
            }
        }
    }

    /// List artifacts with optional filters. Returns summaries ordered by `captured_at DESC`.
    pub async fn list(
        db: &PgPool,
        incident_id: Option<&str>,
        base: Option<&str>,
        quote: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ArtifactSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT id, incident_id, captured_at, base_asset, quote_asset
            FROM replay_artifacts
            WHERE ($1::text IS NULL OR incident_id = $1)
              AND ($2::text IS NULL OR base_asset = $2)
              AND ($3::text IS NULL OR quote_asset = $3)
            ORDER BY captured_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(incident_id)
        .bind(base)
        .bind(quote)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await
        .map_err(|e| {
            ApiError::Internal(anyhow::anyhow!("Failed to list artifacts: {}", e).into())
        })?;

        Ok(rows
            .into_iter()
            .map(|r| ArtifactSummary {
                id: r.get("id"),
                incident_id: r.get("incident_id"),
                captured_at: r.get("captured_at"),
                base_asset: r.get("base_asset"),
                quote_asset: r.get("quote_asset"),
            })
            .collect())
    }

    /// Delete artifacts older than `retention`. Returns the number of rows deleted.
    pub async fn prune_older_than(db: &PgPool, retention: Duration) -> Result<u64> {
        let cutoff = Utc::now() - retention;
        let result = sqlx::query(r#"DELETE FROM replay_artifacts WHERE captured_at < $1"#)
            .bind(cutoff)
            .execute(db)
            .await
            .map_err(|e| {
                ApiError::Internal(anyhow::anyhow!("Failed to prune artifacts: {}", e).into())
            })?;

        Ok(result.rows_affected())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn make_artifact(base: &str, quote: &str, amount: &str) -> ReplayArtifact {
        ReplayArtifact {
            id: Uuid::new_v4(),
            schema_version: CURRENT_SCHEMA_VERSION,
            incident_id: None,
            captured_at: Utc::now(),
            base: base.to_string(),
            quote: quote.to_string(),
            amount: amount.to_string(),
            slippage_bps: 50,
            quote_type: "sell".to_string(),
            liquidity_snapshot: vec![LiquidityCandidate {
                venue_type: "sdex".to_string(),
                venue_ref: "offer1".to_string(),
                price: "1.0000000".to_string(),
                available_amount: "100.0000000".to_string(),
            }],
            health_config_snapshot: HealthConfigSnapshot {
                freshness_threshold_secs_sdex: 30,
                freshness_threshold_secs_amm: 60,
                staleness_threshold_secs: 30,
                min_tvl_threshold_e7: 1_000_000_000,
            },
            original_output: serde_json::json!({
                "price": "1.0000000",
                "selected_source": "sdex:offer1"
            }),
        }
    }

    #[test]
    fn artifact_serde_round_trip_unit() {
        let artifact = make_artifact("native", "USDC", "100.0000000");
        let json = serde_json::to_string(&artifact).expect("serialize");
        let back: ReplayArtifact = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(artifact.id, back.id);
        assert_eq!(artifact.base, back.base);
        assert_eq!(artifact.schema_version, back.schema_version);
        assert_eq!(artifact.liquidity_snapshot, back.liquidity_snapshot);
    }

    #[test]
    fn artifact_schema_version_is_current() {
        let artifact = make_artifact("native", "USDC", "1.0000000");
        assert_eq!(artifact.schema_version, CURRENT_SCHEMA_VERSION);
    }

    prop_compose! {
        /// Arbitrary liquidity candidate for property tests.
        fn arb_candidate()(
            venue_type in prop::sample::select(vec!["sdex", "amm"]),
            venue_ref in "[a-z0-9]{4,16}",
            price_int in 1u64..1_000_000u64,
            amount_int in 1u64..1_000_000u64,
        ) -> LiquidityCandidate {
            LiquidityCandidate {
                venue_type: venue_type.to_string(),
                venue_ref,
                price: format!("{:.7}", price_int as f64 / 1_000_000.0),
                available_amount: format!("{:.7}", amount_int as f64 / 1_000_000.0),
            }
        }
    }

    prop_compose! {
        /// Arbitrary ReplayArtifact for property tests.
        fn arb_artifact()(
            base in prop::sample::select(vec!["native", "USDC", "BTC"]),
            quote in prop::sample::select(vec!["native", "USDC", "ETH"]),
            amount_int in 1u64..1_000_000u64,
            candidates in prop::collection::vec(arb_candidate(), 1..10),
        ) -> ReplayArtifact {
            let amount = format!("{:.7}", amount_int as f64 / 100.0);
            let price = candidates[0].price.clone();
            let source = format!("{}:{}", candidates[0].venue_type, candidates[0].venue_ref);
            ReplayArtifact {
                id: Uuid::nil(), // deterministic for tests
                schema_version: CURRENT_SCHEMA_VERSION,
                incident_id: None,
                captured_at: Utc::now(),
                base: base.to_string(),
                quote: quote.to_string(),
                amount: amount.clone(),
                slippage_bps: 50,
                quote_type: "sell".to_string(),
                liquidity_snapshot: candidates,
                health_config_snapshot: HealthConfigSnapshot {
                    freshness_threshold_secs_sdex: 30,
                    freshness_threshold_secs_amm: 60,
                    staleness_threshold_secs: 30,
                    min_tvl_threshold_e7: 1_000_000_000,
                },
                original_output: serde_json::json!({
                    "price": price,
                    "selected_source": source,
                }),
            }
        }
    }

    proptest! {
        /// Property 1 (partial): artifact serde round-trip preserves all fields.
        ///
        /// Feature: quote-replay-system, Property 1: artifact structural invariants
        #[test]
        fn prop_artifact_serde_round_trip(artifact in arb_artifact()) {
            let json = serde_json::to_string(&artifact).expect("serialize");
            let back: ReplayArtifact = serde_json::from_str(&json).expect("deserialize");
            prop_assert_eq!(artifact.schema_version, back.schema_version);
            prop_assert_eq!(&artifact.base, &back.base);
            prop_assert_eq!(&artifact.quote, &back.quote);
            prop_assert_eq!(&artifact.amount, &back.amount);
            prop_assert_eq!(artifact.liquidity_snapshot.len(), back.liquidity_snapshot.len());
        }

        /// Property 1 (structural): schema_version equals CURRENT_SCHEMA_VERSION.
        ///
        /// Feature: quote-replay-system, Property 1: artifact structural invariants
        #[test]
        fn prop_artifact_has_required_fields(artifact in arb_artifact()) {
            prop_assert_eq!(artifact.schema_version, CURRENT_SCHEMA_VERSION);
            prop_assert!(!artifact.base.is_empty());
            prop_assert!(!artifact.quote.is_empty());
            prop_assert!(!artifact.amount.is_empty());
        }
    }
}
