# Implementation Tasks — `cli-price-history`

## Overview

Purely additive implementation: new SDK types → new SDK method → lib re-exports →
CLI subcommand + formatting → unit/snapshot tests → property-based + integration tests.
No existing files are modified except to extend `types.rs`, `client.rs`, `lib.rs`, and
`stellarroute.rs`.

---

- [x] 1. Add SDK types `PriceHistoryPoint` and `PriceHistoryResponse` to `crates/sdk-rust/src/types.rs`
  - Append `PriceHistoryPoint` struct with `timestamp: i64` and `price: String` fields, both pub, deriving `Debug, Clone, PartialEq, Serialize, Deserialize`
  - Append `PriceHistoryResponse` struct with fields `base_asset: AssetInfo`, `quote_asset: AssetInfo`, `window: String`, `source: String`, `generated_at: i64`, `points: Vec<PriceHistoryPoint>`, all pub, deriving `Debug, Clone, PartialEq, Serialize, Deserialize`
  - Field names must match the API wire format exactly (verified against `crates/api/src/models/response.rs`)
  - Run `cargo build -p stellarroute-sdk` and confirm zero errors

- [x] 2. Add `price_history` method to `StellarRouteClient` in `crates/sdk-rust/src/client.rs`
  - Add `use crate::types::{PriceHistoryResponse};` to the existing `use crate::types::{…}` import block
  - Add `pub async fn price_history(&self, base: &str, quote: &str) -> Result<PriceHistoryResponse>` method to `StellarRouteClient` impl block, calling `self.get(&format!("api/v1/price-history/{base}/{quote}")).await`
  - Add doc comment mirroring the pattern of `orderbook`, noting HTTP 400 → `ValidationError`, HTTP 404 → `NoRoute`, HTTP 429 → `RateLimited`
  - Run `cargo build -p stellarroute-sdk` and confirm zero errors

- [x] 3. Re-export new types from `crates/sdk-rust/src/lib.rs`
  - Add `PriceHistoryPoint` and `PriceHistoryResponse` to the existing `pub use types::{…}` re-export list
  - Run `cargo build -p stellarroute-sdk` and confirm zero errors

- [x] 4. Add `PriceHistory` subcommand variant and formatting to `crates/sdk-rust/src/bin/stellarroute.rs`
  - [x] 4.1 Add `PriceHistoryResponse` and `PriceHistoryPoint` to the existing `use stellarroute_sdk::{…}` import
  - [x] 4.2 Add `PriceHistory { base: String, quote: String }` variant to the `Commands` enum with `#[command(about = "Fetch 24-hour price history for a trading pair")]` and `value_parser = parse_asset` on both args (matching the `Orderbook` pattern)
  - [x] 4.3 Add `Commands::PriceHistory { base, quote }` arm to the `match cli.command` block in `run`, delegating to `render_price_history`
  - [x] 4.4 Implement `async fn render_price_history(client: &StellarRouteClient, base: &str, quote: &str, output: OutputFormat) -> Result<String, SdkError>` that calls `client.price_history` then `format_price_history`
  - [x] 4.5 Implement `fn format_price_history(response: &PriceHistoryResponse, output: OutputFormat) -> Result<String, SdkError>` with three match arms:
    - `Json`: `serde_json::to_string_pretty(response).map_err(Into::into)`
    - `Table`: header block `"pair: <base_display> / <quote_display>\nwindow: <window>\n"` followed by blank line then `format_table(&["timestamp", "price"], rows)` where rows come from `response.points`
    - `Human`: summary lines `"pair: …"`, `"window: …"`, `"source: …"`, then either `"no data"` (empty points) or one `"<timestamp>  <price>"` line per point in ascending timestamp order
  - Run `cargo build -p stellarroute-sdk` and confirm zero errors
  - Run `cargo clippy --workspace --all-features --exclude stellarroute-contracts -- -D warnings` and confirm zero warnings

- [x] 5. Add unit and snapshot tests in `crates/sdk-rust/src/bin/stellarroute.rs` `#[cfg(test)]` block
  - Add `fn sample_price_history_response() -> PriceHistoryResponse` helper with at least 3 points and representative field values (matching the pattern of `sample_pairs_response`)
  - Add `snapshot_price_history_human` test: call `format_price_history(&sample_price_history_response(), OutputFormat::Human)` and `insta::assert_snapshot!`
  - Add `snapshot_price_history_table` test: call with `OutputFormat::Table`, normalize with `normalize_for_snapshot`, and `insta::assert_snapshot!`
  - Add `snapshot_price_history_json` test: call with `OutputFormat::Json` and `insta::assert_snapshot!`
  - Add `price_history_human_empty_points` test: construct a response with `points: vec![]`, assert human output contains `"no data"`
  - Add `price_history_table_empty_points` test: same response, assert table output contains the header block and the separator line but no data rows
  - Add `price_history_rejects_invalid_asset` test: `Cli::try_parse_from(["stellarroute", "price-history", "bad:too:many:parts", "USDC"])` asserts `ErrorKind::ValueValidation`
  - Extend `clap_help_is_well_formed` test (or add a new `price_history_listed_in_help` test) to confirm `"price-history"` appears in the rendered help string
  - Run `cargo test --workspace --lib --exclude stellarroute-contracts --exclude stellarroute-api` and confirm all tests pass

- [x] 6. Add integration tests in `crates/sdk-rust/tests/client_integration.rs`
  - [x] 6.1 Add `price_history_returns_typed_response` test: mount a wiremock GET `/api/v1/price-history/native/USDC` returning a full `PriceHistoryResponse` JSON body; call `client.price_history("native", "USDC")`; assert `base_asset.is_native()`, `points.len()`, and a sampled `price` value
  - [x] 6.2 Add `price_history_400_maps_to_validation_error` test: mock returns 400 with `{"error":"validation_error","message":"…"}`; assert `SdkError::Api { code: ApiErrorCode::ValidationError, status: 400, .. }`
  - [x] 6.3 Add `price_history_404_maps_to_no_route_error` test: mock returns 404 with `{"error":"no_route","message":"…"}`; assert `SdkError::Api { code: ApiErrorCode::NoRoute, .. }`
  - [x] 6.4 Add `price_history_429_exhausted_maps_to_rate_limited` test: mock always returns 429 with `Retry-After: 1`; use `client_with_retries(&server, 1)`; assert `SdkError::RateLimited`
  - [x] 6.5 Add `price_history_empty_points_deserializes` test: mock returns 200 with `"points": []`; assert `resp.points.is_empty()`
  - [x] 6.6 Add `#[ignore = "requires live StellarRoute API"] price_history_live_smoke` test following the same `#[ignore]` pattern as the existing live tests
  - Run `cargo test -p stellarroute-sdk` and confirm all new tests pass

- [x] 7. Add property-based tests using `proptest`
  - Add `proptest.workspace = true` to the `[dev-dependencies]` section of `crates/sdk-rust/Cargo.toml` (proptest is already in `[workspace.dependencies]`)
  - [x] 7.1 Add proptest strategies for `AssetInfo` (arbitrary asset_type, optional code/issuer), `PriceHistoryPoint` (arbitrary i64 timestamp, arbitrary price string), and `PriceHistoryResponse` (arbitrary fields, 0–24 points)
  - [x] 7.2 Implement Property 1 (`json_output_round_trips`): `proptest! { #[test] fn json_output_round_trips(response in arb_price_history_response()) { ... } }` — tag comment `// Feature: cli-price-history, Property 1: JSON output is a faithful serialization`; assert `serde_json::from_str::<PriceHistoryResponse>(&json_out).unwrap()` equals the original
  - [x] 7.3 Implement Property 2 (`table_output_header_contains_pair_and_window`): tag `// Feature: cli-price-history, Property 2`; assert rendered table string contains `"pair: "` + `base_asset.display_name()` + ` / ` + `quote_asset.display_name()` and `"window: "` + window value
  - [x] 7.4 Implement Property 3 (`table_output_rows_mirror_every_point`): tag `// Feature: cli-price-history, Property 3`; count non-header non-separator rows, assert equals `response.points.len()`; assert each row contains its corresponding timestamp and price
  - [x] 7.5 Implement Property 4 (`human_output_points_are_ascending`): use generator with at least 1 point; tag `// Feature: cli-price-history, Property 4`; extract timestamp values from human output lines (lines that start with a digit after the summary header); assert they are non-decreasing
  - [x] 7.6 Implement Property 6 (`all_api_errors_give_exit_code_4`): proptest over `(u8, String, u16)` mapped to `SdkError::Api { code: ApiErrorCode::Other(…), message, status }`; tag `// Feature: cli-price-history, Property 6`; assert `exit_code_for_sdk_error` returns `EXIT_RUNTIME_ERROR` (4) for every input
  - Configure proptest to run minimum 100 iterations per test (default proptest config is 256; no override needed)
  - Run `cargo test --workspace --lib --exclude stellarroute-contracts --exclude stellarroute-api` and confirm all property tests pass

- [x] 8. Final CI gate verification
  - Run `cargo test -p stellarroute-api --test swap_integration --test swap_submit_integration --test openapi_swap_contract` and confirm all pass
  - Run `cargo clippy --workspace --all-features --exclude stellarroute-contracts -- -D warnings` and confirm zero warnings
  - Run `cargo test --workspace --lib --exclude stellarroute-contracts --exclude stellarroute-api` and confirm all pass
  - Run `cargo fmt --all -- --check` and confirm no formatting issues
