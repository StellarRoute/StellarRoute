import type { AdapterNetworkId } from '../types';

/** Common EVM networks StellarRoute may target later. */
export const EVM_NETWORKS = {
  ethereumMainnet: 'eip155:1' as AdapterNetworkId,
  ethereumSepolia: 'eip155:11155111' as AdapterNetworkId,
  baseMainnet: 'eip155:8453' as AdapterNetworkId,
  baseSepolia: 'eip155:84532' as AdapterNetworkId,
  arbitrumMainnet: 'eip155:42161' as AdapterNetworkId,
  arbitrumSepolia: 'eip155:421614' as AdapterNetworkId,
} as const;

const ADD_CHAIN_PARAMS: Record<
  string,
  {
    chainId: string;
    chainName: string;
    nativeCurrency: { name: string; symbol: string; decimals: number };
    rpcUrls: string[];
    blockExplorerUrls?: string[];
  }
> = {
  'eip155:1': {
    chainId: '0x1',
    chainName: 'Ethereum Mainnet',
    nativeCurrency: { name: 'Ether', symbol: 'ETH', decimals: 18 },
    rpcUrls: ['https://ethereum.publicnode.com'],
    blockExplorerUrls: ['https://etherscan.io'],
  },
  'eip155:11155111': {
    chainId: '0xaa36a7',
    chainName: 'Sepolia',
    nativeCurrency: { name: 'Sepolia Ether', symbol: 'ETH', decimals: 18 },
    rpcUrls: ['https://ethereum-sepolia.publicnode.com'],
    blockExplorerUrls: ['https://sepolia.etherscan.io'],
  },
  'eip155:8453': {
    chainId: '0x2105',
    chainName: 'Base',
    nativeCurrency: { name: 'Ether', symbol: 'ETH', decimals: 18 },
    rpcUrls: ['https://mainnet.base.org'],
    blockExplorerUrls: ['https://basescan.org'],
  },
  'eip155:84532': {
    chainId: '0x14a34',
    chainName: 'Base Sepolia',
    nativeCurrency: { name: 'Ether', symbol: 'ETH', decimals: 18 },
    rpcUrls: ['https://sepolia.base.org'],
    blockExplorerUrls: ['https://sepolia.basescan.org'],
  },
  'eip155:42161': {
    chainId: '0xa4b1',
    chainName: 'Arbitrum One',
    nativeCurrency: { name: 'Ether', symbol: 'ETH', decimals: 18 },
    rpcUrls: ['https://arb1.arbitrum.io/rpc'],
    blockExplorerUrls: ['https://arbiscan.io'],
  },
  'eip155:421614': {
    chainId: '0x66eee',
    chainName: 'Arbitrum Sepolia',
    nativeCurrency: { name: 'Ether', symbol: 'ETH', decimals: 18 },
    rpcUrls: ['https://sepolia-rollup.arbitrum.io/rpc'],
    blockExplorerUrls: ['https://sepolia.arbiscan.io'],
  },
};

export function chainIdHexToCaip2(chainIdHex: string): AdapterNetworkId {
  const normalized = chainIdHex.startsWith('0x')
    ? chainIdHex
    : `0x${chainIdHex}`;
  const decimal = Number.parseInt(normalized, 16);
  if (!Number.isFinite(decimal)) {
    return `eip155:${normalized}` as AdapterNetworkId;
  }
  return `eip155:${decimal}` as AdapterNetworkId;
}

export function caip2ToChainIdHex(network: AdapterNetworkId): string | null {
  if (!network.startsWith('eip155:')) return null;
  const id = network.slice('eip155:'.length);
  const decimal = Number.parseInt(id, 10);
  if (!Number.isFinite(decimal)) return null;
  return `0x${decimal.toString(16)}`;
}

export function getAddChainParams(network: AdapterNetworkId) {
  return ADD_CHAIN_PARAMS[network] ?? null;
}

export function defaultEvmAppNetwork(): AdapterNetworkId {
  const configured = process.env.NEXT_PUBLIC_EVM_NETWORK?.trim();
  if (configured && configured.startsWith('eip155:')) {
    return configured as AdapterNetworkId;
  }
  // Prefer testnet until mainnet cross-chain routes exist.
  return EVM_NETWORKS.ethereumSepolia;
}
