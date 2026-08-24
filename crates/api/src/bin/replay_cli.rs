//! replay-cli — command-line tool for the deterministic quote replay system.
//!
//! # Usage
//!
//! ```text
//! replay-cli fetch <artifact_id>
//! replay-cli run   <artifact_id>
//! replay-cli diff  <artifact_id>
//! replay-cli list  [--incident <id>] [--base <asset>] [--quote <asset>] [--limit N]
//! ```
//!
//! Reads `DATABASE_URL` from the environment.
//! Exits with code 1 on any error; errors go to stderr.

use clap::{Parser, Subcommand};
use sqlx::postgres::PgPoolOptions;
use stellarroute_api::replay::{artifact::ReplayArtifact, diff::DiffEngine, engine::ReplayEngine};
use uuid::Uuid;

#[derive(Parser)]
#[command(
    name = "replay-cli",
    about = "Deterministic quote replay tool for StellarRoute incident analysis",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Fetch and print a stored replay artifact as JSON
    Fetch {
        /// Artifact UUID
        artifact_id: String,
    },
    /// Run the replay pipeline and print the ReplayOutput as JSON
    Run {
        /// Artifact UUID
        artifact_id: String,
    },
    /// Run the replay pipeline, diff against original, and print the DiffReport as JSON
    Diff {
        /// Artifact UUID
        artifact_id: String,
    },
    /// List stored artifacts (most recent first)
    List {
        /// Filter by incident ID
        #[arg(long)]
        incident: Option<String>,
        /// Filter by base asset (e.g. "native" or "USDC")
        #[arg(long)]
        base: Option<String>,
        /// Filter by quote asset
        #[arg(long)]
        quote: Option<String>,
        /// Maximum number of results (default: 20, max: 100)
        #[arg(long, default_value = "20")]
        limit: i64,
    },
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

    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL environment variable is not set"))?;

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to database: {}", e))?;

    match cli.command {
        Commands::Fetch { artifact_id } => {
            let id = parse_uuid(&artifact_id)?;
            let artifact = ReplayArtifact::fetch(&pool, id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            println!("{}", serde_json::to_string_pretty(&artifact)?);
        }

        Commands::Run { artifact_id } => {
            let id = parse_uuid(&artifact_id)?;
            let artifact = ReplayArtifact::fetch(&pool, id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            let output = ReplayEngine::run(&artifact).map_err(|e| anyhow::anyhow!("{}", e))?;
            println!("{}", serde_json::to_string_pretty(&output)?);
        }

        Commands::Diff { artifact_id } => {
            let id = parse_uuid(&artifact_id)?;
            let artifact = ReplayArtifact::fetch(&pool, id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            let output = ReplayEngine::run(&artifact).map_err(|e| anyhow::anyhow!("{}", e))?;
            let report = DiffEngine::diff(&artifact, &output);
            println!("{}", serde_json::to_string_pretty(&report)?);
        }

        Commands::List {
            incident,
            base,
            quote,
            limit,
        } => {
            let limit = limit.clamp(1, 100);
            let summaries = ReplayArtifact::list(
                &pool,
                incident.as_deref(),
                base.as_deref(),
                quote.as_deref(),
                limit,
                0,
            )
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
            println!("{}", serde_json::to_string_pretty(&summaries)?);
        }
    }

    Ok(())
}

fn parse_uuid(s: &str) -> anyhow::Result<Uuid> {
    Uuid::parse_str(s).map_err(|_| anyhow::anyhow!("Invalid artifact ID '{}': must be a UUID", s))
}
