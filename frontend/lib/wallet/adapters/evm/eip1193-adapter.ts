import { withTimeout } from '../detect';
import {
  isRpcMethodNotFound,
  normalizeProviderError,
  WalletAdapterError,
} from '../errors';
import { resolveExecutionSupport } from '../execution-support';
import { createLiveSigningTracker } from '../live-state';
import type {
  AdapterCapabilities,
  AdapterNetworkId,
  ChainWalletAdapter,
  ChainWalletSession,
  ChainNetworkInfo,
  SendTransactionRequest,
  SendTransactionResult,
  SignMessageRequest,
  SignTransactionRequest,
  SignedMessageResult,
  SignedTransactionResult,
} from '../types';
import {
  caip2ToChainIdHex,
  chainIdHexToCaip2,
  defaultEvmAppNetwork,
  getAddChainParams,
} from './networks';
import { eip1193Request, type Eip1193Provider } from './provider';

const DETECT_TIMEOUT_MS = 800;

export type Eip1193AdapterOptions = {
  id: string;
  label: string;
  installUrl?: string;
  notInstalledMessage: string;
  notInstalledResolution: string;
  detectInstalled: () => Promise<boolean>;
  /** Passive provider read (no modal / no prompt). */
  getProvider: () => Eip1193Provider | null;
  /**
   * Optional silent restore (e.g. WalletConnect persisted session).
   * Must not open a QR / approval modal.
   */
  hydrateProvider?: () => Promise<Eip1193Provider | null>;
  /**
   * Ensure a provider is available for connect / switch.
   * WalletConnect may open a QR modal here; injected throws if missing.
   */
  acquireProvider: (
    expectedNetwork?: AdapterNetworkId
  ) => Promise<Eip1193Provider>;
  /** Optional session teardown (e.g. WalletConnect disconnect). */
  releaseProvider?: () => Promise<void>;
};

async function readAccounts(provider: Eip1193Provider): Promise<string[]> {
  try {
    const accounts = await eip1193Request<string[]>(provider, 'eth_accounts');
    return Array.isArray(accounts) ? accounts : [];
  } catch {
    return [];
  }
}

async function readNetwork(
  provider: Eip1193Provider,
  expectedNetwork?: AdapterNetworkId
): Promise<ChainNetworkInfo> {
  const chainIdHex = await eip1193Request<string>(provider, 'eth_chainId');
  const network = chainIdHexToCaip2(chainIdHex);
  const expected = expectedNetwork;
  return {
    network,
    raw: chainIdHex,
    expected,
    matchesExpected: expected ? network === expected : true,
  };
}

async function buildSession(
  adapterId: string,
  provider: Eip1193Provider,
  address: string,
  expectedNetwork?: AdapterNetworkId
): Promise<ChainWalletSession> {
  const networkInfo = await readNetwork(provider, expectedNetwork);
  return {
    adapterId,
    chainFamily: 'evm',
    account: { address },
    network: networkInfo.network,
    isConnected: true,
  };
}

async function switchEthereumNetwork(
  adapterId: string,
  provider: Eip1193Provider,
  network: AdapterNetworkId
): Promise<ChainNetworkInfo> {
  const chainId = caip2ToChainIdHex(network);
  if (!chainId) {
    throw new WalletAdapterError(
      `Invalid EVM network id: ${network}`,
      'invalid_request',
      adapterId
    );
  }

  try {
    await eip1193Request(provider, 'wallet_switchEthereumChain', [{ chainId }]);
  } catch (err) {
    const code =
      err && typeof err === 'object' && 'code' in err
        ? (err as { code?: number }).code
        : undefined;
    if (code === 4902) {
      const params = getAddChainParams(network);
      if (!params) {
        throw new WalletAdapterError(
          `Wallet does not know network ${network} and no add-chain metadata is configured`,
          'network_mismatch',
          adapterId
        );
      }
      try {
        await eip1193Request(provider, 'wallet_addEthereumChain', [params]);
      } catch (addErr) {
        throw normalizeProviderError(
          addErr,
          `Failed to add EVM network ${network}`,
          adapterId
        );
      }
    } else {
      throw normalizeProviderError(
        err,
        `Failed to switch EVM network to ${network}`,
        adapterId
      );
    }
  }

  return readNetwork(provider, network);
}

function assertEvmTx(
  adapterId: string,
  request: SignTransactionRequest | SendTransactionRequest
): asserts request is Extract<
  SignTransactionRequest | SendTransactionRequest,
  { kind: 'evm_transaction' }
> {
  if (request.kind !== 'evm_transaction') {
    throw new WalletAdapterError(
      `EVM adapter cannot handle payload kind "${request.kind}"`,
      'invalid_request',
      adapterId
    );
  }
}

/** Shared EIP-1193 ChainWalletAdapter used by injected + WalletConnect. */
export function createEip1193ChainAdapter(
  options: Eip1193AdapterOptions
): ChainWalletAdapter {
  const {
    id: adapterId,
    label,
    installUrl,
    notInstalledMessage,
    notInstalledResolution,
    detectInstalled,
    getProvider,
    hydrateProvider,
    acquireProvider,
    releaseProvider,
  } = options;

  const live = createLiveSigningTracker();
  let lastExpected: AdapterNetworkId | undefined;

  const refreshLive = async (
    provider: Eip1193Provider | null,
    expectedNetwork?: AdapterNetworkId
  ) => {
    const expected = expectedNetwork ?? lastExpected;
    if (expected) lastExpected = expected;
    if (!provider) {
      live.patch({ connected: false, canSign: false, networkMatch: true });
      return;
    }
    const accounts = await readAccounts(provider);
    const connected = Boolean(accounts[0]);
    let networkMatch = true;
    if (connected && expected) {
      try {
        const info = await readNetwork(provider, expected);
        networkMatch = info.matchesExpected;
      } catch {
        networkMatch = false;
      }
    }
    live.patch({
      connected,
      networkMatch,
      canSign: connected && networkMatch,
    });
  };

  return {
    id: adapterId,
    label,
    chainFamily: 'evm',
    installUrl,

    detectInstalled,

    async connect(expectedNetwork?: AdapterNetworkId) {
      try {
        const provider = await acquireProvider(expectedNetwork);
        const accounts = await eip1193Request<string[]>(
          provider,
          'eth_requestAccounts'
        );
        const address = accounts?.[0];
        if (!address) {
          throw new WalletAdapterError(
            'EVM wallet did not return an account',
            'not_connected',
            adapterId
          );
        }

        const target = expectedNetwork ?? defaultEvmAppNetwork();
        lastExpected = target;
        const networkInfo = await readNetwork(provider, target);
        if (!networkInfo.matchesExpected) {
          try {
            await switchEthereumNetwork(adapterId, provider, target);
          } catch (err) {
            const normalized = normalizeProviderError(
              err,
              'Failed to switch EVM network',
              adapterId
            );
            if (normalized.code === 'user_rejected') throw normalized;
          }
        }

        const session = await buildSession(adapterId, provider, address, target);
        await refreshLive(provider, target);
        return session;
      } catch (err) {
        live.patch({ connected: false, canSign: false });
        throw normalizeProviderError(
          err,
          'Failed to connect EVM wallet',
          adapterId
        );
      }
    },

    async disconnect() {
      live.reset();
      if (releaseProvider) {
        await releaseProvider();
      }
    },

    async getSession() {
      let provider = getProvider();
      if (!provider && hydrateProvider) {
        provider = await hydrateProvider();
      }
      if (!provider) {
        live.patch({ connected: false, canSign: false });
        return null;
      }
      const accounts = await readAccounts(provider);
      if (!accounts[0]) {
        live.patch({ connected: false, canSign: false });
        return null;
      }
      const session = await buildSession(
        adapterId,
        provider,
        accounts[0],
        lastExpected
      );
      await refreshLive(provider, lastExpected);
      return session;
    },

    async getNetwork(expectedNetwork?: AdapterNetworkId) {
      const provider = getProvider();
      if (!provider) {
        throw new WalletAdapterError(
          notInstalledMessage,
          (await detectInstalled()) ? 'not_connected' : 'not_installed',
          adapterId
        );
      }
      try {
        if (expectedNetwork) lastExpected = expectedNetwork;
        const info = await readNetwork(
          provider,
          expectedNetwork ?? lastExpected
        );
        const accounts = await readAccounts(provider);
        live.patch({
          connected: Boolean(accounts[0]),
          networkMatch: info.matchesExpected,
          canSign: Boolean(accounts[0]) && info.matchesExpected,
        });
        return info;
      } catch (err) {
        throw normalizeProviderError(
          err,
          'Failed to read EVM network',
          adapterId
        );
      }
    },

    async switchNetwork(network: AdapterNetworkId) {
      const provider = getProvider();
      if (!provider) {
        throw new WalletAdapterError(
          notInstalledMessage,
          (await detectInstalled()) ? 'not_connected' : 'not_installed',
          adapterId
        );
      }
      lastExpected = network;
      const info = await switchEthereumNetwork(adapterId, provider, network);
      await refreshLive(provider, network);
      return info;
    },

    async signMessage(request: SignMessageRequest) {
      const provider = getProvider();
      if (!provider) {
        throw new WalletAdapterError(
          'Connect an EVM wallet before signing',
          'not_connected',
          adapterId
        );
      }
      const accounts = await readAccounts(provider);
      const address = accounts[0];
      if (!address) {
        throw new WalletAdapterError(
          'Connect an EVM wallet before signing',
          'not_connected',
          adapterId
        );
      }
      if (lastExpected) {
        const info = await readNetwork(provider, lastExpected);
        if (!info.matchesExpected) {
          throw new WalletAdapterError(
            'Wallet network does not match the app. Switch networks before signing.',
            'network_mismatch',
            adapterId
          );
        }
      }

      try {
        const message =
          request.encoding === 'hex' && request.message.startsWith('0x')
            ? request.message
            : `0x${Array.from(new TextEncoder().encode(request.message))
                .map((b) => b.toString(16).padStart(2, '0'))
                .join('')}`;

        const signature = await eip1193Request<string>(
          provider,
          'personal_sign',
          [message, address]
        );

        return {
          signature,
          address,
        } satisfies SignedMessageResult;
      } catch (err) {
        throw normalizeProviderError(
          err,
          'EVM message signing failed',
          adapterId
        );
      }
    },

    async signTransaction(request: SignTransactionRequest) {
      assertEvmTx(adapterId, request);
      const provider = getProvider();
      if (!provider) {
        throw new WalletAdapterError(
          'Connect an EVM wallet before signing',
          'not_connected',
          adapterId
        );
      }
      const accounts = await readAccounts(provider);
      const address = accounts[0];
      if (!address) {
        throw new WalletAdapterError(
          'Connect an EVM wallet before signing',
          'not_connected',
          adapterId
        );
      }
      if (lastExpected) {
        const info = await readNetwork(provider, lastExpected);
        if (!info.matchesExpected) {
          throw new WalletAdapterError(
            'Wallet network does not match the app. Switch networks before signing.',
            'network_mismatch',
            adapterId
          );
        }
      }

      const tx = {
        ...request.transaction,
        from: request.transaction.from ?? address,
      };

      try {
        const signedTransaction = await eip1193Request<string>(
          provider,
          'eth_signTransaction',
          [tx]
        );
        return {
          kind: 'evm_transaction',
          signedTransaction,
        } satisfies SignedTransactionResult;
      } catch (err) {
        const normalized = normalizeProviderError(
          err,
          'EVM transaction signing failed',
          adapterId
        );
        if (
          normalized.code === 'user_rejected' ||
          normalized.code === 'network_mismatch'
        ) {
          throw normalized;
        }
        if (isRpcMethodNotFound(err)) {
          throw new WalletAdapterError(
            'This wallet does not support eth_signTransaction. Use sendTransaction to broadcast instead.',
            'unsupported_capability',
            adapterId
          );
        }
        throw normalized;
      }
    },

    async sendTransaction(request: SendTransactionRequest) {
      assertEvmTx(adapterId, request);
      const provider = getProvider();
      if (!provider) {
        throw new WalletAdapterError(
          'Connect an EVM wallet before sending',
          'not_connected',
          adapterId
        );
      }
      const accounts = await readAccounts(provider);
      const address = accounts[0];
      if (!address) {
        throw new WalletAdapterError(
          'Connect an EVM wallet before sending',
          'not_connected',
          adapterId
        );
      }
      if (lastExpected) {
        const info = await readNetwork(provider, lastExpected);
        if (!info.matchesExpected) {
          throw new WalletAdapterError(
            'Wallet network does not match the app. Switch networks before sending.',
            'network_mismatch',
            adapterId
          );
        }
      }

      const tx = {
        ...request.transaction,
        from: request.transaction.from ?? address,
      };

      try {
        const hash = await eip1193Request<string>(
          provider,
          'eth_sendTransaction',
          [tx]
        );
        return {
          kind: 'evm_transaction',
          hash,
        } satisfies SendTransactionResult;
      } catch (err) {
        throw normalizeProviderError(
          err,
          'EVM transaction send failed',
          adapterId
        );
      }
    },

    async checkCapabilities(expectedNetwork?: AdapterNetworkId) {
      const installed = await detectInstalled();
      const provider = getProvider();
      const accounts = provider ? await readAccounts(provider) : [];
      const address = accounts[0] ?? null;
      let networkMatch = true;
      let networkReason: string | undefined;
      const expected = expectedNetwork ?? lastExpected;

      if (provider && expected) {
        try {
          const info = await readNetwork(provider, expected);
          networkMatch = info.matchesExpected;
          if (!networkMatch) {
            networkReason = `Wallet on ${info.network}, expected ${expected}`;
          }
        } catch {
          networkMatch = false;
          networkReason = 'Failed to read wallet network';
        }
      }

      live.patch({
        connected: Boolean(address),
        networkMatch,
        canSign: Boolean(address) && networkMatch,
      });

      const statuses: AdapterCapabilities['statuses'] = [
        {
          capability: 'connect',
          allowed: installed,
          reason: installed ? undefined : notInstalledMessage,
          resolution: installed ? undefined : notInstalledResolution,
        },
        {
          capability: 'disconnect',
          allowed: true,
        },
        {
          capability: 'view_address',
          allowed: Boolean(address),
          reason: address ? undefined : 'No account authorized',
          resolution: address ? undefined : 'Connect the wallet to grant access',
        },
        {
          capability: 'view_network',
          allowed: networkMatch,
          reason: networkReason,
          resolution: networkMatch
            ? undefined
            : 'Switch wallet network to match the app',
        },
        {
          capability: 'sign_message',
          allowed: Boolean(address) && networkMatch,
          reason: !address
            ? 'Not connected'
            : !networkMatch
              ? 'Network mismatch'
              : undefined,
        },
        {
          capability: 'sign_transaction',
          allowed: Boolean(address) && networkMatch,
          reason: !address
            ? 'Not connected'
            : !networkMatch
              ? 'Network mismatch'
              : 'eth_signTransaction may be unsupported; prefer sendTransaction',
          resolution:
            !address || !networkMatch
              ? 'Connect and match networks'
              : 'Use sendTransaction if signing is unavailable',
        },
        {
          capability: 'send_transaction',
          allowed: Boolean(address) && networkMatch,
          reason: !address
            ? 'Not connected'
            : !networkMatch
              ? 'Network mismatch'
              : undefined,
        },
        {
          capability: 'switch_network',
          allowed: installed,
          reason: installed ? undefined : notInstalledMessage,
        },
      ];

      return { checkedAt: Date.now(), statuses };
    },

    getExecutionSupport(routeHint) {
      return resolveExecutionSupport('evm', routeHint, live.read());
    },
  };
}

/** Probe whether an EIP-1193 provider answers eth_chainId. */
export async function probeEip1193Provider(
  provider: Eip1193Provider | null
): Promise<boolean> {
  if (!provider) return false;
  try {
    await withTimeout(
      eip1193Request(provider, 'eth_chainId'),
      DETECT_TIMEOUT_MS,
      null
    );
    return true;
  } catch {
    return Boolean(provider);
  }
}
