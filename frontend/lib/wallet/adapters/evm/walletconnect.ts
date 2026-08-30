import { WalletAdapterError } from '../errors';
import type { AdapterNetworkId, ChainWalletAdapter } from '../types';
import { createEip1193ChainAdapter } from './eip1193-adapter';
import type { Eip1193Provider } from './provider';
import {
  caip2ToChainIdDecimal,
  getWalletConnectMetadata,
  getWalletConnectOptionalChains,
  getWalletConnectProjectId,
  getWalletConnectRpcMap,
  isWalletConnectConfigured,
} from './walletconnect-config';

const ADAPTER_ID = 'evm-walletconnect';

type WalletConnectEthereumProvider = Eip1193Provider & {
  accounts: string[];
  session?: unknown;
  connect: (opts?: {
    chains?: number[];
    optionalChains?: number[];
  }) => Promise<void>;
  disconnect: () => Promise<void>;
  enable?: () => Promise<string[]>;
};

let providerPromise: Promise<WalletConnectEthereumProvider> | null = null;
let providerInstance: WalletConnectEthereumProvider | null = null;

async function initProvider(): Promise<WalletConnectEthereumProvider> {
  const projectId = getWalletConnectProjectId();
  if (!projectId) {
    throw new WalletAdapterError(
      'WalletConnect is not configured. Set NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID.',
      'not_installed',
      ADAPTER_ID
    );
  }

  if (providerInstance) return providerInstance;
  if (providerPromise) return providerPromise;

  providerPromise = (async () => {
    const { EthereumProvider } = await import(
      '@walletconnect/ethereum-provider'
    );
    const optionalChains = getWalletConnectOptionalChains();
    const instance = (await EthereumProvider.init({
      projectId,
      showQrModal: true,
      optionalChains,
      rpcMap: getWalletConnectRpcMap(),
      metadata: getWalletConnectMetadata(),
      optionalMethods: [
        'eth_sendTransaction',
        'eth_signTransaction',
        'eth_sign',
        'personal_sign',
        'eth_signTypedData',
        'eth_signTypedData_v4',
        'wallet_switchEthereumChain',
        'wallet_addEthereumChain',
      ],
      optionalEvents: ['chainChanged', 'accountsChanged'],
    })) as unknown as WalletConnectEthereumProvider;

    providerInstance = instance;
    return instance;
  })();

  try {
    return await providerPromise;
  } catch (err) {
    providerPromise = null;
    providerInstance = null;
    throw err;
  }
}

function getActiveProvider(): Eip1193Provider | null {
  return providerInstance;
}

/** Restore a persisted WC session without opening the QR modal. */
async function hydrateProvider(): Promise<Eip1193Provider | null> {
  if (typeof window === 'undefined' || !isWalletConnectConfigured()) {
    return null;
  }
  try {
    const provider = await initProvider();
    if (provider.session && provider.accounts?.length) {
      return provider;
    }
    return null;
  } catch {
    return null;
  }
}

async function acquireProvider(
  expectedNetwork?: AdapterNetworkId
): Promise<Eip1193Provider> {
  if (typeof window === 'undefined') {
    throw new WalletAdapterError(
      'WalletConnect is only available in the browser',
      'not_installed',
      ADAPTER_ID
    );
  }

  const provider = await initProvider();

  // Persisted WC session: reuse without opening the QR modal again.
  if (provider.session && provider.accounts?.length) {
    return provider;
  }

  const preferred = expectedNetwork
    ? caip2ToChainIdDecimal(expectedNetwork)
    : null;
  const optionalChains = getWalletConnectOptionalChains();

  try {
    await provider.connect({
      ...(preferred != null ? { chains: [preferred] } : {}),
      optionalChains,
    });
  } catch (err) {
    // Some wallets reject required `chains`; fall back to optional-only.
    if (preferred != null) {
      await provider.connect({ optionalChains });
    } else {
      throw err;
    }
  }

  return provider;
}

async function releaseProvider(): Promise<void> {
  const instance = providerInstance;
  providerInstance = null;
  providerPromise = null;
  if (!instance) return;
  try {
    await instance.disconnect();
  } catch {
    // Session may already be gone; drop local handle either way.
  }
}

export function createWalletConnectEvmAdapter(): ChainWalletAdapter {
  return createEip1193ChainAdapter({
    id: ADAPTER_ID,
    label: 'WalletConnect',
    installUrl: 'https://cloud.reown.com',
    notInstalledMessage:
      'WalletConnect is not configured. Set NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID from https://cloud.reown.com',
    notInstalledResolution:
      'Add NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID to enable mobile and QR wallets',
    detectInstalled: async () => isWalletConnectConfigured(),
    getProvider: getActiveProvider,
    hydrateProvider,
    acquireProvider,
    releaseProvider,
  });
}

/** Test helper — clears the module-level WC provider singleton. */
export function resetWalletConnectProviderForTests(): void {
  providerInstance = null;
  providerPromise = null;
}
