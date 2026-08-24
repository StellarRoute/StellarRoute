# Internal Security Review Checklist — Router + API + Wallet

**Milestone:** M5 — Audit (internal review gate before external audit and mainnet capital)

This checklist is the internal security review that must be completed before
the external contract audit begins and before the mainnet flag is flipped.
It covers the three user-fund-relevant surfaces: the Soroban router contract
(`crates/contracts`), the public API (`crates/api`), and the frontend wallet
integration (`frontend/lib/wallet`).

**How to use this document**

- Each item is checked off by a reviewer only after inspecting the current
  code on `main` (not from memory of a previous review).
- Any item that fails review becomes a **finding**: open a GitHub issue
  labelled `security`, link it in the [Findings log](#findings-log), and
  leave the checklist item unchecked until the fix is merged and re-reviewed.
- **Launch gate:** the mainnet flag must not be flipped while any finding of
  severity Critical or High is open. See [Mainnet gate](#mainnet-gate).

---

## 1. Router contract (`crates/contracts`)

### Access control & governance

- [x] Every state-changing entrypoint authenticates the caller
      (`require_auth()` against stored admin, or multisig proposal flow via
      `propose` / `approve_proposal` / `execute_proposal`).
- [x] `initialize()` can only succeed once (double-init returns
      `AlreadyInitialized`).
- [x] Admin transfer (`set_admin`) and multisig migration
      (`migrate_to_multisig`) are admin-gated and emit events.
- [x] Guardian pause (`guardian_pause`) is restricted to the configured
      guardian set and cannot unpause.
- [x] Upgrade path is timelocked (`propose_upgrade` → delay →
      `execute_upgrade`) and cancellable (`cancel_upgrade`).

### Swap execution & value safety

- [x] `execute_swap` validates route shape (non-empty, hop count cap) and
      amounts (positive input, `min_output` enforced) before any transfer.
- [x] Slippage guarantee: transaction reverts when actual output is below
      the caller's `min_output`.
- [x] Multi-hop failure atomicity: a failed hop rolls back the entire swap
      (see `test_multihop_rollback.rs`).
- [x] Only admin-registered pools are callable; pool addresses from
      user-supplied routes are checked against the registry.
- [x] Fee arithmetic uses checked operations; `overflow-checks = true` in
      the release profile; fee rate is bounds-checked at configuration time.
- [x] Fee distribution (`distribute_fees`) cannot send funds to
      unconfigured/zero destinations and records distribution history.
- [x] Token allowlist (`add_token` / `remove_token`) gates which assets can
      appear in routes.

### Denial of service & resource limits

- [x] Route length is capped, bounding cross-contract call depth and
      instruction budget per invocation.
- [x] Storage TTLs are extended on writes (instance and persistent);
      TTL-expiry risk is documented and monitored
      (see `known-issues.md` §5).
- [x] Input-validation fuzz targets exist for `validate_route` /
      `execute_swap` (`fuzz_targets.rs`, runbook in `fuzzing.md`).

### Known gaps carried into external audit

- [ ] `get_quote()` does not check `is_paused` (`known-issues.md` §2) —
      accepted for now, front-end/SDK must check `is_paused()`.
- [ ] `register_pool` error-name mismatch (`known-issues.md` §4) — cosmetic,
      tracked for cleanup.

## 2. API (`crates/api`)

### Authentication & authorization

- [x] Production boots refuse to start without `ADMIN_AUTH_TOKEN`
      (`validate_admin_auth_startup`) and with auth disabled unless the
      break-glass `ALLOW_INSECURE_PUBLIC_API=1` override is explicit
      (`validate_auth_startup`).
- [x] All `/api/v1/admin/*` and `/api/v1/system/*` mutations require
      `AdminAuth`; requests are denied by default when the token is unset
      (verified by `unauthenticated_admin_mutations.rs`).
- [x] `/metrics` and `/api/v1/replay/*` are admin-gated in production
      (`production_metrics_replay_lockdown.rs`).
- [x] Kill switch and canary config mutations require admin auth
      (`kill_switch_integration.rs`, `canary_auth_integration.rs`).
- [x] Integrator API keys (`API_KEYS`) are required in production for quote
      and replay surfaces; `PUBLIC_GET_ROUTES` is the scoped alternative to
      disabling auth globally.

### Input validation & abuse resistance

- [x] Rate limiting middleware on quote/orderbook/pairs endpoints, with
      configurable per-route limits and Redis-backed enforcement when
      available.
- [x] Path/query parameters (asset pairs, amounts, slippage bps) are parsed
      into typed models; malformed input returns structured 4xx errors, not
      500s.
- [x] CORS is an explicit allowlist in production (no wildcard).
- [x] WebSocket connections are capped (`WS_MAX_CONNECTIONS`) with
      ping/pong liveness and backpressure timeouts.

### Secrets & data handling

- [x] No secrets in logs: startup and health paths do not print credential
      material; deploy artifacts contain only non-secret fields.
- [x] Outbound consumer webhooks are HMAC-SHA256 signed with per-consumer
      secrets stored server-side (see
      `docs/deployment/secrets-management.md` and `docs/key_rotation.md`).
- [x] Admin mutations are recorded to the admin audit log
      (`admin_audit.rs`); audit-log export applies redaction.
- [x] Database access goes through sqlx with bound parameters (no string
      SQL construction from user input).

### Operational safety

- [x] Kill switch can halt the live quote path without a deploy
      (`docs/RUNBOOK_KILL_SWITCH.md`).
- [x] Quote freshness/health/policy filters exclude stale or unhealthy
      venues from quotes (`stellarroute-routing::health`).
- [x] Graceful shutdown drains in-flight requests
      (`SHUTDOWN_DRAIN_TIMEOUT_S`).

## 3. Frontend wallet (`frontend/lib/wallet`)

### Key & signing safety

- [x] Private keys never leave the wallet extension: the app builds an
      unsigned XDR (`xdr-builder.ts`) and requests signatures via the
      Freighter/xBull APIs; no key material is handled or stored by the app.
- [x] The signed transaction is submitted as returned by the wallet
      (`submit.ts`); the app never mutates a transaction after signing.
- [x] Account-change detection: the connected address is re-checked before
      signing so a wallet account switch cannot sign for the wrong account
      (`checkAddressChange.test.ts`).

### Transaction integrity

- [x] Swap transactions embed the quoted `min_output`/slippage bound so the
      contract reverts on unfavorable execution rather than trusting the UI.
- [x] Network passphrase is pinned per environment (testnet vs mainnet);
      transactions built for one network cannot be replayed on another.
- [x] XDR construction is covered by unit tests (`xdr-builder.test.ts`)
      including malformed-input cases.

### UX-level security

- [x] The UI surfaces the route, price impact, fees, and minimum received
      before the user signs — what is signed matches what is shown.
- [x] Quote expiration is enforced client-side: expired quotes cannot be
      submitted and must be re-fetched.
- [x] Wallet connection errors and rejections fail closed (no retry loop
      that could prompt-fatigue the user into signing).

---

## Findings log

Findings from this review are tracked as GitHub issues labelled `security`
and linked here. A finding is **Closed** only when the fix is merged and the
relevant checklist item has been re-reviewed.

| ID | Severity | Area | Issue | Status |
|---|---|---|---|---|
| ISR-1 | Low | Contract | [#known-issue] `get_quote()` lacks pause check (`known-issues.md` §2) | Open — accepted for testnet; revisit before mainnet |
| ISR-2 | Info | Contract | [#known-issue] `register_pool` error-name mismatch (`known-issues.md` §4) | Open — cosmetic |

Severity scale:

- **Critical** — direct loss of user funds or full contract/API compromise.
- **High** — funds at risk under plausible conditions, or auth bypass.
- **Medium** — degraded security guarantees needing unlikely preconditions.
- **Low** — hardening gaps, defense in depth.
- **Info** — no direct security impact.

## Mainnet gate

The mainnet flag flip is blocked until all of the following hold:

- [ ] Every checklist item above is either checked or covered by a linked,
      triaged finding.
- [ ] **Zero open Critical or High findings** in the findings log.
- [ ] The external contract audit is complete and its findings remediated
      (tracked separately under issue #995).
- [ ] Sign-off recorded below by two reviewers.

| Reviewer | Scope reviewed | Commit | Date |
|---|---|---|---|
| _pending_ | Contracts | | |
| _pending_ | API + wallet | | |
