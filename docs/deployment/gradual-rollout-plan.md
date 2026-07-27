# Gradual Rollout Plan: Limited Pairs → Full Markets

**Issue:** #1020  
**Milestone:** M5 — Launch  
**Audience:** Operators, on-call engineers, release managers  
**Related:** [Roadmap §5.6.2 Gradual Rollout](../../Roadmap.md), [Routing canary](../routing_canary.md), [Kill switch runbook](../RUNBOOK_KILL_SWITCH.md), [Status page](../../frontend/docs/status-page-feature.md)

This plan describes how StellarRoute goes live with a **limited trading-pair set**, then expands to **full markets** over the first 24–48 hours (and subsequent stages), using explicit **metric gates**, **rollback criteria**, and **status-page communication templates**.

---

## Goals

1. Reduce blast radius on first mainnet (or public testnet) exposure.
2. Prove quote quality, swap success, and operational readiness before enabling all markets.
3. Make promotion and rollback decisions objective (metrics + timeboxes), not ad hoc.
4. Keep users informed via the public status page and incident channels.

---

## Rollout stages

| Stage | Name | Duration (target) | Markets / features enabled | Exit gate (all must pass) |
|-------|------|-------------------|----------------------------|---------------------------|
| **0** | Pre-flight | Before public traffic | No public swaps; internal smoke only | Pre-launch checklist complete (audits, monitoring, kill switch, legal placeholder reviewed) |
| **1** | Limited pairs | First **0–24h** | Allowlist only (see below); multi-hop optional off | Metric gates green for ≥6 consecutive hours |
| **2** | Expanded pairs | **24–48h** | Add high-liquidity pairs; multi-hop on for allowlisted assets | Metric gates green for ≥12 consecutive hours; no open Sev-1 |
| **3** | Full markets | After Stage 2 | Remaining markets per catalog policy; normal feature flags | Metric gates green for ≥24h; rollback rehearsal documented |
| **4** | Steady state | Ongoing | Full catalog + standard change management | Canary / feature-flag process owns further changes |

### Stage 1 allowlist (default)

Start with the deepest, most operationally familiar pairs. Adjust per network config; do not invent unlisted assets at launch.

| Priority | Pair (example) | Notes |
|----------|----------------|-------|
| P0 | `XLM/USDC` | Primary smoke + trader path |
| P0 | `XLM/USDT` (if listed) | Only if liquidity and trustlines are verified |
| P1 | One additional blue-chip quote vs XLM | Add only after P0 gates pass |

Enable via feature flag / market allowlist (for example `LIVE_PAIR_ALLOWLIST` or the equivalent config used by the API and frontend). All other pairs remain hidden or return a clear “not yet available” empty state.

### Feature flags during Stage 1–2

| Flag / control | Stage 1 | Stage 2 | Stage 3+ |
|----------------|---------|---------|----------|
| Public swap UI | On (allowlist) | On | On |
| Multi-hop routing | Off or max 1 intermediate | On (capped hops) | Normal policy |
| New AMM venues | Off | Canary sample | Gradual |
| Admin kill switch | Armed | Armed | Armed |
| Routing canary | Optional sample | Recommended | Optional |

---

## Metrics gates

Promote to the next stage **only** when **all** gates pass for the required consecutive window. Values below are launch defaults; tune in monitoring dashboards and record overrides in the incident channel.

### Reliability & correctness

| Metric | Stage 1 gate | Stage 2+ gate | Source |
|--------|--------------|---------------|--------|
| Quote API success rate (`2xx` / total) | ≥ 99.0% | ≥ 99.5% | API / Prometheus |
| Quote p95 latency | ≤ 500 ms | ≤ 400 ms | API |
| Swap prepare → submit success (user-signed) | ≥ 95% of attempts | ≥ 98% | Frontend / backend telemetry |
| On-chain swap failure rate (post-submit) | ≤ 2% | ≤ 1% | Indexer / Horizon |
| Slippage exceeded / user abort rate | Track; investigate if > 15% of confirms | Track; investigate if > 10% | Frontend |
| Indexer lag | Within runbook SLO | Within runbook SLO | `/health`, lag alerts |
| Error budget burn (5xx) | No sustained burn > 2× budget | No sustained burn > 1× budget | SLO probes |

### Safety

| Signal | Gate |
|--------|------|
| Sev-1 / Sev-0 open | **Block promotion**; consider rollback |
| Kill switch activated | Remain in current or prior stage until cleared |
| Canary consecutive violations ≥ threshold | Disable candidate policy; do not expand pairs |
| Unexpected admin / auth failures | Block Stage 2+ |

### Qualitative exit criteria (Stage 1 → 2)

- [ ] At least one successful end-to-end live swap on each P0 pair (documented tx hashes).
- [ ] Status page reflects accurate stage (see templates below).
- [ ] On-call acknowledges readiness in the launch channel.
- [ ] No unresolved P0 docs gaps for risk disclosure or first-swap user guide.

---

## Rollback criteria

**Trigger rollback** (or freeze promotion) when **any** of the following is true:

| Severity | Trigger | Immediate action |
|----------|---------|------------------|
| Sev-0 | Fund loss, critical contract bug, or unsafe routing producing systematically bad prices | Kill switch / pause swaps; revert to Stage 0 UI (disable confirm); status **Major outage** |
| Sev-1 | Quote success < 95% for 15 minutes; swap failure > 5% for 30 minutes; indexer lag beyond critical threshold | Freeze pair expansion; disable multi-hop; consider Stage 1 allowlist only |
| Sev-1 | Security incident (key leak, auth bypass) | Pause public API write paths; rotate credentials per [key rotation](../key_rotation.md) |
| Sev-2 | Elevated latency or error rate without fund risk | Hold stage; communicate **Degraded performance**; no expansion |

### Rollback procedure (pairs / features)

1. **Announce** using the rollback status template (below).
2. **Tighten allowlist** to last known-good pair set (or empty → quotes-only).
3. **Disable** multi-hop / candidate routing policies ([routing canary](../routing_canary.md)).
4. **Confirm** kill-switch state if needed ([kill switch runbook](../RUNBOOK_KILL_SWITCH.md)).
5. **Verify** `/health` and `/health/deps`; watch quote and swap dashboards for 30 minutes.
6. **Post-mortem** before any re-expansion (root cause, blast radius, gate adjustments).

Soroban contract WASM cannot be natively rolled back; follow [deployment README](./README.md#mainnet-rollback-and-upgrade) for contract-level recovery. Product/feature rollback is primarily **config + flags + allowlist**.

---

## Decision log (operators)

Record each stage change in the launch channel (and optionally `docs/deployment/` notes):

```text
UTC time:
From stage → To stage:
Allowlist after change:
Gates evidence (dashboard links / screenshots):
Decision owner:
Rollback plan if gates fail in next window:
```

---

## Status-page communication templates

Use the in-app `/status` page and any mirrored status provider. Keep titles short; put details and ETA in the body. Update when the stage changes or when rolling back.

### Template A — Stage 1 start (limited pairs)

**Title:** Limited markets live — gradual rollout in progress  

**Body:**

> StellarRoute is live with a **limited pair allowlist** for the first 24 hours while we monitor quote quality and swap success.  
> **Available now:** XLM/USDC (and any other pairs listed on this page).  
> Additional markets will unlock after reliability gates pass.  
> Traders: use small sizes and review slippage before confirming. See our risk disclosure and first-swap guide in Docs.

**Status component:** Operational (Limited markets)

### Template B — Stage 2 expansion

**Title:** Market expansion — more pairs enabled  

**Body:**

> We have expanded the live allowlist after Stage 1 metric gates passed. Multi-hop routing may now be enabled for allowlisted assets.  
> Monitoring continues through the 24–48h window. Report issues via GitHub or community channels.

**Status component:** Operational

### Template C — Full markets

**Title:** Full market catalog enabled  

**Body:**

> Gradual rollout complete: the full supported market catalog is available under normal operational policy.  
> Feature changes will follow standard canary / change-management process.

**Status component:** Operational

### Template D — Degraded / holding stage

**Title:** Degraded performance — pair expansion paused  

**Body:**

> We are investigating elevated error rates / latency. **Existing limited pairs remain available** where healthy; **new markets will not be enabled** until gates recover.  
> Next update within 30 minutes.

**Status component:** Degraded

### Template E — Rollback / major incident

**Title:** Service disruption — swaps paused or restricted  

**Body:**

> We have rolled back to a safer configuration (restricted allowlist and/or swaps paused) due to [brief cause].  
> Quotes may remain available in read-only mode.  
> Funds on-wallet are not custodied by StellarRoute; do not retry large swaps until this banner clears.  
> Next update within 15 minutes.

**Status component:** Major outage / Partial outage

### Template F — All clear after incident

**Title:** Incident resolved — gradual rollout resumed at Stage N  

**Body:**

> The incident from [time] is resolved. We are resuming the gradual rollout at **Stage N** with allowlist: [pairs].  
> Post-incident review will follow within 48 hours.

**Status component:** Operational (or Limited markets)

---

## Comms checklist (every stage change)

- [ ] Status page updated with the matching template  
- [ ] Launch / Discord / Discussions short notice posted  
- [ ] On-call and release manager acknowledged  
- [ ] Decision log filled  
- [ ] GitHub issue / milestone comment if public launch tracking is used  

---

## Verify

```bash
rg -n 'rollout|Gradual' Roadmap.md docs/deployment
```

Expected: this file, Roadmap Phase 5.6.2 references, and deployment index links mention gradual rollout stages, gates, and rollback.
