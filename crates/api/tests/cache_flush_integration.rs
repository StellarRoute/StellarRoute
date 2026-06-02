//! Integration tests for the admin cache flush endpoint.

use axum::{body::Body, http::{Request, StatusCode}};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use std::{sync::Arc, time::Duration};
use stellarroute_api::{
    cache::{self, CacheManager},
    models::{AssetInfo, OrderbookResponse, QuoteResponse},
    routes,
    state::{AppState, CachePolicy},
};
use tower::ServiceExt;

#[tokio::test]
#[ignore = "requires a Redis instance available at REDIS_URL or localhost:6379"]
async fn admin_cache_flush_removes_cached_pair_entries() {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string());
    let cache = match CacheManager::new(&redis_url).await {
        Ok(cache) => cache,
        Err(err) => {
            eprintln!("Skipping admin cache flush test because Redis is unavailable: {}", err);
            return;
        }
    };

    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/postgres")
        .expect("failed to create lazy DB pool");

    let state = Arc::new(
        AppState::with_cache_and_policy(pool, cache.clone(), CachePolicy::default())
            .with_admin_auth_token("test-secret"),
    );
    let router = routes::create_router(state.clone());

    let quote_key = cache::keys::quote("XLM", "USDC", "1", 50, "sell", false);
    let orderbook_key = cache::keys::orderbook("XLM", "USDC");

    let quote_value = QuoteResponse {
        base_asset: AssetInfo::native(),
        quote_asset: AssetInfo::credit("USDC".to_string(), None),
        amount: "1".to_string(),
        price: "0.5".to_string(),
        total: "0.5".to_string(),
        quote_type: "sell".to_string(),
        path: Vec::new(),
        timestamp: 0,
        expires_at: None,
        source_timestamp: None,
        ttl_seconds: None,
        rationale: None,
        price_impact: None,
        exclusion_diagnostics: None,
        data_freshness: None,
    };

    let orderbook_value = OrderbookResponse {
        base_asset: AssetInfo::native(),
        quote_asset: AssetInfo::credit("USDC".to_string(), None),
        bids: Vec::new(),
        asks: Vec::new(),
        timestamp: 0,
    };

    {
        let mut cache_lock = state.cache.as_ref().unwrap().lock().await;
        cache_lock
            .set(&quote_key, &quote_value, Duration::from_secs(30))
            .await
            .expect("failed to set quote cache");
        cache_lock
            .set(&orderbook_key, &orderbook_value, Duration::from_secs(30))
            .await
            .expect("failed to set orderbook cache");
    }

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
        .expect("Request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Body read failed");
    let json: Value = serde_json::from_slice(&body).expect("Invalid JSON");

    assert_eq!(json["deleted_quote_keys"].as_u64(), Some(1));
    assert_eq!(json["deleted_orderbook_keys"].as_u64(), Some(1));
    assert_eq!(json["total_deleted"].as_u64(), Some(2));

    let mut cache_lock = state.cache.as_ref().unwrap().lock().await;
    assert!(cache_lock.get::<OrderbookResponse>(&orderbook_key).await.is_none());
    assert!(cache_lock.get::<QuoteResponse>(&quote_key).await.is_none());
}
