//! Integration tests for asset metadata endpoints

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use sqlx::PgPool;
use stellarroute_api::{state::DatabasePools, Server, ServerConfig};
use tower::ServiceExt;

const FIXTURE_CODE: &str = "TST1306";
const FIXTURE_ISSUER: &str = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";

fn default_db_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
    })
}

async fn live_router(pool: PgPool) -> axum::Router {
    Server::new(ServerConfig::default(), DatabasePools::new(pool, None))
        .await
        .into_router()
}

async fn apply_migrations(pool: &PgPool) {
    let mut transaction = pool.begin().await.expect("migration transaction failed");
    sqlx::query("SELECT pg_advisory_xact_lock(1306)")
        .execute(&mut *transaction)
        .await
        .expect("migration lock failed");

    for (name, migration) in [
        ("0001_init.sql", include_str!("../../indexer/migrations/0001_init.sql")),
        (
            "0002_performance_indexes.sql",
            include_str!("../../indexer/migrations/0002_performance_indexes.sql"),
        ),
        (
            "0003_trading_pairs_and_snapshots.sql",
            include_str!("../../indexer/migrations/0003_trading_pairs_and_snapshots.sql"),
        ),
        (
            "0004_normalized_liquidity.sql",
            include_str!("../../indexer/migrations/0004_normalized_liquidity.sql"),
        ),
        (
            "0005_venue_health_scores.sql",
            include_str!("../../indexer/migrations/0005_venue_health_scores.sql"),
        ),
        (
            "0006_maintenance_policies.sql",
            include_str!("../../indexer/migrations/0006_maintenance_policies.sql"),
        ),
        (
            "0007_backfill_and_normalized_storage.sql",
            include_str!("../../indexer/migrations/0007_backfill_and_normalized_storage.sql"),
        ),
        (
            "0008_soroban_discovery_cursors.sql",
            include_str!("../../indexer/migrations/0008_soroban_discovery_cursors.sql"),
        ),
        (
            "0009_finalize_unified_liquidity.sql",
            include_str!("../../indexer/migrations/0009_finalize_unified_liquidity.sql"),
        ),
        (
            "0010_asset_metadata.sql",
            include_str!("../../indexer/migrations/0010_asset_metadata.sql"),
        ),
        (
            "0011_trace_context_provenance.sql",
            include_str!("../../indexer/migrations/0011_trace_context_provenance.sql"),
        ),
        (
            "0012_contract_swap_activity.sql",
            include_str!("../../indexer/migrations/0012_contract_swap_activity.sql"),
        ),
        (
            "0013_amm_pools.sql",
            include_str!("../../indexer/migrations/0013_amm_pools.sql"),
        ),
        (
            "0014_assets_native_singleton.sql",
            include_str!("../../indexer/migrations/0014_assets_native_singleton.sql"),
        ),
        (
            "0015_swap_prepared_quotes.sql",
            include_str!("../../indexer/migrations/0015_swap_prepared_quotes.sql"),
        ),
    ] {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS _schema_migrations (
                name TEXT PRIMARY KEY,
                applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )",
        )
        .execute(&mut *transaction)
        .await
        .expect("migration ledger setup failed");

        let applied: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM _schema_migrations WHERE name = $1)",
        )
        .bind(name)
        .fetch_one(&mut *transaction)
        .await
        .expect("migration status query failed");

        if !applied {
            sqlx::raw_sql(migration)
                .execute(&mut *transaction)
                .await
                .expect("migration failed");
            sqlx::query("INSERT INTO _schema_migrations (name) VALUES ($1)")
                .bind(name)
                .execute(&mut *transaction)
                .await
                .expect("migration ledger update failed");
        }
    }

    transaction
        .commit()
        .await
        .expect("migration commit failed");
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("failed to read response body");
    serde_json::from_slice(&bytes).expect("response body is not valid JSON")
}

async fn seed_asset_fixture(pool: &PgPool) {
    sqlx::query(
        r#"
        INSERT INTO asset_metadata
            (asset_type, asset_code, asset_issuer, decimals, domain, icon_url, source)
        VALUES ($1, $2, $3, $4, $5, $6, 'manual')
        ON CONFLICT (asset_type, asset_code, asset_issuer) DO UPDATE SET
            decimals = EXCLUDED.decimals,
            domain = EXCLUDED.domain,
            icon_url = EXCLUDED.icon_url,
            source = EXCLUDED.source,
            fetched_at = NOW()
        "#,
    )
    .bind("credit_alphanum4")
    .bind(FIXTURE_CODE)
    .bind(FIXTURE_ISSUER)
    .bind(7_i16)
    .bind("example.com")
    .bind("https://example.com/tst1306.svg")
    .execute(pool)
    .await
    .expect("asset fixture seed failed");
}

async fn cleanup_asset_fixture(pool: &PgPool) {
    sqlx::query("DELETE FROM asset_metadata WHERE asset_code = $1 AND asset_issuer = $2")
        .bind(FIXTURE_CODE)
        .bind(FIXTURE_ISSUER)
        .execute(pool)
        .await
        .expect("asset fixture cleanup failed");
}

#[test]
fn asset_metadata_serializes_to_spec_shape() {
    use stellarroute_api::models::AssetMetadataResponse;

    let meta = AssetMetadataResponse {
        code: "USDC".to_string(),
        issuer: Some("GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string()),
        decimals: 7,
        asset_type: "credit_alphanum4".to_string(),
        display_name: Some("USDC".to_string()),
        icon_url: Some("https://example.com/icon.png".to_string()),
        domain: Some("centre.io".to_string()),
    };

    let json = serde_json::to_value(&meta).expect("serialization failed");

    assert_eq!(json["code"], "USDC");
    assert!(json["issuer"].is_string());
    assert_eq!(json["decimals"], 7);
    assert_eq!(json["asset_type"], "credit_alphanum4");
    assert_eq!(json["display_name"], "USDC");
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn get_native_asset_metadata_returns_200() {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
    });

    let pool = PgPool::connect(&db_url)
        .await
        .expect("Failed to connect to database");

    let router = Server::new(ServerConfig::default(), DatabasePools::new(pool, None))
        .await
        .into_router();

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/assets/XLM")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read body");

    let json: Value = serde_json::from_slice(&body).expect("Body is not valid JSON");

    assert_eq!(json["data"]["code"], "XLM");
    assert_eq!(json["data"]["asset_type"], "native");
    assert_eq!(json["data"]["decimals"], 7);
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn get_bulk_asset_metadata_returns_200() {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://stellarroute:stellarroute_dev@localhost:5432/stellarroute".to_string()
    });

    let pool = PgPool::connect(&db_url)
        .await
        .expect("Failed to connect to database");

    let router = Server::new(ServerConfig::default(), DatabasePools::new(pool, None))
        .await
        .into_router();

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/assets?codes=XLM,native")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read body");

    let json: Value = serde_json::from_slice(&body).expect("Body is not valid JSON");

    let assets = json["data"]["assets"]
        .as_array()
        .expect("Expected 'assets' array");
    assert!(!assets.is_empty());
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn get_asset_metadata_fixture_returns_200_with_optional_issuer() {
    let pool = PgPool::connect(&default_db_url()).await.expect("connect");
    apply_migrations(&pool).await;
    seed_asset_fixture(&pool).await;

    let router = live_router(pool.clone()).await;
    let response = router
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/assets/{FIXTURE_CODE}?issuer={FIXTURE_ISSUER}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    let expected: Value = serde_json::from_str(include_str!(
        "fixtures/assets/issued_asset_metadata.json"
    ))
    .expect("issued asset fixture is not valid JSON");
    assert_eq!(json["data"], expected);

    cleanup_asset_fixture(&pool).await;
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL database (set DATABASE_URL)"]
async fn get_asset_metadata_fixture_returns_404_for_unknown_asset() {
    let pool = PgPool::connect(&default_db_url()).await.expect("connect");
    apply_migrations(&pool).await;
    let router = live_router(pool).await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/assets/TST1306_MISSING")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let json = body_json(response).await;
    let expected: Value = serde_json::from_str(include_str!(
        "fixtures/assets/missing_asset_error.json"
    ))
    .expect("missing asset fixture is not valid JSON");
    assert_eq!(json["data"]["error"], expected["error"]);
}
