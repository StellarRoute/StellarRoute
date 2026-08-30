# Bug Bounty Scope — StellarRoute (Draft)

**Version:** 0.1 (draft — not yet active)  
**Milestone:** M5 — Security  
**Issue:** #999  
**Status:** Draft. This program is **not yet live**. The scope and rewards
below are subject to change before the program opens. Researchers should
not submit reports until the program is formally announced.

---

## Safe Harbor

StellarRoute maintainers commit to the following for security researchers
acting in good faith under this policy:

- We will not pursue civil or criminal action against researchers who
  discover and responsibly disclose vulnerabilities in scope.
- We will not issue DMCA takedowns for research conducted under this policy.
- We consider good-faith security research to be authorised access for the
  purposes of the Computer Fraud and Abuse Act and equivalent laws.

**Good-faith conditions:**

- Research must be limited to testnet. Do not perform testing against
  mainnet contracts or production API endpoints unless explicitly authorised
  in writing by the maintainers.
- Do not exfiltrate, modify, or destroy production data.
- Do not perform Denial-of-Service attacks against production or shared
  staging infrastructure.
- Do not disclose findings publicly until the maintainers confirm the issue
  is resolved or agree to a coordinated disclosure timeline (maximum 90 days
  from initial report).
- Do not use findings for personal financial gain (e.g., frontrunning or
  exploiting vulnerabilities before disclosure).

---

## How to Report

1. Email **security@[domain TBD]** or open a [GitHub Security Advisory](https://github.com/StellarRoute/StellarRoute/security/advisories/new) (private disclosure).
2. Include:
   - Affected component and version/commit.
   - Clear proof-of-concept or reproduction steps.
   - Impact assessment (what can an attacker achieve?).
   - Suggested mitigation (optional but appreciated).
3. You will receive an acknowledgement within **2 business days**.
4. We aim to triage and assign a severity within **5 business days**.
5. Patches will be released according to severity (see Reward Tiers).

---

## In Scope

### Soroban Smart Contracts (`crates/contracts/`)

The deployed Soroban router contract on **Stellar testnet** is the
highest-priority target.

| Area | Examples |
|---|---|
| Access control bypass | Calling admin functions without being the registered admin |
| Fund loss or misrouting | A route that causes the contract to send tokens to the wrong address or lose funds |
| Initialization re-entrancy | Calling `initialize()` more than once |
| Storage manipulation | Reading or writing contract storage via unexpected paths |
| Fee bypass | Executing a swap without paying the configured fee |
| Emergency pause bypass | Executing a swap when the contract is paused |
| Arithmetic errors | Overflow/underflow in fee or price-impact calculations |

### API Server (`crates/api/`)

The REST API hosted at the public testnet endpoint.

| Area | Examples |
|---|---|
| Authentication bypass | Accessing any `/api/v1/admin/*` endpoint without a valid admin token |
| Privilege escalation | Using a regular API key to perform admin actions |
| Injection | SQL injection, command injection, or SSRF via any API parameter |
| Information disclosure | Reading other users' data or internal system secrets via API responses |
| Kill switch manipulation | Remotely triggering the kill switch without credentials |

### Quote Integrity

| Area | Examples |
|---|---|
| Quote spoofing | Causing the API to return a manipulated quote that systematically favours an attacker |
| Cache poisoning | Injecting a malicious value into the Redis cache that persists beyond one TTL |
| Freshness bypass | Causing stale market data to be served without the `StaleMarketData` error being raised |

---

## Out of Scope

The following are **explicitly out of scope** and reports about them will
not be rewarded:

- Mainnet contract addresses (testing on mainnet is prohibited).
- Stellar network-level issues (report these to the Stellar Development Foundation).
- Denial-of-Service attacks against production or shared infrastructure.
- Social engineering, phishing, or physical attacks against maintainers.
- Issues requiring physical access to a server.
- Vulnerabilities in third-party dependencies that are already publicly
  known and tracked (see `audit/known-issues.md`).
- Missing security headers (Content-Security-Policy, etc.) on the frontend —
  these are noted but do not qualify for a bounty at this stage.
- Self-XSS (attacks that require the victim to execute JavaScript in their
  own browser console).
- Rate-limiting bypass that does not enable a concrete attack beyond
  resource consumption.
- Theoretical vulnerabilities without a proof-of-concept.

---

## Reward Tiers

Rewards are paid in USDC on Stellar testnet during the test phase; mainnet
USDC once the program is live. Amounts are approximate and subject to
severity assessment by the maintainers.

| Tier | Severity | Examples | Reward (USD equiv.) |
|---|---|---|---|
| Critical | CVSS 9.0–10.0 | Fund loss via contract exploit; complete admin auth bypass with demonstrated impact | $2,000 – $10,000 |
| High | CVSS 7.0–8.9 | Privilege escalation; persistent cache poisoning; fee bypass | $500 – $2,000 |
| Medium | CVSS 4.0–6.9 | Information disclosure of operator config; quote spoofing bounded by slippage; temporary DoS | $100 – $500 |
| Low | CVSS 0.1–3.9 | Minor information disclosure; non-exploitable logic issue | $25 – $100 |
| Informational | N/A | Hardening suggestions, documentation gaps that create risk | Acknowledgement only |

**Duplicate policy:** The first report of a valid issue receives the full
reward. Subsequent duplicate reports receive acknowledgement only.

**Reward adjustments:** The maintainers reserve the right to adjust rewards
based on exploitability, quality of the report, and whether a fix was
suggested.

---

## Program Status

| Environment | Status |
|---|---|
| Testnet API | Not yet deployed |
| Testnet Contracts | Not yet deployed |
| Mainnet | **Out of scope** until further notice |

This document will be updated and the program announced through the
[GitHub Discussions](https://github.com/StellarRoute/StellarRoute/discussions)
page when it goes live.
