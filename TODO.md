# TODO

## Simulation dry-run endpoint (M2 — Routing Engine)

### Step 1 — Implement handler
- [ ] Replace placeholder error in `crates/api/src/routes/simulation_route.rs::simulate_route_dry_run`
- [ ] Validate request body: non-empty hops, hop-chain continuity, amount > 0
- [ ] Apply slippage defaults + per-hop overrides keyed by `venue_ref`

### Step 2 — Reuse quote/diagnostics pipeline
- [ ] Compute per-hop feasibility and expected output using existing routing/health/policy logic (same approach as `/api/v1/quote`)
- [ ] Ensure exclusion diagnostics are produced consistently

### Step 3 — Guarantee no side effects
- [ ] Confirm no wallet signing / on-chain execution is triggered

### Step 4 — Tests
- [ ] Add/extend integration tests using fixture routes for `/api/v1/simulate/route`
- [ ] Assert per-hop diagnostics consistency with `/api/v1/quote`
- [ ] Assert no execution side effects

### Step 5 — Verify
- [ ] Run `cargo test` (or the relevant subset) and fix any compilation issues

