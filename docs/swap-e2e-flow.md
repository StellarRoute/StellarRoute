**End-to-end Swap UX Flow Spec**

Purpose: Definitive UX flow for a swap from quoting through confirmation and status tracking. Covers happy path and key failure branches, loading/stale-quote/retry states, front-end handoff notes, and reviewer sign-off.

**Scope**: web/trading UI swap flow (desktop & mobile responsive)

**High-level flow (Mermaid)**
```mermaid
flowchart TD
  A[Start: User opens Swap modal] --> B[Select base & quote tokens]
  B --> C[Request quote]
  C --> D{Quote received?}
  D -- Yes --> E[Show quote summary & price impact]
  D -- No --> F[Show error & retry option]
  E --> G[User reviews & sets amount]
  G --> H[Re-quote (optional) / Validate balances]
  H --> I{Quote fresh?}
  I -- Yes --> J[Enable Confirm button]
  I -- No --> K[Show stale-quote warning + Update quote button]
  J --> L[User taps Confirm]
  L --> M[Show loading: submitting swap]
  M --> N{Submission success?}
  N -- Yes --> O[Show pending status + tx id]
  N -- No --> P[Show submission error + retry/cancel]
  O --> Q[Poll tx status]
  Q --> R{Finalized?}
  R -- Success --> S[Success screen: details + share/tx link]
  R -- Failed --> T[Failure screen: reason, suggestions]

  %% Failure branches
  F --> U[Network error / RPC down]
  P --> V[Insufficient fee / bad signature / slippage]
  T --> W[Offer refund guidance / contact support]
```

**Detailed states & UX notes**
- **Token selection**: autosuggest tokens, show balances & trusted status. Disable tokens with zero balance when a transfer is required (but allow if user wants to receive).
- **Quote request**: optimistic UX — show skeleton card while requesting. If request >750ms show progress dot + "fetching best route" message.
- **Quote received**: display `amountOut`, `priceImpact`, `networkFeeEstimate`, `routingBreakdown` (if multi-hop), `ttl` (quote expiry in seconds). Show a small `last updated` timestamp.
- **Stale quote handling**: quotes include `expires_at` and a `quote_id` with server nonce. If `now > expires_at` or re-quote changes >0.5% price impact, show a modal/badge: "Quote expired or changed — update quote" with `Update quote` button. Disable `Confirm` until updated.
- **Loading & submission**:
  - Confirmation click shows an animated progress state and immediately disables UI to prevent double submits.
  - Provide an explicit cancel/back control while submission in-flight where possible (i.e., if transaction not yet signed/submitted).
- **Retry states**:
  - Re-quote retry: exponential backoff up to 3 attempts, show attempt count and final failure copy "Unable to fetch quote — try again later".
  - Submission retry: allow re-submit for transient errors (network timeouts) but detect duplicate tx risk and warn users if duplicate signing may create multiple on-chain actions.
- **Failure branches**:
  - Insufficient balance: show quick action to deposit or swap a different amount; include links to deposit guides.
  - High slippage: suggest increasing allowed slippage or reduce amount; show estimated worst-case outcome for user's chosen slippage.
  - RPC/Horizon down: show global banner and disable quoting/confirm actions with retry.
- **Status tracking**:
  - After submission show a `pending` card with tx hash link, estimated time, and stepper (submitted → in-ledger → confirmed).
  - Polling cadence: 2s for first 30s, 5s for up to 3 minutes, then 30s until finalized or timeout (10 minutes). Allow re-check button.
  - Provide notification hooks (browser notifications / in-app toast) when tx finalizes.

**Frontend handoff notes (Annotation for implementers)**
- API contract: `/quote` returns { quote_id, amountOut, amountIn, priceImpactPct, expires_at, routes[], feeEstimate, last_updated }
- Confirm flow requires two steps: 1) `POST /swap/prepare` -> returns unsigned payload, 2) Client signs and `POST /swap/submit` with signature and `quote_id`.
- Ensure idempotency: `swap/submit` should reject duplicate `quote_id` submissions unless user explicitly requests re-submit; frontend must warn about duplicate signs.
- UI should surface these flags from the API: `is_stale`, `requires_slippage_confirmation`, `insufficient_balance`, `estimated_time_seconds`.
- Accessibility: ensure all status transitions are announced to screen readers; use `aria-live` for pending→finalized updates.

**Design tokens & microcopy**
- Primary actions: `Confirm` (primary), `Update quote` (secondary), `Retry` (link style when inline). Keep copy concise.
- Error copy examples: "Quote expired — update the quote to continue." "Submission failed: insufficient fee — try increasing fee or funding account."

**Review & Sign-off checklist**
- [ ] UX review by product/design
- [ ] Engineering review (frontend) — @maintainers
- [ ] Security review for re-entrancy and double-submit edge cases
- [ ] Add to Stellar Wave program & Drips Wave with complexity=High

**Wave / Ticket notes**
- Add to Stellar Wave program. Set `complexity=High` in Drips Wave tracking metadata. Include link to this doc and issue #296.

**Maintenance & follow-ups for implementers**
- Instrument analytics events: `quote_requested`, `quote_updated`, `confirm_clicked`, `swap_submitted`, `swap_finalized`, `swap_failed` with minimal payload (quote_id, route_id, amountIn, amountOut, priceImpact).
- Add e2e tests for: successful swap, expired quote flow, submission failure + retry, insufficient balance path.

**Contacts**
- Maintainer: @StellarRoute/maintainers
- Product: please tag product owner when requesting UX sign-off.

End of spec.
