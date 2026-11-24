//! PostgreSQL integration tests for atomic & advisory-locked indexer migrations (#1650).
//!
//! These tests require a reachable PostgreSQL server via `TEST_DATABASE_URL` or
//! `DATABASE_URL` with `CREATEDB` privilege (for example, local `postgres:16-alpine`).
//!
//! Run with:
//! ```bash
//! TEST_DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/postgres \
//!   cargo test -p stellarroute-indexer --test migration_concurrency_integration -- --ignored --nocapture
//! ```

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use stellarroute_indexer::db::Database;
use tracing_subscriber::fmt::MakeWriter;
use uuid::Uuid;

fn base_database_url() -> String {
    std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("Set TEST_DATABASE_URL or DATABASE_URL to run ignored migration integration tests")
}

fn replace_db_name(url: &str, db_name: &str) -> String {
    let (without_query, query) = match url.split_once('?') {
        Some((prefix, q)) => (prefix, Some(q)),
        None => (url, None),
    };
    let slash_idx = without_query
        .rfind('/')
        .expect("PostgreSQL URL must contain '/' before database name");
    let prefix = &without_query[..=slash_idx];
    match query {
        Some(q) => format!("{}{}?{}", prefix, db_name, q),
        None => format!("{}{}", prefix, db_name),
    }
}

struct TempDatabase {
    admin_pool: PgPool,
    db_name: String,
    db_url: String,
}

impl TempDatabase {
    async fn create(prefix: &str) -> Self {
        let base_url = base_database_url();
        let admin_url = replace_db_name(&base_url, "postgres");
        let admin_pool = PgPoolOptions::new()
            .max_connections(2)
            .acquire_timeout(Duration::from_secs(10))
            .connect(&admin_url)
            .await
            .expect("connect to postgres admin db");

        let db_name = format!("{}_{}", prefix, Uuid::new_v4().simple());
        sqlx::query(&format!("CREATE DATABASE {}", db_name))
            .execute(&admin_pool)
            .await
            .expect("create temporary database");

        let db_url = replace_db_name(&base_url, &db_name);
        Self {
            admin_pool,
            db_name,
            db_url,
        }
    }

    async fn connect_pool(&self, max_connections: u32) -> PgPool {
        PgPoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(Duration::from_secs(15))
            .connect(&self.db_url)
            .await
            .expect("connect to temporary database")
    }

    async fn cleanup(self) {
        let _ = sqlx::query(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = $1 AND pid <> pg_backend_pid()",
        )
        .bind(&self.db_name)
        .execute(&self.admin_pool)
        .await;

        let _ = sqlx::query(&format!("DROP DATABASE IF EXISTS {}", self.db_name))
            .execute(&self.admin_pool)
            .await;
    }
}

#[derive(Clone, Default)]
struct LogBuffer(Arc<Mutex<Vec<u8>>>);

impl LogBuffer {
    fn contents(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
    }
}

struct BufferGuard(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for BufferGuard {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for LogBuffer {
    type Writer = BufferGuard;

    fn make_writer(&'a self) -> Self::Writer {
        BufferGuard(self.0.clone())
    }
}

#[tokio::test]
#[ignore = "Requires PostgreSQL (set TEST_DATABASE_URL or DATABASE_URL)"]
async fn concurrent_migrate_calls_apply_each_migration_once_and_log_wait() {
    let temp = TempDatabase::create("sr_mig_conc").await;

    let pool_a = temp.connect_pool(4).await;
    let pool_b = temp.connect_pool(4).await;
    let pool_c = temp.connect_pool(4).await;

    let db_a = Database::from_pool(pool_a.clone());
    let db_b = Database::from_pool(pool_b.clone());
    let db_c = Database::from_pool(pool_c.clone());

    let log_buf = LogBuffer::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(log_buf.clone())
        .with_ansi(false)
        .with_max_level(tracing::Level::INFO)
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    // Run three replicas concurrently on the full embedded indexer migration set.
    let (res_a, res_b, res_c) = tokio::join!(db_a.migrate(), db_b.migrate(), db_c.migrate());
    res_a.expect("replica A migrate should succeed");
    res_b.expect("replica B migrate should succeed");
    res_c.expect("replica C migrate should succeed");

    let applied_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _schema_migrations")
        .fetch_one(&pool_a)
        .await
        .expect("count applied migrations");
    assert_eq!(
        applied_count, 15,
        "Expected all 15 embedded migrations applied once"
    );

    let logs = log_buf.contents();
    assert!(
        logs.contains("Waiting for indexer migration advisory lock held by another replica"),
        "Expected contended replica to log advisory lock wait; captured logs:\n{}",
        logs
    );

    drop(pool_a);
    drop(pool_b);
    drop(pool_c);
    temp.cleanup().await;
}

#[tokio::test]
#[ignore = "Requires PostgreSQL (set TEST_DATABASE_URL or DATABASE_URL)"]
async fn failing_transactional_migration_leaves_no_ledger_row_or_partial_schema() {
    let temp = TempDatabase::create("sr_mig_rollback").await;
    let pool = temp.connect_pool(2).await;
    let db = Database::from_pool(pool.clone());

    let migrations: &[(&str, &str)] = &[
        (
            "0001_good.sql",
            "CREATE TABLE committed_probe (id INT PRIMARY KEY);",
        ),
        (
            "0002_bad_midway.sql",
            "CREATE TABLE partial_probe (id INT PRIMARY KEY);
             INSERT INTO partial_probe (id) VALUES (1);
             ALTER TABLE does_not_exist_1650 ADD COLUMN broken INT;",
        ),
    ];

    let err = db
        .migrate_with_list(migrations)
        .await
        .expect_err("broken second migration must fail");
    assert!(
        err.to_string().contains("0002_bad_midway.sql"),
        "Error should identify failing migration: {}",
        err
    );

    let ledger_names: Vec<String> =
        sqlx::query_scalar("SELECT name FROM _schema_migrations ORDER BY name")
            .fetch_all(&pool)
            .await
            .expect("query _schema_migrations");
    assert_eq!(ledger_names, vec!["0001_good.sql".to_string()]);

    let partial_regclass: Option<String> =
        sqlx::query_scalar("SELECT to_regclass('public.partial_probe')::text")
            .fetch_one(&pool)
            .await
            .expect("check partial_probe regclass");
    assert_eq!(
        partial_regclass, None,
        "Partial DDL from failed transactional migration must be rolled back"
    );

    drop(pool);
    temp.cleanup().await;
}

#[tokio::test]
#[ignore = "Requires PostgreSQL (set TEST_DATABASE_URL or DATABASE_URL)"]
async fn existing_populated_schema_migrations_migrates_without_rerunning_applied_files() {
    let temp = TempDatabase::create("sr_mig_existing").await;
    let pool = temp.connect_pool(2).await;

    sqlx::raw_sql(
        "CREATE TABLE _schema_migrations (
            name TEXT PRIMARY KEY,
            applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        INSERT INTO _schema_migrations (name) VALUES ('0001_already_applied.sql');",
    )
    .execute(&pool)
    .await
    .expect("seed existing _schema_migrations");

    let db = Database::from_pool(pool.clone());
    let migrations: &[(&str, &str)] = &[
        // Would fail with division by zero if re-executed.
        ("0001_already_applied.sql", "SELECT 1 / 0;"),
        (
            "0002_incremental.sql",
            "CREATE TABLE incremental_ok (id INT PRIMARY KEY);",
        ),
    ];

    db.migrate_with_list(migrations)
        .await
        .expect("incremental migration on populated ledger should succeed");

    let ledger_names: Vec<String> =
        sqlx::query_scalar("SELECT name FROM _schema_migrations ORDER BY name")
            .fetch_all(&pool)
            .await
            .expect("query _schema_migrations");
    assert_eq!(
        ledger_names,
        vec![
            "0001_already_applied.sql".to_string(),
            "0002_incremental.sql".to_string()
        ]
    );

    drop(pool);
    temp.cleanup().await;
}

#[tokio::test]
#[ignore = "Requires PostgreSQL (set TEST_DATABASE_URL or DATABASE_URL)"]
async fn no_transaction_directive_executes_concurrent_index_ddl_under_lock() {
    let temp = TempDatabase::create("sr_mig_notx").await;
    let pool = temp.connect_pool(2).await;
    let db = Database::from_pool(pool.clone());

    let migrations: &[(&str, &str)] = &[
        (
            "0001_base_table.sql",
            "CREATE TABLE conc_items (id INT PRIMARY KEY, val TEXT NOT NULL);",
        ),
        (
            "0002_concurrent_idx.sql",
            "-- stellarroute:no-transaction\nCREATE INDEX CONCURRENTLY idx_conc_items_val ON conc_items (val);",
        ),
    ];

    db.migrate_with_list(migrations)
        .await
        .expect("CREATE INDEX CONCURRENTLY with opt-out directive should succeed");

    let idx_exists: Option<String> =
        sqlx::query_scalar("SELECT to_regclass('public.idx_conc_items_val')::text")
            .fetch_one(&pool)
            .await
            .expect("query index regclass");
    assert_eq!(idx_exists.as_deref(), Some("idx_conc_items_val"));

    drop(pool);
    temp.cleanup().await;
}
