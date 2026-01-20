# StellarRoute Findings & Research

**Purpose:** Store research discoveries, technical findings, and important information gathered during development.

---

## Stellar/Soroban Research

### Stellar Horizon API
- **Status:** In progress
- **Notes:** We confirmed the Offers resource page and endpoints via Stellar Docs snapshot.
- **Confirmed endpoints (Horizon):**
  - `GET /offers`
  - `GET /offers/:offer_id`
  - `GET /offers/:offer_id/trades`
  - Source: Stellar Docs “Offers” resource page (snapshot captured in-session)
- **Orderbook endpoint:** Still needs confirmation from Stellar Docs (search page wasn’t returning the actual endpoint page in our browser session).

### Soroban Development
- **Status:** Need to research
- **Notes:** 
  - Soroban is Stellar's smart contract platform
  - Uses Rust SDK
  - Need to understand AMM contract interfaces
- **Links:** 
  - https://developers.stellar.org/docs/build/smart-contracts/overview
  - https://developers.stellar.org/docs/tools/sdks/contract-sdks

---

## Technology Stack Decisions

### Backend Framework
- **Candidate:** Axum or Actix-web
- **Decision:** Pending research
- **Reasoning:** Need to evaluate performance, ecosystem, and Rust async support

### Database ORM
- **Candidates:** sqlx or diesel
- **Decision:** Pending research
- **Reasoning:** Need to evaluate type safety, async support, and migration capabilities

---

## SDK/Library Discoveries

### Rust Stellar SDK
- **Status:** Need to verify
- **Package:** rust-stellar-sdk (verify actual package name)
- **Notes:** Need to find official Rust SDK for Stellar

---

## Key Insights

- Stellar uses WASM for smart contracts (Soroban)
- Need to support both SDEX (orderbook) and Soroban AMM pools
- Performance target: <500ms API latency

---

## Open Questions

1. What is the official Rust SDK for Stellar Horizon API?
2. What are the existing Soroban AMM contract interfaces?
3. What are the best practices for indexing Stellar orderbooks in real-time?

## Phase 1.2 (SDEX Indexer) Notes

### What we can implement immediately
- Use `reqwest` directly (no confirmed “official” Rust Horizon SDK yet) and model Horizon JSON responses with `serde`.
- Start indexing from `GET /offers` (confirmed), persist normalized offers to Postgres.
- Add an ingestion cursor/state table so we can later switch to paging/streaming safely.

### Pending confirmations
- Exact orderbook snapshot endpoint and query params (commonly `/order_book?...`) — must confirm from Stellar Horizon docs before implementing the final orderbook snapshot fetcher.

## Environment Setup Notes

### Rust Installation
- **Issue:** SSL connection error when attempting automated Rust installation
- **Resolution:** Need manual Rust installation or verify network connectivity
- **Manual Installation Command:** `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **After Installation:** Run `rustup target add wasm32-unknown-unknown` for Soroban support

### Soroban CLI Installation
- **Issue:** Homebrew tap `brew install stellar/soroban/soroban` fails with "Repository not found" error
- **Error:** `fatal: repository 'https://github.com/stellar/homebrew-soroban/' not found`
- **Root Cause:** The Homebrew tap repository doesn't exist or has been moved
- **Resolution:** Use alternative installation methods:
  1. **Via Cargo:** `cargo install --locked soroban-cli` (recommended)
  2. **Via Installer Script:** `curl -sSfL https://github.com/stellar/soroban-tools/releases/latest/download/soroban-install.sh | sh`
  3. **Manual Binary:** Download from [Soroban Tools Releases](https://github.com/stellar/soroban-tools/releases)
- **Status:** Updated SETUP.md with correct installation instructions

---

## Notes

- Update this file after every research/discovery session
- Include links and references
- Note important technical details
