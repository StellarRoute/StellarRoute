# Design Document — `cli-price-history`

## Overview

The `price-history` feature adds a new `price-history <BASE> <QUOTE>` subcommand to
the `stellarroute` CLI binary and a matching `price_history` method to the
`StellarRouteClient` SDK struct. The feature is purely additive: no existing
code paths, public types, or test suites are modified.

### Key design goals

- Mirror the conventions of every existing CLI subcommand (`health`, `pairs`, `quote`,
  `orderbook`): `clap`-based argument parsing, `parse_asset` validation, the three
  output formats (`json` / `table` / `human`), `exit_code_for_sdk_error` for exit codes,
  and the shared `format_table` helper.
- Define `PriceHistoryResponse` and `PriceHistoryPoint` as owned SDK types (in
  `crates/sdk-rust/src/types.rs`) so the CLI and any third-party consumer have a
  single import surface.
- Never touch `crates/api/src/routes/price_history.rs` or any other frozen route file.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  stellarroute CLI  (crates/sdk-rust/src/bin/stellarroute.rs)│
│                                                             │
│  Commands::PriceHistory { base, quote }                     │
│         │                                                   │
│         ▼                                                   │
│  render_price_history(client, base, quote, output)          │
│         │                                                   │
│         ▼                                                   │
│  StellarRouteClient::price_history(base, quote)             │
│         │                                                   │
│         ▼                                                   │
│  GET /api/v1/price-history/{base}/{quote}                   │
│         │                                                   │
│         ▼                                                   │
│  PriceHistoryResponse  ──►  format_price_history(…, output) │
│                              ├─ json  → serde_json::to_string_pretty
│                              ├─ table → format_table helper │
│                              └─ human → summary + point list│
└─────────────────────────────────────────────────────────────┘
```

The API handler in `crates/api/src/routes/price_history.rs` already exists and is
untouched. The new code lives entirely in the SDK crate.

---

## Components and Interfaces

### 1. SDK Types — `crates/sdk-rust/src/types.rs`

Two new structs, appended after the existing type definitions:

```rust
/// A single historical price sample returned by the price-history endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriceHistoryPoint {
    /// Unix timestamp in milliseconds for the aggregated price bucket.
    pub timestamp: i64,
    /// Average mid-market price for the bucket, encoded as a decimal string.
    pub price: String,
}

/// Response from `GET /api/v1/price-history/{base}/{quote}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriceHistoryResponse {
    pub base_asset: AssetInfo,
    pub quote_asset: AssetInfo,
    /// Time window covered by the series (e.g. "24h").
    pub window: String,
    /// Data source description returned by the API.
    pub source: String,
    /// Unix timestamp in milliseconds when the response was generated.
    pub generated_at: i64,
    /// Ordered list of price samples, ascending by timestamp.
    pub points: Vec<PriceHistoryPoint>,
}
```

Field names match the API wire format exactly (verified against
`crates/api/src/models/response.rs`). No renaming or `#[serde(rename)]` needed.

### 2. SDK Client method — `crates/sdk-rust/src/client.rs`

A new `pub async fn price_history` added to the `StellarRouteClient` impl block,
following the same pattern as `orderbook`:

```rust
/// `GET /api/v1/price-history/{base}/{quote}` — fetch 24-hour price series.
///
/// Returns [`SdkError::Api`] with [`ApiErrorCode::ValidationError`] for HTTP 400,
/// [`ApiErrorCode::NoRoute`] for HTTP 404 (pair not found), and
/// [`SdkError::RateLimited`] after exhausted retries on HTTP 429.
pub async fn price_history(
    &self,
    base: &str,
    quote: &str,
) -> Result<PriceHistoryResponse> {
    self.get(&format!("api/v1/price-history/{base}/{quote}")).await
}
```

The existing `get` helper handles URL construction, retry logic, rate-limit backoff,
and error mapping — no new infrastructure is needed.

### 3. Lib re-exports — `crates/sdk-rust/src/lib.rs`

`PriceHistoryResponse` and `PriceHistoryPoint` are added to the flat re-export list:

```rust
pub use types::{
    // ... existing exports ...
    PriceHistoryPoint,
    PriceHistoryResponse,
};
```

### 4. CLI subcommand — `crates/sdk-rust/src/bin/stellarroute.rs`

#### 4a. Import addition

```rust
use stellarroute_sdk::{
    // existing …
    PriceHistoryPoint, PriceHistoryResponse,
};
```

#### 4b. New `Commands` variant

```rust
#[command(about = "Fetch 24-hour price history for a trading pair")]
PriceHistory {
    #[arg(value_parser = parse_asset, help = "Base asset: native, CODE, or CODE:ISSUER")]
    base: String,
    #[arg(value_parser = parse_asset, help = "Quote asset: native, CODE, or CODE:ISSUER")]
    quote: String,
},
```

The existing `parse_asset` validator is reused verbatim; no new validator is needed.

#### 4c. `run` dispatch arm

```rust
Commands::PriceHistory { base, quote } => {
    render_price_history(&client, &base, &quote, cli.output)
        .await
        .map_err(|error| (exit_code_for_sdk_error(&error), error.to_string()))
}
```

#### 4d. `render_price_history` and `format_price_history`

```rust
async fn render_price_history(
    client: &StellarRouteClient,
    base: &str,
    quote: &str,
    output: OutputFormat,
) -> Result<String, SdkError> {
    let response = client.price_history(base, quote).await?;
    format_price_history(&response, output)
}

fn format_price_history(
    response: &PriceHistoryResponse,
    output: OutputFormat,
) -> Result<String, SdkError> {
    match output {
        OutputFormat::Json => {
            serde_json::to_string_pretty(response).map_err(Into::into)
        }
        OutputFormat::Table => {
            let header = format!(
                "pair: {} / {}\nwindow: {}\n",
                response.base_asset.display_name(),
                response.quote_asset.display_name(),
                response.window
            );
            let rows = response
                .points
                .iter()
                .map(|p| vec![p.timestamp.to_string(), p.price.clone()])
                .collect::<Vec<_>>();
            Ok(format!(
                "{}\n{}",
                header,
                format_table(&["timestamp", "price"], rows)
            ))
        }
        OutputFormat::Human => {
            let mut lines = vec![
                format!(
                    "pair: {} / {}",
                    response.base_asset.display_name(),
                    response.quote_asset.display_name()
                ),
                format!("window: {}", response.window),
                format!("source: {}", response.source),
            ];
            if response.points.is_empty() {
                lines.push("no data".to_string());
            } else {
                for point in &response.points {
                    lines.push(format!("{}  {}", point.timestamp, point.price));
                }
            }
            Ok(lines.join("\n"))
        }
    }
}
```

---

## Data Models

### `PriceHistoryPoint`

| Field       | Type     | Wire name   | Notes                         |
|-------------|----------|-------------|-------------------------------|
| `timestamp` | `i64`    | `timestamp` | ms since Unix epoch           |
| `price`     | `String` | `price`     | Decimal string, e.g. "0.1050" |

### `PriceHistoryResponse`

| Field        | Type                    | Wire name      | Notes                              |
|--------------|-------------------------|----------------|------------------------------------|
| `base_asset` | `AssetInfo`             | `base_asset`   | Reuses existing SDK type           |
| `quote_asset`| `AssetInfo`             | `quote_asset`  | Reuses existing SDK type           |
| `window`     | `String`                | `window`       | Currently always `"24h"`           |
| `source`     | `String`                | `source`       | e.g. `"orderbook_snapshots…"`      |
| `generated_at` | `i64`               | `generated_at` | ms since Unix epoch                |
| `points`     | `Vec<PriceHistoryPoint>`| `points`       | Ordered ascending by timestamp     |

All field names are kept identical to the API wire format and the existing
`crates/api/src/models/response.rs` definitions.

### No schema migration

This feature adds no new database tables and makes no changes to the existing
OpenAPI schema. The SDK types are separate from the API model types by design
(the two crates are independent compilation units).

---

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid
executions of a system — essentially, a formal statement about what the software is
supposed to do. Properties serve as the bridge between human-readable specifications
and machine-verifiable correctness guarantees.*

Property-based testing applies here because:
- The formatting functions are pure: `format_price_history(response, output) → String`.
- Input variation (number of points, asset codes, price strings, timestamp values)
  meaningfully exercises edge cases (empty points, long asset codes, special price
  strings, non-monotonic timestamps).
- 100+ iterations are cheap (in-memory, no I/O).
- The URL construction logic similarly has a large valid input space.

**Property reflection** (redundancy elimination):

- Req 3.2 (field names preserved) is structurally implied by 3.1 (JSON round-trip via
  `serde_json::to_string_pretty`); eliminated.
- Req 3.3 (empty points → `"points": []`) is a special case of Property 1 with
  empty points; covered by the generator.
- Req 4.3 (timestamp as ms integer) is covered within Property 3 (table row content
  check); eliminated as standalone.
- Req 4.4 (empty points → empty table body) is a special case of Properties 2 and 3
  combined; covered by the generators.
- Req 5.3 (empty points → "no data") is a special case of Property 4 with empty
  points; covered by generator.
- Req 6.1 (all SdkError::Api → exit code 4) subsumes requirements 6.2 and 6.4 for
  the price-history path; one property covers all Api variants.
- Properties 3 and 4 (table column content and human point ordering) are distinct and
  not redundant: table checks column presence while human checks ordering.

---

### Property 1: JSON output is a faithful serialization of the response

*For any* `PriceHistoryResponse`, the `--output json` rendering SHALL produce a
string that, when parsed back with `serde_json`, yields an equal value with all
original field names preserved.

**Validates: Requirements 3.1, 3.2, 3.3**

---

### Property 2: Table output header always contains pair and window

*For any* `PriceHistoryResponse`, the `--output table` rendering SHALL contain a
line starting with `"pair: "` that includes both `base_asset.display_name()` and
`quote_asset.display_name()`, and a line containing `"window: "` with the response's
`window` value.

**Validates: Requirements 4.1, 4.4**

---

### Property 3: Table output rows mirror every point exactly

*For any* `PriceHistoryResponse` with N points, the `--output table` rendering SHALL
produce exactly N data rows (after the separator line), each row containing both the
millisecond-epoch integer timestamp and the price string from the corresponding
`PriceHistoryPoint`.

**Validates: Requirements 4.2, 4.3**

---

### Property 4: Human output points are ascending by timestamp

*For any* `PriceHistoryResponse` with at least one point, the `--output human`
rendering SHALL list each point's `<timestamp>  <price>` on its own line, and the
sequence of timestamps read from those lines SHALL be non-decreasing.

**Validates: Requirements 5.1, 5.2**

---

### Property 5: URL construction is exact for any valid asset pair

*For any* pair of valid asset identifier strings `(base, quote)`, the HTTP request
issued by `StellarRouteClient::price_history` SHALL target the path
`api/v1/price-history/{base}/{quote}` using the GET method, with `base` and `quote`
substituted verbatim.

**Validates: Requirements 1.2, 2.4**

---

### Property 6: All `SdkError::Api` variants produce exit code 4

*For any* `SdkError::Api { code, message, status }` value — regardless of the error
code, message content, or HTTP status — `exit_code_for_sdk_error` SHALL return
`EXIT_RUNTIME_ERROR` (4).

**Validates: Requirements 6.1, 6.2, 6.4**

---

## Error Handling

| Scenario                             | SDK Error                               | CLI Exit Code |
|--------------------------------------|-----------------------------------------|---------------|
| HTTP 400 from API                    | `SdkError::Api { code: ValidationError }` | 4           |
| HTTP 404 from API                    | `SdkError::Api { code: NoRoute }`       | 4             |
| HTTP 429 exhausted retries           | `SdkError::RateLimited`                 | 4             |
| HTTP 5xx                             | `SdkError::Api { code: InternalError }` | 4             |
| Network error / timeout              | `SdkError::Http`                        | 4             |
| Malformed JSON response              | `SdkError::Deserialization`             | 4             |
| Invalid `--api-url` (config error)   | `SdkError::InvalidConfig`               | 3             |
| Invalid asset argument (parse_asset) | `clap` error                            | 2             |

All error mappings flow through the existing `exit_code_for_sdk_error` function
without modification.

Error messages are printed to stderr via the existing `run` → `Err((code, message))`
path. The SDK's `SdkError::RateLimited` Display impl already includes the reset
timestamp context required by Requirement 6.2.

---

## Testing Strategy

### Unit tests (in `crates/sdk-rust/src/bin/stellarroute.rs` `#[cfg(test)]`)

These test specific examples, snapshots, and edge cases:

- `clap_help_is_well_formed` (extended) — existing test, confirms `PriceHistory`
  variant is declared cleanly.
- `snapshot_price_history_human` — snapshot of human output for a sample response.
- `snapshot_price_history_table` — snapshot of table output for a sample response.
- `snapshot_price_history_json` — snapshot of JSON output for a sample response.
- `price_history_human_empty_points` — human output with empty points shows "no data".
- `price_history_table_empty_points` — table output with empty points shows header +
  empty table body.
- `price_history_rejects_invalid_asset` — `Cli::try_parse_from` with a bad asset
  returns `ValueValidation` error.
- `price_history_exit_code_api_error` — `exit_code_for_sdk_error(SdkError::Api{…})` → 4
  (shared with existing mapping test; may be same test body extended).

### Property-based tests (in `crates/sdk-rust/tests/client_integration.rs`)

The project uses `proptest = "1.5"` (already in workspace dependencies). Tests are
added to the integration test file with `#[cfg(test)]` proptest macros.

Each property test runs a minimum of **100 iterations** and is tagged with a comment
referencing the design property:

```
// Feature: cli-price-history, Property N: <property text>
```

**Property test implementations:**

- **Property 1** — Generate arbitrary `PriceHistoryResponse` (arbitrary asset infos,
  window strings, source strings, 0–24 points with random timestamps and price strings).
  Assert `serde_json::from_str::<PriceHistoryResponse>(&json_out).unwrap() == original`.

- **Property 2** — Same generator. Assert table output contains `"pair: "` + both
  display names and `"window: "` + window value.

- **Property 3** — Generator with N ≥ 0 points. Assert number of non-header,
  non-separator lines equals N, and each line contains the corresponding timestamp as
  a decimal integer and the price string.

- **Property 4** — Generator with N ≥ 1 points (shuffled timestamps). Assert the
  timestamps extracted from human output lines are non-decreasing.

- **Property 5** — Covered by mock-based integration tests: for representative
  valid asset pairs (native, CODE, CODE:ISSUER), verify the mock server receives GET
  at the correct path. Full arbitrary-input URL property is validated by unit-testing
  the format string directly.

- **Property 6** — Proptest over `(ApiErrorCode, String, u16)`. Assert
  `exit_code_for_sdk_error(SdkError::Api{…})` always returns 4.

### Integration tests (in `crates/sdk-rust/tests/client_integration.rs`)

Mock-server tests following the wiremock pattern already established in that file:

- `price_history_returns_typed_response` — mock 200, assert correct deserialization.
- `price_history_400_maps_to_validation_error` — mock 400 with `validation_error` body.
- `price_history_404_maps_to_no_route_error` — mock 404.
- `price_history_429_exhausted_maps_to_rate_limited` — mock repeated 429.
- `price_history_empty_points_deserializes` — mock 200 with empty `points` array.
- `price_history_ignored_live` — `#[ignore]` live API smoke test.

### CI gate

The feature must not introduce new failures in any of:

```
cargo test -p stellarroute-api --test swap_integration \
           --test swap_submit_integration \
           --test openapi_swap_contract
cargo clippy --workspace --all-features --exclude stellarroute-contracts -- -D warnings
cargo test --workspace --lib --exclude stellarroute-contracts --exclude stellarroute-api
```

Because the feature is purely additive and the SDK crate has no `cfg(test)` or
`#[test]` annotations in `lib.rs` / `client.rs` that touch the API crate, these gates
are unaffected by design.
