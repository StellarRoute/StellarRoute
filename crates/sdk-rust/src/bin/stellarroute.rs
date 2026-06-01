use clap::{builder::TypedValueParser, CommandFactory, Parser, Subcommand, ValueEnum};
use serde::Serialize;
use std::ffi::OsStr;
use std::num::NonZeroUsize;
use stellarroute_sdk::{
    HealthResponse, OrderbookLevel, OrderbookResponse, PairsResponse, QuoteRequest, QuoteResponse,
    QuoteType, SdkError, StellarRouteClient,
};

const EXIT_SUCCESS: i32 = 0;
const EXIT_USAGE_ERROR: i32 = 2;
const EXIT_CONFIG_ERROR: i32 = 3;
const EXIT_RUNTIME_ERROR: i32 = 4;

#[derive(Parser, Debug)]
#[command(
    name = "stellarroute",
    about = "Query the StellarRoute API from the terminal",
    long_about = "Query the StellarRoute API from terminal workflows with machine-friendly or human-friendly output.",
    after_help = "Output formats:\n  json | table | human\n\nExit codes:\n  0 success\n  2 CLI usage/validation error\n  3 invalid client configuration\n  4 runtime/API error",
    version
)]
struct Cli {
    #[arg(
        long,
        global = true,
        env = "STELLARROUTE_API_URL",
        default_value = "http://127.0.0.1:3000",
        help = "Base URL for the StellarRoute API"
    )]
    api_url: String,

    #[arg(
        long,
        global = true,
        value_enum,
        default_value_t = OutputFormat::Human,
        help = "Output format: json, table, or human"
    )]
    output: OutputFormat,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lower")]
enum OutputFormat {
    Json,
    Table,
    Human,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "Check API health")]
    Health,
    #[command(about = "List available trading pairs")]
    Pairs {
        #[arg(long, default_value_t = 10, help = "Maximum number of pairs to print")]
        limit: usize,
    },
    #[command(about = "Get a price quote for a trading pair")]
    Quote {
        #[arg(
            value_parser = parse_asset,
            help = "Base asset: native, CODE, or CODE:ISSUER"
        )]
        base: String,
        #[arg(
            value_parser = parse_asset,
            help = "Quote asset: native, CODE, or CODE:ISSUER"
        )]
        quote: String,
        #[arg(
            long,
            value_parser = PositiveAmountParser,
            help = "Trade amount as a positive decimal string"
        )]
        amount: Option<String>,
        #[arg(
            long,
            value_enum,
            default_value_t = QuoteTypeArg::Sell,
            help = "Whether the amount is for selling or buying the base asset"
        )]
        quote_type: QuoteTypeArg,
    },
    #[command(about = "Show the orderbook for a trading pair")]
    Orderbook {
        #[arg(
            value_parser = parse_asset,
            help = "Base asset: native, CODE, or CODE:ISSUER"
        )]
        base: String,
        #[arg(
            value_parser = parse_asset,
            help = "Quote asset: native, CODE, or CODE:ISSUER"
        )]
        quote: String,
        #[arg(
            long,
            default_value_t = NonZeroUsize::new(10).expect("non-zero"),
            help = "Maximum number of levels to print per side"
        )]
        levels: NonZeroUsize,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum QuoteTypeArg {
    Sell,
    Buy,
}

impl From<QuoteTypeArg> for QuoteType {
    fn from(value: QuoteTypeArg) -> Self {
        match value {
            QuoteTypeArg::Sell => QuoteType::Sell,
            QuoteTypeArg::Buy => QuoteType::Buy,
        }
    }
}

#[derive(Clone)]
struct PositiveAmountParser;

impl TypedValueParser for PositiveAmountParser {
    type Value = String;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let raw = value.to_str().ok_or_else(|| {
            clap::Error::raw(
                clap::error::ErrorKind::InvalidUtf8,
                "Amount must be valid UTF-8",
            )
        })?;

        match raw.parse::<f64>() {
            Ok(amount) if amount.is_finite() && amount > 0.0 => Ok(raw.to_string()),
            _ => {
                let mut cmd = cmd.clone();
                Err(cmd.error(
                    clap::error::ErrorKind::ValueValidation,
                    format!(
                        "{} must be a positive number",
                        arg.map(|a| a.to_string())
                            .unwrap_or_else(|| "amount".to_string())
                    ),
                ))
            }
        }
    }
}

#[tokio::main]
async fn main() {
    Cli::command().debug_assert();

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let kind = error.kind();
            let _ = error.print();
            let code = match kind {
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => {
                    EXIT_SUCCESS
                }
                _ => EXIT_USAGE_ERROR,
            };
            std::process::exit(code);
        }
    };

    match run(cli).await {
        Ok(output) => {
            println!("{output}");
        }
        Err((code, message)) => {
            eprintln!("Error: {message}");
            std::process::exit(code);
        }
    }
}

async fn run(cli: Cli) -> Result<String, (i32, String)> {
    let client = StellarRouteClient::new(&cli.api_url)
        .map_err(|error| (exit_code_for_sdk_error(&error), error.to_string()))?;

    match cli.command {
        Commands::Health => render_health(&client, cli.output)
            .await
            .map_err(|error| (exit_code_for_sdk_error(&error), error.to_string())),
        Commands::Pairs { limit } => render_pairs(&client, limit, cli.output)
            .await
            .map_err(|error| (exit_code_for_sdk_error(&error), error.to_string())),
        Commands::Quote {
            base,
            quote,
            amount,
            quote_type,
        } => render_quote(
            &client,
            QuoteRequest {
                base: &base,
                quote: &quote,
                amount: amount.as_deref(),
                quote_type: quote_type.into(),
            },
            cli.output,
        )
        .await
        .map_err(|error| (exit_code_for_sdk_error(&error), error.to_string())),
        Commands::Orderbook {
            base,
            quote,
            levels,
        } => render_orderbook(&client, &base, &quote, levels.get(), cli.output)
            .await
            .map_err(|error| (exit_code_for_sdk_error(&error), error.to_string())),
    }
}

async fn render_health(
    client: &StellarRouteClient,
    output: OutputFormat,
) -> Result<String, SdkError> {
    let response = client.health().await?;

    format_health(&response, output)
}

async fn render_pairs(
    client: &StellarRouteClient,
    limit: usize,
    output: OutputFormat,
) -> Result<String, SdkError> {
    let response = client.pairs().await?;

    format_pairs(&response, limit, output)
}

async fn render_quote(
    client: &StellarRouteClient,
    request: QuoteRequest<'_>,
    output: OutputFormat,
) -> Result<String, SdkError> {
    let response = client.quote(request).await?;

    format_quote(&response, output)
}

async fn render_orderbook(
    client: &StellarRouteClient,
    base: &str,
    quote: &str,
    levels: usize,
    output: OutputFormat,
) -> Result<String, SdkError> {
    let mut response = client.orderbook(base, quote).await?;
    response.asks.truncate(levels);
    response.bids.truncate(levels);

    format_orderbook(&response, output)
}

fn format_health(response: &HealthResponse, output: OutputFormat) -> Result<String, SdkError> {
    match output {
        OutputFormat::Human => {
            let mut lines = vec![
                format!("status: {}", response.status),
                format!("version: {}", response.version),
                format!("timestamp: {}", response.timestamp),
            ];

            if !response.components.is_empty() {
                lines.push("components:".to_string());
                let mut components = response.components.iter().collect::<Vec<_>>();
                components.sort_by(|a, b| a.0.cmp(b.0));
                for (name, status) in components {
                    lines.push(format!("  {name}: {status}"));
                }
            }

            Ok(lines.join("\n"))
        }
        OutputFormat::Table => {
            let summary = format_table(
                &["field", "value"],
                vec![
                    vec!["status".to_string(), response.status.clone()],
                    vec!["version".to_string(), response.version.clone()],
                    vec!["timestamp".to_string(), response.timestamp.clone()],
                ],
            );

            if response.components.is_empty() {
                return Ok(summary);
            }

            let mut components = response.components.iter().collect::<Vec<_>>();
            components.sort_by(|a, b| a.0.cmp(b.0));
            let component_rows = components
                .into_iter()
                .map(|(name, status)| vec![name.clone(), status.clone()])
                .collect::<Vec<_>>();

            Ok(format!(
                "{}\n\ncomponents\n{}",
                summary,
                format_table(&["name", "status"], component_rows)
            ))
        }
        OutputFormat::Json => {
            #[derive(Serialize)]
            struct ComponentRow {
                name: String,
                status: String,
            }

            #[derive(Serialize)]
            struct HealthJson {
                status: String,
                version: String,
                timestamp: String,
                components: Vec<ComponentRow>,
            }

            let mut components = response.components.iter().collect::<Vec<_>>();
            components.sort_by(|a, b| a.0.cmp(b.0));
            let components = components
                .into_iter()
                .map(|(name, status)| ComponentRow {
                    name: name.clone(),
                    status: status.clone(),
                })
                .collect::<Vec<_>>();

            serde_json::to_string_pretty(&HealthJson {
                status: response.status.clone(),
                version: response.version.clone(),
                timestamp: response.timestamp.clone(),
                components,
            })
            .map_err(Into::into)
        }
    }
}

fn format_pairs(
    response: &PairsResponse,
    limit: usize,
    output: OutputFormat,
) -> Result<String, SdkError> {
    let shown_pairs = response
        .pairs
        .iter()
        .take(limit)
        .cloned()
        .collect::<Vec<_>>();

    match output {
        OutputFormat::Human => {
            let mut lines = vec![format!("total pairs: {}", response.total)];
            for pair in &shown_pairs {
                lines.push(format!(
                    "{} / {} | offers: {} | canonical: {} / {}",
                    pair.base, pair.counter, pair.offer_count, pair.base_asset, pair.counter_asset
                ));
            }
            Ok(lines.join("\n"))
        }
        OutputFormat::Table => {
            let rows = shown_pairs
                .iter()
                .map(|pair| {
                    vec![
                        pair.base.clone(),
                        pair.counter.clone(),
                        pair.offer_count.to_string(),
                        pair.base_asset.clone(),
                        pair.counter_asset.clone(),
                    ]
                })
                .collect::<Vec<_>>();

            let table = format_table(
                &["base", "counter", "offers", "base_asset", "counter_asset"],
                rows,
            );
            Ok(format!(
                "total pairs: {}\nshowing: {}\n\n{}",
                response.total,
                shown_pairs.len(),
                table
            ))
        }
        OutputFormat::Json => {
            #[derive(Serialize)]
            struct PairsJson {
                total: usize,
                showing: usize,
                pairs: Vec<stellarroute_sdk::TradingPair>,
            }

            serde_json::to_string_pretty(&PairsJson {
                total: response.total,
                showing: shown_pairs.len(),
                pairs: shown_pairs,
            })
            .map_err(Into::into)
        }
    }
}

fn format_quote(response: &QuoteResponse, output: OutputFormat) -> Result<String, SdkError> {
    match output {
        OutputFormat::Human => {
            let mut lines = vec![
                format!(
                    "pair: {} / {}",
                    response.base_asset.display_name(),
                    response.quote_asset.display_name()
                ),
                format!("amount: {}", response.amount),
                format!("quote type: {}", response.quote_type),
                format!("price: {}", response.price),
                format!("total: {}", response.total),
                format!("route steps: {}", response.path.len()),
            ];

            for (index, step) in response.path.iter().enumerate() {
                lines.push(format!(
                    "{}. {} -> {} @ {} via {}",
                    index + 1,
                    step.from_asset.display_name(),
                    step.to_asset.display_name(),
                    step.price,
                    step.source
                ));
            }

            Ok(lines.join("\n"))
        }
        OutputFormat::Table => {
            let summary = format_table(
                &["field", "value"],
                vec![
                    vec![
                        "pair".to_string(),
                        format!(
                            "{} / {}",
                            response.base_asset.display_name(),
                            response.quote_asset.display_name()
                        ),
                    ],
                    vec!["amount".to_string(), response.amount.clone()],
                    vec!["quote_type".to_string(), response.quote_type.clone()],
                    vec!["price".to_string(), response.price.clone()],
                    vec!["total".to_string(), response.total.clone()],
                ],
            );

            let rows = response
                .path
                .iter()
                .enumerate()
                .map(|(idx, step)| {
                    vec![
                        (idx + 1).to_string(),
                        step.from_asset.display_name(),
                        step.to_asset.display_name(),
                        step.price.clone(),
                        step.source.clone(),
                    ]
                })
                .collect::<Vec<_>>();

            let steps = format_table(&["step", "from", "to", "price", "source"], rows);
            Ok(format!("{}\n\nroute\n{}", summary, steps))
        }
        OutputFormat::Json => serde_json::to_string_pretty(response).map_err(Into::into),
    }
}

fn format_orderbook(
    response: &OrderbookResponse,
    output: OutputFormat,
) -> Result<String, SdkError> {
    match output {
        OutputFormat::Human => {
            let mut lines = vec![
                format!(
                    "pair: {} / {}",
                    response.base_asset.display_name(),
                    response.quote_asset.display_name()
                ),
                format!("timestamp: {}", response.timestamp),
                "asks:".to_string(),
            ];

            for level in &response.asks {
                lines.push(format!(
                    "  price={} amount={} total={}",
                    level.price, level.amount, level.total
                ));
            }

            lines.push("bids:".to_string());
            for level in &response.bids {
                lines.push(format!(
                    "  price={} amount={} total={}",
                    level.price, level.amount, level.total
                ));
            }

            Ok(lines.join("\n"))
        }
        OutputFormat::Table => {
            let asks_rows = response.asks.iter().map(level_to_row).collect::<Vec<_>>();
            let bids_rows = response.bids.iter().map(level_to_row).collect::<Vec<_>>();

            Ok(format!(
                "pair: {} / {}\ntimestamp: {}\n\nasks\n{}\n\nbids\n{}",
                response.base_asset.display_name(),
                response.quote_asset.display_name(),
                response.timestamp,
                format_table(&["price", "amount", "total"], asks_rows),
                format_table(&["price", "amount", "total"], bids_rows)
            ))
        }
        OutputFormat::Json => serde_json::to_string_pretty(response).map_err(Into::into),
    }
}

fn level_to_row(level: &OrderbookLevel) -> Vec<String> {
    vec![
        level.price.clone(),
        level.amount.clone(),
        level.total.clone(),
    ]
}

fn format_table(headers: &[&str], rows: Vec<Vec<String>>) -> String {
    let mut widths = headers
        .iter()
        .map(|header| header.len())
        .collect::<Vec<_>>();

    for row in &rows {
        for (idx, cell) in row.iter().enumerate() {
            if idx >= widths.len() {
                widths.push(cell.len());
            } else {
                widths[idx] = widths[idx].max(cell.len());
            }
        }
    }

    let header_line = headers
        .iter()
        .enumerate()
        .map(|(idx, header)| format!("{header:<width$}", width = widths[idx]))
        .collect::<Vec<_>>()
        .join(" | ")
        .trim_end()
        .to_string();

    let separator = widths
        .iter()
        .map(|width| "-".repeat(*width))
        .collect::<Vec<_>>()
        .join("-+-");

    let row_lines = rows
        .iter()
        .map(|row| {
            widths
                .iter()
                .enumerate()
                .map(|(idx, width)| {
                    let cell = row.get(idx).cloned().unwrap_or_default();
                    format!("{cell:<width$}", width = *width)
                })
                .collect::<Vec<_>>()
                .join(" | ")
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>();

    if row_lines.is_empty() {
        format!("{}\n{}", header_line, separator)
    } else {
        format!("{}\n{}\n{}", header_line, separator, row_lines.join("\n"))
    }
}

fn exit_code_for_sdk_error(error: &SdkError) -> i32 {
    match error {
        SdkError::InvalidConfig(_) => EXIT_CONFIG_ERROR,

        SdkError::Http(_)
        | SdkError::Api { .. }
        | SdkError::Deserialization(_)
        | SdkError::RateLimited { .. } => EXIT_RUNTIME_ERROR,
    }
}

fn parse_asset(value: &str) -> Result<String, String> {
    if value == "native" {
        return Ok(value.to_string());
    }

    let parts: Vec<&str> = value.split(':').collect();
    if parts.is_empty() || parts.len() > 2 {
        return Err(format!(
            "invalid asset '{value}'; expected native, CODE, or CODE:ISSUER"
        ));
    }

    let code = parts[0];
    if code.is_empty() || code.len() > 12 || !code.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(format!(
            "invalid asset '{value}'; asset code must be 1-12 ASCII letters or digits"
        ));
    }

    if let Some(issuer) = parts.get(1) {
        if issuer.len() != 56 || !issuer.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(format!(
                "invalid asset '{value}'; issuer must be a 56-character Stellar account id"
            ));
        }
    }

    Ok(value.to_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use stellarroute_sdk::{ApiErrorCode, AssetInfo, PathStep, TradingPair};

    #[test]
    fn clap_help_is_well_formed() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_valid_quote_command() {
        let cli = Cli::try_parse_from([
            "stellarroute",
            "quote",
            "native",
            "USDC:GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            "--amount",
            "10.5",
            "--quote-type",
            "buy",
        ])
        .expect("command should parse");

        match cli.command {
            Commands::Quote {
                base,
                quote,
                amount,
                quote_type,
            } => {
                assert_eq!(base, "native");
                assert_eq!(
                    quote,
                    "USDC:GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"
                );
                assert_eq!(amount.as_deref(), Some("10.5"));
                assert!(matches!(quote_type, QuoteTypeArg::Buy));
            }
            _ => panic!("expected quote command"),
        }
    }

    #[test]
    fn rejects_invalid_amount() {
        let error =
            Cli::try_parse_from(["stellarroute", "quote", "native", "USDC", "--amount", "0"])
                .expect_err("amount should fail");

        assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
    }

    #[test]
    fn rejects_invalid_asset() {
        let error =
            Cli::try_parse_from(["stellarroute", "orderbook", "bad:too:many:parts", "USDC"])
                .expect_err("asset should fail");

        assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
    }

    #[test]
    fn rejects_invalid_output_format_explicitly() {
        let error = Cli::try_parse_from(["stellarroute", "--output", "xml", "health"])
            .expect_err("output format should fail");

        assert_eq!(error.kind(), clap::error::ErrorKind::InvalidValue);
        let message = error.to_string();
        assert!(message.contains("json"));
        assert!(message.contains("table"));
        assert!(message.contains("human"));
    }

    #[test]
    fn snapshot_pairs_output_human() {
        let rendered = format_pairs(&sample_pairs_response(), 2, OutputFormat::Human)
            .expect("formatting should succeed");
        insta::assert_snapshot!(rendered, @r###"
total pairs: 2
XLM / USDC | offers: 12 | canonical: native / USDC:GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF
XLM / EURC | offers: 4 | canonical: native / EURC:GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBW7
"###);
    }

    #[test]
    fn snapshot_pairs_output_table() {
        let rendered = normalize_for_snapshot(
            &format_pairs(&sample_pairs_response(), 2, OutputFormat::Table)
                .expect("formatting should succeed"),
        );
        insta::assert_snapshot!(rendered, @r###"
total pairs: 2
showing: 2

base | counter | offers | base_asset | counter_asset
<sep>
XLM  | USDC    | 12     | native     | USDC:GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF
XLM  | EURC    | 4      | native     | EURC:GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBW7
"###);
    }

    #[test]
    fn snapshot_pairs_output_json() {
        let rendered = format_pairs(&sample_pairs_response(), 1, OutputFormat::Json)
            .expect("formatting should succeed");
        insta::assert_snapshot!(rendered, @r###"
{
  "total": 2,
  "showing": 1,
  "pairs": [
    {
      "base": "XLM",
      "counter": "USDC",
      "base_asset": "native",
      "counter_asset": "USDC:GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
      "offer_count": 12,
      "last_updated": null
    }
  ]
}
"###);
    }

    #[test]
    fn snapshot_quote_output_table() {
        let rendered = normalize_for_snapshot(
            &format_quote(&sample_quote_response(), OutputFormat::Table)
                .expect("formatting should succeed"),
        );
        insta::assert_snapshot!(rendered, @r###"
field      | value
<sep>
pair       | native / USDC
amount     | 10.0000000
quote_type | sell
price      | 0.1050000
total      | 1.0500000

route
step | from   | to   | price     | source
<sep>
1    | native | USDC | 0.1050000 | sdex
"###);
    }

    #[test]
    fn exit_code_mapping_is_stable() {
        assert_eq!(
            exit_code_for_sdk_error(&SdkError::InvalidConfig("bad url".to_string())),
            EXIT_CONFIG_ERROR
        );
        assert_eq!(
            exit_code_for_sdk_error(&SdkError::Api {
                code: ApiErrorCode::InternalError,
                message: "api error".to_string(),
                status: 500,
            }),
            EXIT_RUNTIME_ERROR
        );
    }

    fn sample_pairs_response() -> PairsResponse {
        PairsResponse {
            total: 2,
            pairs: vec![
                TradingPair {
                    base: "XLM".to_string(),
                    counter: "USDC".to_string(),
                    base_asset: "native".to_string(),
                    counter_asset: "USDC:GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"
                        .to_string(),
                    offer_count: 12,
                    last_updated: None,
                },
                TradingPair {
                    base: "XLM".to_string(),
                    counter: "EURC".to_string(),
                    base_asset: "native".to_string(),
                    counter_asset: "EURC:GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBW7"
                        .to_string(),
                    offer_count: 4,
                    last_updated: None,
                },
            ],
        }
    }

    fn sample_quote_response() -> QuoteResponse {
        QuoteResponse {
            base_asset: AssetInfo {
                asset_type: "native".to_string(),
                asset_code: None,
                asset_issuer: None,
            },
            quote_asset: AssetInfo {
                asset_type: "credit_alphanum4".to_string(),
                asset_code: Some("USDC".to_string()),
                asset_issuer: None,
            },
            amount: "10.0000000".to_string(),
            price: "0.1050000".to_string(),
            total: "1.0500000".to_string(),
            quote_type: "sell".to_string(),
            path: vec![PathStep {
                from_asset: AssetInfo {
                    asset_type: "native".to_string(),
                    asset_code: None,
                    asset_issuer: None,
                },
                to_asset: AssetInfo {
                    asset_type: "credit_alphanum4".to_string(),
                    asset_code: Some("USDC".to_string()),
                    asset_issuer: None,
                },
                price: "0.1050000".to_string(),
                source: "sdex".to_string(),
            }],
            timestamp: 1_742_908_400,
        }
    }

    fn normalize_for_snapshot(value: &str) -> String {
        value
            .lines()
            .map(|line| {
                let line = line.trim_end();
                if !line.is_empty() && line.chars().all(|ch| ch == '-' || ch == '+') {
                    "<sep>".to_string()
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
