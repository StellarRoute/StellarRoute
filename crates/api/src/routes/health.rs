//! Health check endpoint

use axum::{
    extract::State,
    http::{HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::{collections::HashMap, sync::Arc};
use tracing::warn;

use crate::{
    cache::CacheHealthStatus,
    indexer_lag::{LagSnapshot, SyncStatus},
    middleware::RequestId,
    models::{ApiResponse, DependenciesHealthResponse, HealthResponse},
    state::AppState,
};

/// Response header carrying the aggregate indexer data freshness
/// (`ok` | `warning` | `critical` | `unknown`).
///
/// Additive, read-only hint so staging/edge probes can gate on indexer
/// freshness without parsing the JSON body.
pub const INDEXER_FRESHNESS_HEADER: &str = "x-indexer-freshness";

/// Severity ranking used to aggregate per-source freshness.
///
/// `critical` outranks `warning`, which outranks `unknown` (no measurement yet
/// or Horizon unreachable), which outranks `ok`.
fn freshness_severity(status: SyncStatus) -> u8 {
    match status {
        SyncStatus::Ok => 0,
        SyncStatus::Unknown => 1,
        SyncStatus::Warning => 2,
        SyncStatus::Critical => 3,
    }
}

/// Aggregate indexer freshness across all monitored sources — the worst
/// (most degraded) status wins.  With no snapshots yet freshness is unknown.
fn aggregate_indexer_freshness(snapshots: &[LagSnapshot]) -> SyncStatus {
    snapshots
        .iter()
        .map(|snap| snap.status)
        .max_by_key(|status| freshness_severity(*status))
        .unwrap_or(SyncStatus::Unknown)
}

/// Attach the indexer-freshness probe header to a health response.
fn apply_indexer_freshness_headers(response: &mut Response, snapshots: &[LagSnapshot]) {
    let freshness = aggregate_indexer_freshness(snapshots);
    if let Ok(value) = HeaderValue::from_str(freshness.as_str()) {
        response
            .headers_mut()
            .insert(HeaderName::from_static(INDEXER_FRESHNESS_HEADER), value);
    }
}

/// Evaluate optional Redis cache health for component maps.
///
/// Redis is a performance cache, not a required dependency. A degraded cache
/// must be reported but must not flip overall service health to unhealthy.
async fn probe_redis_status(state: &AppState) -> CacheHealthStatus {
    let Some(cache) = &state.cache else {
        return CacheHealthStatus::NotConfigured;
    };

    match cache.try_lock() {
        Ok(mut guard) => guard.health_status().await,
        Err(_) => CacheHealthStatus::Healthy,
    }
}

/// Health check endpoint
///
/// Probes PostgreSQL and Redis (if configured) and returns per-component
/// statuses.  Returns **200 OK** when everything is healthy, **503
/// Service Unavailable** when any required dependency is down.
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "All dependencies healthy", body = HealthResponse),
        (status = 503, description = "One or more dependencies unhealthy", body = HealthResponse),
    )
)]
pub async fn health_check(
    State(state): State<Arc<AppState>>,
    request_id: RequestId,
) -> impl IntoResponse {
    let timestamp = chrono::Utc::now().to_rfc3339();
    let mut components: HashMap<String, String> = HashMap::new();
    let mut all_healthy = true;

    // --- PostgreSQL ---
    let db_status = match sqlx::query("SELECT 1").execute(state.db.read_pool()).await {
        Ok(_) => "healthy".to_string(),
        Err(e) => {
            warn!("Database health check failed: {}", e);
            all_healthy = false;
            "unhealthy".to_string()
        }
    };
    components.insert("database".to_string(), db_status);

    // --- Redis (optional performance cache) ---
    let redis_status = probe_redis_status(&state).await;
    components.insert(
        "redis".to_string(),
        redis_status.as_component_status().to_string(),
    );
    if redis_status == CacheHealthStatus::Degraded {
        warn!("Redis cache subsystem degraded during health check");
    }

    // --- Indexer lag ---
    let lag_snapshots = state.indexer_lag.snapshots().await;
    for snap in &lag_snapshots {
        let component_key = format!("indexer_lag_{}", snap.source);
        let component_val = match snap.status {
            crate::indexer_lag::SyncStatus::Ok => "healthy".to_string(),
            crate::indexer_lag::SyncStatus::Warning => {
                warn!(
                    source = %snap.source,
                    lag_ledgers = snap.lag_ledgers,
                    "Indexer lag elevated during health check"
                );
                // Warning does not flip all_healthy — it's a soft signal
                format!("warning (lag: {} ledgers)", snap.lag_ledgers)
            }
            crate::indexer_lag::SyncStatus::Critical => {
                warn!(
                    source = %snap.source,
                    lag_ledgers = snap.lag_ledgers,
                    "Indexer lag CRITICAL during health check"
                );
                all_healthy = false;
                format!("unhealthy (lag: {} ledgers)", snap.lag_ledgers)
            }
            crate::indexer_lag::SyncStatus::Unknown => "unknown".to_string(),
        };
        components.insert(component_key, component_val);
    }

    let status = if all_healthy {
        "healthy".to_string()
    } else {
        "unhealthy".to_string()
    };

    let body = HealthResponse {
        status,
        timestamp,
        version: state.version.clone(),
        components,
    };

    let http_status = if all_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let envelope = ApiResponse::new(body, request_id.to_string());
    let mut response = (http_status, Json(envelope)).into_response();
    apply_indexer_freshness_headers(&mut response, &lag_snapshots);
    response
}

/// Dependency readiness check for infrastructure and external providers.
#[utoipa::path(
    get,
    path = "/health/deps",
    tag = "health",
    responses(
        (status = 200, description = "Dependencies healthy", body = DependenciesHealthResponse),
        (status = 503, description = "One or more dependencies degraded", body = DependenciesHealthResponse),
    )
)]
pub async fn dependency_health(
    State(state): State<Arc<AppState>>,
    request_id: RequestId,
) -> impl IntoResponse {
    let timestamp = chrono::Utc::now().to_rfc3339();
    let mut components: HashMap<String, String> = HashMap::new();
    let mut all_ok = true;

    // --- PostgreSQL ---
    // Feeds the database circuit breaker so the live path can fail fast rather
    // than queue behind an unreachable Postgres.
    let db_status = match sqlx::query("SELECT 1").execute(state.db.read_pool()).await {
        Ok(_) => {
            state
                .external_dependency_health
                .record_database_result(true);
            "healthy".to_string()
        }
        Err(e) => {
            warn!("Dependency DB health check failed: {}", e);
            state
                .external_dependency_health
                .record_database_result(false);
            all_ok = false;
            "degraded".to_string()
        }
    };
    components.insert("database".to_string(), db_status);

    // --- Redis (optional performance cache) ---
    let redis_status = probe_redis_status(&state).await;
    components.insert(
        "redis".to_string(),
        redis_status.as_component_status().to_string(),
    );

    // --- Horizon / Soroban RPC ---
    let horizon_status = state.external_dependency_health.probe_horizon().await;
    if horizon_status.starts_with("degraded") {
        all_ok = false;
    }
    components.insert("horizon".to_string(), horizon_status);

    let soroban_status = state.external_dependency_health.probe_soroban().await;
    if soroban_status.starts_with("degraded") {
        all_ok = false;
    }
    components.insert("soroban_rpc".to_string(), soroban_status);

    // --- Indexer lag ---
    let lag_snapshots = state.indexer_lag.snapshots().await;
    for snap in &lag_snapshots {
        let component_key = format!("indexer_lag_{}", snap.source);
        let component_val = match snap.status {
            crate::indexer_lag::SyncStatus::Ok => {
                format!("ok (lag: {} ledgers)", snap.lag_ledgers)
            }
            crate::indexer_lag::SyncStatus::Warning => {
                format!(
                    "warning (lag: {} ledgers, {:.0}s)",
                    snap.lag_ledgers, snap.lag_seconds
                )
            }
            crate::indexer_lag::SyncStatus::Critical => {
                all_ok = false;
                format!(
                    "degraded (lag: {} ledgers, {:.0}s)",
                    snap.lag_ledgers, snap.lag_seconds
                )
            }
            crate::indexer_lag::SyncStatus::Unknown => "unknown".to_string(),
        };
        components.insert(component_key, component_val);
    }

    let status = if all_ok { "ok" } else { "degraded" }.to_string();
    let body = DependenciesHealthResponse {
        status,
        timestamp,
        components,
    };

    let http_status = if all_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let envelope = ApiResponse::new(body, request_id.to_string());
    let mut response = (http_status, Json(envelope)).into_response();
    apply_indexer_freshness_headers(&mut response, &lag_snapshots);
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer_lag::LagThresholds;

    fn snapshot(source: &str, last_indexed_ledger: u64, horizon_ledger: u64) -> LagSnapshot {
        LagSnapshot::compute(
            source,
            last_indexed_ledger,
            horizon_ledger,
            &LagThresholds::default(),
        )
    }

    #[test]
    fn freshness_is_unknown_without_snapshots() {
        assert_eq!(aggregate_indexer_freshness(&[]), SyncStatus::Unknown);
    }

    #[test]
    fn freshness_is_ok_when_all_sources_are_ok() {
        let snaps = vec![snapshot("sdex", 995, 1000), snapshot("amm", 992, 1000)];
        assert_eq!(aggregate_indexer_freshness(&snaps), SyncStatus::Ok);
    }

    #[test]
    fn freshness_takes_the_worst_status() {
        let snaps = vec![snapshot("sdex", 995, 1000), snapshot("amm", 900, 1000)];
        assert_eq!(aggregate_indexer_freshness(&snaps), SyncStatus::Critical);
    }

    #[test]
    fn freshness_warning_outranks_ok_regardless_of_order() {
        let warning_first = vec![snapshot("sdex", 990, 1000), snapshot("amm", 995, 1000)];
        assert_eq!(
            aggregate_indexer_freshness(&warning_first),
            SyncStatus::Warning
        );

        let ok_first = vec![snapshot("sdex", 995, 1000), snapshot("amm", 990, 1000)];
        assert_eq!(aggregate_indexer_freshness(&ok_first), SyncStatus::Warning);
    }

    #[test]
    fn freshness_unknown_outranks_ok() {
        let snaps = vec![
            snapshot("sdex", 995, 1000),
            LagSnapshot {
                status: SyncStatus::Unknown,
                ..snapshot("amm", 0, 0)
            },
        ];
        assert_eq!(aggregate_indexer_freshness(&snaps), SyncStatus::Unknown);
    }

    #[test]
    fn applies_freshness_header_to_response() {
        let mut response = (StatusCode::OK, Json(serde_json::json!({}))).into_response();
        apply_indexer_freshness_headers(&mut response, &[snapshot("sdex", 995, 1000)]);

        let value = response
            .headers()
            .get(INDEXER_FRESHNESS_HEADER)
            .and_then(|v| v.to_str().ok());
        assert_eq!(value, Some("ok"));
    }

    #[test]
    fn applies_unknown_header_when_no_snapshots() {
        let mut response = (StatusCode::OK, Json(serde_json::json!({}))).into_response();
        apply_indexer_freshness_headers(&mut response, &[]);

        let value = response
            .headers()
            .get(INDEXER_FRESHNESS_HEADER)
            .and_then(|v| v.to_str().ok());
        assert_eq!(value, Some("unknown"));
    }
}
