import { withTimeout } from '../detect';
import {
  normalizeProviderError,
  WalletAdapterError,
} from '../errors';
import { resolveExecutionSupport } from '../execution-support';
import { createLiveSigningTracker } from '../live-state';
import type {
  AdapterCapabilities,
  AdapterNetworkId,
  ChainNetworkInfo,
  ChainWalletAdapter,
  ChainWalletSession,
  SendTransactionRequest,
  SendTransactionResult,
  SignMessageRequest,
  SignTransactionRequest,
  SignedMessageResult,
  SignedTransactionResult,
} from '../types';
import { defaultSolanaAppNetwork, normalizeSolanaCluster } from './networks';
import {
  bytesToBase64,
  getInjectedSolanaWallet,
  isSolanaWalletTransaction,
  publicKeyToAddress,
  type SolanaInjectedWallet,
} from './provider';

const DETECT_TIMEOUT_MS = 800;
const ADAPTER_ID = 'solana-injected';

function requireWallet(): SolanaInjectedWallet {
  const wallet = getInjectedSolanaWallet();
  if (!wallet) {
    throw new WalletAdapterError(
      'No Solana wallet detected. Install Phantom or another injected Solana wallet.',
      'not_installed',
      ADAPTER_ID
    );
  }
  return wallet;
}

function readCluster(wallet: SolanaInjectedWallet): AdapterNetworkId {
  const raw =
    wallet.network ??
    wallet.chain ??
    (typeof wallet.rpcEndpoint === 'string' ? wallet.rpcEndpoint : null);

  if (typeof raw === 'string') {
    if (raw.includes('devnet')) return 'solana:devnet';
    if (raw.includes('testnet')) return 'solana:testnet';
    const normalized = normalizeSolanaCluster(raw);
    if (normalized) return normalized;
  }

  // Phantom does not always expose cluster; default to app preference.
  return defaultSolanaAppNetwork();
}

function buildSession(
  wallet: SolanaInjectedWallet,
  address: string
): ChainWalletSession {
  return {
    adapterId: ADAPTER_ID,
    chainFamily: 'solana',
    account: { address, publicKey: address },
    network: readCluster(wallet),
    isConnected: true,
  };
}

function networkInfo(
  wallet: SolanaInjectedWallet,
  expectedNetwork?: AdapterNetworkId
): ChainNetworkInfo {
  const network = readCluster(wallet);
  const expected = expectedNetwork;
  const exposed =
    wallet.network != null || wallet.chain != null || wallet.rpcEndpoint != null;
  return {
    network,
    raw: wallet.network ?? wallet.chain ?? wallet.rpcEndpoint,
    expected,
    matchesExpected: expected
      ? exposed
        ? network === expected
        : true
      : true,
  };
}

function assertSolanaTx(
  request: SignTransactionRequest | SendTransactionRequest
): asserts request is Extract<
  SignTransactionRequest | SendTransactionRequest,
  { kind: 'solana_transaction' }
> {
  if (request.kind !== 'solana_transaction') {
    throw new WalletAdapterError(
      `Solana adapter cannot handle payload kind "${request.kind}"`,
      'invalid_request',
      ADAPTER_ID
    );
  }
}

function requireWalletTransaction(transaction: unknown): {
  serialize: (...args: unknown[]) => Uint8Array;
} {
  if (isSolanaWalletTransaction(transaction)) {
    return transaction;
  }
  throw new WalletAdapterError(
    'Solana sign/send requires a wallet-compatible Transaction object with serialize(). Raw base64/bytes are not accepted until a web3.js decode path exists.',
    'unsupported_capability',
    ADAPTER_ID
  );
}

function encodeMessage(request: SignMessageRequest): Uint8Array {
  if (request.encoding === 'hex') {
    const hex = request.message.startsWith('0x')
      ? request.message.slice(2)
      : request.message;
    const out = new Uint8Array(hex.length / 2);
    for (let i = 0; i < out.length; i += 1) {
      out[i] = Number.parseInt(hex.slice(i * 2, i * 2 + 2), 16);
    }
    return out;
  }
  return new TextEncoder().encode(request.message);
}

function signatureToBase64(signature: Uint8Array | number[]): string {
  const bytes =
    signature instanceof Uint8Array ? signature : new Uint8Array(signature);
  return bytesToBase64(bytes);
}

export function createInjectedSolanaAdapter(): ChainWalletAdapter {
  const live = createLiveSigningTracker();
  let lastExpected: AdapterNetworkId | undefined;

  const refreshLive = (
    wallet: SolanaInjectedWallet | null,
    expectedNetwork?: AdapterNetworkId
  ) => {
    const expected = expectedNetwork ?? lastExpected;
    if (expected) lastExpected = expected;
    if (!wallet) {
      live.patch({ connected: false, canSign: false, networkMatch: true });
      return;
    }
    const address = wallet.publicKey
      ? publicKeyToAddress(wallet.publicKey)
      : null;
    const connected = Boolean(address) && wallet.isConnected !== false;
    const info = networkInfo(wallet, expected);
    const canSignTx = Boolean(wallet.signTransaction);
    live.patch({
      connected,
      networkMatch: info.matchesExpected,
      canSign: connected && info.matchesExpected && canSignTx,
    });
  };

  return {
    id: ADAPTER_ID,
    label: 'Solana Wallet',
    chainFamily: 'solana',
    installUrl: 'https://phantom.app/',

    async detectInstalled() {
      if (typeof window === 'undefined') return false;
      const wallet = getInjectedSolanaWallet();
      if (!wallet) return false;
      try {
        await withTimeout(Promise.resolve(true), DETECT_TIMEOUT_MS, true);
        return true;
      } catch {
        return false;
      }
    },

    async connect(expectedNetwork?: AdapterNetworkId) {
      const wallet = requireWallet();
      try {
        const result = await wallet.connect();
        const address = publicKeyToAddress(result.publicKey);
        const expected = expectedNetwork ?? defaultSolanaAppNetwork();
        lastExpected = expected;
        const info = networkInfo(wallet, expected);
        // Soft connect: mismatch is surfaced via getNetwork / live state / hook.
        const session = {
          ...buildSession(wallet, address),
          network: info.network,
        };
        refreshLive(wallet, expected);
        return session;
      } catch (err) {
        live.patch({ connected: false, canSign: false });
        throw normalizeProviderError(
          err,
          'Failed to connect Solana wallet',
          ADAPTER_ID
        );
      }
    },

    async disconnect() {
      const wallet = getInjectedSolanaWallet();
      live.reset();
      if (!wallet?.disconnect) return;
      try {
        await wallet.disconnect();
      } catch (err) {
        throw normalizeProviderError(
          err,
          'Failed to disconnect Solana wallet',
          ADAPTER_ID
        );
      }
    },

    async getSession() {
      const wallet = getInjectedSolanaWallet();
      if (!wallet?.publicKey) {
        live.patch({ connected: false, canSign: false });
        return null;
      }
      if (wallet.isConnected === false) {
        live.patch({ connected: false, canSign: false });
        return null;
      }
      const session = buildSession(wallet, publicKeyToAddress(wallet.publicKey));
      refreshLive(wallet, lastExpected);
      return session;
    },

    async getNetwork(expectedNetwork?: AdapterNetworkId) {
      const wallet = requireWallet();
      if (expectedNetwork) lastExpected = expectedNetwork;
      const info = networkInfo(wallet, expectedNetwork ?? lastExpected);
      refreshLive(wallet, expectedNetwork ?? lastExpected);
      return info;
    },

    async signMessage(request: SignMessageRequest) {
      const wallet = requireWallet();
      if (typeof wallet.signMessage !== 'function') {
        throw new WalletAdapterError(
          'Solana wallet does not support message signing',
          'unsupported_capability',
          ADAPTER_ID
        );
      }
      if (!wallet.publicKey && !wallet.isConnected) {
        throw new WalletAdapterError(
          'Connect a Solana wallet before signing',
          'not_connected',
          ADAPTER_ID
        );
      }
      if (lastExpected) {
        const info = networkInfo(wallet, lastExpected);
        if (!info.matchesExpected) {
          throw new WalletAdapterError(
            'Wallet network does not match the app. Switch networks before signing.',
            'network_mismatch',
            ADAPTER_ID
          );
        }
      }

      try {
        const result = await wallet.signMessage(encodeMessage(request), 'utf8');
        const signatureBytes =
          result instanceof Uint8Array
            ? result
            : 'signature' in result
              ? result.signature
              : null;
        if (!signatureBytes) {
          throw new WalletAdapterError(
            'Solana wallet did not return a signature',
            'provider_error',
            ADAPTER_ID
          );
        }
        const address = wallet.publicKey
          ? publicKeyToAddress(wallet.publicKey)
          : '';
        return {
          signature: signatureToBase64(signatureBytes),
          address,
          publicKey: address || undefined,
        } satisfies SignedMessageResult;
      } catch (err) {
        throw normalizeProviderError(
          err,
          'Solana message signing failed',
          ADAPTER_ID
        );
      }
    },

    async signTransaction(request: SignTransactionRequest) {
      assertSolanaTx(request);
      const wallet = requireWallet();
      if (typeof wallet.signTransaction !== 'function') {
        throw new WalletAdapterError(
          'Solana wallet does not support transaction signing',
          'unsupported_capability',
          ADAPTER_ID
        );
      }
      if (lastExpected) {
        const info = networkInfo(wallet, lastExpected);
        if (!info.matchesExpected) {
          throw new WalletAdapterError(
            'Wallet network does not match the app. Switch networks before signing.',
            'network_mismatch',
            ADAPTER_ID
          );
        }
      }

      const tx = requireWalletTransaction(request.transaction);

      try {
        const signed = await wallet.signTransaction(tx);
        let signedBytes: Uint8Array;
        if (signed instanceof Uint8Array) {
          signedBytes = signed;
        } else if (signed && typeof signed.serialize === 'function') {
          signedBytes = signed.serialize();
        } else {
          throw new WalletAdapterError(
            'Solana wallet returned an unrecognized signed transaction',
            'provider_error',
            ADAPTER_ID
          );
        }
        return {
          kind: 'solana_transaction',
          signedTransaction: bytesToBase64(signedBytes),
        } satisfies SignedTransactionResult;
      } catch (err) {
        throw normalizeProviderError(
          err,
          'Solana transaction signing failed',
          ADAPTER_ID
        );
      }
    },

    async sendTransaction(request: SendTransactionRequest) {
      assertSolanaTx(request);
      const wallet = requireWallet();
      if (typeof wallet.signAndSendTransaction !== 'function') {
        throw new WalletAdapterError(
          'Solana wallet does not support signAndSendTransaction',
          'unsupported_capability',
          ADAPTER_ID
        );
      }
      if (lastExpected) {
        const info = networkInfo(wallet, lastExpected);
        if (!info.matchesExpected) {
          throw new WalletAdapterError(
            'Wallet network does not match the app. Switch networks before sending.',
            'network_mismatch',
            ADAPTER_ID
          );
        }
      }

      const tx = requireWalletTransaction(request.transaction);

      try {
        const result = await wallet.signAndSendTransaction(tx, request.options);
        if (!result?.signature) {
          throw new WalletAdapterError(
            'Solana wallet did not return a signature',
            'provider_error',
            ADAPTER_ID
          );
        }
        return {
          kind: 'solana_transaction',
          signature: result.signature,
        } satisfies SendTransactionResult;
      } catch (err) {
        throw normalizeProviderError(
          err,
          'Solana transaction send failed',
          ADAPTER_ID
        );
      }
    },

    async checkCapabilities(expectedNetwork?: AdapterNetworkId) {
      const wallet = getInjectedSolanaWallet();
      const installed = Boolean(wallet);
      const address = wallet?.publicKey
        ? publicKeyToAddress(wallet.publicKey)
        : null;
      const expected = expectedNetwork ?? lastExpected;
      const info = wallet
        ? networkInfo(wallet, expected)
        : {
            matchesExpected: true,
            network: defaultSolanaAppNetwork() as AdapterNetworkId,
          };

      refreshLive(wallet, expected);

      const statuses: AdapterCapabilities['statuses'] = [
        {
          capability: 'connect',
          allowed: installed,
          reason: installed ? undefined : 'No Solana provider',
          resolution: installed
            ? undefined
            : 'Install Phantom or another Solana wallet',
        },
        {
          capability: 'disconnect',
          allowed: Boolean(wallet?.disconnect),
          reason: wallet?.disconnect
            ? undefined
            : 'Wallet does not expose disconnect',
        },
        {
          capability: 'view_address',
          allowed: Boolean(address),
          reason: address ? undefined : 'No account authorized',
          resolution: address ? undefined : 'Connect the wallet to grant access',
        },
        {
          capability: 'view_network',
          allowed: info.matchesExpected,
          reason: info.matchesExpected
            ? undefined
            : `Wallet on ${info.network}, expected ${expected}`,
          resolution: info.matchesExpected
            ? undefined
            : 'Switch the Solana cluster in your wallet to match the app',
        },
        {
          capability: 'sign_message',
          allowed:
            Boolean(wallet?.signMessage) &&
            Boolean(address) &&
            info.matchesExpected,
          reason: !info.matchesExpected ? 'Network mismatch' : undefined,
        },
        {
          capability: 'sign_transaction',
          allowed:
            Boolean(wallet?.signTransaction) &&
            Boolean(address) &&
            info.matchesExpected,
          reason: !wallet?.signTransaction
            ? 'Wallet cannot sign transactions'
            : !info.matchesExpected
              ? 'Network mismatch'
              : 'Requires a Transaction object with serialize(); raw bytes unsupported',
        },
        {
          capability: 'send_transaction',
          allowed:
            Boolean(wallet?.signAndSendTransaction) &&
            Boolean(address) &&
            info.matchesExpected,
          reason: !wallet?.signAndSendTransaction
            ? 'Wallet cannot send transactions'
            : !info.matchesExpected
              ? 'Network mismatch'
              : 'Requires a Transaction object with serialize(); raw bytes unsupported',
        },
        {
          capability: 'switch_network',
          allowed: false,
          reason:
            'Injected Solana wallets do not support programmatic cluster switch',
          resolution: 'Change the network inside the wallet extension',
        },
      ];

      return { checkedAt: Date.now(), statuses };
    },

    getExecutionSupport(routeHint) {
      return resolveExecutionSupport('solana', routeHint, live.read());
    },
  };
}
