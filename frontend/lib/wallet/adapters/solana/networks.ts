import type { AdapterNetworkId } from '../types';

export const SOLANA_NETWORKS = {
  mainnet: 'solana:mainnet' as AdapterNetworkId,
  devnet: 'solana:devnet' as AdapterNetworkId,
  testnet: 'solana:testnet' as AdapterNetworkId,
} as const;

const CLUSTER_ALIASES: Record<string, AdapterNetworkId> = {
  'mainnet-beta': SOLANA_NETWORKS.mainnet,
  mainnet: SOLANA_NETWORKS.mainnet,
  'solana:mainnet': SOLANA_NETWORKS.mainnet,
  'solana:mainnet-beta': SOLANA_NETWORKS.mainnet,
  devnet: SOLANA_NETWORKS.devnet,
  'solana:devnet': SOLANA_NETWORKS.devnet,
  testnet: SOLANA_NETWORKS.testnet,
  'solana:testnet': SOLANA_NETWORKS.testnet,
};

export function normalizeSolanaCluster(
  raw: string | null | undefined
): AdapterNetworkId | null {
  if (!raw) return null;
  const key = raw.trim().toLowerCase();
  return CLUSTER_ALIASES[key] ?? null;
}

export function defaultSolanaAppNetwork(): AdapterNetworkId {
  const configured = process.env.NEXT_PUBLIC_SOLANA_NETWORK?.trim();
  const normalized = normalizeSolanaCluster(configured);
  if (normalized) return normalized;
  // Prefer devnet until mainnet cross-chain routes exist.
  return SOLANA_NETWORKS.devnet;
}
