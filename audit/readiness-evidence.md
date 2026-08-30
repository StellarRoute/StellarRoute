# External Audit Package — Readiness Evidence

Separates **materials we can truthfully prepare in-repo** from **stages that
require humans / a third-party auditor**. A software agent must not mark
auditor selection, fieldwork, or the final report complete without evidence.

## Current product execution scope (launch-adjacent, not Soroban audit scope)

| Item | Status |
|---|---|
| Classic one-hop SDEX `PathPaymentStrictSend` | Current live swap execution scope |
| Soroban / AMM / multi-hop prepare | **Unsupported** (`unsupported_execution_mode` / `unsupported_route`) |
| Placeholder prepare envelopes | **Not acceptable** — prepare must return a real unsigned XDR |

## Ready to hand an auditor (contract documentation package)

| Artifact | Path |
|---|---|
| Scope | [`scope.md`](scope.md) |
| Assumptions | [`assumptions.md`](assumptions.md) |
| Architecture | [`architecture.md`](architecture.md) |
| Threat model | [`threat-model.md`](threat-model.md) |
| Known issues | [`known-issues.md`](known-issues.md) |
| Internal review checklist | [`internal-security-review.md`](internal-security-review.md) |
| Fuzz runbook | [`fuzzing.md`](fuzzing.md) |
| Engagement tracker | [`external-audit.md`](external-audit.md) — stage 3 drafted only |

## Related off-contract surfaces

| Surface | Where |
|---|---|
| Classic swap HTTP contract | `POST /api/v1/swap/prepare`, `/submit` |
| JS SDK | `prepareSwap` / `submitSwap` / `executeSwap` / `confirmSwap` |
| Frontend Freighter path | `frontend/lib/swap/api-execution.ts` (`real_xdr`) |
| Testnet evidence | `docs/readiness/live-swap-testnet-checklist.md` + `scripts/live-swap-api-smoke.mjs` |
| CCTP Stellar → Sepolia signed-live | `docs/cctp/signed-live-stellar-to-sepolia.md` + `docs/readiness/evidence/cctp-signed-live-stellar-to-sepolia-2026-08-14.json` (mint `0x713cc8b1…bed6`) |
| CCTP Sepolia → Stellar signed-live | `docs/cctp/signed-live-sepolia-to-stellar.md` + `docs/readiness/evidence/cctp-signed-live-sepolia-to-stellar-2026-08-14.json` (burn `0x339b96cc…`, mint `13d2025d…`) |

## Explicitly NOT complete

| Stage | Why |
|---|---|
| Auditor selection | Quotes / SOW / commercial decision |
| Commit / hash freeze | Requires engaged auditor + freeze SHA + WASM hash |
| Audit fieldwork | Third-party activity |
| Findings remediation verification | Auditor sign-off |
| Final report publication | Auditor deliverable |

Update [`external-audit.md`](external-audit.md) only when corresponding human
evidence exists.
