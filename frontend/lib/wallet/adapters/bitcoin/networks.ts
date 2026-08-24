import type { AdapterNetworkId } from '../types';
import type { BitcoinNetworkRaw } from './types';

export function normalizeBitcoinNetwork(
  raw: BitcoinNetworkRaw | undefined | null
): AdapterNetworkId {
  const key = (raw ?? '').toString().trim().toLowerCase();
  if (
    key === 'livenet' ||
    key === 'mainnet' ||
    key === 'main' ||
    key === 'bitcoin'
  ) {
    return 'bitcoin:mainnet';
  }
  if (key === 'testnet' || key === 'test') {
    return 'bitcoin:testnet';
  }
  if (key === 'signet') {
    return 'bitcoin:signet';
  }
  return key ? (`bitcoin:${key}` as AdapterNetworkId) : 'bitcoin:mainnet';
}

export function bitcoinNetworkToUnisat(
  network: AdapterNetworkId
): 'livenet' | 'testnet' | null {
  if (network === 'bitcoin:mainnet') return 'livenet';
  if (network === 'bitcoin:testnet') return 'testnet';
  return null;
}

export function networksMatch(
  actual: AdapterNetworkId,
  expected?: AdapterNetworkId
): boolean {
  if (!expected) return true;
  return actual === expected;
}
