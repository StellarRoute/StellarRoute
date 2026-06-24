# Wallet Production Readiness Runbook

This document maps every wallet-related code path to its current status — **stub / test-only** vs **production-ready** — and provides a Phase B on-chain swap checklist for contributors landing real signing support.

---

## 1. Feature Matrix

| Feature | Entry point | Status | Notes |
|---|---|---|---|
| **Connect (Freighter)** | `connectWallet("freighter")` in `lib/wallet/index.ts` | ✅ Production ready | Calls `requestAccess()` + `getAddress()` + `getNetworkDetails()` from `@stellar/freighter-api` |
| **Connect (xBull)** | `connectWallet("xbull")` in `lib/wallet/index.ts` | ✅ Production ready | Reads `window.xbull.connect()` — requires xBull extension installed |
| **Detect installed wallets** | `getAvailableWallets()` in `lib/wallet/index.ts` | ✅ Production ready | Freighter: `isAllowed()` API; xBull: `window.xbull` presence check |
| **View address** | `getAddress()` (Freighter) / `connect().publicKey` (xBull) | ✅ Production ready | Called during `connectWallet` and `refreshWalletSession` |
| **View network** | `getNetworkDetails()` (Freighter) / hardcoded `"testnet"` (xBull) | ⚠️ Partial | xBull always returns `"testnet"` — needs real network detection for mainnet |
| **Sign transaction** | `signTransactionWithWallet(xdr, walletId)` in `lib/wallet/index.ts` | ✅ Production ready | Freighter: `signTransaction()`; xBull: `window.xbull.sign()` |
| **Sign transaction stub** | `signTransactionStub(xdr)` in `lib/wallet/index.ts` | 🔴 Stub only | Returns `{ ok: false }` — used in tests and out-of-scope flows. **Never call in production.** |
| **Spendable balance** | `stubSpendableBalance` in `useWalletBalance.ts` | 🔴 Stub only | Returns a hardcoded/mock balance. Replace with Horizon `accounts/{id}` call before Phase B. |
| **Auto-reconnect** | `WalletProvider` effect in `wallet-provider.tsx` | ✅ Production ready | Reads `stellarroute.wallet.lastWalletId` from `localStorage`, respects `autoReconnectPreferred` flag |
| **Reconnect on focus/online** | `WalletProvider` window event listeners | ✅ Production ready | Throttled to 5 s to avoid hammering the wallet extension |
| **Network mismatch detection** | `networkMismatch` in `WalletProvider` | ✅ Production ready | Compares app `network` state with `walletNetwork` returned by wallet |
| **Capability check** | `refreshCapabilities()` / `checkWalletCapabilities()` | ⚠️ Mock | `refreshCapabilities` in the provider sets `{ statuses: [] }` — wire to `checkWalletCapabilities` from `lib/wallet/index.ts` for Phase B |
| **Account switch detection** | `accountSwitchState` in `WalletProvider` | ✅ Production ready | Detects address change after `refreshAccount()` |
| **Sync mismatch / resync** | `syncMismatch` + `resyncWallet()` in `WalletProvider` | ✅ Production ready | Calls `refreshAccount()` and clears the mismatch flag |
| **Transaction lifecycle** | `useTransactionLifecycle` hook | ⚠️ Partial | Signs via `signTransactionWithWallet` but submit path (`submit.ts`) does not broadcast to Horizon yet — see Phase B checklist |
| **Transaction broadcast** | `lib/wallet/submit.ts` | 🔴 Stub | `submitTransaction` is scaffolded but does not POST to Horizon. Phase B task. |

---

## 2. Stubbed vs Production-ready Paths

### 🔴 Stubs (must be replaced before mainnet)

| Symbol | File | What it does | Replace with |
|---|---|---|---|
| `signTransactionStub` | `lib/wallet/index.ts` | Echoes XDR back, `ok: false` | Remove call sites; always use `signTransactionWithWallet` |
| `stubSpendableBalance` | `hooks/useWalletBalance.ts` | Returns mock balance | Fetch `GET /accounts/{address}` from Horizon; parse `balances[]` |
| `submitTransaction` (stub body) | `lib/wallet/submit.ts` | Not yet posting to Horizon | POST XDR to `https://horizon.stellar.org/transactions` (or testnet equivalent) |
| `refreshCapabilities` mock | `components/providers/wallet-provider.tsx` | Sets empty `statuses: []` | Call `checkWalletCapabilities(walletId, network)` from `lib/wallet/index.ts` |

### ⚠️ Partial implementations

| Path | Gap | Action needed |
|---|---|---|
| xBull network detection | Always returns `"testnet"` | Implement `window.xbull.getNetwork()` or equivalent; update `connectWallet` and `refreshWalletSession` |
| Transaction lifecycle submit step | `useTransactionLifecycle` calls sign but not broadcast | Wire the signed XDR into `submitTransaction` once it POSTs to Horizon |

---

## 3. Phase B On-chain Swap Checklist

Before enabling real on-chain swaps (Phase B):

- [ ] **Replace `stubSpendableBalance`** — fetch live balance from Horizon `accounts/{address}`; use `buying_liabilities` / `selling_liabilities` for available balance calculation
- [ ] **Implement `submitTransaction`** in `lib/wallet/submit.ts` — POST signed XDR to Horizon `/transactions`; handle `400 tx_bad_auth`, `400 op_underfunded`, and network timeouts
- [ ] **Wire `refreshCapabilities`** in `WalletProvider` — replace mock with `checkWalletCapabilities(walletId, network)` so `WalletCapabilitiesBanner` reflects real status
- [ ] **Fix xBull network detection** — replace hardcoded `"testnet"` with a real API call so `networkMismatch` works on mainnet
- [ ] **Remove all `signTransactionStub` call sites** — search for `signTransactionStub` and ensure only `signTransactionWithWallet` is used in non-test code
- [ ] **Validate network passphrase mapping** — ensure `networkPassphrase` passed to `signTransaction` matches Stellar's canonical values (`Test SDF Network ; September 2015` / `Public Global Stellar Network ; September 2015`)
- [ ] **Test Freighter + xBull on testnet end-to-end** — confirm sign → broadcast → Horizon confirmation flow with real wallets
- [ ] **Add Horizon error taxonomy to `lib/api/trader-error-copy.ts`** — map `tx_bad_seq`, `op_no_trust`, `op_line_full`, etc. to user-facing messages

---

## 4. Freighter and xBull Integration Patterns

### Freighter

Freighter exposes a Promise-based API via `@stellar/freighter-api`. All calls return `{ error?: { message } }` shaped results — always check `.error` before using the value.

```ts
// Connect
const access = await requestAccess();
if (access.error) throw new Error(access.error.message);

// Sign
const res = await signTransaction(xdr, { networkPassphrase });
if (res.error) throw new Error(res.error.message);
return res.signedTxXdr;
```

### xBull

xBull injects `window.xbull`. There is no npm package — detect via `typeof window !== "undefined" && !!window.xbull`.

```ts
const xbull = (window as any).xbull;
const { publicKey } = await xbull.connect();
const signedXdr = await xbull.sign({ xdr, network: 'testnet' });
```

---

## 5. References

- `frontend/lib/wallet/index.ts` — all wallet adapter functions
- `frontend/components/providers/wallet-provider.tsx` — React context; lifecycle, reconnect, mismatch detection
- `frontend/lib/wallet/submit.ts` — Horizon broadcast stub
- `frontend/hooks/useWalletBalance.ts` — balance stub
- `frontend/hooks/useTransactionLifecycle.ts` — sign + submit orchestration
- [Freighter API docs](https://docs.freighter.app)
- [Horizon REST API](https://developers.stellar.org/api/horizon)

---

*This document is referenced from [CONTRIBUTING.md](../../CONTRIBUTING.md) and [docs/development/SETUP.md](SETUP.md).*
