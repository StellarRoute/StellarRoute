# StellarRoute Rust SDK

Full Rust integration guide for `stellarroute-sdk`.

## Dependency

Add the SDK to your Rust project.

### Local workspace

```toml
[dependencies]
stellarroute-sdk = { path = "../crates/sdk-rust" }
```

### Published crate (future)

```toml
[dependencies]
stellarroute-sdk = "0.1"
```

## Importing the SDK

```rust
use stellarroute_sdk::{
    ClientBuilder,
    QuoteRequest,
    QuoteType,
    StellarRouteClient,
    SdkError,
};
```

## Client initialization

Use `ClientBuilder` for custom configuration, or `StellarRouteClient::new` for defaults.

```rust
use std::time::Duration;

let client = ClientBuilder::new("http://127.0.0.1:3000")
    .timeout(Duration::from_secs(10))
    .user_agent("my-backend/1.0")
    .max_retries(3)
    .base_backoff(Duration::from_millis(500))
    .build()?;
```

Or the convenience constructor:

```rust
let client = StellarRouteClient::new("http://127.0.0.1:3000")?;
```

## Automatic retries

By default (`max_retries = 0`), the client returns errors immediately. Set
`max_retries` on the builder to enable automatic retries with exponential
backoff on 429 (rate-limited) and 5xx (server error) responses.

```rust
let client = ClientBuilder::new("http://localhost:3000")
    .max_retries(3)
    .base_backoff(Duration::from_millis(500)) // default
    .build()?;
```

Retry behavior:
- **429 responses**: Honors the `Retry-After` header when present; otherwise
  uses exponential backoff (`base_backoff × 2^attempt`).
- **5xx responses**: Uses exponential backoff, capped at 30 seconds.
- **Other errors** (4xx): Not retried — returned immediately.
- When all retries are exhausted, the last error is returned.

## Async usage

The Rust SDK is async-first and works with Tokio.

```rust
use stellarroute_sdk::{QuoteRequest, QuoteType, StellarRouteClient};

#[tokio::main]
async fn main() -> stellarroute_sdk::Result<()> {
    let client = StellarRouteClient::new("http://127.0.0.1:3000")?;

    let pairs = client.pairs().await?;
    println!("{} trading pairs available", pairs.pairs.len());

    let orderbook = client.orderbook("native", "USDC").await?;
    println!("Loaded orderbook: {} bids, {} asks", orderbook.bids.len(), orderbook.asks.len());

    let quote = client
        .quote(QuoteRequest {
            base: "native",
            quote: "USDC",
            amount: Some("100"),
            quote_type: QuoteType::Sell,
        })
        .await?;
    println!("Best quote: price={} total={} amount={}", quote.price, quote.total, quote.amount);

    Ok(())
}
```

## Common API calls

### Get trading pairs

```rust
let pairs = client.pairs().await?;
for pair in &pairs.pairs {
    println!("{} / {}", pair.base, pair.quote);
}
```

### Get an orderbook

```rust
let orderbook = client.orderbook("native", "USDC").await?;
println!("Bids: {} levels", orderbook.bids.len());
println!("Asks: {} levels", orderbook.asks.len());
```

### Get a quote

```rust
use stellarroute_sdk::{QuoteRequest, QuoteType};

let quote = client
    .quote(QuoteRequest {
        base: "native",
        quote: "USDC",
        amount: Some("50"),
        quote_type: QuoteType::Buy,
    })
    .await?;

println!("quote price: {}", quote.price);
```

## Error handling patterns

The SDK returns `stellarroute_sdk::Result<T>` and `stellarroute_sdk::SdkError`.

### Matching errors

```rust
match client.quote(request).await {
    Ok(quote) => println!("price: {}", quote.price),
    Err(err) => match err {
        SdkError::InvalidConfig(msg) => eprintln!("Configuration error: {}", msg),
        SdkError::Http(msg) => eprintln!("Network error: {}", msg),
        SdkError::RateLimited { info } => {
            eprintln!("Rate limit exceeded: {:?}", info);
        }
        SdkError::Api { code, message, status } => {
            eprintln!("API error {} ({}): {}", code, status, message);
        }
    },
}
```

### Common SDK error cases

- `SdkError::InvalidConfig` — malformed `api_url` or invalid `User-Agent` header.
- `SdkError::Http` — failed HTTP request or response parsing.
- `SdkError::RateLimited` — HTTP 429 with rate-limit headers.
- `SdkError::Api` — API returned a JSON error payload.

### API error mapping

`SdkError::Api` exposes `ApiErrorCode` and HTTP status details.
Use it to distinguish:

- `ApiErrorCode::NotFound` for missing pairs or routes.
- `ApiErrorCode::ValidationError` for invalid request parameters.
- `ApiErrorCode::InternalError` for unexpected server failures.

For full API error semantics, see [API error taxonomy](../api/error_taxonomy.md).

## CLI reference

This crate also ships a CLI binary, `stellarroute`.
Run it from the workspace root:

```bash
cargo run -p stellarroute-sdk --bin stellarroute -- <command>
```

### Global flags

- `--api-url <URL>` — API base URL (default: `http://127.0.0.1:3000`)
- `--output <human|table|json>` — output format (default: `human`)
- `STELLARROUTE_API_URL` — alternative environment variable for API URL

### Commands

- `health` — check API health
- `pairs [--limit N]` — list active trading pairs
- `quote <base> <quote> [--amount <amount>] [--quote-type <sell|buy>]` — get a price quote
- `orderbook <base> <quote> [--levels <N>]` — show the orderbook snapshot

### Examples

```bash
cargo run -p stellarroute-sdk --bin stellarroute -- quote native USDC --amount 100 --quote-type sell --output json
cargo run -p stellarroute-sdk --bin stellarroute -- orderbook native USDC --levels 20 --output table
cargo run -p stellarroute-sdk --bin stellarroute -- pairs --limit 30
```

### Quote flags

| Flag | Default | Description |
|---|---|---|
| `--amount <decimal>` | _(omit for indicative price)_ | Positive decimal amount of the base asset to trade, e.g. `100` or `0.5`. When omitted the server uses `1` unit. |
| `--quote-type <sell\|buy>` | `sell` | `sell` — trade away the base asset; `buy` — acquire the base asset. Maps to `quote_type` on `GET /api/v1/quote`. |

Slippage tolerance (`slippage_bps`) is enforced server-side and is not a CLI flag.

### Copy-paste example against localhost

```bash
# Sell 100 XLM for USDC, human output (default)
cargo run -p stellarroute-sdk --bin stellarroute -- \
  --api-url http://127.0.0.1:3000 \
  quote native USDC --amount 100 --quote-type sell

# Buy 50 USDC worth of XLM, JSON output
cargo run -p stellarroute-sdk --bin stellarroute -- \
  --api-url http://127.0.0.1:3000 \
  --output json \
  quote native USDC --amount 50 --quote-type buy

# Indicative price with no amount (server defaults to 1 unit)
cargo run -p stellarroute-sdk --bin stellarroute -- \
  --api-url http://127.0.0.1:3000 \
  quote native USDC
```

### Output formats

- `human` — friendly terminal output
- `table` — text table output
- `json` — machine-readable JSON

### Exit codes

- `0` — success
- `2` — CLI usage/validation error
- `3` — invalid client configuration
- `4` — runtime/API error

## Related documentation

- [Rust SDK CLI reference](../../crates/sdk-rust/README.md)
- [API error taxonomy](../api/error_taxonomy.md)
- [API integrator error guide](../api/integrator-error-guide.md)
- [TypeScript SDK quickstart](../../sdk-js/README.md)
