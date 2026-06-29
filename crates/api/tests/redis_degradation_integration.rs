//! Graceful degradation tests for Redis outages.
//!
//! These tests simulate Redis infrastructure failures without requiring a live
//! Redis instance. Read paths must fall back to compute/DB semantics while
//! health and metrics reflect cache subsystem degradation.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::from_fn,
    routing::{get, post},
    Router,
};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use stellarroute_api::{
    cache::{CacheHealthStatus, CacheManager, CacheResult},
    middleware::request_id_layer,
    routes::{admin, health, metrics},
    state::{AppState, CachePolicy, DatabasePools},
};
use tower::ServiceExt;

fn outage_test_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health::health_check))
        .route("/metrics/cache", get(metrics::cache_metrics))
        .route(
            "/api/v1/admin/cache/flush/:base/:quote",
            post(admin::flush_cache),
        )
        .layer(from_fn(request_id_layer))
        .with_state(state)
}

fn outage_cache_manager() -> CacheManager {
    CacheManager::simulated_outage()
}

#[tokio::test]
async fn redis_outage_cache_reads_return_unavailable_without_panicking() {
    let mut cache = outage_cache_manager();
    let outcome = cache.get::<String>("pairs:list").await;
    assert_eq!(outcome, CacheResult::Unavailable);
    assert_eq!(cache.health_status().await, CacheHealthStatus::Degraded);
}

#[tokio::test]
async fn redis_outage_delete_by_pattern_returns_error_without_crashing() {
    let mut cache = outage_cache_manager();
    let result = cache
        .delete_by_pattern("*quote:*")
        .await
        .expect_err("delete should surface redis error");
    assert!(stellarroute_api::cache::is_redis_infrastructure_error(
        &result
    ));
}

#[tokio::test]
async fn health_reports_redis_degraded_while_service_stays_healthy() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/postgres")
        .expect("lazy pool");
    let db = DatabasePools::new(pool, None);

    let cache = outage_cache_manager();
    let state = Arc::new(AppState::with_cache_and_policy(
        db,
        cache,
        CachePolicy::default(),
    ));
    let router = outage_test_router(state);

    let response = router
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("health request failed");

    let status = response.status();

    // Redis is optional — degraded cache must not flip the service to 503 when DB is healthy.
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: Value = serde_json::from_slice(&body).expect("json");
    let components = json["data"]["components"]
        .as_object()
        .expect("components object");

    assert_eq!(
        components.get("redis").and_then(|v| v.as_str()),
        Some("degraded")
    );

    if status == StatusCode::OK {
        assert_eq!(json["data"]["status"], "healthy");
    }
}

#[tokio::test]
async fn admin_cache_flush_survives_redis_outage() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/postgres")
        .expect("lazy pool");
    let db = DatabasePools::new(pool, None);

    let cache = outage_cache_manager();
    let state = Arc::new(
        AppState::with_cache_and_policy(db, cache, CachePolicy::default())
            .with_admin_auth_token("test-secret"),
    );
    let router = outage_test_router(state);

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/cache/flush/XLM/USDC")
                .header("Authorization", "Bearer test-secret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("flush request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(json["total_deleted"], 0);
}

#[tokio::test]
async fn cache_metrics_endpoint_exposes_redis_error_counter() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/postgres")
        .expect("lazy pool");
    let db = DatabasePools::new(pool, None);

    let cache = outage_cache_manager();
    let state = Arc::new(AppState::with_cache_and_policy(
        db,
        cache,
        CachePolicy::default(),
    ));
    let router = outage_test_router(state.clone());

    {
        let mut cache_lock = state.cache.as_ref().unwrap().lock().await;
        let _ = cache_lock.get::<String>("warmup").await;
    }

    let response = router
        .oneshot(
            Request::builder()
                .uri("/metrics/cache")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("metrics request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: Value = serde_json::from_slice(&body).expect("json");
    assert!(
        json["redis_errors"].as_u64().unwrap_or(0) > 0,
        "redis_errors should be exposed separately from cache misses"
    );
}
