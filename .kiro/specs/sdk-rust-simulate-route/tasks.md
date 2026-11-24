# Implementation Plan

## Tasks

- [x] 1. Add new SDK types to `crates/sdk-rust/src/types.rs`
  - [x] 1.1 Add `DryRunHop` struct with `from_asset`, `to_asset`, `source` (required) and `fee_bps`, `price`, `venue_ref` (optional)
  - [x] 1.2 Add `SlippageOverride` struct with `venue_ref: String` and `slippage_bps: u32`
  - [x] 1.3 Add `SimulateRouteRequest` struct with `hops: Vec<DryRunHop>`, `amount: String`, `slippage_bps: Option<u32>`, `slippage_bps_overrides: Vec<SlippageOverride>`; implement custom `Serialize` using a private wire-shape wrapper so hops serialize under `{ "route": { "hops": [...] } }`
  - [x] 1.4 Add `SimulateQuoteResult` struct mirroring the API `QuoteResponse` wire shape with all optional fields typed as `Option<serde_json::Value>` where the API uses complex types
  - [x] 1.5 Add `SwapHopDto` struct with `source_asset`, `destination_asset`, `venue_type`, `venue_ref` (String), `price` (f64), `fee_bps` (u32)
  - [x] 1.6 Add `SwapPathDto` struct with `hops: Vec<SwapHopDto>` and `estimated_output: i64`
  - [x] 1.7 Add `SimulateRouteResponse` struct with `quote: SimulateQuoteResult`, `exclusion_diagnostics: Option<serde_json::Value>`, `swap_path: SwapPathDto`

- [x] 2. Add `ApiEnvelope` helper and `simulate_route` method to `crates/sdk-rust/src/client.rs`
  - [x] 2.1 Add private `ApiEnvelope<T>` struct that deserializes `{ v, timestamp, request_id, data: T }` and exposes only `data`
  - [x] 2.2 Add `simulate_route(&self, request: SimulateRouteRequest) -> Result<SimulateRouteResponse>` method that POSTs to `api/v1/simulate/route` and unwraps the `ApiEnvelope`
  - [x] 2.3 Add `SimulateRouteRequest` and `SimulateRouteResponse` to the `use crate::types::{...}` import block in `client.rs`

- [x] 3. Re-export new types from `crates/sdk-rust/src/lib.rs`
  - [x] 3.1 Add `DryRunHop`, `SlippageOverride`, `SimulateRouteRequest`, `SimulateQuoteResult`, `SwapHopDto`, `SwapPathDto`, `SimulateRouteResponse` to the `pub use types::{...}` block

- [x] 4. Add mock-based integration tests to `crates/sdk-rust/tests/client_integration.rs`
  - [x] 4.1 Add import for new types: `SimulateRouteRequest, SimulateRouteResponse, DryRunHop, SlippageOverride`
  - [x] 4.2 Write `simulate_route_happy_path` test: mount valid `ApiResponse<RouteDryRunResponse>` JSON (with all required fields populated), call `simulate_route`, assert `quote.price`, `swap_path.hops.len()`, `swap_path.estimated_output`
  - [x] 4.3 Write `simulate_route_uses_post_method_and_correct_path` test: mock requires `method("POST")` and `path("/api/v1/simulate/route")`, verify request succeeds
  - [x] 4.4 Write `simulate_route_sends_user_agent_header` test: mock requires `header_regex("user-agent", r"^stellarroute-sdk-rust/")`, verify request succeeds
  - [x] 4.5 Write `simulate_route_400_maps_to_validation_error` test: mount 400 with `"error": "validation_error"`, assert `SdkError::Api { code: ApiErrorCode::ValidationError, status: 400 }`
  - [x] 4.6 Write `simulate_route_404_no_route_maps_to_no_route_error` test: mount 404 with `"error": "no_route"`, assert `SdkError::Api { code: ApiErrorCode::NoRoute, status: 404 }`
  - [x] 4.7 Write `simulate_route_500_retries_then_succeeds` test: configure `max_retries: 1`, mount 500 once then 200 on second call, assert `Ok(response)` is returned
  - [x] 4.8 Write `simulate_route_optional_fields_absent` test: mount response with no optional fields (no `exclusion_diagnostics`, etc.), assert `exclusion_diagnostics` is `None`

- [x] 5. Add property-based tests in `crates/sdk-rust/tests/simulate_route_pbt.rs`
  - [x] 5.1 Create the file with `proptest` imports and `use stellarroute_sdk::{SimulateRouteRequest, SimulateRouteResponse, DryRunHop, SlippageOverride, SwapHopDto, SwapPathDto, SimulateQuoteResult, AssetInfo, PathStep}`
  - [x] 5.2 Write `arb_dry_run_hop()` proptest strategy producing arbitrary `DryRunHop` values (required fields non-empty, optionals as `Option::arbitrary_with`)
  - [x] 5.3 Write `arb_swap_hop_dto()` strategy producing arbitrary `SwapHopDto` values
  - [x] 5.4 Write `arb_simulate_route_response()` strategy producing arbitrary `SimulateRouteResponse` values with all sub-type variants covered
  - [x] 5.5 Write property test `simulate_route_response_serde_roundtrip` — for all `SimulateRouteResponse` values produced by the strategy, `serde_json::from_str(&serde_json::to_string(&v).unwrap()) == v`; tagged `// Feature: sdk-rust-simulate-route, Property 1: SimulateRouteResponse round-trip serialization`
  - [x] 5.6 Write property test `simulate_route_request_wire_shape` — for all `Vec<DryRunHop>` of length 1..=5, `SimulateRouteRequest` serializes to JSON containing `json["route"]["hops"]` as an array with the same length as the input; tagged `// Feature: sdk-rust-simulate-route, Property 2: Wire-shape invariant for SimulateRouteRequest`

- [x] 6. Verify CI gates pass
  - [x] 6.1 Run `cargo test --workspace --lib --exclude stellarroute-contracts --exclude stellarroute-api` and confirm all new tests pass
  - [x] 6.2 Run `cargo test -p stellarroute-api --test swap_integration --test swap_submit_integration --test openapi_swap_contract` and confirm zero regressions
  - [x] 6.3 Run `cargo clippy --workspace --all-features --exclude stellarroute-contracts -- -D warnings` and confirm zero warnings
  - [x] 6.4 Run `cargo fmt --all -- --check` and confirm no format issues
