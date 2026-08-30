# Soroban Contract Events Catalog

This document lists every public event emitted by the StellarRoute Soroban router contract (`crates/contracts/src/events.rs`). It is intended for integrators building indexers, monitoring tools, or off-chain consumers of on-chain events.

All events share a common topic prefix: `("StellarRoute", <event_short>)`. The full topic tuple is published via `soroban_sdk::Env::events().publish(...)`.

---

## Contract Lifecycle

### `initialized`
- **Topics**: `("StellarRoute", "init")`
- **Payload**: `(admin: Address, fee_rate: u32)`
- **Fires**: Once, when the contract is initialized via `__constructor` or equivalent admin action.
- **Notes**: `fee_rate` is basis points (1/10,000).

### `admin_changed`
- **Topics**: `("StellarRoute", "adm_chg")`
- **Payload**: `(old_admin: Address, new_admin: Address)`
- **Fires**: When the admin address is updated.

### `paused`
- **Topics**: `("StellarRoute", "paused")`
- **Payload**: `()`
- **Fires**: When the contract enters paused state (no swaps/quotes allowed).

### `unpaused`
- **Topics**: `("StellarRoute", "unpaused")`
- **Payload**: `()`
- **Fires**: When the contract exits paused state.

### `pool_registered`
- **Topics**: `("StellarRoute", "reg_pol")`
- **Payload**: `pool_address: Address`
- **Fires**: When a new liquidity pool is registered in the router.

---

## Swap Execution

### `swap_executed`
- **Topics**: `("StellarRoute", "swap", sender: Address)`
- **Payload**: `(amount_in: i128, amount_out: i128, fee: i128, route: Route, ledger_sequence: u32)`
- **Fires**: On every successful swap execution through the router.
- **Notes**: `Route` is a custom Soroban type encoding the hop path. `ledger_sequence` is the ledger where the swap was included.

### `route_validated`
- **Topics**: `("StellarRoute", "rt_val")`
- **Payload**: `(hop_count: u32, expires_at: u64, ledger_sequence: u32)`
- **Fires**: When a route is validated (e.g., during quote generation or pre-execution checks).
- **Notes**: `expires_at` is a Unix timestamp (seconds).

### `quote_generated`
- **Topics**: `("StellarRoute", "quote")`
- **Payload**: `(amount_in: i128, expected_output: i128, fee_amount: i128, price_impact_bps: u32, hop_count: u32, valid_until: u64, ledger_sequence: u32)`
- **Fires**: When a quote is generated on-chain (if the contract exposes quote generation).
- **Notes**: `valid_until` is a Unix timestamp (seconds). `price_impact_bps` is basis points.

### `execution_requested`
- **Topics**: `("StellarRoute", "exe_req", sender: Address)`
- **Payload**: `(amount_in: i128, hop_count: u32, deadline: u64, ledger_sequence: u32)`
- **Fires**: When a swap execution is requested but not yet completed.
- **Notes**: `deadline` is a Unix timestamp (seconds) after which the execution can be cancelled.

### `execution_failed`
- **Topics**: `("StellarRoute", "exe_fail", sender: Address)`
- **Payload**: `(error_code: u32, ledger_sequence: u32)`
- **Fires**: When a requested execution fails (e.g., deadline passed, slippage exceeded, pool error).
- **Notes**: `error_code` is a contract-defined error code (not HTTP).

---

## Multi-sig Governance

### `governance_migrated`
- **Topics**: `("StellarRoute", "gov_mgr")`
- **Payload**: `(old_admin: Address, signer_count: u32, threshold: u32)`
- **Fires**: When governance is migrated from single-admin to multi-sig.

### `proposal_created`
- **Topics**: `("StellarRoute", "prop_new")`
- **Payload**: `(id: u64, proposer: Address, action: ProposalAction)`
- **Fires**: When a new governance proposal is created.
- **Notes**: `ProposalAction` is a contract enum (e.g., `SetFee`, `AddPool`, `Upgrade`, etc.).

### `proposal_approved`
- **Topics**: `("StellarRoute", "prop_apr")`
- **Payload**: `(id: u64, signer: Address, approvals: u32)`
- **Fires**: When a signer approves a proposal. `approvals` is the new total count.

### `proposal_executed`
- **Topics**: `("StellarRoute", "prop_exe")`
- **Payload**: `id: u64`
- **Fires**: When a proposal reaches threshold and is executed.

### `proposal_cancelled`
- **Topics**: `("StellarRoute", "prop_can")`
- **Payload**: `(id: u64, by: Address)`
- **Fires**: When a proposal is cancelled (by proposer or governance).

### `guardian_set`
- **Topics**: `("StellarRoute", "grd_set")`
- **Payload**: `guardian: Address`
- **Fires**: When a guardian address is set/updated.

### `guardian_paused`
- **Topics**: `("StellarRoute", "grd_pse")`
- **Payload**: `guardian: Address`
- **Fires**: When the guardian pauses the contract.

---

## Contract Upgrades & Migrations

### `upgrade_proposed`
- **Topics**: `("StellarRoute", "upg_prp")`
- **Payload**: `(proposer: Address, old_hash: BytesN<32>, new_hash: BytesN<32>, execute_after: u64)`
- **Fires**: When a contract upgrade is proposed.
- **Notes**: `execute_after` is a Unix timestamp (seconds) — the earliest ledger time the upgrade can be executed.

### `upgrade_completed`
- **Topics**: `("StellarRoute", "upg_done")`
- **Payload**: `(old_hash: BytesN<32>, new_hash: BytesN<32>, ledger: u32)`
- **Fires**: When the upgrade is successfully executed.

### `upgrade_cancelled`
- **Topics**: `("StellarRoute", "upg_can")`
- **Payload**: `by: Address`
- **Fires**: When a proposed upgrade is cancelled.

### `migration_completed`
- **Topics**: `("StellarRoute", "mig_done")`
- **Payload**: `(major: u32, minor: u32, patch: u32)`
- **Fires**: When a data/storage migration completes after an upgrade.
- **Notes**: Version is semantic (major.minor.patch).

---

## Token Allowlist

### `token_added`
- **Topics**: `("StellarRoute", "tok_add")`
- **Payload**: `(asset: Asset, added_by: Address)`
- **Fires**: When a token is added to the router's allowlist.

### `token_removed`
- **Topics**: `("StellarRoute", "tok_rm")`
- **Payload**: `(asset: Asset, removed_by: Address)`
- **Fires**: When a token is removed from the allowlist.

### `token_updated`
- **Topics**: `("StellarRoute", "tok_upd")`
- **Payload**: `(asset: Asset, updated_by: Address)`
- **Fires**: When a token's metadata (e.g., decimals, issuer) is updated in the allowlist.
- **Notes**: `Asset` is a custom Soroban type (native or alphanum4/12).

---

## MEV Protection

### `high_impact_swap`
- **Topics**: `("StellarRoute", "hi_imp", sender: Address)`
- **Payload**: `(impact_bps: u32, amount_in: i128)`
- **Fires**: When a swap exceeds the configured price impact threshold.
- **Notes**: Used for monitoring/alerting on potentially exploitative trades.

### `rate_limit_hit`
- **Topics**: `("StellarRoute", "rl_hit", sender: Address)`
- **Payload**: `(swap_count: u32, window: u32)`
- **Fires**: When a sender exceeds the per-window swap rate limit.
- **Notes**: `window` is in seconds.

### `commitment_created`
- **Topics**: `("StellarRoute", "cmt_new", sender: Address)`
- **Payload**: `(commitment_hash: BytesN<32>, deposit_amount: i128)`
- **Fires**: When a user creates a commitment (commit-reveal MEV protection).

### `commitment_revealed`
- **Topics**: `("StellarRoute", "cmt_rvl", sender: Address)`
- **Payload**: `commitment_hash: BytesN<32>`
- **Fires**: When a commitment is revealed for execution.

### `ttl_extended`
- **Topics**: `("StellarRoute", "ttl_ext")`
- **Payload**: `(pools_extended: u32, ledger: u32)`
- **Fires**: When pool TTLs are extended (storage rent / bounded collections maintenance).

### `ttl_warning`
- **Topics**: `("StellarRoute", "ttl_wrn")`
- **Payload**: `(estimated_remaining: u64, threshold: u32)`
- **Fires**: When a pool's estimated TTL remaining falls below the warning threshold.
- **Notes**: `estimated_remaining` is ledgers; `threshold` is the configured warning ledger count.

---

## Fee Distribution

### `fee_collected`
- **Topics**: `("StellarRoute", "fee_col")`
- **Payload**: `(asset: Asset, amount: i128, ledger_sequence: u32)`
- **Fires**: When swap fees are collected into the fee pool.

### `fees_distributed`
- **Topics**: `("StellarRoute", "fee_dist")`
- **Payload**: `(asset: Asset, total_distributed: i128, ledger_sequence: u32)`
- **Fires**: When accumulated fees are distributed to recipients (e.g., LPs, treasury).

### `fees_burned`
- **Topics**: `("StellarRoute", "fee_brn")`
- **Payload**: `(asset: Asset, amount: i128, ledger_sequence: u32)`
- **Fires**: When fees are burned (e.g., deflationary mechanism).

---

## Integration Notes

### Event Filtering
All events share the first topic `"StellarRoute"`. Use this to filter StellarRoute events from other contracts on the same network.

### Short Topic Codes
The second topic is a short symbol (max 9 chars) for compactness:
| Code | Event |
|------|-------|
| `init` | initialized |
| `adm_chg` | admin_changed |
| `reg_pol` | pool_registered |
| `paused` / `unpaused` | paused / unpaused |
| `swap` | swap_executed |
| `rt_val` | route_validated |
| `quote` | quote_generated |
| `exe_req` | execution_requested |
| `exe_fail` | execution_failed |
| `gov_mgr` | governance_migrated |
| `prop_new` / `prop_apr` / `prop_exe` / `prop_can` | proposal_* |
| `grd_set` / `grd_pse` | guardian_* |
| `upg_prp` / `upg_done` / `upg_can` | upgrade_* |
| `mig_done` | migration_completed |
| `tok_add` / `tok_rm` / `tok_upd` | token_* |
| `hi_imp` / `rl_hit` | MEV protection |
| `cmt_new` / `cmt_rvl` | commitment_* |
| `ttl_ext` / `ttl_wrn` | TTL maintenance |
| `fee_col` / `fee_dist` / `fee_brn` | fee distribution |

### Payload Decoding
Events are published via `soroban_sdk::Env::events().publish(topics, data)`. The `data` payload is a tuple matching the signatures above. When consuming via RPC (`getEvents`), the `data` field will be XDR-encoded. Use the contract's type definitions (`crates/contracts/src/types.rs`) to decode.

### Ledger Sequence
Most events include `ledger_sequence: u32` (the ledger where the event was emitted). Use this for ordering and idempotent processing.

### No Breaking Changes
This catalog is **additive only**. New events may be added in future contract versions. Existing event topics, payload structure, and firing conditions will not change without a major version bump and migration.

---

*Generated from `crates/contracts/src/events.rs`*