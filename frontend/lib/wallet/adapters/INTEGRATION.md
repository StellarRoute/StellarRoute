# Multi-chain wallet adapter guide

Unified adapter surface for Stellar wrappers, EVM, Solana, Bitcoin, and TRON.
Cross-chain **swap execution** stays `unsupported` / `no_backend_route` until backend routes exist.

## Layout

| Path | Responsibility |
|------|----------------|
| `adapters/types.ts` | Shared `ChainWalletAdapter`, `ChainFamily`, signing/send payload unions |
| `adapters/registry.ts` | Idempotent default registration + `listAvailableChainWallets` |
| `adapters/session.ts` | Non-React session helpers for `useChainWallet` |
| `adapters/execution-support.ts` | Explicit unsupported/degraded states when routes are absent |
| `adapters/live-state.ts` | Mutable connected / networkMatch / canSign snapshot for sync `getExecutionSupport` |
| `adapters/errors.ts` / `detect.ts` | Normalized errors + SSR-safe injected-provider detection |
| `adapters/stellar/legacy.ts` | Thin wrappers over Freighter/xBull/Albedo/LOBSTR |
| `adapters/evm/*` | EIP-1193 injected (`window.ethereum`) + WalletConnect QR/mobile |
| `adapters/solana/*` | Injected Solana / Phantom |
| `adapters/bitcoin/*` | UniSat + OKX Bitcoin (PSBT + message signing) |
| `adapters/tron/*` | TronLink (account request, network detect, TronWeb sign) |
| `adapters/bridge/*` | Stub bridge provider (`no_backend_route` until wired) |
| `hooks/useChainWallet.ts` | React hook for multi-chain connect/sign/send |

## What stays untouched

- Stellar `SupportedWallet`, `connectWallet`, `signTransactionWithWallet`, and `WalletProvider` behavior in `lib/wallet/index.ts`.
- Swap UI continues to use `hooks/useWallet.ts` / `WalletProvider` for Freighter/xBull/Albedo/LOBSTR.
- Backend routing / prepare-submit paths.
- Private keys and seed phrases are never accepted, stored, or transmitted.

## Default registry ids

`albedo`, `freighter`, `lobstr`, `xbull`, `evm-injected`, `evm-walletconnect`, `solana-injected`, `unisat`, `okx-bitcoin`, `tronlink`.

`ensureDefaultAdapters()` is idempotent and does not clobber pre-registered adapters with the same id.

## Contracts

1. **One `ChainWalletAdapter` surface** — extend `SignTransactionRequest` / `SendTransactionRequest` with new `kind` discriminants; do not fork a second adapter API.
2. **`ChainFamily`** is `'stellar' | 'bitcoin' | 'tron' | 'evm' | 'solana'`.
3. **Network ids** — CAIP-ish strings (`eip155:1`, `solana:mainnet`, `bitcoin:mainnet`, `tron:nile`, `stellar:testnet`).
4. **Soft connect + mismatch** — connect succeeds when networks differ; adapters refresh live state and `useChainWallet` sets `networkMismatch`. User rejection during switch still throws `user_rejected`.
5. **Execution support is live** — `getExecutionSupport` / hook composition read connected, networkMatch, and canSign. Disconnected → `not_connected`; mismatch / missing caps → `degraded`; otherwise non-Stellar routes → `no_backend_route`.
6. **Solana transactions** — require a wallet-compatible object with `serialize()`. Raw base64/bytes return `unsupported_capability` (no fake Transaction wrappers).
7. **Signing gates** — `signMessage`, `signTransaction`, and `sendTransaction` all block on network mismatch (hook + adapters).
8. **Dependency policy** — prefer injected browser APIs. Allowed exception: `@walletconnect/ethereum-provider` for the `evm-walletconnect` adapter (EIP-1193 via QR/mobile). Still avoid `wagmi`, `ethers`, `@solana/web3.js`, and TronWeb npm unless explicitly approved.

## Unsupported / degraded product states

- Any non-`stellar→stellar` swap route → `unsupported` / `no_backend_route` when connected and matched (even same-chain EVM/Solana/BTC/TRON).
- Before connect → `not_connected` (preferred over generic no-route).
- Network mismatch → `degraded` / `network_mismatch`.
- Missing signing methods → `degraded` / `wallet_capability_missing`.
- Missing extension → `detectInstalled() === false`; connect throws `not_installed`.
- Bridge stub always reports `no_backend_route`.

## Smoke usage

```ts
import {
  getAdapter,
  listAvailableChainWallets,
} from '@/lib/wallet/adapters';
import { useChainWallet } from '@/hooks/useChainWallet';

const wallets = await listAvailableChainWallets('bitcoin');
const unisat = getAdapter('unisat');
const session = await unisat?.connect('bitcoin:testnet');
// If wallet is on another network, session still connects; check getNetwork / executionSupport.

const { connect, executionSupport, networkMismatch, signMessage } = useChainWallet({
  chainFamily: 'evm',
  expectedNetwork: 'eip155:11155111',
});
```
