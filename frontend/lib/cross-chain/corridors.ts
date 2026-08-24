import {
  hasBackendRoute,
  type ChainFamily,
} from '@/lib/wallet/adapters';
import type {
  ChainDefinition,
  ChainDisplayId,
  CorridorAvailability,
  CorridorDefinition,
  CorridorId,
} from './types';

export const CHAIN_DEFINITIONS: Record<ChainDisplayId, ChainDefinition> = {
  stellar: {
    id: 'stellar',
    chainFamily: 'stellar',
    label: 'Stellar',
    shortLabel: 'Stellar',
    networkId: 'stellar:testnet',
    assetLabel: 'XLM',
    defaultAssetId: 'native',
  },
  'ethereum-sepolia': {
    id: 'ethereum-sepolia',
    chainFamily: 'evm',
    label: 'Ethereum Sepolia',
    shortLabel: 'ETH Sepolia',
    networkId: 'eip155:11155111',
    assetLabel: 'ETH',
    defaultAssetId: 'native',
  },
  solana: {
    id: 'solana',
    chainFamily: 'solana',
    label: 'Solana',
    shortLabel: 'Solana',
    networkId: 'solana:devnet',
    assetLabel: 'SOL',
    defaultAssetId: 'native',
  },
  bitcoin: {
    id: 'bitcoin',
    chainFamily: 'bitcoin',
    label: 'Bitcoin',
    shortLabel: 'Bitcoin',
    networkId: 'bitcoin:testnet',
    assetLabel: 'BTC',
    defaultAssetId: 'native',
  },
  tron: {
    id: 'tron',
    chainFamily: 'tron',
    label: 'TRON',
    shortLabel: 'TRON',
    networkId: 'tron:nile',
    assetLabel: 'TRX',
    defaultAssetId: 'native',
  },
};

export const CORRIDOR_CATALOG: CorridorDefinition[] = [
  {
    id: 'stellar-native',
    label: 'Stellar native',
    description: 'Same-chain SDEX and Soroban liquidity on Stellar.',
    sourceChainId: 'stellar',
    destChainId: 'stellar',
    protocol: 'stellar-native',
    catalogAvailability: 'executable',
  },
  {
    id: 'evm-to-stellar',
    label: 'ETH Sepolia → Stellar',
    description: 'CCTP testnet corridor — burn USDC on Sepolia, mint on Stellar.',
    sourceChainId: 'ethereum-sepolia',
    destChainId: 'stellar',
    protocol: 'cctp-preview',
    catalogAvailability: 'executable',
  },
  {
    id: 'stellar-to-evm',
    label: 'Stellar → ETH Sepolia',
    description: 'CCTP testnet corridor — burn USDC on Stellar, mint on Sepolia.',
    sourceChainId: 'stellar',
    destChainId: 'ethereum-sepolia',
    protocol: 'cctp-preview',
    catalogAvailability: 'executable',
  },
  {
    id: 'solana-to-stellar',
    label: 'Solana → Stellar',
    description: 'Cross-chain corridor catalog entry — not yet executable.',
    sourceChainId: 'solana',
    destChainId: 'stellar',
    protocol: 'cctp-preview',
    catalogAvailability: 'coming_soon',
  },
  {
    id: 'bitcoin-to-stellar',
    label: 'Bitcoin → Stellar',
    description: 'Cross-chain corridor catalog entry — not yet executable.',
    sourceChainId: 'bitcoin',
    destChainId: 'stellar',
    protocol: 'cctp-preview',
    catalogAvailability: 'coming_soon',
  },
  {
    id: 'tron-to-stellar',
    label: 'TRON → Stellar',
    description: 'Cross-chain corridor catalog entry — not yet executable.',
    sourceChainId: 'tron',
    destChainId: 'stellar',
    protocol: 'cctp-preview',
    catalogAvailability: 'coming_soon',
  },
];

export function chainFamilyForDisplayId(id: ChainDisplayId): ChainFamily {
  return CHAIN_DEFINITIONS[id].chainFamily;
}

export function resolveCorridorAvailability(
  corridor: CorridorDefinition
): CorridorAvailability {
  const source = chainFamilyForDisplayId(corridor.sourceChainId);
  const destination = chainFamilyForDisplayId(corridor.destChainId);
  if (hasBackendRoute(source, destination)) {
    return 'executable';
  }
  return corridor.catalogAvailability === 'executable'
    ? 'coming_soon'
    : corridor.catalogAvailability;
}

export function isCorridorExecutable(corridor: CorridorDefinition): boolean {
  return resolveCorridorAvailability(corridor) === 'executable';
}

export function findCorridorById(id: CorridorId): CorridorDefinition {
  const corridor = CORRIDOR_CATALOG.find((c) => c.id === id);
  if (!corridor) {
    throw new Error(`Unknown corridor: ${id}`);
  }
  return corridor;
}

export const UNMATCHED_CORRIDOR_ID = 'unmatched' as const;

export function findCorridorForChains(
  sourceChainId: ChainDisplayId,
  destChainId: ChainDisplayId
): CorridorDefinition | null {
  return (
    CORRIDOR_CATALOG.find(
      (c) =>
        c.sourceChainId === sourceChainId && c.destChainId === destChainId
    ) ?? null
  );
}

export function catalogMatchesBackendRoutes(): boolean {
  return CORRIDOR_CATALOG.every((corridor) => {
    const source = chainFamilyForDisplayId(corridor.sourceChainId);
    const destination = chainFamilyForDisplayId(corridor.destChainId);
    const backend = hasBackendRoute(source, destination);
    const catalogExecutable = corridor.catalogAvailability === 'executable';
    return backend === catalogExecutable;
  });
}
