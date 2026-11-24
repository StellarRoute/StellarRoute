//! Integration tests for GET /api/v1/route-graph/version.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use stellarroute_api::{
    routes,
    state::{AppState, DatabasePools},
    ApiDoc,
};
use stellarroute_routing::{compaction::CompactedGraph, pathfinder::LiquidityEdge};
use tower::ServiceExt;
use utoipa::OpenApi;

fn lazy_pool() -> sqlx::PgPool {
    PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .expect("failed to create lazy pool")
}

async fn get_json(router: axum::Router, uri: &str) -> (StatusCode, Value) {
    let response = router
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .expect("request failed");
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json = serde_json::from_slice(&body).unwrap();
    (status, json)
}

fn test_graph() -> CompactedGraph {
    CompactedGraph::from_edges(vec![LiquidityEdge {
        from: "native".to_string(),
        to: "USDC:GISSUER".to_string(),
        venue_type: "sdex".to_string(),
        venue_ref: "offer-1".to_string(),
        liquidity: 1_000_000,
        price: 1.25,
        fee_bps: 20,
        ..Default::default()
    }])
}

#[tokio::test]
async fn route_graph_version_returns_current_snapshot_token() {
    let state = AppState::new(DatabasePools::new(lazy_pool(), None));
    state.graph_manager.edges.store(Arc::new(test_graph()));
    let router = routes::create_router(state.into_arc());

    let (status, json) = get_json(router, "/api/v1/route-graph/version").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["v"], 1);
    assert!(json["request_id"].as_str().is_some());

    let data = &json["data"];
    let version = data["version"].as_str().expect("version must be a string");
    let snapshot_hash = data["snapshot_hash"]
        .as_str()
        .expect("snapshot_hash must be a string");

    assert!(version.starts_with("route-graph-v1-"));
    assert!(version.ends_with(snapshot_hash));
    assert_eq!(snapshot_hash.len(), 64);
    assert_eq!(data["asset_count"], 2);
    assert_eq!(data["edge_count"], 1);
    assert!(data["generated_at"].as_i64().is_some());
}

#[test]
fn openapi_documents_route_graph_version_path_and_schema() {
    let spec = serde_json::to_value(ApiDoc::openapi()).expect("OpenAPI serializes to JSON");

    let operation = &spec["paths"]["/api/v1/route-graph/version"]["get"];
    assert!(
        operation.is_object(),
        "GET /api/v1/route-graph/version must be documented"
    );
    assert_eq!(operation["tags"][0], "trading");
    assert!(
        operation["responses"]["200"].is_object(),
        "route graph version endpoint must document a 200 response"
    );

    let schema = &spec["components"]["schemas"]["RouteGraphVersionResponse"];
    assert!(
        schema.is_object(),
        "RouteGraphVersionResponse schema must be registered"
    );
    for field in [
        "version",
        "snapshot_hash",
        "asset_count",
        "edge_count",
        "generated_at",
    ] {
        assert!(
            schema["properties"][field].is_object(),
            "RouteGraphVersionResponse must document `{field}`"
        );
    }
}
