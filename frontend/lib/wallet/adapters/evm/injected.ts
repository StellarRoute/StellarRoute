import { WalletAdapterError } from '../errors';
import type { ChainWalletAdapter } from '../types';
import {
  createEip1193ChainAdapter,
  probeEip1193Provider,
} from './eip1193-adapter';
import {
  getInjectedEip1193Provider,
  type Eip1193Provider,
} from './provider';

const ADAPTER_ID = 'evm-injected';

function requireProvider(): Eip1193Provider {
  const provider = getInjectedEip1193Provider();
  if (!provider) {
    throw new WalletAdapterError(
      'No EVM wallet detected. Install MetaMask or another EIP-1193 wallet.',
      'not_installed',
      ADAPTER_ID
    );
  }
  return provider;
}

export function createInjectedEvmAdapter(): ChainWalletAdapter {
  return createEip1193ChainAdapter({
    id: ADAPTER_ID,
    label: 'EVM Wallet',
    installUrl: 'https://metamask.io/download/',
    notInstalledMessage:
      'No EVM wallet detected. Install MetaMask or another EIP-1193 wallet.',
    notInstalledResolution: 'Install MetaMask or another EVM wallet',
    detectInstalled: async () => {
      if (typeof window === 'undefined') return false;
      return probeEip1193Provider(getInjectedEip1193Provider());
    },
    getProvider: getInjectedEip1193Provider,
    acquireProvider: async () => requireProvider(),
  });
}
