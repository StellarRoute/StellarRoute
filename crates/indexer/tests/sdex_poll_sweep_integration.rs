//! PostgreSQL integration tests for SDEX polling offer sweep & archival (#1653).
//!
//! Run against a live PostgreSQL instance with:
//! ```bash
//! TEST_DATABASE_URL=postgres://postgres:postgres@127.0.0.1:15432/postgres \
//!   cargo test -p stellarroute-indexer --test sdex_poll_sweep_integration -- --ignored --test-threads=1
//! ```

use serde_json::json;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;
use stellarroute_indexer::{
    config::IndexerConfig,
    db::{archival::ArchivalManager, Database},
    horizon::HorizonClient,
    metrics::encode_metrics,
    partition::PartitionManager,
    sdex::{offers_swept_last_poll, offers_swept_total, SdexIndexer},
};
use wiremock::{
    matchers::{method, path, query_param, query_param_is_missing},
    Mock, MockServer, ResponseTemplate,
};

const SELLER: &str = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";

fn replace_db_name(url: &str, db_name: &str) -> String {
    let (without_query, query) = match url.split_once('?') {
        Some((prefix, q)) => (prefix, Some(q)),
        None => (url, None),
    };
    let slash_idx = without_query.rfind('/').expect("valid postgres URL");
    let prefix = &without_query[..=slash_idx];
    match query {
        Some(q) => format!("{}{}?{}", prefix, db_name, q),
        None => format!("{}{}", prefix, db_name),
    }
}

async fn setup_temp_db(prefix: &str) -> (PgPool, String, Database) {
    let base_url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("TEST_DATABASE_URL or DATABASE_URL must be set");
    let admin_url = replace_db_name(&base_url, "postgres");
    let admin_pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&admin_url)
        .await
        .expect("connect postgres admin db");

    let db_name = format!("{}_{}", prefix, uuid::Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE DATABASE {}", db_name))
        .execute(&admin_pool)
        .await
        .expect("create isolated test database");

    let cfg = IndexerConfig {
        database_url: replace_db_name(&base_url, &db_name),
        max_connections: 4,
        min_connections: 1,
        ..IndexerConfig::default()
    };
    let db = Database::new(&cfg).await.expect("connect test database");
    db.migrate().await.expect("migrate test database");
    (admin_pool, db_name, db)
}

async fn teardown_temp_db(admin_pool: PgPool, db_name: &str, db: Database) {
    db.pool().close().await;
    let _ = sqlx::query(
        "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = $1 AND pid <> pg_backend_pid()",
    )
    .bind(db_name)
    .execute(&admin_pool)
    .await;
    let _ = sqlx::query(&format!("DROP DATABASE IF EXISTS {}", db_name))
        .execute(&admin_pool)
        .await;
}

fn offer_json(
    id: i64,
    selling_code: Option<&str>,
    buying_code: Option<&str>,
    amount: &str,
    price: &str,
    price_n: i64,
    price_d: i64,
) -> serde_json::Value {
    let selling = match selling_code {
        None => json!({"asset_type": "native"}),
        Some(code) => json!({
            "asset_type": "credit_alphanum4",
            "asset_code": code,
            "asset_issuer": SELLER
        }),
    };
    let buying = match buying_code {
        None => json!({"asset_type": "native"}),
        Some(code) => json!({
            "asset_type": "credit_alphanum4",
            "asset_code": code,
            "asset_issuer": SELLER
        }),
    };
    json!({
        "id": id.to_string(),
        "paging_token": format!("pt_{}", id),
        "seller": SELLER,
        "selling": selling,
        "buying": buying,
        "amount": amount,
        "price": price,
        "price_r": {"n": price_n, "d": price_d},
        "last_modified_ledger": 90_000 + id
    })
}

/// Mirrors `fetch_orderbook_side` in `crates/api/src/routes/orderbook.rs` to verify
/// `/orderbook` asks/bids no longer return disappeared offers.
async fn query_orderbook_asks(
    pool: &PgPool,
    base_code: Option<&str>,
    quote_code: &str,
) -> Vec<(String, String)> {
    let base_id: uuid::Uuid = match base_code {
        None => sqlx::query_scalar(
            "SELECT id FROM assets WHERE asset_type = 'native' ORDER BY created_at ASC LIMIT 1",
        )
        .fetch_one(pool)
        .await
        .unwrap(),
        Some(code) => sqlx::query_scalar("SELECT id FROM assets WHERE asset_code = $1 LIMIT 1")
            .bind(code)
            .fetch_one(pool)
            .await
            .unwrap(),
    };
    let quote_id: uuid::Uuid =
        sqlx::query_scalar("SELECT id FROM assets WHERE asset_code = $1 LIMIT 1")
            .bind(quote_code)
            .fetch_one(pool)
            .await
            .unwrap();

    sqlx::query_as(
        r#"
        SELECT price::text AS price, amount::text AS amount
        FROM sdex_offers
        WHERE selling_asset_id = $1
          AND buying_asset_id = $2
        ORDER BY price ASC
        LIMIT 50
        "#,
    )
    .bind(base_id)
    .bind(quote_id)
    .fetch_all(pool)
    .await
    .unwrap()
}

#[tokio::test]
#[ignore = "Requires PostgreSQL via TEST_DATABASE_URL or DATABASE_URL"]
async fn complete_poll_removes_disappeared_offers_from_sdex_orderbook_and_normalized_liquidity() {
    let (admin_pool, db_name, db) = setup_temp_db("sr_1653_sweep").await;
    let mock_horizon = MockServer::start().await;

    // Poll 1: 3 offers across 2 pages (page_limit = 2: [1001, 1002], then [1003]).
    Mock::given(method("GET"))
        .and(path("/offers"))
        .and(query_param_is_missing("cursor"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_embedded": {
                "records": [
                    offer_json(1001, None, Some("USDC"), "100.0000000", "0.1200000", 3, 25),
                    offer_json(1002, None, Some("USDC"), "250.0000000", "0.1300000", 13, 100),
                ]
            },
            "_links": {}
        })))
        .mount(&mock_horizon)
        .await;

    Mock::given(method("GET"))
        .and(path("/offers"))
        .and(query_param("cursor", "pt_1002"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_embedded": {
                "records": [
                    offer_json(1003, Some("USDC"), None, "500.0000000", "8.1000000", 81, 10),
                ]
            },
            "_links": {}
        })))
        .mount(&mock_horizon)
        .await;

    let pm = PartitionManager::from_config(&IndexerConfig::default());
    let indexer = SdexIndexer::new(HorizonClient::new(mock_horizon.uri()), db.clone(), pm)
        .with_pagination(2, 10);

    let before_swept = offers_swept_total();
    let count_1 = indexer.index_offers().await.expect("poll 1 succeeds");
    assert_eq!(count_1, 3);

    // Before disappearance: `/orderbook` sees both 0.12 and 0.13 ask levels, and normalized_liquidity has all 3 SDEX rows.
    let asks_before = query_orderbook_asks(db.pool(), None, "USDC").await;
    assert_eq!(asks_before.len(), 2);

    let norm_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM normalized_liquidity WHERE venue_type = 'sdex'")
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(norm_before, 3);

    // Ensure clock advances strictly past Poll 1's updated_at timestamps.
    tokio::time::sleep(Duration::from_millis(15)).await;

    // Poll 2: offer 1002 disappeared on-chain (filled/cancelled); Horizon returns [1001, 1003] on page 1 and [] on page 2.
    mock_horizon.reset().await;
    Mock::given(method("GET"))
        .and(path("/offers"))
        .and(query_param_is_missing("cursor"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_embedded": {
                "records": [
                    offer_json(1001, None, Some("USDC"), "100.0000000", "0.1200000", 3, 25),
                    offer_json(1003, Some("USDC"), None, "500.0000000", "8.1000000", 81, 10),
                ]
            },
            "_links": {}
        })))
        .mount(&mock_horizon)
        .await;

    Mock::given(method("GET"))
        .and(path("/offers"))
        .and(query_param("cursor", "pt_1003"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_embedded": { "records": [] },
            "_links": {}
        })))
        .mount(&mock_horizon)
        .await;

    let count_2 = indexer.index_offers().await.expect("poll 2 succeeds");
    assert_eq!(count_2, 2);

    let remaining_ids: Vec<i64> =
        sqlx::query_scalar("SELECT offer_id FROM sdex_offers ORDER BY offer_id ASC")
            .fetch_all(db.pool())
            .await
            .unwrap();
    assert_eq!(remaining_ids, vec![1001, 1003]);

    // `/orderbook` query no longer returns disappeared offer 1002.
    let asks_after = query_orderbook_asks(db.pool(), None, "USDC").await;
    assert_eq!(asks_after.len(), 1);
    assert!(asks_after[0].0.starts_with("0.12"));

    // `trg_sync_normalized_sdex` removed venue_ref = '1002' from normalized_liquidity too.
    let remaining_norm_refs: Vec<String> = sqlx::query_scalar(
        "SELECT venue_ref FROM normalized_liquidity WHERE venue_type = 'sdex' ORDER BY venue_ref ASC",
    )
    .fetch_all(db.pool())
    .await
    .unwrap();
    assert_eq!(
        remaining_norm_refs,
        vec!["1001".to_string(), "1003".to_string()]
    );

    // Prometheus swept-offers metrics incremented and exposed.
    assert_eq!(offers_swept_total() - before_swept, 1);
    assert_eq!(offers_swept_last_poll(), 1);
    let metrics_text = encode_metrics().expect("encode prometheus metrics");
    assert!(
        metrics_text.contains("stellarroute_indexer_sdex_offers_swept_total"),
        "expected swept counter in Prometheus metrics output"
    );
    assert!(
        metrics_text.contains("stellarroute_indexer_sdex_offers_swept_last_poll"),
        "expected swept last-poll gauge in Prometheus metrics output"
    );

    teardown_temp_db(admin_pool, &db_name, db).await;
}

#[tokio::test]
#[ignore = "Requires PostgreSQL via TEST_DATABASE_URL or DATABASE_URL"]
async fn failed_mid_poll_and_page_cap_hit_never_sweep_offers() {
    let (admin_pool, db_name, db) = setup_temp_db("sr_1653_nosweep").await;
    let mock_horizon = MockServer::start().await;

    // Initial complete poll seeds offers 2001, 2002, 2003 (page_limit = 2, max_pages = 2 -> [2001, 2002], [2003]).
    Mock::given(method("GET"))
        .and(path("/offers"))
        .and(query_param_is_missing("cursor"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_embedded": {
                "records": [
                    offer_json(2001, None, Some("USDC"), "50.0000000", "0.1100000", 11, 100),
                    offer_json(2002, None, Some("USDC"), "60.0000000", "0.1200000", 12, 100),
                ]
            },
            "_links": {}
        })))
        .mount(&mock_horizon)
        .await;

    Mock::given(method("GET"))
        .and(path("/offers"))
        .and(query_param("cursor", "pt_2002"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_embedded": {
                "records": [
                    offer_json(2003, None, Some("USDC"), "70.0000000", "0.1300000", 13, 100),
                ]
            },
            "_links": {}
        })))
        .mount(&mock_horizon)
        .await;

    let pm = PartitionManager::from_config(&IndexerConfig::default());
    let indexer = SdexIndexer::new(HorizonClient::new(mock_horizon.uri()), db.clone(), pm)
        .with_pagination(2, 2);

    assert_eq!(indexer.index_offers().await.unwrap(), 3);
    let swept_baseline = offers_swept_total();
    tokio::time::sleep(Duration::from_millis(15)).await;

    // Case A: Mid-poll Horizon failure (page 1 succeeds with [2001, 2002], page 2 fails 500).
    mock_horizon.reset().await;
    Mock::given(method("GET"))
        .and(path("/offers"))
        .and(query_param_is_missing("cursor"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_embedded": {
                "records": [
                    offer_json(2001, None, Some("USDC"), "50.0000000", "0.1100000", 11, 100),
                    offer_json(2002, None, Some("USDC"), "60.0000000", "0.1200000", 12, 100),
                ]
            },
            "_links": {}
        })))
        .mount(&mock_horizon)
        .await;

    Mock::given(method("GET"))
        .and(path("/offers"))
        .and(query_param("cursor", "pt_2002"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_horizon)
        .await;

    assert!(indexer.index_offers().await.is_err());
    let ids_after_fail: Vec<i64> =
        sqlx::query_scalar("SELECT offer_id FROM sdex_offers ORDER BY offer_id ASC")
            .fetch_all(db.pool())
            .await
            .unwrap();
    assert_eq!(
        ids_after_fail,
        vec![2001, 2002, 2003],
        "failed mid-poll must not sweep untouched offer 2003"
    );
    assert_eq!(offers_swept_total(), swept_baseline);

    // Case B: Page cap hit (2 full pages of 2 new offers [3001, 3002], [3003, 3004], max_pages = 2).
    tokio::time::sleep(Duration::from_millis(15)).await;
    mock_horizon.reset().await;
    Mock::given(method("GET"))
        .and(path("/offers"))
        .and(query_param_is_missing("cursor"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_embedded": {
                "records": [
                    offer_json(3001, None, Some("USDC"), "10.0000000", "0.1400000", 14, 100),
                    offer_json(3002, None, Some("USDC"), "20.0000000", "0.1500000", 15, 100),
                ]
            },
            "_links": {}
        })))
        .mount(&mock_horizon)
        .await;

    Mock::given(method("GET"))
        .and(path("/offers"))
        .and(query_param("cursor", "pt_3002"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_embedded": {
                "records": [
                    offer_json(3003, None, Some("USDC"), "30.0000000", "0.1600000", 16, 100),
                    offer_json(3004, None, Some("USDC"), "40.0000000", "0.1700000", 17, 100),
                ]
            },
            "_links": {}
        })))
        .mount(&mock_horizon)
        .await;

    assert_eq!(indexer.index_offers().await.unwrap(), 4);
    let ids_after_cap: Vec<i64> =
        sqlx::query_scalar("SELECT offer_id FROM sdex_offers ORDER BY offer_id ASC")
            .fetch_all(db.pool())
            .await
            .unwrap();
    assert_eq!(
        ids_after_cap,
        vec![2001, 2002, 2003, 3001, 3002, 3003, 3004],
        "hitting page cap must not sweep offers beyond the capped window"
    );
    assert_eq!(offers_swept_total(), swept_baseline);

    teardown_temp_db(admin_pool, &db_name, db).await;
}

#[tokio::test]
#[ignore = "Requires PostgreSQL via TEST_DATABASE_URL or DATABASE_URL"]
async fn archival_manager_moves_aged_64bit_offers_and_preserves_fresh_rows() {
    let (admin_pool, db_name, db) = setup_temp_db("sr_1653_archive").await;
    let mock_horizon = MockServer::start().await;

    // Seed three offers with 64-bit Stellar offer IDs (> i32::MAX).
    Mock::given(method("GET"))
        .and(path("/offers"))
        .and(query_param_is_missing("cursor"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_embedded": {
                "records": [
                    offer_json(9_000_000_001, None, Some("USDC"), "10.0000000", "0.1100000", 11, 100),
                    offer_json(9_000_000_002, None, Some("USDC"), "20.0000000", "0.1200000", 12, 100),
                    offer_json(9_000_000_003, None, Some("USDC"), "30.0000000", "0.1300000", 13, 100),
                ]
            },
            "_links": {}
        })))
        .mount(&mock_horizon)
        .await;

    let pm = PartitionManager::from_config(&IndexerConfig::default());
    let indexer = SdexIndexer::new(HorizonClient::new(mock_horizon.uri()), db.clone(), pm);
    assert_eq!(indexer.index_offers().await.unwrap(), 3);

    // Backdate two offers to 45 days ago.
    sqlx::query(
        "UPDATE sdex_offers SET updated_at = NOW() - INTERVAL '45 days' WHERE offer_id IN (9000000001, 9000000002)",
    )
    .execute(db.pool())
    .await
    .unwrap();

    let archival = ArchivalManager::new(db.pool().clone());
    let archived = archival
        .archive_old_offers(Some(30))
        .await
        .expect("archive_old_offers succeeds for multiple 64-bit offers");
    assert_eq!(archived, 2);
    assert_eq!(archival.get_archived_count().await.unwrap(), 2);

    let remaining_live: Vec<i64> =
        sqlx::query_scalar("SELECT offer_id FROM sdex_offers ORDER BY offer_id ASC")
            .fetch_all(db.pool())
            .await
            .unwrap();
    assert_eq!(remaining_live, vec![9_000_000_003]);

    let remaining_norm: Vec<String> = sqlx::query_scalar(
        "SELECT venue_ref FROM normalized_liquidity WHERE venue_type = 'sdex' ORDER BY venue_ref ASC",
    )
    .fetch_all(db.pool())
    .await
    .unwrap();
    assert_eq!(remaining_norm, vec!["9000000003".to_string()]);

    teardown_temp_db(admin_pool, &db_name, db).await;
}
