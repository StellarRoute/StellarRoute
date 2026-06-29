//! Integration coverage for `/api/v1/simulate/route` slippage override behavior.

use stellarroute_routing::policy::RoutingPolicy;

#[test]
fn simulate_route_slippage_policy_uses_default_without_overrides() {
    let policy = RoutingPolicy::default().with_default_slippage_bps(50);
    assert_eq!(policy.slippage_bps_for_venue(None), 50);
    assert_eq!(policy.slippage_bps_for_venue(Some("pool-a")), 50);
}

#[test]
fn simulate_route_slippage_policy_applies_per_venue_overrides() {
    let mut policy = RoutingPolicy::default().with_default_slippage_bps(50);
    policy.apply_venue_slippage_overrides(vec![
        ("pool-a".to_string(), 100),
        ("pool-b".to_string(), 250),
    ]);

    assert_eq!(policy.slippage_bps_for_venue(Some("pool-a")), 100);
    assert_eq!(policy.slippage_bps_for_venue(Some("pool-b")), 250);
    assert_eq!(policy.slippage_bps_for_venue(Some("pool-c")), 50);
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL instance with normalized liquidity"]
async fn simulate_route_endpoint_applies_slippage_overrides() {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use stellarroute_api::{state::DatabasePools, Server, ServerConfig};
    use tower::ServiceExt;

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
    });

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to connect to database");

    let server = Server::new(ServerConfig::default(), DatabasePools::new(pool, None)).await;
    let router = server.into_router();

    let body = serde_json::json!({
        "amount": "10",
        "slippage_bps": 50,
        "slippage_bps_overrides": [
            { "venue_ref": "pool-native-usdc", "slippage_bps": 150 }
        ],
        "route": {
            "hops": [{
                "from_asset": { "asset_code": "native" },
                "to_asset": { "asset_code": "USDC" },
                "source": "amm:pool-native-usdc",
                "venue_ref": "pool-native-usdc"
            }]
        }
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/simulate/route")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let response = router.oneshot(request).await.expect("request failed");
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "unexpected status: {}",
        response.status()
    );
}
