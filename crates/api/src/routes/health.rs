//! Health check endpoint

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use std::{collections::HashMap, sync::Arc};
use tracing::warn;

use crate::{
    middleware::RequestId,
    models::{ApiResponse, DependenciesHealthResponse, HealthResponse},
    state::AppState,
};

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

    // --- Redis (optional) ---
    let redis_status = if let Some(cache) = &state.cache {
        match cache.try_lock() {
            Ok(mut guard) => {
                if guard.is_healthy().await {
                    "healthy".to_string()
                } else {
                    warn!("Redis health check failed");
                    all_healthy = false;
                    "unhealthy".to_string()
                }
            }
            Err(_) => {
                // Lock contention — treat as healthy rather than a false alert
                "healthy".to_string()
            }
        }
    } else {
        // Redis not configured — report as not_configured so callers know
        "not_configured".to_string()
    };
    components.insert("redis".to_string(), redis_status);

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
    (http_status, Json(envelope)).into_response()
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
    let db_status = match sqlx::query("SELECT 1").execute(state.db.read_pool()).await {
        Ok(_) => "healthy".to_string(),
        Err(e) => {
            warn!("Dependency DB health check failed: {}", e);
            all_ok = false;
            "degraded".to_string()
        }
    };
    components.insert("database".to_string(), db_status);

    // --- Redis (optional) ---
    let redis_status = if let Some(cache) = &state.cache {
        match cache.try_lock() {
            Ok(mut guard) => {
                if guard.is_healthy().await {
                    "healthy".to_string()
                } else {
                    all_ok = false;
                    "degraded".to_string()
                }
            }
            Err(_) => {
                // Lock contention — treat as healthy rather than a false alert
                "healthy".to_string()
            }
        }
    } else {
        "not_configured".to_string()
    };
    components.insert("redis".to_string(), redis_status);

    // --- Horizon / Soroban RPC ---
    // Keep this lightweight: if you want active probes, you can wire them up
    // similarly to the earlier implementation using `reqwest`.
    components.insert("horizon".to_string(), "not_configured".to_string());
    components.insert("soroban_rpc".to_string(), "not_configured".to_string());

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
    (http_status, Json(envelope)).into_response()
}
