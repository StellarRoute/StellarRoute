# AI Agent Product Contract

## 1. Principles and Non-Custodial Architecture

StellarRoute's AI agent provides a conversational and programmatic assistant layer on top of existing DEX aggregation, bridging, and offramp capabilities. The architecture is strictly non-custodial and fail-closed:

1. **Agent Proposes Intents**: The agent only parses user intent and produces structured, typed intent proposals.
2. **User Confirms**: The user must explicitly inspect and approve all proposed parameters (asset, amount, recipient/payee, fees, slippage) in the user interface before any transaction payload is constructed.
3. **Wallet Signs**: The user's external, non-custodial wallet (Freighter, xBull, Albedo, LOBSTR for Stellar; MetaMask/WalletConnect for EVM destination) signs the transaction. StellarRoute never holds private keys, seed phrases, or card PANs.
4. **Confirm-Before-Sign**: No automated or implicit execution occurs. Every spend requires explicit user review and confirmation before wallet signing.
5. **Unknown Text Never Becomes a Spend**: Unrecognized, ambiguous, or conversational prompts are never coerced into financial transactions. If input fails schema validation or cannot be deterministically resolved to a supported intent, it is rejected or treated as informational chat with zero wallet involvement.
6. **Vendor-Agnostic**: The system has no coupling to any specific LLM vendor. Intent parsing operates over clear schemas without vendor-specific execution hooks.
7. **No Changes to Quote Ranking**: The agent never influences or overrides the existing quote ranking, best-executable venue selection, or routing engine algorithms (`crates/routing`, `crates/api/src/routes/quote.rs`).

---

## 2. V1 Tools, Underlying Modules, and Feature Flags

All agent tools sit on top of existing production services and are gated behind default-off feature flags.

| Tool | Purpose | Existing Underlying Module | Feature Flags Keeping It Off |
|---|---|---|---|
| `convert` | Swap between Stellar assets (SDEX & Soroban AMM) | `crates/api/src/routes/quote.rs`, `crates/api/src/routes/swap.rs`, `frontend/lib/swap/` | `AI_AGENT_ENABLED` (backend, default false)<br>`ai_agent` (frontend, default false) |
| `send` | Transfer Stellar assets to a destination address | Horizon transaction builder / `submit_swap`, `frontend/lib/wallet/` | `AI_AGENT_ENABLED` (backend, default false)<br>`ai_agent` (frontend, default false) |
| `receive` | Display account address, generate QR code or payment request | Client-side wallet address hooks (`useWallet`) | `AI_AGENT_ENABLED` (backend, default false)<br>`ai_agent` (frontend, default false) |
| `bridge` | Cross-chain transfer between Stellar and EVM via Circle CCTP | `crates/api/src/cctp/`, `/api/v2/bridge/cctp/*`, `frontend/lib/cctp/` | `CCTP_ENABLED` (backend, fail-closed default false)<br>`AI_AGENT_ENABLED` (backend, default false)<br>`ai_agent` (frontend, default false) |
| `offramp` | Cash out crypto (e.g. USDC) to Nigerian Naira (NGN) | `frontend/lib/offramp/`, Paycrest offramp corridor (`/offramp`) | `AI_AGENT_ENABLED` (backend, default false)<br>`ai_agent` (frontend, default false) |
| `subscribe` | Set up recurring subscription payments | Client-side intent scheduler / recurring payment allowance manager | `AI_AGENT_ENABLED` (backend, default false)<br>`ai_agent` (frontend, default false) |

---

## 3. Corridor and Integration Boundaries

### Circle CCTP
- Cross-chain bridge operations remain strictly fail-closed behind the `CCTP_ENABLED` environment variable.
- When `CCTP_ENABLED` is false or unset, bridge routes return 404/503 and the agent cannot propose executable bridge transactions.

### NGN Offramp
- Offramp quotes remain indicative unless the user explicitly initiates the verified offramp flow via the existing `/offramp` route.
- Offramp executes via Paycrest corridors; StellarRoute does not custody fiat, bank credentials, or intermediary deposits.

---

## 4. Safety and Additive Guarantees

- **Default-off**: The AI agent is completely disabled in production unless `AI_AGENT_ENABLED=true` (API) and `NEXT_PUBLIC_FLAG_AI_AGENT=true` (frontend) are explicitly configured.
- **Fail-closed**: All new endpoints return 404 when disabled.
- **Additive-only**: Live user flows on `/swap`, `/offramp`, and `/cross-chain-swap` remain identical and untouched.
