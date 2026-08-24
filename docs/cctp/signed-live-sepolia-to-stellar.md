# Signed-live proof: Ethereum Sepolia → Stellar Testnet (CCTP)

**Status:** Achieved (testnet)  
**Date:** 2026-08-14  
**Scope:** One-way USDC corridor via Circle CCTP v2 (`evm_to_stellar`) — testnet only. Public enablement stays fail-closed until staging checklist is completed.

This is the reverse of [`signed-live-stellar-to-sepolia.md`](./signed-live-stellar-to-sepolia.md). Product acceptance was a **full MetaMask (Sepolia) + Freighter (Stellar Testnet)** round-trip on `/swap` (cross-chain deck), including Fast finality and destination `mint_and_forward`.

## Canonical destination mint (Aug 14, 2026)

| Field | Value |
| --- | --- |
| Sepolia burn tx | [`0x339b96ccb6c3bcc0eb4c37d70fb5b8e6f3ee4c6fd1e7c032e93827faab6a5e73`](https://sepolia.etherscan.io/tx/0x339b96ccb6c3bcc0eb4c37d70fb5b8e6f3ee4c6fd1e7c032e93827faab6a5e73) |
| Sepolia block | `11490069` |
| Burn contract (TokenMessenger) | `0x8fe6b999dc680ccfdd5bf7eb0974218be2542daa` |
| Burn sender | `0xa632da1e4d5dd7fb236a0b798ff9331e3a9930df` |
| Stellar mint tx (`mint_and_forward`) | [`13d2025db39b461756954e1266864ea39c126cada55ddf24db9ec364138d16f2`](https://stellar.expert/explorer/testnet/tx/13d2025db39b461756954e1266864ea39c126cada55ddf24db9ec364138d16f2) |
| Mint timestamp (UTC) | `2026-08-14T22:22:29Z` |
| Amount | `5` USDC burn; destination credited `4.9995000` USDC (Fast `fee_executed` `0.0005`) |
| Finality | Fast (`minFinalityThreshold` 1000) |
| Mint recipient / fee payer (G) | `GBSTOZPBODWWNR4LIX56BH5IGGDHABGO43YGCZOSMVDVXMOZOHZIN5YI` |
| CCTP source domain | `0` (Ethereum Sepolia) |
| CCTP destination domain | `27` (Stellar) |
| Transfer id | `de61d422-9db3-4332-aa58-33043c7a5bdb` |

Sanitized public evidence JSON:

- [`docs/readiness/evidence/cctp-signed-live-sepolia-to-stellar-2026-08-14.json`](../readiness/evidence/cctp-signed-live-sepolia-to-stellar-2026-08-14.json)

## Acceptance path (operator)

1. Deploy/run API with CCTP deps (Sepolia RPC, Soroban RPC, Horizon, Iris sandbox, HMAC key). Keep `CCTP_ENABLED=true` only on the proof host.
2. Open `/swap` with cross-chain deck enabled (`swap_ui_v2`): source **Ethereum Sepolia**, destination **Stellar**.
3. Connect MetaMask (Sepolia USDC) + Freighter (recipient G).
4. Quote (Fast) → approve USDC → `depositForBurnWithHook` → wait Iris attestation.
5. If prompted, Freighter signs **ChangeTrust** for Circle Testnet USDC, then Freighter signs **`mint_and_forward`**.
6. Confirm destination USDC on Horizon and record hashes (as above).

## What this proves / does not prove

**Proves**

- Sepolia → Stellar Testnet USDC mint path with Circle Iris + `CctpForwarder.mint_and_forward`.
- Non-custodial destination mint: Freighter pays Stellar fees; MetaMask pays Sepolia gas.
- Fast finality path on the reverse corridor (net of Iris Fast fee).

**Does not prove**

- Mainnet availability.
- Public production `CCTP_ENABLED` (staging enablement is a separate checklist).
- Solana or other destinations.

## Operator posture

- Default remains fail-closed: `CCTP_ENABLED=false` → `503 cctp_not_enabled`.
- Staging flip: [`docs/deployment/cctp-staging-enablement-checklist.md`](../deployment/cctp-staging-enablement-checklist.md).
- Never classify ChangeTrust or ERC-20 approve txs as burns/mints.
- Freighter-signed mint envelopes must be verified with signatures stripped so the payload hash matches prepare.

## Related docs

- Forward proof: [`signed-live-stellar-to-sepolia.md`](./signed-live-stellar-to-sepolia.md)
- API contract: [`../api/cctp-v2-contract.md`](../api/cctp-v2-contract.md)
- Attestation trust: [`attestation-verification.md`](./attestation-verification.md)
