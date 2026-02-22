//! Health check endpoint

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use std::{collections::HashMap, sync::Arc};
use tracing::warn;

use crate::{models::HealthResponse, state::AppState};

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
pub async fn health_check(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let timestamp = chrono::Utc::now().to_rfc3339();
    let mut components: HashMap<String, String> = HashMap::new();
    let mut all_healthy = true;

    // --- PostgreSQL ---
    let db_status = match sqlx::query("SELECT 1").execute(&state.db).await {
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

    (http_status, Json(body)).into_response()
}
