# TODO

## Simulation dry-run endpoint (M2 — Routing Engine)

### Step 1 — Implement handler
- [x] Replace placeholder error in `crates/api/src/routes/simulation_route.rs::simulate_route_dry_run`
- [x] Validate request body: non-empty hops, hop-chain continuity, amount > 0
- [x] Apply slippage defaults + per-hop overrides keyed by `venue_ref`

### Step 2 — Reuse quote/diagnostics pipeline
- [x] Compute per-hop feasibility and expected output using existing routing/health/policy logic (same approach as `/api/v1/quote`)
- [x] Ensure exclusion diagnostics are produced consistently

### Step 3 — Guarantee no side effects
- [x] Confirm no wallet signing / on-chain execution is triggered

### Step 4 — Tests
- [x] Add/extend integration tests using fixture routes for `/api/v1/simulate/route`
- [x] Assert per-hop diagnostics consistency with `/api/v1/quote` (shape parity verified in `simulation_route_integration.rs::dry_run_response_quote_shape_matches_quote_endpoint_contract`)
- [x] Assert no execution side effects (pure-function test in `simulation_route_integration.rs::route_conversion_and_policy_application_are_side_effect_free`)

### Step 5 — Verify
- [ ] Run `cargo test` (or the relevant subset) and fix any compilation issues

