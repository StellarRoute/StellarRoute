# Threat Model: Quote Manipulation and Sandwich Risks

**Version:** 1.0  
**Milestone:** M5 — Security  
**Issue:** #996  
**Last updated:** 2026-07-26

---

## 1. Scope

This document covers off-chain risks to the StellarRoute API quote pipeline
and on-chain execution risks for users acting on returned quotes. It does
**not** re-cover Soroban contract security (see `audit/scope.md`) or
infrastructure/supply-chain risks (see `audit/known-issues.md`).

---

## 2. System Summary

The quote pipeline works as follows:

1. **Indexer** polls Stellar Horizon and Soroban RPCs, writing orderbook and AMM
   pool state into Postgres (`sdex_offers`, `amm_pool_reserves`).
2. **API** reads the `normalized_liquidity` view, applies freshness/health
   filters, selects the best-price venue, and returns a signed quote response.
3. **Client (browser/SDK)** displays the quote and optionally submits a swap
   transaction to the Stellar network with a user-chosen slippage tolerance.
4. **Soroban router contract** may be invoked to execute the swap on-chain.

The API does **not** execute trades. It is a read/routing service. Execution
risk is borne by the client and resolved on-chain.

---

## 3. Attacker Goals and Threat Scenarios

### T-1 — Stale Quote Exploitation

**Attacker goal:** Cause users to receive and act on a quote that no longer
reflects current market prices, resulting in worse execution.

**Attack vector:** The indexer has finite polling latency (typically 5–30 s for
SDEX, longer for Soroban depending on RPC). An attacker who can rapidly move
an orderbook (e.g., by placing and cancelling large orders) can widen the
gap between the cached quote and live on-chain state within the quote TTL
window.

**Severity:** Medium. Impact is bounded by slippage tolerance on the client
side. The attacker can only profit if the user executes immediately after
receiving a stale quote without re-checking.

**Mitigations in code/config:**

| Mitigation | Location |
|---|---|
| Quote TTL of 2 s with jittered expiry (`cache_policy.quote_ttl`) | `crates/api/src/routes/quote.rs` L677–684 |
| `FreshnessGuard` rejects venues with stale data (`QUOTE_MAX_AGE_SECS`) | `crates/api/src/routes/quote.rs` L653; `crates/routing/src/health/freshness.rs` |
| `StaleMarketData` error returned when stale > 0 venues remain | `crates/api/src/routes/quote.rs` L542–550 |
| `stale_count` / `fresh_count` in response `exclusion_diagnostics` | quote response model |
| Indexer lag monitoring and alerts | `docs/indexer-lag-monitoring.md` |

**Residual risk:** If the indexer falls behind by more than `QUOTE_MAX_AGE_SECS`
(configurable, default 30 s), the freshness guard will exclude venues and may
return no quote rather than a stale one. Clients should display quote age and
prompt for refresh before submission.

---

### T-2 — Sandwich Attack (Mempool Front-running)

**Attacker goal:** Observe a pending swap transaction in Stellar's mempool,
insert a buy order before it (driving up price), let the victim's swap
execute at the worse price, then immediately sell (extracting the price
difference — MEV).

**Attack vector:** Stellar's Horizon API makes pending transactions visible.
An attacker running a monitoring bot can see a swap about to execute for a
large amount, submit a competing order with higher sequence priority, and
profit from the resulting price movement.

**Severity:** Low–Medium on SDEX (order matching is fee-prioritised, not
strict mempool ordering; SDEX clears in ledger batches which limits
predictable front-running compared to EVM); Higher risk for thin AMM pools
where a single swap materially moves the price.

**Mitigations in code/config:**

| Mitigation | Location |
|---|---|
| `slippage_bps` query param enforces client-side `min_amount_out` | `crates/api/src/routes/quote.rs` L57, L180 |
| Default slippage is 50 bps (configurable by user) | `crates/api/src/routes/quote.rs` L180 |
| Price impact calculated and returned in quote response | quote response `price_impact_bps` field |
| Kill switch can exclude venues with anomalous spread | `crates/api/src/routes/kill_switch.rs` |
| Canary evaluations detect abnormal routing health | `crates/api/src/routes/canary.rs` |

**Residual risk:** StellarRoute cannot prevent sandwiching at the network
layer. Users should:
- Set conservative slippage tolerances (≤ 50 bps for liquid pairs).
- Avoid large single-transaction swaps on thin AMM pools.
- Use the `price_impact_bps` field to warn before submitting high-impact trades.

The frontend should warn users when `price_impact_bps > 100` and block
submission when it exceeds a configurable threshold (e.g., 500 bps).

---

### T-3 — Quote Manipulation via Orderbook Spoofing

**Attacker goal:** Place large spoofed limit orders that the indexer ingests,
making the API return an artificially favourable quote. When the user submits
the swap, the spoof orders are cancelled and the trade executes at a worse
price.

**Attack vector:** A market maker creates large orders at a beneficial price,
waits for indexer polling to ingest them, and then cancels the orders as soon
as the victim's swap transaction enters the mempool.

**Severity:** Medium. Stellar's SDEX requires a small XLM reserve per open
offer, which imposes a cost on spoofers, limiting the practicality of
large-scale spoofing. AMM pools are less susceptible because reserves are
locked on-chain.

**Mitigations in code/config:**

| Mitigation | Location |
|---|---|
| Venue health scorer down-weights offers with anomalous spread or low liquidity | `crates/routing/src/health/` |
| `min_amount_out` in swap execution enforces price guarantee | client-side / Soroban contract |
| Anomaly detection in routing engine flags statistical outliers | `crates/routing/src/health/anomaly.rs` |
| Short quote TTL (2 s) limits window between quote receipt and submission | `crates/api/src/routes/quote.rs` |

**Residual risk:** An adversary with enough XLM reserves to maintain spoof
orders through two polling cycles can still trick the indexer. Multi-source
consensus (SDEX + AMM cross-check) partially mitigates this; single-venue
large-spread quotes are already penalised by the health scorer.

---

### T-4 — Griefing via Kill Switch Abuse (Admin Route)

**Attacker goal:** An attacker who obtains the `ADMIN_AUTH_TOKEN` can trigger
the kill switch to disable all AMM or SDEX sources, causing quote degradation
or outages for all users.

**Attack vector:** Leaked admin token (env var, CI secret, or log leakage)
allows unauthorized `POST /api/v1/admin/kill-switch` calls.

**Severity:** High if admin token is compromised. Impact is service-level
(routing degrades or fails), not financial.

**Mitigations in code/config:**

| Mitigation | Location |
|---|---|
| `AdminAuth` extractor required for all mutating admin routes | `crates/api/src/middleware/admin.rs` |
| API refuses to start in production without `ADMIN_AUTH_TOKEN` configured | `crates/api/src/state.rs` |
| Admin actions logged to structured audit log | `crates/api/src/admin_audit.rs` |
| Key rotation runbook available | `docs/key_rotation.md` |

**Residual risk:** Admin token is a single shared secret. Future work should
replace it with short-lived signed tokens or mTLS for admin routes. Token
rotation procedure must be followed on any suspected compromise.

---

### T-5 — API Denial-of-Service via Quote Flooding

**Attacker goal:** Exhaust API rate limits or database connections by
flooding `/api/v1/quote` with requests, denying service to legitimate users.

**Attack vector:** Unauthenticated or API-key-authenticated clients sending
high-frequency quote requests, particularly for obscure pairs that bypass the
Redis cache.

**Severity:** Medium. Mitigated by rate limiting and Redis caching.

**Mitigations in code/config:**

| Mitigation | Location |
|---|---|
| Per-IP rate limiting (100 req/min default) | `crates/api/src/middleware/rate_limit.rs` |
| Redis cache with 2 s TTL absorbs repeated identical requests | `crates/api/src/routes/quote.rs` L673–684 |
| Database pool guardrails (connection limits, statement timeouts) | `docs/deployment/db-pool-tuning.md` |
| Health endpoint excluded from rate limits for monitoring | `crates/api/src/middleware/auth.rs` |

**Residual risk:** The current rate limiter is per-IP; distributed attacks
from multiple IPs are not mitigated at the application layer. Infrastructure-
level rate limiting (WAF, CDN) is recommended for production deployments.

---

## 4. Out-of-Scope Risks

The following are noted but not analysed in this document:

- **Soroban contract bugs:** Covered in `audit/scope.md` and the dedicated
  smart-contract audit.
- **Stellar network-level attacks:** Outside StellarRoute's threat surface.
- **User wallet key compromise:** User responsibility; StellarRoute never
  handles private keys.
- **Database exfiltration:** Infrastructure / deployment security concern.
- **Supply-chain (dependency CVEs):** Addressed via the dependency audit gate
  (see `#997`).

---

## 5. Residual Risk Summary for Users

| Risk | Residual Exposure | User Action Required |
|---|---|---|
| Stale quote execution | Quote may be ≤ 2 s stale | Check `expires_at` before submitting; re-fetch if near expiry |
| Sandwich attack | Bounded by `slippage_bps` | Use ≤ 50 bps for large trades; heed price-impact warnings |
| Orderbook spoofing | Cost-bounded by Stellar reserve requirement | Do not execute if quote price diverges significantly from market |
| Kill switch abuse | Token must be compromised first | Report anomalous outages immediately |
| DoS from flooding | Rate-limited per IP | Operators: add WAF/CDN layer in production |
