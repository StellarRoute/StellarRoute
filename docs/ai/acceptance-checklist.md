# AI Agent Acceptance Checklist (AI-01 → AI-48, frozen surfaces)

Last agent issue before the card track (AI-49, closes #1459). Pass/fail list only — no code changes in this file's PR.

## 1. Flags default off

- [ ] `AI_AGENT_ENABLED` unset/false → `POST /api/v1/agent/intents/validate` returns 404 (`crates/api/src/routes/agent.rs`)
- [ ] `NEXT_PUBLIC_AI_AGENT` unset/false → `/ai` shows disabled state, no composer side effects
- [ ] No `NEXT_PUBLIC_FLAG_AI_AGENT` invented in code, `.env.example`, or Vercel env (canonical: `NEXT_PUBLIC_AI_AGENT`)
- [ ] Production Vercel env untouched; preview-only vars stay on preview deployments

## 2. 404 when off

- [ ] New agent routes return 404 when flags unset/false (validate, tools, runner)
- [ ] Bridge routes return 404/503 when `CCTP_ENABLED` false/unset
- [ ] With flags unset: no user-visible difference on `/swap`, `/offramp`, `/cross-chain-swap`

## 3. Confirm before sign

- [ ] Every spend requires explicit user review + Confirm before wallet signing
- [ ] Unknown/ambiguous text never becomes a spend (rejected or informational chat)
- [ ] Preview cards never auto-broadcast; Cancel discards with zero wallet involvement
- [ ] Subscribe registers locally only; each installment needs per-payment approval

## 4. No swap.rs or quote ranking edits

- [ ] `crates/api/src/routes/swap.rs` behavior unchanged (classic one-hop SDEX prepare → wallet sign → submit)
- [ ] `crates/api/src/routes/quote.rs` ranking path unchanged; agent never overrides venue selection
- [ ] Wallet connect/sign adapters for the live swap CTA unchanged
- [ ] Existing OpenAPI field names/types and error codes unchanged; CORS allowlists unchanged; `real_xdr` pinning unchanged

## 5. CCTP default unchanged

- [ ] `CCTP_ENABLED` defaults false; enabling also needs HMAC key + Sepolia RPC
- [ ] Bridge previews are informational/copy only; no burn/mint broadcast from agent

## 6. Six tools handoff / local behavior

| Tool | Issues | Expected behavior |
|------|--------|-------------------|
| convert | AI-14 #1424, AI-15 | Read-only quote preview; Confirm deep-links `/swap?from&to&amount`; no POST to swap prepare |
| send | AI-07 #1417, AI-16 #1426, AI-17 #1427 | Unsigned preview; Cancel clears card, emits `agent_cancel`, no signing |
| receive | AI-18 #1428, AI-19 #1429 | Address + QR display; copy works; no transaction |
| bridge | AI-20 #1430, AI-21 #1431 | Routing info + manual copy; no burn/mint |
| offramp | AI-22 #1432, AI-23 #1433 | Intent preview; Confirm deep-links `/offramp`; no payout order |
| subscribe | AI-24 #1434 … AI-27 #1437 | Local schedule; per-cycle approval; never auto-signs |
| runner | AI-29 #1439 | Sequential steps with per-step gates; rejecting step 1 aborts rest |

## 7. Contract tests green unchanged

- [ ] Swap / quote / OpenAPI contract tests pass without edits
- [ ] Lean CI (clippy, lib tests) unaffected

## Non-goals

- Key custody: StellarRoute never holds private keys, seeds, or card PANs
- Automatic subscription pulls: no background auto-execution of recurring payments

## Production files touched by this PR

- `docs/ai/acceptance-checklist.md` (new) — checklist only, why: closes #1459 with zero production behavior change
