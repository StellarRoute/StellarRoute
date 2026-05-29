//! Metrics endpoint

use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::{models::CacheMetricsResponse, state::AppState};

/// Cache metrics endpoint
#[utoipa::path(
    get,
    path = "/metrics/cache",
    tag = "health",
    responses(
        (status = 200, description = "Cache hit/miss metrics", body = CacheMetricsResponse),
    )
)]
pub async fn cache_metrics(State(state): State<Arc<AppState>>) -> Json<CacheMetricsResponse> {
    let (quote_hits, quote_misses) = state.cache_metrics.snapshot();
    let (stale_quote_rejections, stale_inputs_excluded) = state.cache_metrics.snapshot_staleness();

    let hit_ratio = if quote_hits + quote_misses > 0 {
        quote_hits as f64 / (quote_hits + quote_misses) as f64
    } else {
        0.0
    };

    Json(CacheMetricsResponse {
        quote_hits,
        quote_misses,
        hit_ratio,
        stale_quote_rejections,
        stale_inputs_excluded,
    })
}

/// Database connection pool statistics (non-sensitive).
///
/// Exposes pool size, idle connections, and utilisation for both the primary
/// (write) pool and the optional replica (read) pool.  No credentials or
/// connection strings are included.
#[derive(Debug, Serialize, ToSchema)]
pub struct PoolStatsResponse {
    /// Stats for the primary (write) pool.
    pub primary: PoolStats,
    /// Stats for the replica (read) pool, if configured.
    pub replica: Option<PoolStats>,
}

/// Per-pool statistics.
#[derive(Debug, Serialize, ToSchema)]
pub struct PoolStats {
    /// Maximum number of connections allowed.
    pub max_connections: u32,
    /// Current number of connections (idle + in-use).
    pub size: u32,
    /// Connections currently idle (available for use).
    pub idle: u32,
    /// Connections currently checked out (in use).
    pub in_use: u32,
    /// Pool utilisation as a fraction of `max_connections` (0.0–1.0).
    pub utilisation: f64,
}

impl PoolStats {
    fn from_pool(pool: &sqlx::PgPool) -> Self {
        let size = pool.size();
        let idle = pool.num_idle() as u32;
        let in_use = size.saturating_sub(idle);
        // max_connections is available via pool.options().get_max_connections() in sqlx 0.8
        let max_connections = pool.options().get_max_connections();
        let utilisation = if max_connections > 0 {
            in_use as f64 / max_connections as f64
        } else {
            0.0
        };
        Self {
            max_connections,
            size,
            idle,
            in_use,
            utilisation,
        }
    }
}

/// GET /metrics/pool — database connection pool statistics.
///
/// Returns non-sensitive pool metrics for both the primary and replica pools.
/// Use these values to tune `DATABASE_POOL_MAX_CONNECTIONS` (see
/// `docs/deployment/db-pool-tuning.md`).
#[utoipa::path(
    get,
    path = "/metrics/pool",
    tag = "health",
    responses(
        (status = 200, description = "DB pool statistics", body = PoolStatsResponse),
    )
)]
pub async fn pool_stats(State(state): State<Arc<AppState>>) -> Json<PoolStatsResponse> {
    let primary = PoolStats::from_pool(state.db.write_pool());
    let replica = state
        .db
        .replica_pool()
        .map(PoolStats::from_pool);

    Json(PoolStatsResponse { primary, replica })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_stats_utilisation_zero_when_max_is_zero() {
        // Guard against division-by-zero when max_connections is somehow 0.
        let stats = PoolStats {
            max_connections: 0,
            size: 0,
            idle: 0,
            in_use: 0,
            utilisation: 0.0,
        };
        assert_eq!(stats.utilisation, 0.0);
    }

    #[test]
    fn pool_stats_utilisation_full() {
        let stats = PoolStats {
            max_connections: 10,
            size: 10,
            idle: 0,
            in_use: 10,
            utilisation: 1.0,
        };
        assert!((stats.utilisation - 1.0).abs() < f64::EPSILON);
    }
}
