//! audit-export — command-line tool for exporting redacted route and swap submit audit logs.
//!
//! # Usage
//!
//! ```text
//! audit-export [--table <all|route|swap>] [--dry-run] [--from <ISO8601>] [--to <ISO8601>]
//!              [--limit N] [--output-file <PATH>] [--s3-bucket <BUCKET>] [--s3-prefix <PREFIX>]
//!              [--s3-endpoint <URL>] [--db-url <URL>]
//! ```
//!
//! Reads `DATABASE_URL` from the environment if `--db-url` is not specified.
//! Redacts all sensitive fields (asset issuers, raw account public keys, secrets)
//! using `AuditRedactor` before outputting.

use clap::{Parser, ValueEnum};
use sqlx::postgres::PgPoolOptions;
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;
use stellarroute_api::audit::{
    AuditExclusion, AuditInputs, AuditOutcome, AuditRedactor, AuditSelected, RouteAuditEntry,
    SwapSubmitAuditEntry, SwapSubmitOutcome,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum TargetTable {
    All,
    Route,
    Swap,
}

#[derive(Parser)]
#[command(
    name = "audit-export",
    about = "Export redacted StellarRoute audit logs to stdout, local file, or object storage",
    version
)]
struct Cli {
    /// Target audit table to export
    #[arg(long, value_enum, default_value_t = TargetTable::All)]
    table: TargetTable,

    /// Print redacted NDJSON entries to stdout (dry-run mode)
    #[arg(long, short = 'd')]
    dry_run: bool,

    /// Filter logs starting from timestamp (ISO8601, e.g. 2026-08-01T00:00:00Z)
    #[arg(long)]
    from: Option<String>,

    /// Filter logs up to timestamp (ISO8601, e.g. 2026-08-30T23:59:59Z)
    #[arg(long)]
    to: Option<String>,

    /// Maximum number of records to export per table
    #[arg(long, default_value = "10000")]
    limit: i64,

    /// Path to local output file (.jsonl)
    #[arg(long)]
    output_file: Option<PathBuf>,

    /// Target S3/object storage bucket name for export upload
    #[arg(long)]
    s3_bucket: Option<String>,

    /// Object storage key prefix/folder (default: "audit/")
    #[arg(long, default_value = "audit/")]
    s3_prefix: String,

    /// Custom object storage endpoint URL (e.g. MinIO, Cloudflare R2)
    #[arg(long)]
    s3_endpoint: Option<String>,

    /// PostgreSQL database connection URL
    #[arg(long)]
    db_url: Option<String>,
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}

async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let db_url = match cli.db_url {
        Some(url) => url,
        None => std::env::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL environment variable is not set"))?,
    };

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&db_url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to database: {}", e))?;

    let from_dt = cli.from.as_deref().map(parse_iso8601).transpose()?;
    let to_dt = cli.to.as_deref().map(parse_iso8601).transpose()?;
    let limit = cli.limit.clamp(1, 1_000_000);

    let mut route_entries: Vec<RouteAuditEntry> = Vec::new();
    let mut swap_entries: Vec<SwapSubmitAuditEntry> = Vec::new();

    if cli.table == TargetTable::All || cli.table == TargetTable::Route {
        route_entries = fetch_route_audit_logs(&pool, from_dt, to_dt, limit).await?;
    }

    if cli.table == TargetTable::All || cli.table == TargetTable::Swap {
        swap_entries = fetch_swap_submit_audit_logs(&pool, from_dt, to_dt, limit).await?;
    }

    // Ensure all entries are strictly redacted before serialization
    for entry in &mut route_entries {
        AuditRedactor::redact(entry);
    }
    for entry in &mut swap_entries {
        if !entry.account.contains('#') && entry.account != "native" {
            entry.account = AuditRedactor::redact_account(&entry.account);
        }
    }

    let mut lines: Vec<String> = Vec::with_capacity(route_entries.len() + swap_entries.len());
    for entry in &route_entries {
        lines.push(serde_json::to_string(entry)?);
    }
    for entry in &swap_entries {
        lines.push(serde_json::to_string(entry)?);
    }

    let total_records = lines.len();

    if cli.dry_run || (cli.output_file.is_none() && cli.s3_bucket.is_none()) {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        for line in &lines {
            writeln!(handle, "{}", line)?;
        }
        eprintln!("Exported {} redacted audit log records to stdout", total_records);
        return Ok(());
    }

    if let Some(ref path) = cli.output_file {
        let mut file = File::create(path)?;
        for line in &lines {
            writeln!(file, "{}", line)?;
        }
        eprintln!(
            "Successfully exported {} redacted audit log records to {}",
            total_records,
            path.display()
        );
    }

    if let Some(ref bucket) = cli.s3_bucket {
        upload_to_object_storage(bucket, &cli.s3_prefix, cli.s3_endpoint.as_deref(), &lines).await?;
    }

    Ok(())
}

fn parse_iso8601(s: &str) -> anyhow::Result<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|_| anyhow::anyhow!("Invalid timestamp '{}': expected ISO8601 / RFC3339 format", s))
}

async fn fetch_route_audit_logs(
    pool: &sqlx::PgPool,
    from: Option<chrono::DateTime<chrono::Utc>>,
    to: Option<chrono::DateTime<chrono::Utc>>,
    limit: i64,
) -> anyhow::Result<Vec<RouteAuditEntry>> {
    let rows = sqlx::query(
        r#"
        SELECT request_id, trace_id, logged_at, latency_ms,
               outcome, cache_hit, inputs, selected, exclusions
        FROM route_audit_log
        WHERE ($1::timestamptz IS NULL OR logged_at >= $1)
          AND ($2::timestamptz IS NULL OR logged_at <= $2)
        ORDER BY logged_at ASC
        LIMIT $3
        "#,
    )
    .bind(from)
    .bind(to)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("Failed to fetch route audit logs: {}", e))?;

    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        use sqlx::Row;
        let inputs: AuditInputs = serde_json::from_value(row.get::<serde_json::Value, _>("inputs"))?;
        let selected: Option<AuditSelected> = row
            .get::<Option<serde_json::Value>, _>("selected")
            .map(serde_json::from_value)
            .transpose()?;
        let exclusions: Vec<AuditExclusion> =
            serde_json::from_value(row.get::<serde_json::Value, _>("exclusions"))?;

        let outcome_str: &str = row.get("outcome");
        let outcome = match outcome_str {
            "success" => AuditOutcome::Success,
            "no_route" => AuditOutcome::NoRoute,
            "stale_data" => AuditOutcome::StaleData,
            _ => AuditOutcome::Error,
        };

        entries.push(RouteAuditEntry {
            schema_version: stellarroute_api::audit::schema::AUDIT_SCHEMA_VERSION,
            request_id: row.get("request_id"),
            trace_id: row.get("trace_id"),
            logged_at: row.get("logged_at"),
            latency_ms: row.get::<i32, _>("latency_ms") as u64,
            outcome,
            cache_hit: row.get("cache_hit"),
            inputs,
            selected,
            exclusions,
        });
    }

    Ok(entries)
}

async fn fetch_swap_submit_audit_logs(
    pool: &sqlx::PgPool,
    from: Option<chrono::DateTime<chrono::Utc>>,
    to: Option<chrono::DateTime<chrono::Utc>>,
    limit: i64,
) -> anyhow::Result<Vec<SwapSubmitAuditEntry>> {
    let rows = sqlx::query(
        r#"
        SELECT quote_id, tx_hash, account, request_id, trace_id,
               logged_at, latency_ms, outcome, error_class, metadata
        FROM swap_submit_audit_log
        WHERE ($1::timestamptz IS NULL OR logged_at >= $1)
          AND ($2::timestamptz IS NULL OR logged_at <= $2)
        ORDER BY logged_at ASC
        LIMIT $3
        "#,
    )
    .bind(from)
    .bind(to)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("Failed to fetch swap submit audit logs: {}", e))?;

    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        use sqlx::Row;
        let metadata: serde_json::Value = row.get("metadata");
        let outcome_str: &str = row.get("outcome");
        let outcome = match outcome_str {
            "prepared" => SwapSubmitOutcome::Prepared,
            "submitted" => SwapSubmitOutcome::Submitted,
            _ => SwapSubmitOutcome::Failed,
        };

        entries.push(SwapSubmitAuditEntry {
            schema_version: stellarroute_api::audit::schema::AUDIT_SCHEMA_VERSION,
            quote_id: row.get("quote_id"),
            tx_hash: row.get("tx_hash"),
            account: row.get("account"),
            request_id: row.get("request_id"),
            trace_id: row.get("trace_id"),
            logged_at: row.get("logged_at"),
            latency_ms: row.get::<i32, _>("latency_ms") as u64,
            outcome,
            error_class: row.get("error_class"),
            metadata,
        });
    }

    Ok(entries)
}

async fn upload_to_object_storage(
    bucket: &str,
    prefix: &str,
    endpoint: Option<&str>,
    lines: &[String],
) -> anyhow::Result<()> {
    let now = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let key = format!("{}{}_audit_export.jsonl", prefix, now);
    let tmp_path = std::env::temp_dir().join(format!("audit_export_{}.jsonl", now));

    let mut file = File::create(&tmp_path)?;
    for line in lines {
        writeln!(file, "{}", line)?;
    }
    drop(file);

    eprintln!(
        "Uploading {} records to s3://{}/{}",
        lines.len(),
        bucket,
        key
    );

    // Try executing aws cli command if available
    let s3_uri = format!("s3://{}/{}", bucket, key);
    let mut cmd = std::process::Command::new("aws");
    cmd.arg("s3").arg("cp").arg(&tmp_path).arg(&s3_uri);
    if let Some(ep) = endpoint {
        cmd.arg("--endpoint-url").arg(ep);
    }

    match cmd.status() {
        Ok(status) if status.success() => {
            eprintln!("Successfully uploaded audit export to {}", s3_uri);
            let _ = std::fs::remove_file(tmp_path);
            Ok(())
        }
        Ok(status) => {
            eprintln!(
                "aws s3 cp exited with status {}; file remains at {}",
                status,
                tmp_path.display()
            );
            Err(anyhow::anyhow!("S3 upload command failed with status {}", status))
        }
        Err(e) => {
            eprintln!(
                "AWS CLI unavailable ({}); export file saved locally at {}",
                e,
                tmp_path.display()
            );
            Ok(())
        }
    }
}
