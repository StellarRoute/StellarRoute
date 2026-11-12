//! API database migration runner and ledger (`_api_schema_migrations`).
//!
//! Mirrors the indexer's advisory-locked, per-file transactional migration pattern
//! while using an independent ledger table ([`API_MIGRATIONS_LEDGER_TABLE`]) and
//! advisory lock key ([`API_MIGRATION_ADVISORY_LOCK_KEY`]).
//!
//! ### Opt-in execution
//! - Normal API startup does **not** execute migrations unless `API_AUTO_MIGRATE=true`.
//! - Operators and deploy workflows invoke `stellarroute-api migrate` (or
//!   `stellarroute-api --migrate-only`).
//! - Existing staging databases whose tables were previously created via manual `psql`
//!   scripts can run `stellarroute-api migrate --bootstrap` (or `--bootstrap-through <file>`)
//!   to record already-present migrations in `_api_schema_migrations` without re-running DDL.

use sqlx::{Acquire, PgConnection, PgPool};
use thiserror::Error;
use tracing::{debug, info};

/// Ledger table tracking applied `crates/api/migrations/*.sql` files.
pub const API_MIGRATIONS_LEDGER_TABLE: &str = "_api_schema_migrations";

/// PostgreSQL session-level advisory lock key (`"SR_APIMG"` in ASCII) serializing
/// concurrent API migration runs across replicas and deploy jobs.
pub const API_MIGRATION_ADVISORY_LOCK_KEY: i64 = 0x5352_5F41_5049_4D47;

/// Leading comment directive opting a migration file out of an enclosing `BEGIN ... COMMIT`
/// block (for statements like `CREATE INDEX CONCURRENTLY`), while still executing under
/// the session-level `pg_advisory_lock`.
pub const NO_TRANSACTION_DIRECTIVE: &str = "stellarroute:no-transaction";

/// Errors produced by the API migration runner.
#[derive(Debug, Error)]
pub enum ApiMigrationError {
    #[error("Database connection error: {0}")]
    Connection(sqlx::Error),

    #[error("Migration '{name}' failed: {message}")]
    Execution { name: String, message: String },

    #[error("Unknown migration name passed to bootstrap: '{0}'")]
    UnknownMigration(String),
}

/// Catalog probe used by `--bootstrap` to detect whether an existing staging database
/// already has the schema artifacts introduced by a given migration without re-running DDL.
#[derive(Debug, Clone, Copy)]
pub enum SchemaProbe {
    /// Table exists in the `public` schema.
    TableExists(&'static str),
    /// Column exists on `(table, column)` in the `public` schema.
    ColumnExists {
        table: &'static str,
        column: &'static str,
    },
    /// Named `pg_constraint` exists on `table` in the `public` schema.
    ConstraintExists {
        table: &'static str,
        constraint: &'static str,
    },
    /// Named `pg_constraint` exists on `table` and its definition contains `substring`.
    ConstraintDefContains {
        table: &'static str,
        constraint: &'static str,
        substring: &'static str,
    },
    /// Function/routine exists in the `public` schema.
    RoutineExists(&'static str),
}

/// Embedded API migration descriptor.
#[derive(Debug, Clone, Copy)]
pub struct ApiMigrationSpec {
    pub name: &'static str,
    pub sql: &'static str,
    pub probe: SchemaProbe,
}

/// Complete, ordered registry of `crates/api/migrations/*.sql`.
pub const API_MIGRATIONS: &[ApiMigrationSpec] = &[
    ApiMigrationSpec {
        name: "0001_route_computation_jobs.sql",
        sql: include_str!("../migrations/0001_route_computation_jobs.sql"),
        probe: SchemaProbe::TableExists("route_computation_jobs"),
    },
    ApiMigrationSpec {
        name: "0002_replay_artifacts.sql",
        sql: include_str!("../migrations/0002_replay_artifacts.sql"),
        probe: SchemaProbe::TableExists("replay_artifacts"),
    },
    ApiMigrationSpec {
        name: "0003_priority_queue.sql",
        sql: include_str!("../migrations/0003_priority_queue.sql"),
        probe: SchemaProbe::ColumnExists {
            table: "route_computation_jobs",
            column: "priority",
        },
    },
    ApiMigrationSpec {
        name: "0004_route_audit_log.sql",
        sql: include_str!("../migrations/0004_route_audit_log.sql"),
        probe: SchemaProbe::TableExists("route_audit_log"),
    },
    ApiMigrationSpec {
        name: "0005_quote_purger.sql",
        sql: include_str!("../migrations/0005_quote_purger.sql"),
        probe: SchemaProbe::TableExists("quote_purge_metrics"),
    },
    ApiMigrationSpec {
        name: "0011_contract_registry.sql",
        sql: include_str!("../migrations/0011_contract_registry.sql"),
        probe: SchemaProbe::TableExists("contract_registry"),
    },
    ApiMigrationSpec {
        name: "0012_swap_submit_audit_log.sql",
        sql: include_str!("../migrations/0012_swap_submit_audit_log.sql"),
        probe: SchemaProbe::RoutineExists("purge_swap_submit_audit_log_older_than"),
    },
    ApiMigrationSpec {
        name: "0013_swap_prepared_quotes.sql",
        sql: include_str!("../migrations/0013_swap_prepared_quotes.sql"),
        probe: SchemaProbe::TableExists("swap_prepared_quotes"),
    },
    ApiMigrationSpec {
        name: "0014_swap_prepared_quotes_security.sql",
        sql: include_str!("../migrations/0014_swap_prepared_quotes_security.sql"),
        probe: SchemaProbe::ColumnExists {
            table: "swap_prepared_quotes",
            column: "sender_account",
        },
    },
    ApiMigrationSpec {
        name: "0015_cctp_transfers.sql",
        sql: include_str!("../migrations/0015_cctp_transfers.sql"),
        probe: SchemaProbe::TableExists("cctp_transfers"),
    },
    ApiMigrationSpec {
        name: "0016_cctp_transfers_hardening.sql",
        sql: include_str!("../migrations/0016_cctp_transfers_hardening.sql"),
        probe: SchemaProbe::ConstraintExists {
            table: "cctp_transfers",
            constraint: "cctp_transfers_status_check",
        },
    },
    ApiMigrationSpec {
        name: "0017_cctp_mint_metadata.sql",
        sql: include_str!("../migrations/0017_cctp_mint_metadata.sql"),
        probe: SchemaProbe::ColumnExists {
            table: "cctp_transfers",
            column: "mint_payload_hash",
        },
    },
    ApiMigrationSpec {
        name: "0018_cctp_approval_tx_hash.sql",
        sql: include_str!("../migrations/0018_cctp_approval_tx_hash.sql"),
        probe: SchemaProbe::ColumnExists {
            table: "cctp_transfers",
            column: "source_approval_tx_hash",
        },
    },
    ApiMigrationSpec {
        name: "0019_cctp_approval_verified_at.sql",
        sql: include_str!("../migrations/0019_cctp_approval_verified_at.sql"),
        probe: SchemaProbe::ColumnExists {
            table: "cctp_transfers",
            column: "source_approval_verified_at",
        },
    },
    ApiMigrationSpec {
        name: "0020_card_tables.sql",
        sql: include_str!("../migrations/0020_card_tables.sql"),
        probe: SchemaProbe::TableExists("card_applications"),
    },
    ApiMigrationSpec {
        name: "20260730_cctp_review_fixes.sql",
        sql: include_str!("../migrations/20260730_cctp_review_fixes.sql"),
        probe: SchemaProbe::TableExists("cctp_active_prepares"),
    },
    ApiMigrationSpec {
        name: "20260731_cctp_prepare_lock_hardening.sql",
        sql: include_str!("../migrations/20260731_cctp_prepare_lock_hardening.sql"),
        probe: SchemaProbe::ConstraintExists {
            table: "cctp_active_prepares",
            constraint: "cctp_active_prepares_transfer_fk",
        },
    },
    ApiMigrationSpec {
        name: "20260801_cctp_http_gate.sql",
        sql: include_str!("../migrations/20260801_cctp_http_gate.sql"),
        probe: SchemaProbe::TableExists("cctp_quote_idempotency"),
    },
    ApiMigrationSpec {
        name: "20260802_cctp_http_hardening.sql",
        sql: include_str!("../migrations/20260802_cctp_http_hardening.sql"),
        probe: SchemaProbe::ColumnExists {
            table: "cctp_quote_idempotency",
            column: "state",
        },
    },
    ApiMigrationSpec {
        name: "20260803_cctp_reattest_lease.sql",
        sql: include_str!("../migrations/20260803_cctp_reattest_lease.sql"),
        probe: SchemaProbe::ColumnExists {
            table: "cctp_transfers",
            column: "reattest_lease_owner_hash",
        },
    },
    ApiMigrationSpec {
        name: "20260814_cctp_finality_fast.sql",
        sql: include_str!("../migrations/20260814_cctp_finality_fast.sql"),
        probe: SchemaProbe::ConstraintDefContains {
            table: "cctp_transfers",
            constraint: "cctp_transfers_finality_check",
            substring: "fast",
        },
    },
];

/// Summary of a [`run_api_migrations`] execution.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MigrationReport {
    pub applied: Vec<String>,
    pub skipped: usize,
}

/// Summary of a [`bootstrap_existing_api_migrations`] or [`bootstrap_through_migration`] run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BootstrapReport {
    pub recorded: Vec<String>,
    pub already_recorded: usize,
}

/// CLI migration action parsed from `stellarroute-api` arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliMigrationAction {
    /// Run pending API migrations (`stellarroute-api migrate` or `--migrate-only`).
    Migrate { bootstrap_existing: bool },
    /// Only mark detected existing migrations in `_api_schema_migrations` without running DDL.
    BootstrapOnly,
    /// Mark all migrations up through `through_name` (inclusive) as applied, optionally running
    /// any remaining migrations afterwards when `run_remaining` is `true`.
    BootstrapThrough {
        through_name: String,
        run_remaining: bool,
    },
}

/// Parse `API_AUTO_MIGRATE` boolean string (`true`/`1`/`yes`/`on`). Defaults to `false`.
pub fn is_auto_migrate_enabled_from_str(raw: Option<&str>) -> bool {
    raw.map(|v| {
        matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
    .unwrap_or(false)
}

/// Check whether `API_AUTO_MIGRATE=true` is enabled in the process environment.
pub fn is_auto_migrate_enabled() -> bool {
    let val = std::env::var("API_AUTO_MIGRATE").ok();
    is_auto_migrate_enabled_from_str(val.as_deref())
}

/// Parse CLI arguments passed to `stellarroute-api` (excluding `argv[0]`).
///
/// Returns `Ok(None)` when no migration subcommand or flag was supplied, leaving normal
/// HTTP server startup untouched.
pub fn parse_cli_migration_action<I, S>(
    args: I,
) -> std::result::Result<Option<CliMigrationAction>, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let tokens: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();
    if tokens.is_empty() {
        return Ok(None);
    }

    let mut is_migrate_cmd = false;
    let mut bootstrap = false;
    let mut bootstrap_only = false;
    let mut bootstrap_through: Option<String> = None;

    let mut idx = 0;
    while idx < tokens.len() {
        match tokens[idx].as_str() {
            "migrate" | "--migrate-only" => {
                is_migrate_cmd = true;
            }
            "--bootstrap" | "--bootstrap-migrations" => {
                is_migrate_cmd = true;
                bootstrap = true;
            }
            "--bootstrap-only" => {
                is_migrate_cmd = true;
                bootstrap_only = true;
            }
            "--bootstrap-through" => {
                is_migrate_cmd = true;
                idx += 1;
                let Some(target) = tokens.get(idx) else {
                    return Err("--bootstrap-through requires a migration filename".to_string());
                };
                bootstrap_through = Some(target.clone());
            }
            other if other.starts_with("--bootstrap-through=") => {
                is_migrate_cmd = true;
                let target = other.trim_start_matches("--bootstrap-through=");
                if target.is_empty() {
                    return Err("--bootstrap-through requires a migration filename".to_string());
                }
                bootstrap_through = Some(target.to_string());
            }
            other => {
                if is_migrate_cmd {
                    return Err(format!("Unrecognized migration argument: {}", other));
                }
                return Ok(None);
            }
        }
        idx += 1;
    }

    if !is_migrate_cmd {
        return Ok(None);
    }

    if let Some(through_name) = bootstrap_through {
        return Ok(Some(CliMigrationAction::BootstrapThrough {
            through_name,
            run_remaining: !bootstrap_only,
        }));
    }

    if bootstrap_only {
        return Ok(Some(CliMigrationAction::BootstrapOnly));
    }

    Ok(Some(CliMigrationAction::Migrate {
        bootstrap_existing: bootstrap,
    }))
}

/// Returns `true` if a migration file begins with `-- stellarroute:no-transaction`
/// (or `-- no-transaction`) before its first SQL statement.
pub fn is_non_transactional_migration(sql: &str) -> bool {
    let trimmed = sql.trim_start_matches('\u{feff}');
    for line in trimmed.lines() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        if let Some(comment) = l.strip_prefix("--") {
            let directive = comment.trim().to_ascii_lowercase();
            if directive == NO_TRANSACTION_DIRECTIVE || directive == "no-transaction" {
                return true;
            }
            continue;
        }
        break;
    }
    false
}

async fn acquire_advisory_lock(
    conn: &mut PgConnection,
) -> std::result::Result<(), ApiMigrationError> {
    let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
        .bind(API_MIGRATION_ADVISORY_LOCK_KEY)
        .fetch_one(&mut *conn)
        .await
        .map_err(ApiMigrationError::Connection)?;

    if !acquired {
        info!(
            lock_key = API_MIGRATION_ADVISORY_LOCK_KEY,
            "Waiting for API migration advisory lock held by another process"
        );
        sqlx::query("SELECT pg_advisory_lock($1)")
            .bind(API_MIGRATION_ADVISORY_LOCK_KEY)
            .execute(&mut *conn)
            .await
            .map_err(ApiMigrationError::Connection)?;
        info!(
            lock_key = API_MIGRATION_ADVISORY_LOCK_KEY,
            "Acquired API migration advisory lock after waiting"
        );
    }
    Ok(())
}

async fn release_advisory_lock(conn: &mut PgConnection) {
    let _ = sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(API_MIGRATION_ADVISORY_LOCK_KEY)
        .execute(&mut *conn)
        .await;
}

async fn ensure_ledger_table(
    conn: &mut PgConnection,
) -> std::result::Result<(), ApiMigrationError> {
    let ddl = format!(
        r#"
        CREATE TABLE IF NOT EXISTS {table} (
            name TEXT PRIMARY KEY,
            applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
        table = API_MIGRATIONS_LEDGER_TABLE
    );
    sqlx::query(&ddl)
        .execute(&mut *conn)
        .await
        .map_err(|e| ApiMigrationError::Execution {
            name: API_MIGRATIONS_LEDGER_TABLE.to_string(),
            message: e.to_string(),
        })?;
    Ok(())
}

async fn probe_schema_exists(
    conn: &mut PgConnection,
    probe: SchemaProbe,
) -> std::result::Result<bool, ApiMigrationError> {
    match probe {
        SchemaProbe::TableExists(table) => sqlx::query_scalar(
            r#"
                SELECT EXISTS (
                    SELECT 1
                    FROM information_schema.tables
                    WHERE table_schema = 'public'
                      AND table_name = $1
                )
                "#,
        )
        .bind(table)
        .fetch_one(&mut *conn)
        .await
        .map_err(ApiMigrationError::Connection),
        SchemaProbe::ColumnExists { table, column } => sqlx::query_scalar(
            r#"
                SELECT EXISTS (
                    SELECT 1
                    FROM information_schema.columns
                    WHERE table_schema = 'public'
                      AND table_name = $1
                      AND column_name = $2
                )
                "#,
        )
        .bind(table)
        .bind(column)
        .fetch_one(&mut *conn)
        .await
        .map_err(ApiMigrationError::Connection),
        SchemaProbe::ConstraintExists { table, constraint } => sqlx::query_scalar(
            r#"
                SELECT EXISTS (
                    SELECT 1
                    FROM pg_constraint c
                    JOIN pg_class r ON r.oid = c.conrelid
                    JOIN pg_namespace n ON n.oid = r.relnamespace
                    WHERE n.nspname = 'public'
                      AND r.relname = $1
                      AND c.conname = $2
                )
                "#,
        )
        .bind(table)
        .bind(constraint)
        .fetch_one(&mut *conn)
        .await
        .map_err(ApiMigrationError::Connection),
        SchemaProbe::ConstraintDefContains {
            table,
            constraint,
            substring,
        } => {
            let def: Option<String> = sqlx::query_scalar(
                r#"
                SELECT pg_get_constraintdef(c.oid)
                FROM pg_constraint c
                JOIN pg_class r ON r.oid = c.conrelid
                JOIN pg_namespace n ON n.oid = r.relnamespace
                WHERE n.nspname = 'public'
                  AND r.relname = $1
                  AND c.conname = $2
                LIMIT 1
                "#,
            )
            .bind(table)
            .bind(constraint)
            .fetch_optional(&mut *conn)
            .await
            .map_err(ApiMigrationError::Connection)?;
            Ok(def
                .map(|d| {
                    d.to_ascii_lowercase()
                        .contains(&substring.to_ascii_lowercase())
                })
                .unwrap_or(false))
        }
        SchemaProbe::RoutineExists(routine) => sqlx::query_scalar(
            r#"
                SELECT EXISTS (
                    SELECT 1
                    FROM pg_proc p
                    JOIN pg_namespace n ON n.oid = p.pronamespace
                    WHERE n.nspname = 'public'
                      AND p.proname = $1
                )
                "#,
        )
        .bind(routine)
        .fetch_one(&mut *conn)
        .await
        .map_err(ApiMigrationError::Connection),
    }
}

async fn bootstrap_existing_inner(
    conn: &mut PgConnection,
) -> std::result::Result<BootstrapReport, ApiMigrationError> {
    ensure_ledger_table(&mut *conn).await?;
    let mut report = BootstrapReport::default();

    let check_sql = format!(
        "SELECT EXISTS (SELECT 1 FROM {} WHERE name = $1)",
        API_MIGRATIONS_LEDGER_TABLE
    );
    let insert_sql = format!(
        "INSERT INTO {} (name) VALUES ($1) ON CONFLICT (name) DO NOTHING",
        API_MIGRATIONS_LEDGER_TABLE
    );

    for spec in API_MIGRATIONS {
        let already: bool = sqlx::query_scalar(&check_sql)
            .bind(spec.name)
            .fetch_one(&mut *conn)
            .await
            .map_err(ApiMigrationError::Connection)?;
        if already {
            report.already_recorded += 1;
            continue;
        }

        if probe_schema_exists(&mut *conn, spec.probe).await? {
            sqlx::query(&insert_sql)
                .bind(spec.name)
                .execute(&mut *conn)
                .await
                .map_err(|e| ApiMigrationError::Execution {
                    name: spec.name.to_string(),
                    message: e.to_string(),
                })?;
            info!(
                migration = spec.name,
                "Bootstrapped existing API migration into ledger without re-running DDL"
            );
            report.recorded.push(spec.name.to_string());
        }
    }

    Ok(report)
}

/// Inspect existing database catalog objects and mark already-applied API migrations in
/// `_api_schema_migrations` **without re-running any migration DDL**.
pub async fn bootstrap_existing_api_migrations(
    pool: &PgPool,
) -> std::result::Result<BootstrapReport, ApiMigrationError> {
    let mut conn = pool
        .acquire()
        .await
        .map_err(ApiMigrationError::Connection)?;
    acquire_advisory_lock(&mut conn).await?;
    let result = bootstrap_existing_inner(&mut conn).await;
    release_advisory_lock(&mut conn).await;
    result
}

async fn bootstrap_through_inner(
    conn: &mut PgConnection,
    end_idx: usize,
) -> std::result::Result<BootstrapReport, ApiMigrationError> {
    ensure_ledger_table(&mut *conn).await?;
    let mut report = BootstrapReport::default();
    let insert_sql = format!(
        "INSERT INTO {} (name) VALUES ($1) ON CONFLICT (name) DO NOTHING",
        API_MIGRATIONS_LEDGER_TABLE
    );

    for spec in &API_MIGRATIONS[..=end_idx] {
        let res = sqlx::query(&insert_sql)
            .bind(spec.name)
            .execute(&mut *conn)
            .await
            .map_err(|e| ApiMigrationError::Execution {
                name: spec.name.to_string(),
                message: e.to_string(),
            })?;
        if res.rows_affected() > 0 {
            info!(
                migration = spec.name,
                "Bootstrapped API migration into ledger via --bootstrap-through"
            );
            report.recorded.push(spec.name.to_string());
        } else {
            report.already_recorded += 1;
        }
    }

    Ok(report)
}

/// Explicitly record all migrations in [`API_MIGRATIONS`] up through `through_name` (inclusive)
/// as applied in `_api_schema_migrations` without executing DDL.
pub async fn bootstrap_through_migration(
    pool: &PgPool,
    through_name: &str,
) -> std::result::Result<BootstrapReport, ApiMigrationError> {
    let Some(end_idx) = API_MIGRATIONS.iter().position(|m| m.name == through_name) else {
        return Err(ApiMigrationError::UnknownMigration(
            through_name.to_string(),
        ));
    };

    let mut conn = pool
        .acquire()
        .await
        .map_err(ApiMigrationError::Connection)?;
    acquire_advisory_lock(&mut conn).await?;
    let result = bootstrap_through_inner(&mut conn, end_idx).await;
    release_advisory_lock(&mut conn).await;
    result
}

/// Apply all pending [`API_MIGRATIONS`] under `pg_advisory_lock` with per-file transactions.
pub async fn run_api_migrations(
    pool: &PgPool,
) -> std::result::Result<MigrationReport, ApiMigrationError> {
    let pairs: Vec<(&str, &str)> = API_MIGRATIONS.iter().map(|m| (m.name, m.sql)).collect();
    run_migration_slice(pool, &pairs).await
}

async fn run_migration_slice_inner(
    conn: &mut PgConnection,
    migrations: &[(&str, &str)],
) -> std::result::Result<MigrationReport, ApiMigrationError> {
    ensure_ledger_table(&mut *conn).await?;
    let mut report = MigrationReport::default();

    for (name, sql) in migrations {
        if execute_single_migration(&mut *conn, name, sql).await? {
            report.applied.push((*name).to_string());
        } else {
            report.skipped += 1;
        }
    }

    Ok(report)
}

/// Execute an arbitrary ordered list of `(name, sql)` migrations against `_api_schema_migrations`.
pub async fn run_migration_slice(
    pool: &PgPool,
    migrations: &[(&str, &str)],
) -> std::result::Result<MigrationReport, ApiMigrationError> {
    info!("Running API database migrations");

    let mut conn = pool
        .acquire()
        .await
        .map_err(ApiMigrationError::Connection)?;
    acquire_advisory_lock(&mut conn).await?;
    let result = run_migration_slice_inner(&mut conn, migrations).await;
    release_advisory_lock(&mut conn).await;

    if let Ok(ref r) = result {
        info!(
            applied = r.applied.len(),
            skipped = r.skipped,
            "API database migrations completed"
        );
    }

    result
}

async fn execute_single_migration(
    conn: &mut PgConnection,
    name: &str,
    sql: &str,
) -> std::result::Result<bool, ApiMigrationError> {
    let check_sql = format!(
        "SELECT EXISTS (SELECT 1 FROM {} WHERE name = $1)",
        API_MIGRATIONS_LEDGER_TABLE
    );
    let insert_sql = format!(
        "INSERT INTO {} (name) VALUES ($1)",
        API_MIGRATIONS_LEDGER_TABLE
    );

    if is_non_transactional_migration(sql) {
        let applied: bool = sqlx::query_scalar(&check_sql)
            .bind(name)
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| ApiMigrationError::Execution {
                name: name.to_string(),
                message: e.to_string(),
            })?;
        if applied {
            debug!(migration = name, "API migration already applied, skipping");
            return Ok(false);
        }

        info!(
            migration = name,
            "Applying non-transactional API migration under advisory lock"
        );
        sqlx::raw_sql(sql)
            .execute(&mut *conn)
            .await
            .map_err(|e| ApiMigrationError::Execution {
                name: name.to_string(),
                message: e.to_string(),
            })?;
        sqlx::query(&insert_sql)
            .bind(name)
            .execute(&mut *conn)
            .await
            .map_err(|e| ApiMigrationError::Execution {
                name: name.to_string(),
                message: e.to_string(),
            })?;
        info!(migration = name, "API migration applied successfully");
        return Ok(true);
    }

    let mut tx = conn
        .begin()
        .await
        .map_err(|e| ApiMigrationError::Execution {
            name: name.to_string(),
            message: e.to_string(),
        })?;

    let applied: bool = sqlx::query_scalar(&check_sql)
        .bind(name)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ApiMigrationError::Execution {
            name: name.to_string(),
            message: e.to_string(),
        })?;
    if applied {
        debug!(migration = name, "API migration already applied, skipping");
        return Ok(false);
    }

    info!(migration = name, "Applying API migration");
    sqlx::raw_sql(sql)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiMigrationError::Execution {
            name: name.to_string(),
            message: e.to_string(),
        })?;
    sqlx::query(&insert_sql)
        .bind(name)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiMigrationError::Execution {
            name: name.to_string(),
            message: e.to_string(),
        })?;
    tx.commit()
        .await
        .map_err(|e| ApiMigrationError::Execution {
            name: name.to_string(),
            message: e.to_string(),
        })?;

    info!(migration = name, "API migration applied successfully");
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::PathBuf;

    #[test]
    fn every_sql_file_in_crates_api_migrations_is_registered_in_order() {
        let mig_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations");
        let mut disk_files: Vec<String> = std::fs::read_dir(&mig_dir)
            .expect("read crates/api/migrations")
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let name = entry.file_name().into_string().ok()?;
                name.ends_with(".sql").then_some(name)
            })
            .collect();
        disk_files.sort();

        let registered: Vec<String> = API_MIGRATIONS.iter().map(|m| m.name.to_string()).collect();
        let unique: HashSet<&str> = API_MIGRATIONS.iter().map(|m| m.name).collect();

        assert_eq!(
            unique.len(),
            API_MIGRATIONS.len(),
            "duplicate migration names"
        );
        assert_eq!(
            registered, disk_files,
            "every file in crates/api/migrations/ must be registered in API_MIGRATIONS in sorted order"
        );
        for spec in API_MIGRATIONS {
            assert!(
                !spec.sql.trim().is_empty(),
                "migration {} must not be empty",
                spec.name
            );
        }
    }

    #[test]
    fn cli_arg_parsing_and_auto_migrate_opt_in_defaults() {
        // Default server boot: no args -> None; API_AUTO_MIGRATE unset/false -> false
        assert_eq!(
            parse_cli_migration_action(Vec::<String>::new()).unwrap(),
            None
        );
        assert!(!is_auto_migrate_enabled_from_str(None));
        assert!(!is_auto_migrate_enabled_from_str(Some("")));
        assert!(!is_auto_migrate_enabled_from_str(Some("false")));
        assert!(!is_auto_migrate_enabled_from_str(Some("0")));
        assert!(is_auto_migrate_enabled_from_str(Some("true")));
        assert!(is_auto_migrate_enabled_from_str(Some("1")));
        assert!(is_auto_migrate_enabled_from_str(Some("YES")));

        assert_eq!(
            parse_cli_migration_action(["migrate"]).unwrap(),
            Some(CliMigrationAction::Migrate {
                bootstrap_existing: false
            })
        );
        assert_eq!(
            parse_cli_migration_action(["--migrate-only"]).unwrap(),
            Some(CliMigrationAction::Migrate {
                bootstrap_existing: false
            })
        );
        assert_eq!(
            parse_cli_migration_action(["migrate", "--bootstrap"]).unwrap(),
            Some(CliMigrationAction::Migrate {
                bootstrap_existing: true
            })
        );
        assert_eq!(
            parse_cli_migration_action(["--migrate-only", "--bootstrap"]).unwrap(),
            Some(CliMigrationAction::Migrate {
                bootstrap_existing: true
            })
        );
        assert_eq!(
            parse_cli_migration_action(["migrate", "--bootstrap-only"]).unwrap(),
            Some(CliMigrationAction::BootstrapOnly)
        );
        assert_eq!(
            parse_cli_migration_action([
                "migrate",
                "--bootstrap-through",
                "0019_cctp_approval_verified_at.sql"
            ])
            .unwrap(),
            Some(CliMigrationAction::BootstrapThrough {
                through_name: "0019_cctp_approval_verified_at.sql".to_string(),
                run_remaining: true,
            })
        );
        assert!(parse_cli_migration_action(["migrate", "--unknown"]).is_err());
    }

    #[test]
    fn non_transactional_directive_detection() {
        assert!(!is_non_transactional_migration("CREATE TABLE t (id INT);"));
        assert!(is_non_transactional_migration(
            "-- stellarroute:no-transaction\nCREATE INDEX CONCURRENTLY idx_t ON t (id);"
        ));
    }

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

    async fn setup_temp_pg(prefix: &str) -> (PgPool, String, PgPool) {
        let base_url = std::env::var("TEST_DATABASE_URL")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .expect("TEST_DATABASE_URL or DATABASE_URL required");
        let admin_url = replace_db_name(&base_url, "postgres");
        let admin_pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&admin_url)
            .await
            .expect("connect admin db");
        let db_name = format!("{}_{}", prefix, uuid::Uuid::new_v4().simple());
        sqlx::query(&format!("CREATE DATABASE {}", db_name))
            .execute(&admin_pool)
            .await
            .expect("create temp db");
        let test_pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(6)
            .connect(&replace_db_name(&base_url, &db_name))
            .await
            .expect("connect temp db");
        (admin_pool, db_name, test_pool)
    }

    async fn drop_temp_pg(admin_pool: PgPool, db_name: &str, test_pool: PgPool) {
        test_pool.close().await;
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

    #[tokio::test]
    #[ignore = "Requires PostgreSQL via TEST_DATABASE_URL or DATABASE_URL"]
    async fn migrate_twice_is_noop_on_second_run() {
        let (admin_pool, db_name, pool) = setup_temp_pg("sr_1651_twice").await;

        let first = run_api_migrations(&pool)
            .await
            .expect("first run applies all API migrations");
        assert_eq!(first.applied.len(), API_MIGRATIONS.len());
        assert_eq!(first.skipped, 0);

        let second = run_api_migrations(&pool)
            .await
            .expect("second run is a no-op");
        assert!(second.applied.is_empty());
        assert_eq!(second.skipped, API_MIGRATIONS.len());

        let ledger_count: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM {}",
            API_MIGRATIONS_LEDGER_TABLE
        ))
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(ledger_count as usize, API_MIGRATIONS.len());

        drop_temp_pg(admin_pool, &db_name, pool).await;
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL via TEST_DATABASE_URL or DATABASE_URL"]
    async fn existing_staging_db_bootstraps_without_rerunning_ddl_and_applies_missing_files() {
        let (admin_pool, db_name, pool) = setup_temp_pg("sr_1651_boot").await;

        // Simulate an existing staging DB where all migrations EXCEPT `0020_card_tables.sql`
        // were previously piped manually via `psql` (with no `_api_schema_migrations` ledger).
        // Note: `0011_contract_registry.sql` creates trigger `contract_registry_updated_at`
        // without `IF NOT EXISTS`, so re-running its DDL would fail immediately.
        for spec in API_MIGRATIONS {
            if spec.name == "0020_card_tables.sql" {
                continue;
            }
            sqlx::raw_sql(spec.sql)
                .execute(&pool)
                .await
                .unwrap_or_else(|e| panic!("pre-seed {} failed: {}", spec.name, e));
        }

        let boot = bootstrap_existing_api_migrations(&pool)
            .await
            .expect("bootstrap succeeds without re-running DDL");
        assert_eq!(boot.recorded.len(), API_MIGRATIONS.len() - 1);
        assert!(!boot.recorded.iter().any(|n| n == "0020_card_tables.sql"));

        let mig = run_api_migrations(&pool)
            .await
            .expect("subsequent migrate applies only 0020_card_tables.sql");
        assert_eq!(mig.applied, vec!["0020_card_tables.sql".to_string()]);
        assert_eq!(mig.skipped, API_MIGRATIONS.len() - 1);

        let card_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'card_applications')",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(card_exists);

        drop_temp_pg(admin_pool, &db_name, pool).await;
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL via TEST_DATABASE_URL or DATABASE_URL"]
    async fn failing_migration_rolls_back_partial_ddl_and_ledger_entry() {
        let (admin_pool, db_name, pool) = setup_temp_pg("sr_1651_rollback").await;

        let specs = [
            (
                "0001_ok.sql",
                "CREATE TABLE api_mig_ok (id INT PRIMARY KEY);",
            ),
            (
                "0002_broken.sql",
                "CREATE TABLE api_mig_partial (id INT PRIMARY KEY); SELECT * FROM nonexistent_table_1651;",
            ),
        ];

        let err = run_migration_slice(&pool, &specs).await;
        assert!(err.is_err());

        let partial_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'api_mig_partial')",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(
            !partial_exists,
            "failed transactional migration must roll back partial DDL"
        );

        let recorded: Vec<String> = sqlx::query_scalar(&format!(
            "SELECT name FROM {} ORDER BY name ASC",
            API_MIGRATIONS_LEDGER_TABLE
        ))
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(recorded, vec!["0001_ok.sql".to_string()]);

        drop_temp_pg(admin_pool, &db_name, pool).await;
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL via TEST_DATABASE_URL or DATABASE_URL"]
    async fn ec2_staging_manual_cctp_and_indexer_0015_bootstraps_cleanly() {
        let (admin_pool, db_name, pool) = setup_temp_pg("sr_1651_ec2").await;

        // Reproduce the exact legacy deploy-ec2-staging.sh state:
        // 1) crates/indexer/migrations/0015_swap_prepared_quotes.sql piped manually
        // 2) every crates/api/migrations/*cctp*.sql piped manually
        let indexer_0015 =
            include_str!("../../../crates/indexer/migrations/0015_swap_prepared_quotes.sql");
        sqlx::raw_sql(indexer_0015)
            .execute(&pool)
            .await
            .expect("seed indexer 0015_swap_prepared_quotes.sql");
        for spec in API_MIGRATIONS {
            if spec.name.contains("cctp") {
                sqlx::raw_sql(spec.sql)
                    .execute(&pool)
                    .await
                    .unwrap_or_else(|e| panic!("seed {} failed: {}", spec.name, e));
            }
        }

        let boot = bootstrap_existing_api_migrations(&pool)
            .await
            .expect("bootstrap legacy EC2 staging DB");
        assert!(boot
            .recorded
            .contains(&"0013_swap_prepared_quotes.sql".to_string()));
        assert!(boot
            .recorded
            .contains(&"0014_swap_prepared_quotes_security.sql".to_string()));
        assert!(boot
            .recorded
            .contains(&"20260814_cctp_finality_fast.sql".to_string()));
        assert!(!boot.recorded.contains(&"0020_card_tables.sql".to_string()));

        let mig = run_api_migrations(&pool)
            .await
            .expect("apply remaining API migrations cleanly");
        assert!(mig.applied.contains(&"0020_card_tables.sql".to_string()));
        assert!(mig
            .applied
            .contains(&"0012_swap_submit_audit_log.sql".to_string()));

        let second = run_api_migrations(&pool)
            .await
            .expect("second run is a complete no-op");
        assert!(second.applied.is_empty());
        assert_eq!(second.skipped, API_MIGRATIONS.len());

        drop_temp_pg(admin_pool, &db_name, pool).await;
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL via TEST_DATABASE_URL or DATABASE_URL"]
    async fn concurrent_api_migrations_serialize_under_advisory_lock() {
        let (admin_pool, db_name, pool) = setup_temp_pg("sr_1651_conc").await;

        let (r1, r2, r3, r4) = tokio::join!(
            run_api_migrations(&pool),
            run_api_migrations(&pool),
            run_api_migrations(&pool),
            run_api_migrations(&pool)
        );

        let mut total_applied = 0usize;
        for res in [r1, r2, r3, r4] {
            let report = res.expect("concurrent migrate succeeds");
            total_applied += report.applied.len();
        }
        assert_eq!(
            total_applied,
            API_MIGRATIONS.len(),
            "each migration must be applied once across concurrent runners"
        );

        drop_temp_pg(admin_pool, &db_name, pool).await;
    }
}
