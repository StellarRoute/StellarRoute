use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

use stellarroute_routing::fixtures::FixtureBuilder;
use stellarroute_routing::pathfinder::LiquidityEdge;

#[derive(Parser, Debug)]
#[command(name = "fixture-gen")]
#[command(about = "Generate deterministic routing graph fixtures for integration tests")]
#[command(author, version, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate a minimal market fixture (XLM → USDC)
    Minimal {
        /// Output format: json or sql
        #[arg(short, long, default_value = "json")]
        format: OutputFormat,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Generate a multi-hop market fixture (XLM → USDC → EURC)
    MultiHop {
        /// Output format: json or sql
        #[arg(short, long, default_value = "json")]
        format: OutputFormat,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Generate a thin liquidity market fixture
    ThinLiquidity {
        /// Output format: json or sql
        #[arg(short, long, default_value = "json")]
        format: OutputFormat,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// List all available fixtures
    List,
}

#[derive(Clone, Debug)]
enum OutputFormat {
    Json,
    Sql,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "sql" => Ok(OutputFormat::Sql),
            _ => Err(format!("Unknown format: {}. Use 'json' or 'sql'", s)),
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Minimal { format, output } => {
            generate_fixture("minimal-market", &format, output)?;
        }
        Commands::MultiHop { format, output } => {
            generate_fixture("multi-hop", &format, output)?;
        }
        Commands::ThinLiquidity { format, output } => {
            generate_fixture("thin-liquidity", &format, output)?;
        }
        Commands::List => list_fixtures(),
    }

    Ok(())
}

fn generate_fixture(name: &str, format: &OutputFormat, output: Option<PathBuf>) -> Result<()> {
    let builder = match name {
        "minimal-market" => FixtureBuilder::minimal_market(),
        "multi-hop" => FixtureBuilder::multi_hop_market(),
        "thin-liquidity" => FixtureBuilder::thin_liquidity_market(),
        _ => anyhow::bail!("Unknown fixture: {}", name),
    };

    match format {
        OutputFormat::Json => output_json(&builder, name, output)?,
        OutputFormat::Sql => output_sql(&builder, name, output)?,
    }

    Ok(())
}

fn output_json(builder: &FixtureBuilder, name: &str, output: Option<PathBuf>) -> Result<()> {
    let edges = builder.build_edges();

    let fixture = json!({
        "name": name,
        "description": format!("Routing graph fixture: {}", name),
        "assets": builder.assets().iter().map(|a| {
            match &a.asset_type {
                stellarroute_routing::fixtures::AssetType::Native => {
                    json!({"key": a.key, "type": "native"})
                }
                stellarroute_routing::fixtures::AssetType::CreditAlphanum4 { code, issuer } => {
                    json!({"key": a.key, "type": "credit4", "code": code, "issuer": issuer})
                }
                stellarroute_routing::fixtures::AssetType::CreditAlphanum12 { code, issuer } => {
                    json!({"key": a.key, "type": "credit12", "code": code, "issuer": issuer})
                }
            }
        }).collect::<Vec<_>>(),
        "sdex_offers": builder.sdex_offers().iter().map(|o| {
            json!({
                "offer_id": o.offer_id,
                "seller": o.seller,
                "selling_asset": o.selling_asset.key,
                "buying_asset": o.buying_asset.key,
                "amount": o.amount,
                "price": o.price,
            })
        }).collect::<Vec<_>>(),
        "amm_pools": builder.amm_pools().iter().map(|p| {
            json!({
                "pool_address": p.pool_address,
                "selling_asset": p.selling_asset.key,
                "buying_asset": p.buying_asset.key,
                "reserve_selling": p.reserve_selling,
                "reserve_buying": p.reserve_buying,
                "fee_bps": p.fee_bps,
            })
        }).collect::<Vec<_>>(),
        "edges": edges.iter().map(|e| {
            json!({
                "from": e.from,
                "to": e.to,
                "venue_type": e.venue_type,
                "venue_ref": e.venue_ref,
                "liquidity": e.liquidity,
                "price": e.price,
                "fee_bps": e.fee_bps,
            })
        }).collect::<Vec<_>>(),
    });

    let content = serde_json::to_string_pretty(&fixture)
        .context("failed to serialize fixture to JSON")?;

    if let Some(path) = output {
        fs::write(&path, content)
            .context(format!("failed to write JSON fixture to {:?}", path))?;
        println!("✓ JSON fixture written to {}", path.display());
    } else {
        println!("{}", content);
    }

    Ok(())
}

fn output_sql(builder: &FixtureBuilder, name: &str, output: Option<PathBuf>) -> Result<()> {
    let mut sql = String::new();

    // SQL header
    sql.push_str(&format!("-- Fixture: {}\n", name));
    sql.push_str("-- This SQL script loads fixtures into the normalized_liquidity table\n\n");

    sql.push_str("BEGIN TRANSACTION;\n\n");

    // Insert assets
    sql.push_str("-- Assets\n");
    for asset in builder.assets() {
        match &asset.asset_type {
            stellarroute_routing::fixtures::AssetType::Native => {
                sql.push_str(&format!(
                    "INSERT INTO assets (key, type) VALUES ('{}', 'native') ON CONFLICT DO NOTHING;\n",
                    asset.key
                ));
            }
            stellarroute_routing::fixtures::AssetType::CreditAlphanum4 { code, issuer } => {
                sql.push_str(&format!(
                    "INSERT INTO assets (key, type, code, issuer) VALUES ('{}', 'credit4', '{}', '{}') ON CONFLICT DO NOTHING;\n",
                    asset.key, code, issuer
                ));
            }
            stellarroute_routing::fixtures::AssetType::CreditAlphanum12 { code, issuer } => {
                sql.push_str(&format!(
                    "INSERT INTO assets (key, type, code, issuer) VALUES ('{}', 'credit12', '{}', '{}') ON CONFLICT DO NOTHING;\n",
                    asset.key, code, issuer
                ));
            }
        }
    }

    sql.push_str("\n-- SDEX Offers\n");
    for offer in builder.sdex_offers() {
        sql.push_str(&format!(
            "INSERT INTO sdex_offers (offer_id, seller, selling_asset, buying_asset, amount, price, last_modified_ledger) \
             VALUES ({}, '{}', '{}', '{}', '{}', '{}', {}) ON CONFLICT DO NOTHING;\n",
            offer.offer_id,
            offer.seller,
            offer.selling_asset.key,
            offer.buying_asset.key,
            offer.amount,
            offer.price,
            offer.last_modified_ledger
        ));
    }

    sql.push_str("\n-- AMM Pools\n");
    for pool in builder.amm_pools() {
        sql.push_str(&format!(
            "INSERT INTO amm_pool_reserves (pool_address, selling_asset, buying_asset, reserve_selling, reserve_buying, fee_bps, last_updated_ledger) \
             VALUES ('{}', '{}', '{}', '{}', '{}', {}, {}) ON CONFLICT DO NOTHING;\n",
            pool.pool_address,
            pool.selling_asset.key,
            pool.buying_asset.key,
            pool.reserve_selling,
            pool.reserve_buying,
            pool.fee_bps,
            pool.last_updated_ledger
        ));
    }

    sql.push_str("\nCOMMIT;\n");

    if let Some(path) = output {
        fs::write(&path, sql).context(format!("failed to write SQL fixture to {:?}", path))?;
        println!("✓ SQL fixture written to {}", path.display());
    } else {
        println!("{}", sql);
    }

    Ok(())
}

fn list_fixtures() {
    println!("Available fixtures:\n");
    println!("  minimal-market    XLM → USDC via 1 SDEX offer + 1 AMM pool (single-hop scenario)");
    println!("  multi-hop         XLM → USDC → EURC via SDEX + AMM pools (2-hop scenario)");
    println!("  thin-liquidity    XLM → USDC with very low reserves (liquidity-floor test)\n");
    println!("Usage: fixture-gen <COMMAND> --format <FORMAT> --output <FILE>\n");
    println!("Formats: json (default), sql\n");
    println!("Examples:");
    println!("  fixture-gen minimal --format json --output fixtures/minimal.json");
    println!("  fixture-gen multi-hop --format sql --output fixtures/multi-hop.sql");
}
