import type { WalletNetwork } from '@/lib/wallet/types';

export const NETWORK_STORAGE_KEY = 'stellarroute.network.selected';

export type AppNetwork = 'testnet' | 'mainnet';

const ALL_NETWORKS: AppNetwork[] = ['testnet', 'mainnet'];

function parseBooleanFlag(value: string | undefined): boolean {
  if (!value) return false;
  const normalized = value.trim().toLowerCase();
  return ['1', 'true', 'yes', 'on'].includes(normalized);
}

export function isMainnetLimitedEnabled(): boolean {
  return parseBooleanFlag(process.env.NEXT_PUBLIC_MAINNET_LIMITED);
}

const MAINNET_WALLET_ALIASES = new Set(['mainnet', 'public', 'pubnet', 'production']);
const TESTNET_WALLET_ALIASES = new Set(['testnet', 'test']);

export function normalizeAppNetwork(
  network: WalletNetwork | null | undefined,
): AppNetwork | null {
  if (!network) return null;
  const key = String(network).trim().toLowerCase();

  if (key === 'testnet' || key === 'mainnet') {
    return key;
  }
  if (MAINNET_WALLET_ALIASES.has(key)) {
    return 'mainnet';
  }
  if (TESTNET_WALLET_ALIASES.has(key)) {
    return 'testnet';
  }
  if (key.includes('public global stellar network')) {
    return 'mainnet';
  }
  if (key.includes('test sdf future network')) {
    return null;
  }
  if (key.includes('test sdf network')) {
    return 'testnet';
  }
  return null;
}

export function getAllowedNetworks(): AppNetwork[] {
  if (isMainnetLimitedEnabled()) {
    return [...ALL_NETWORKS];
  }
  return ['testnet'];
}

export function isNetworkAllowed(network: WalletNetwork): boolean {
  const normalized = normalizeAppNetwork(network);
  if (!normalized) return false;
  return getAllowedNetworks().includes(normalized);
}

export function getDefaultNetwork(): AppNetwork {
  const configured = normalizeAppNetwork(process.env.NEXT_PUBLIC_DEFAULT_NETWORK);
  const allowed = getAllowedNetworks();

  if (configured && allowed.includes(configured)) {
    return configured;
  }

  return 'testnet';
}

export function resolveInitialNetwork(
  persisted: WalletNetwork | null,
  fallback: WalletNetwork = getDefaultNetwork(),
): AppNetwork {
  const candidate = normalizeAppNetwork(persisted) ?? normalizeAppNetwork(fallback);
  if (candidate && isNetworkAllowed(candidate)) {
    return candidate;
  }
  return getDefaultNetwork();
}

export function loadPersistedNetwork(): AppNetwork | null {
  if (typeof window === 'undefined') {
    return null;
  }

  const stored = window.localStorage.getItem(NETWORK_STORAGE_KEY);
  return normalizeAppNetwork(stored);
}

export function persistNetwork(network: WalletNetwork): void {
  if (typeof window === 'undefined') {
    return;
  }

  const normalized = normalizeAppNetwork(network);
  if (!normalized || !isNetworkAllowed(normalized)) {
    return;
  }

  window.localStorage.setItem(NETWORK_STORAGE_KEY, normalized);
}
