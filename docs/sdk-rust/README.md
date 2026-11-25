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

### Execute a swap (prepare → sign → submit)

Backend integrators get the same two-step execution path as the JS SDK. The SDK never
touches signing keys — `prepare_swap` returns an unsigned XDR envelope, you sign it with
your own keypair, and `submit_swap` broadcasts it.

```rust
use stellarroute_sdk::{Client, RoutesRequest, SwapPrepareRequest, SwapSubmitRequest};

# async fn run() -> stellarroute_sdk::Result<()> {
let client = Client::new("https://api.stellarroute.io")?;

// 1. Pick a route.
let routes = client
    .routes(RoutesRequest {
        base: "native",
        quote: "USDC",
        amount: 1_000_000,
        slippage_bps: Some(50),
        quote_type: None,
    })
    .await?;
let best = routes.routes.first().expect("at least one route");

// 2. Prepare — server validates the path and returns an unsigned envelope.
let prepared = client
    .prepare_swap(
        SwapPrepareRequest::from_route(best, "100", "GABC...")
            .slippage_bps(50)
            .min_output("98"),
    )
    .await?;

// 3. Sign `prepared.xdr_envelope` with your own keypair, then submit.
let signed_xdr = my_signer(&prepared.xdr_envelope);
let receipt = client.submit_swap(SwapSubmitRequest::new(signed_xdr)).await?;

if receipt.is_success() {
    println!("swap confirmed: {}", receipt.tx_hash);
}
# Ok(())
# }
# fn my_signer(xdr: &str) -> String { xdr.to_string() }
```

Notes:

- `SwapPrepareResponse::valid_until_ledger` bounds how long the envelope stays valid —
  re-prepare rather than submitting a stale envelope.
- `submit_swap` may return `status == "pending"`; poll or watch the tx hash on Horizon
  before treating the swap as final.
- `prepare_swap` surfaces `ApiErrorCode::NoRoute` when the path went stale between
  `routes` and `prepare`, and `ApiErrorCode::ValidationError` for a malformed request.

### Batch quotes

```rust
use stellarroute_sdk::{BatchQuoteRequest, QuoteRequestItem};

let resp = client
    .batch_quote(BatchQuoteRequest {
        quotes: vec![QuoteRequestItem {
            base: "native".to_string(),
            quote: "USDC".to_string(),
            amount: None,
            slippage_bps: None,
            quote_type: None,
        }],
    })
    .await?;

for item in &resp.results {
    match &item.quote {
        Some(quote) => println!("item {}: price {}", item.index, quote.price),
        None => println!("item {} failed: {:?}", item.index, item.error),
    }
}
```

`batch_quote` only fails the whole call for batch-level problems (empty batch, too
many items, malformed body) — those map to `ApiErrorCode::ValidationError`. Per-item
failures (e.g. one pair has no route) are reported in `BatchQuoteResponse::results`
alongside successful items, so check each result's `status`/`quote`/`error` rather
than relying on the call failing.

### CCTP bridge (preview, disabled by default)

Mirrors the `/api/v2/bridge/cctp/*` contract documented in
[`docs/api/cctp-v2-contract.md`](../api/cctp-v2-contract.md). Execution is
fail-closed behind `CCTP_ENABLED` on the server — these methods exist so the
SDK's types match the wire contract, not to change that default.

```rust
use stellarroute_sdk::{CctpDirection, CctpFinality, CctpQuoteRequest, CCTP_TESTNET_CORRIDOR_ID};

// `source_asset`/`destination_asset` are the documented testnet corridor's
// `ChainAsset` values — see `docs/api/cctp-v2-contract.md`.
let quote = client
    .cctp_quote(
        CctpQuoteRequest {
            corridor_id: CCTP_TESTNET_CORRIDOR_ID.to_string(),
            provider: "circle-cctp".to_string(),
            direction: CctpDirection::StellarToEvm,
            source_chain_id: "stellar:testnet".to_string(),
            destination_chain_id: "eip155:11155111".to_string(),
            source_asset: stellar_testnet_usdc(),
            destination_asset: sepolia_usdc(),
            amount: "100".to_string(),
            recipient: "0x...".to_string(),
            sender: None,
            mint_submitter: None,
            finality: CctpFinality::Standard,
        },
        Some("idem-key-1"), // optional Idempotency-Key for safe retries
    )
    .await?;

// Subsequent calls need the one-time access token from the quote.
let status = client
    .cctp_get_transfer(&quote.transfer_id, &quote.access_token)
    .await?;
```

- The second `cctp_quote` argument is an optional `Idempotency-Key`; a byte-identical
  replay with the same key returns the cached response.
- `cctp_get_transfer`, `cctp_prepare_burn`, `cctp_submit_burn`, `cctp_prepare_mint`,
  `cctp_submit_mint`, and `cctp_reattest` all require the `access_token` returned by
  `cctp_quote`, sent as the `x-cctp-transfer-access` header.
- Until the server enables CCTP, every call returns `SdkError::Api` with HTTP 503
  (`cctp_not_enabled`).

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
- `5` — API reachable but reporting degraded health (`health`)

## Related documentation

- [Rust SDK CLI reference](../../crates/sdk-rust/README.md)
- [StellarRoute CLI cookbook](cli-cookbook.md)
- [API error taxonomy](../api/error_taxonomy.md)
- [API integrator error guide](../api/integrator-error-guide.md)
- [TypeScript SDK quickstart](../../sdk-js/README.md)
