//! Venue health score persistence

use sqlx::PgPool;
use tracing::warn;

/// A health score record to persist.
/// Mirrors `stellarroute_routing::health::scorer::HealthRecord` but is
/// defined here to avoid a hard dependency on the routing crate.
pub struct HealthScoreRecord {
    pub venue_ref: String,
    pub venue_type: String, // "sdex" or "amm"
    pub score: f64,
    pub signals: serde_json::Value,
    pub computed_at: chrono::DateTime<chrono::Utc>,
}

/// Writes computed health scores to the `venue_health_scores` table.
pub struct HealthScoreWriter {
    pool: PgPool,
}

impl HealthScoreWriter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a health score record.
    ///
    /// DB errors are logged at `warn` level and returned as `IndexerError::DatabaseQuery`.
    pub async fn write(&self, record: &HealthScoreRecord) -> Result<(), crate::error::IndexerError> {
        let result = sqlx::query(
            r#"
            insert into venue_health_scores (venue_ref, venue_type, score, signals, computed_at)
            values ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(&record.venue_ref)
        .bind(&record.venue_type)
        .bind(record.score)
        .bind(&record.signals)
        .bind(record.computed_at)
        .execute(&self.pool)
        .await;

        if let Err(e) = result {
            warn!(
                venue_ref = %record.venue_ref,
                error = %e,
                "Failed to persist health score; continuing without error"
            );
            return Err(e.into());
        }

        Ok(())
    }
}
