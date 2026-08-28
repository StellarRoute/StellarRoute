# CCTP v2 Attestation Verification

## Trust model

- **On-chain destination `MessageTransmitterV2` attester set is authoritative** for `signatureThreshold` and enabled attester membership.
- **Iris `/v2/publicKeys`** is discovery/cross-check only; never used as threshold authority.
- **Every on-chain enabled attester must appear in Iris v2 key-derived addresses** before a generation is considered ready (fail closed).
- **Cryptographic verification** mirrors Circle `Attestable` / Stellar `attestable::storage` rules:
  - digest = `keccak256(raw_message)` (no personal-sign prefix)
  - attestation length = `65 * signatureThreshold`
  - low-`s` enforced on both destination paths (intersection with Soroban SDK rules)
  - `v` ∈ {27, 28, 0, 1}
  - recovered addresses strictly increasing; all must be enabled on destination

## Pinned sources

| Component | Source |
|-----------|--------|
| EVM rules | `circlefin/evm-cctp-contracts` `src/roles/Attestable.sol` @ `a92a2b4e7e6e` |
| Stellar rules | `circlefin/stellar-cctp` `packages/cctp-roles/src/attestable/storage.rs` @ `45746f2c8031` |
| Test vectors | `stellar-cctp` `packages/cctp-roles/src/test_utils/attestable.rs` @ `45746f2c8031` |
| Iris API | `GET /v2/publicKeys` (sandbox: `iris-api-sandbox.circle.com`) |

`ATTESTER_ADDRESS_3` in local fixtures is a **test-only** enabled attester, not a Circle fixture signer.

## Crypto dependencies

- `tiny-keccak` (Keccak-256) — dual MIT/Apache-2.0
- `k256` (secp256k1 recover) — Apache-2.0 / MIT

## Cache / rotation

| Cache | Default TTL | Max stale | Refresh |
|-------|-------------|-----------|---------|
| Attestation trust generation (Iris + Sepolia + Stellar) | 15m | 24h | `ensure_fresh` on verify + background task; single-flight atomic swap |

Env overrides: `CCTP_IRIS_KEYS_TTL_SECS`, `CCTP_IRIS_KEYS_STALE_MAX_SECS`, `CCTP_ATTESTER_SNAPSHOT_TTL_SECS`, `CCTP_ATTESTER_SNAPSHOT_STALE_MAX_SECS`.

Protocol caps: `MAX_IRIS_PUBLIC_KEYS=256`, `MAX_ENABLED_ATTESTERS=256`, `MAX_SIGNATURE_THRESHOLD=64`.

## Operator alerts

- `stellarroute_cctp_iris_keys_refresh_total{outcome="failure"}`
- `stellarroute_cctp_attester_snapshot_refresh_total{outcome="failure"}`
- `stellarroute_cctp_attestation_verify_total{reason!="ok"}`

## Bootstrap

- Use `CctpRuntime::from_config_async` during server startup to bootstrap attestation trust.
- Sync `from_config` wires EVM components only; attestation verifier stays `NotReady` (no `block_on`).

## Public safety (current phase)

- When `CCTP_ENABLED=false` (default), `/api/v2/bridge/cctp/*` handlers return **503** `cctp_not_enabled`.
- Signed-live Stellar → Sepolia destination mint is proven on public testnets
  ([`signed-live-stellar-to-sepolia.md`](./signed-live-stellar-to-sepolia.md)); that does **not**
  by itself make the corridor public-executable.
- `CircleAttestationVerifier` may become **ready** when Iris + EVM + Stellar RPC readers bootstrap, but **corridor is not live** until Stellar burn/approval/mint verifiers ship.
- `is_public_executable()` stays **false** until all runtime components ready.

## Live read-only checks

```bash
./scripts/cctp-live-attester-read.sh
```

## Signed-live Stellar → Sepolia proof (testnet)

Public destination mint (2026-08-14):

- Sepolia mint: [`0x713cc8b174d775bf7a3a97f33c53a37f698c93bc66b378dfa55ccfcc7f1cbed6`](https://sepolia.etherscan.io/tx/0x713cc8b174d775bf7a3a97f33c53a37f698c93bc66b378dfa55ccfcc7f1cbed6)
- Narrative + claims: [`signed-live-stellar-to-sepolia.md`](./signed-live-stellar-to-sepolia.md)
- Evidence: `docs/readiness/evidence/cctp-signed-live-stellar-to-sepolia-2026-08-14.json`

This proves attestation + destination mint on public testnets. It does **not** flip
`CCTP_ENABLED` or claim mainnet / reverse-corridor readiness.

## Signed-live testnet harness key handling

`scripts/cctp-signed-live-stellar-to-sepolia-proof.sh` accepts the Sepolia
signing key only through `CCTP_EVM_MINT_KEY_FILE`. The path must resolve to a
regular, non-symlink file with mode `0600`; plaintext key environment variables
are intentionally unsupported.

The harness builds and invokes `cctp-evm-signer` with key, transaction, and RPC
file paths only. The helper reads the key internally, verifies the signer and
mint recipient match, pins Sepolia and its `MessageTransmitterV2`, validates
zero value, `receiveMessage(bytes,bytes)` calldata, and the gas cap, then signs
and broadcasts the raw EIP-1559 transaction. Its only stdout is the public
transaction hash. Temporary request and RPC files are removed by the harness
exit trap.
