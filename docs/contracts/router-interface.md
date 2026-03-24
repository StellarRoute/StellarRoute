# Soroban Router Contract Interface and Event Schema

This document defines the canonical public interface for routing flows and the event schema for indexing and debugging.

## Public Routing Interface

The router exposes three core methods:

- `validate(route: Route) -> Result<(), ContractError>`
- `quote(amount_in: i128, route: Route) -> Result<QuoteResult, ContractError>`
- `execute(sender: Address, params: SwapParams) -> Result<SwapResult, ContractError>`

Existing methods `get_quote` and `execute_swap` remain available for backward compatibility; `quote` and `execute` are stable aliases for integrators.

## Authorization Assumptions

- `validate`: no auth required; this is a read-only/preflight check.
- `quote`: no auth required; this is a read-only pricing simulation.
- `execute`: requires `sender.require_auth()` via `execute_swap`.
- Admin/governance methods keep their existing auth model and are out of scope for this interface.

## Validation Rules

`validate` and `quote` both enforce:

- route is non-empty and max hop count is respected
- route has not expired when `route.expires_at > 0`
- all assets pass token allowlist checks when allowlist is active
- all pools in the route are registered

## Event Schema

All routing events share the `"StellarRoute"` contract topic prefix and a short event key for indexability.

### `rt_val` (route validated)

- **topics**: `("StellarRoute", "rt_val")`
- **data**: `(hop_count: u32, expires_at: u64, ledger: u32)`

### `quote` (quote generated)

- **topics**: `("StellarRoute", "quote")`
- **data**:
  `(amount_in: i128, expected_output: i128, fee_amount: i128, price_impact_bps: u32, hop_count: u32, valid_until: u64, ledger: u32)`

### `exe_req` (execution requested)

- **topics**: `("StellarRoute", "exe_req", sender: Address)`
- **data**: `(amount_in: i128, hop_count: u32, deadline: u64, ledger: u32)`

### `swap` (execution succeeded)

- **topics**: `("StellarRoute", "swap", sender: Address)`
- **data**: `(amount_in: i128, amount_out: i128, fee: i128, route: Route, ledger: u32)`

### `exe_fail` (execution failed)

- **topics**: `("StellarRoute", "exe_fail", sender: Address)`
- **data**: `(error_code: u32, ledger: u32)`

## Indexing Guidance

- Group by topic key to build lifecycle views: `exe_req` -> `swap` or `exe_fail`.
- Use `sender` in indexed topics for per-user traces.
- Decode `error_code` with `ContractError` enum to support deterministic failure analytics.
