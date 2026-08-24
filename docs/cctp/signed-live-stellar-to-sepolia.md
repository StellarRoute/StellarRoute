# Signed-live proof: Stellar Testnet → Ethereum Sepolia (CCTP)

**Status:** Achieved (testnet)  
**Date:** 2026-08-14  
**Scope:** Forward USDC corridor via Circle CCTP v2 — testnet only; not mainnet; not public `CCTP_ENABLED` by default. Reverse is separately proven in [`signed-live-sepolia-to-stellar.md`](./signed-live-sepolia-to-stellar.md).

This is StellarRoute’s first publicly cited **destination mint** proof for the Stellar → Sepolia USDC rail: a wallet-authorized burn on Stellar Testnet, Circle attestation, and a successful `receiveMessage` mint on Sepolia.

## Canonical destination mint (Aug 14, 2026)

| Field | Value |
| --- | --- |
| Sepolia mint tx | [`0x713cc8b174d775bf7a3a97f33c53a37f698c93bc66b378dfa55ccfcc7f1cbed6`](https://sepolia.etherscan.io/tx/0x713cc8b174d775bf7a3a97f33c53a37f698c93bc66b378dfa55ccfcc7f1cbed6) |
| Block | `11484164` |
| Timestamp (UTC) | `2026-08-14T02:07:12Z` |
| Amount | `25` USDC (canonical 6dp: `25000000`) |
| Mint recipient | `0xa632DA1E4D5DD7FB236a0b798ff9331e3a9930Df` |
| CCTP source domain | `27` (Stellar) |
| Method | `receiveMessage(bytes,bytes)` on MessageTransmitter / CCTP mint path |
| Burn token (Stellar USDC contract) | `CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA` |

Sanitized public evidence JSON:

- [`docs/readiness/evidence/cctp-signed-live-stellar-to-sepolia-2026-08-14.json`](../readiness/evidence/cctp-signed-live-stellar-to-sepolia-2026-08-14.json)

## Earlier instrumented harness run (July 31, 2026)

An earlier **scripted** signed-live harness also completed end-to-end (1 USDC) with full saga timings. Keep that file for timing/reproducibility claims:

- Evidence: [`docs/readiness/evidence/cctp-signed-live-stellar-to-sepolia.json`](../readiness/evidence/cctp-signed-live-stellar-to-sepolia.json)
- Harness: `scripts/cctp-signed-live-stellar-to-sepolia-proof.sh`
- Observed timings from that run: burn→attestation **33s**, total saga **63s**

## What this proves / does not prove

**Proves**

- Stellar Testnet → Sepolia USDC mint path works with Circle Iris attestation and on-chain `receiveMessage`.
- Non-custodial destination mint: EVM wallet submits/pays gas for the Sepolia mint step.
- Product direction is real on public testnets, not only unit tests or unsigned probes.

**Does not prove**

- Mainnet availability.
- That staging/production APIs have `CCTP_ENABLED=true` for the public.
- That every prepare/submit path is open without operator configuration.

Reverse corridor (Sepolia → Stellar) is separately proven: [`signed-live-sepolia-to-stellar.md`](./signed-live-sepolia-to-stellar.md).

## Operator posture (unchanged)

- Default remains fail-closed: `CCTP_ENABLED=false` → `503 cctp_not_enabled`.
- Enabling requires HMAC key + Sepolia RPC + readiness assessment (`docs/api/cctp-v2-contract.md`, `docs/development/environment-variables.md`).
- Stellar burn remains two-step (`approve` then `deposit_for_burn`); never classify approval txs as burns.

## Next work (post-proof)

1. Staging enablement checklist after attestation/verifier readiness stays green: [`../deployment/cctp-staging-enablement-checklist.md`](../deployment/cctp-staging-enablement-checklist.md).
2. Harden public UI saga + explorer links against both-direction mint evidence.
3. Mainnet gates only after audit + explicit ops approval.

## Related docs

- Attestation trust model: [`attestation-verification.md`](./attestation-verification.md)
- Stellar verifier status: [`stellar-verifier-blockers.md`](./stellar-verifier-blockers.md)
- API contract: [`../api/cctp-v2-contract.md`](../api/cctp-v2-contract.md)
- SCF architecture: [`../architecture/scf-technical-architecture.md`](../architecture/scf-technical-architecture.md)
