//! Replay API endpoints.
//!
//! - `GET  /api/v1/replay`                  — list artifacts
//! - `GET  /api/v1/replay/:id`              — fetch artifact
//! - `POST /api/v1/replay/:id/run`          — run replay pipeline
//! - `POST /api/v1/replay/:id/diff`         — run replay + diff

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    error::{ApiError, Result},
    replay::{
        artifact::{ArtifactSummary, ReplayArtifact},
        diff::{DiffEngine, DiffReport},
        engine::{ReplayEngine, ReplayOutput},
    },
    state::AppState,
};

// ---------------------------------------------------------------------------
// Query params
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ListParams {
    pub incident_id: Option<String>,
    pub base: Option<String>,
    pub quote: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

// ---------------------------------------------------------------------------
// Response wrappers
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
pub struct ListResponse {
    pub artifacts: Vec<ArtifactSummary>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// List stored replay artifacts with optional filters.
pub async fn list_artifacts(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListParams>,
) -> Result<Json<ListResponse>> {
    let limit = params.limit.clamp(1, 100);
    let artifacts = ReplayArtifact::list(
        &state.db,
        params.incident_id.as_deref(),
        params.base.as_deref(),
        params.quote.as_deref(),
        limit,
        params.offset,
    )
    .await?;

    Ok(Json(ListResponse { artifacts }))
}

/// Fetch a single replay artifact by ID.
pub async fn get_artifact(
    State(state): State<Arc<AppState>>,
    Path(artifact_id): Path<String>,
) -> Result<Json<ReplayArtifact>> {
    let id = parse_uuid(&artifact_id)?;
    let artifact = ReplayArtifact::fetch(&state.db, id).await?;
    Ok(Json(artifact))
}

/// Run the replay pipeline for a stored artifact and return the output.
pub async fn run_replay(
    State(state): State<Arc<AppState>>,
    Path(artifact_id): Path<String>,
) -> Result<Json<ReplayOutput>> {
    let id = parse_uuid(&artifact_id)?;
    let artifact = ReplayArtifact::fetch(&state.db, id).await?;
    let output = ReplayEngine::run(&artifact)?;
    Ok(Json(output))
}

/// Run the replay pipeline and diff the output against the stored original.
pub async fn diff_replay(
    State(state): State<Arc<AppState>>,
    Path(artifact_id): Path<String>,
) -> Result<Json<DiffReport>> {
    let id = parse_uuid(&artifact_id)?;
    let artifact = ReplayArtifact::fetch(&state.db, id).await?;
    let output = ReplayEngine::run(&artifact)?;
    let report = DiffEngine::diff(&artifact, &output);
    Ok(Json(report))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_uuid(s: &str) -> Result<Uuid> {
    Uuid::parse_str(s).map_err(|_| ApiError::Validation(format!("Invalid artifact ID: {}", s)))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_uuid_valid() {
        let id = Uuid::new_v4();
        assert_eq!(parse_uuid(&id.to_string()).unwrap(), id);
    }

    #[test]
    fn parse_uuid_invalid_returns_validation_error() {
        let err = parse_uuid("not-a-uuid").unwrap_err();
        assert!(matches!(err, ApiError::Validation(_)));
    }

    #[test]
    fn list_params_default_limit() {
        let params: ListParams = serde_json::from_str("{}").unwrap();
        assert_eq!(params.limit, 20);
        assert_eq!(params.offset, 0);
    }
}
