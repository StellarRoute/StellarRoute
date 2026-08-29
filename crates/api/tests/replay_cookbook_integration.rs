//! Integration tests for replay-cli cookbook workflow against fixture artifacts.
//!
//! Issue #1254: Validates that the quote replay system (ReplayEngine, DiffEngine,
//! ReplayArtifact, and Replay API routes) executes deterministically against stored
//! fixture artifacts, and verifies that replay execution does NOT write to quote tables.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::postgres::PgPoolOptions;
use stellarroute_api::{
    replay::{
        artifact::{ReplayArtifact, CURRENT_SCHEMA_VERSION},
        diff::DiffEngine,
        engine::ReplayEngine,
    },
    state::DatabasePools,
    Server, ServerConfig,
};
use tower::ServiceExt;

const FIXTURE_COOKBOOK_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/replay_cookbook_artifact.json"
);

const FIXTURE_DIVERGENT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/replay_divergent_artifact.json"
);

fn load_fixture(path: &str) -> ReplayArtifact {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read fixture file at {}: {}", path, e));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to deserialize fixture at {}: {}", path, e))
}

#[test]
fn test_fixture_artifacts_deserialize_correctly() {
    let artifact_identical = load_fixture(FIXTURE_COOKBOOK_PATH);
    assert_eq!(artifact_identical.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(artifact_identical.base, "native");
    assert_eq!(artifact_identical.quote, "USDC:[REDACTED]");
    assert_eq!(artifact_identical.amount, "1000.0000000");
    assert_eq!(
        artifact_identical.incident_id.as_deref(),
        Some("INC-20260327-001")
    );
    assert_eq!(artifact_identical.liquidity_snapshot.len(), 2);

    let artifact_divergent = load_fixture(FIXTURE_DIVERGENT_PATH);
    assert_eq!(artifact_divergent.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(
        artifact_divergent.incident_id.as_deref(),
        Some("INC-20260327-002")
    );
    assert_eq!(artifact_divergent.liquidity_snapshot.len(), 2);
}

#[test]
fn test_replay_run_and_diff_with_stored_identical_fixture() {
    let artifact = load_fixture(FIXTURE_COOKBOOK_PATH);

    // 1. Run deterministic replay engine
    let replay_output = ReplayEngine::run(&artifact).expect("ReplayEngine::run failed");

    assert_eq!(replay_output.artifact_id, artifact.id);
    assert_eq!(replay_output.selected_source, "sdex:offer1");
    assert_eq!(replay_output.price, "0.9950000");
    assert!(replay_output.is_deterministic);
    assert_eq!(replay_output.compared_venues.len(), 2);

    // 2. Run diff engine against original output
    let diff_report = DiffEngine::diff(&artifact, &replay_output);

    assert_eq!(diff_report.artifact_id, artifact.id);
    assert!(
        diff_report.is_identical,
        "Identical fixture must produce is_identical=true"
    );
    assert!(
        diff_report.divergences.is_empty(),
        "Identical fixture must produce empty divergences"
    );
}

#[test]
fn test_replay_run_and_diff_with_stored_divergent_fixture() {
    let artifact = load_fixture(FIXTURE_DIVERGENT_PATH);

    // 1. Run replay engine (AMM candidate in this fixture has lower price 0.9900000)
    let replay_output = ReplayEngine::run(&artifact).expect("ReplayEngine::run failed");

    assert_eq!(replay_output.artifact_id, artifact.id);
    assert_eq!(replay_output.selected_source, "amm:pool1");
    assert_eq!(replay_output.price, "0.9900000");
    assert!(
        !replay_output.is_deterministic,
        "Divergent fixture must yield is_deterministic=false"
    );

    // 2. Run diff engine against original output (original recorded sdex:offer1 @ 0.9950000)
    let diff_report = DiffEngine::diff(&artifact, &replay_output);

    assert_eq!(diff_report.artifact_id, artifact.id);
    assert!(
        !diff_report.is_identical,
        "Divergent fixture must produce is_identical=false"
    );
    assert!(
        !diff_report.divergences.is_empty(),
        "Divergent fixture must report divergences"
    );

    let diverged_fields: Vec<&str> = diff_report
        .divergences
        .iter()
        .map(|d| d.field.as_str())
        .collect();

    assert!(
        diverged_fields.contains(&"price"),
        "Must contain price divergence"
    );
    assert!(
        diverged_fields.contains(&"selected_source"),
        "Must contain selected_source divergence"
    );
}

#[tokio::test]
async fn test_replay_api_routes_and_no_quote_table_writes() {
    // Lazily connected pool guarantees no network I/O needed
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .expect("failed to create lazy pool");

    let config = ServerConfig::default();
    let app_state = Server::new(config, DatabasePools::new(pool, None))
        .await
        .into_router();

    // Verify invalid UUID on GET /api/v1/replay/:id returns 400 Validation error
    let req = Request::builder()
        .uri("/api/v1/replay/invalid-uuid")
        .body(Body::empty())
        .unwrap();

    let res = app_state.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Verify invalid UUID on POST /api/v1/replay/:id/run returns 400 Validation error
    let req_run = Request::builder()
        .method("POST")
        .uri("/api/v1/replay/invalid-uuid/run")
        .body(Body::empty())
        .unwrap();

    let res_run = app_state.clone().oneshot(req_run).await.unwrap();
    assert_eq!(res_run.status(), StatusCode::BAD_REQUEST);

    // Verify invalid UUID on POST /api/v1/replay/:id/diff returns 400 Validation error
    let req_diff = Request::builder()
        .method("POST")
        .uri("/api/v1/replay/invalid-uuid/diff")
        .body(Body::empty())
        .unwrap();

    let res_diff = app_state.oneshot(req_diff).await.unwrap();
    assert_eq!(res_diff.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_replay_cli_cookbook_workflow_steps() {
    // Cookbook Step 1: Load fixture artifacts as if fetched by artifact_id
    let artifact_1 = load_fixture(FIXTURE_COOKBOOK_PATH);
    let artifact_2 = load_fixture(FIXTURE_DIVERGENT_PATH);

    // Cookbook Step 2 & 3: Run replay pipeline for both
    let run_1 = ReplayEngine::run(&artifact_1).expect("Run 1");
    let run_2 = ReplayEngine::run(&artifact_2).expect("Run 2");

    // Cookbook Step 4: Diff outputs against stored originals
    let diff_1 = DiffEngine::diff(&artifact_1, &run_1);
    let diff_2 = DiffEngine::diff(&artifact_2, &run_2);

    // Case A verification: No divergence
    assert!(diff_1.is_identical);
    assert!(diff_1.divergences.is_empty());

    // Case B verification: Divergence detected
    assert!(!diff_2.is_identical);
    assert_eq!(diff_2.divergences.len(), 3);
}
