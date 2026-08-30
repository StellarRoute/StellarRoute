import type { AdapterNetworkId } from '../types';

export function normalizeTronNetwork(
  hostOrLabel: string | undefined | null
): AdapterNetworkId {
  const key = (hostOrLabel ?? '').toString().trim().toLowerCase();
  if (!key) return 'tron:mainnet';

  if (
    key.includes('nile') ||
    key.includes('nileex') ||
    key === 'tron:nile'
  ) {
    return 'tron:nile';
  }
  if (key.includes('shasta') || key === 'tron:shasta') {
    return 'tron:shasta';
  }
  if (
    key.includes('trongrid.io') ||
    key.includes('mainnet') ||
    key === 'tron:mainnet' ||
    key === 'mainnet'
  ) {
    // trongrid.io without nile/shasta → mainnet
    if (key.includes('nile') || key.includes('shasta')) {
      // already handled above
    }
    return 'tron:mainnet';
  }
  if (key.startsWith('tron:')) {
    return key as AdapterNetworkId;
  }
  return `tron:${key}` as AdapterNetworkId;
}

export function networksMatch(
  actual: AdapterNetworkId,
  expected?: AdapterNetworkId
): boolean {
  if (!expected) return true;
  return actual === expected;
}
