# External Soroban Contract Audit — Engagement & Remediation Tracking

**Milestone:** M5 — Audit. The external audit is a **launch gate**: the
mainnet flag is not flipped and real user funds are not accepted until the
audit is complete, all Critical/High findings are remediated, and the final
report is published.

This document tracks the engagement end to end: auditor selection, the
frozen audit commit, published scope and assumptions, findings remediation,
re-audit, and report publication.

## Status

| Stage | Status |
|---|---|
| 1. Auditor selection | ☐ Not started |
| 2. Commit / hash freeze | ☐ Blocked on stage 1 |
| 3. Scope + assumptions published | ☑ Drafted in this directory (see below) |
| 4. Audit fieldwork | ☐ Blocked on stage 2 |
| 5. Findings remediation | ☐ Blocked on stage 4 |
| 6. Re-audit (if required) | ☐ Blocked on stage 5 |
| 7. Report published | ☐ Blocked on stage 6 |

Update this table (and the sections below) as the engagement progresses;
every stage change should reference the PR or issue that advanced it.

## 1. Auditor selection

Selection criteria, in priority order:

1. **Soroban/Stellar experience** — prior audits of Soroban contracts (not
   just EVM); familiarity with Soroban storage TTLs, `require_auth()`
   semantics, and cross-contract call budgets.
2. **DEX/AMM domain expertise** — routing, slippage, fee accounting, and
   MEV-adjacent attack surfaces.
3. **Published track record** — public reports we can review for depth.
4. **Re-audit terms** — fixed-price or included verification pass for
   remediated findings.

Candidate firms to solicit quotes from (Soroban Security Audit Bank /
SDF-recognized auditors): Veridise, OtterSec, Certora, CoinFabrik, Runtime
Verification. Record quotes, timelines, and the decision rationale here
before signing.

| Firm | Quote received | Timeline | Notes | Decision |
|---|---|---|---|---|
| _pending_ | | | | |

## 2. Commit / hash freeze

Once the auditor is engaged, the audit target is frozen and recorded here.
No changes to `crates/contracts/` land on `main` during fieldwork except
fixes the auditor requests; anything else waits in feature branches.

| Field | Value |
|---|---|
| Frozen git commit (full SHA) | _pending engagement_ |
| Git tag | `audit-freeze-YYYYMMDD` (create with `git tag -s`) |
| Router WASM SHA-256 | _pending_ |
| Soroban SDK version | `21.0.0` |
| Rust toolchain | pinned via `rust-toolchain` / CI |

Reproduce the frozen artifact hash:

```bash
git checkout <frozen-commit>
cd crates/contracts
cargo build --release --target wasm32-unknown-unknown
sha256sum ../../target/wasm32-unknown-unknown/release/stellarroute_contracts.wasm
```

The nightly `verify-contracts.yml` workflow already rebuilds from source and
compares the deployed bytecode hash; after the freeze, the frozen hash
recorded above must match the deployed testnet contract for the duration of
the audit.

## 3. Scope and assumptions

The scope and assumptions are published in this directory and are handed to
the auditor as-is:

- [`scope.md`](scope.md) — in-scope files and functions (all of
  `crates/contracts/src/`; indexer/API/frontend are out of scope for the
  contract audit but covered by the internal review).
- [`assumptions.md`](assumptions.md) — runtime and operational assumptions,
  trust boundaries.
- [`architecture.md`](architecture.md) — contract architecture and threat
  model overview; deeper scenarios in [`threat-model.md`](threat-model.md).
- [`known-issues.md`](known-issues.md) — accepted risks the auditor should
  not re-report unless the risk assessment changes.
- Internal pre-audit review: `internal-security-review.md` (issue #994) must
  be complete with no open Critical/High findings **before** fieldwork
  starts.

Any scope change during the engagement is recorded as a dated amendment in
this section.

## 4. Findings & remediation tracking

Every finding from the external report gets a row here **and** a linked
GitHub issue labelled `security` + `audit-finding`. Findings are remediated
on `main` via normal PR review; the fix PR is linked in the row.

| Finding | Severity | GitHub issue | Fix PR | Status | Auditor verified |
|---|---|---|---|---|---|
| _pending report_ | | | | | |

Remediation policy:

- **Critical / High** — must be fixed and auditor-verified before mainnet.
  A re-audit (or at minimum a fix-verification pass) is mandatory if any
  Critical finding required non-trivial code change.
- **Medium** — fixed before mainnet, or explicitly accepted with a written
  rationale added to [`known-issues.md`](known-issues.md).
- **Low / Informational** — triaged into the backlog; acceptance rationale
  recorded in [`known-issues.md`](known-issues.md) if not fixed.

## 5. Re-audit

If required by the remediation policy above, the re-audit targets a new
frozen commit (recorded as a second row in the freeze table) containing only
the remediation changes. The re-audit outcome is appended to the findings
table (auditor-verified column).

## 6. Report publication

When the engagement closes:

1. Commit the final report (PDF) to `audit/reports/` — e.g.
   `audit/reports/2026-<firm>-router-audit.pdf` — or link the auditor's
   canonical published URL if they host it.
2. Add the report link to the table below, to [`README.md`](README.md)
   (this directory), and to the **Security & Audits** section of the
   top-level project `README.md`.
3. Announce via GitHub Discussions / release notes.

| Report | Auditor | Frozen commit | Link |
|---|---|---|---|
| _pending_ | | | |

## Launch gate summary

The mainnet flag flip requires all of:

- [ ] Audit complete against the frozen commit
- [ ] All Critical/High findings remediated and auditor-verified
- [ ] Medium findings fixed or formally accepted in `known-issues.md`
- [ ] Re-audit passed (if triggered by the remediation policy)
- [ ] Final report published and linked from the top-level README
