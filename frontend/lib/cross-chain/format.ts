import type { ChainDisplayId } from './types';
import { CHAIN_DEFINITIONS } from './corridors';

export function shortenAddress(address: string, visible = 4): string {
  if (address.length <= visible * 2 + 3) return address;
  return `${address.slice(0, visible + 2)}…${address.slice(-visible)}`;
}

export function formatChainPairLabel(
  sourceChainId: ChainDisplayId,
  destChainId: ChainDisplayId
): string {
  const source = CHAIN_DEFINITIONS[sourceChainId].shortLabel;
  const dest = CHAIN_DEFINITIONS[destChainId].shortLabel;
  return `${source} → ${dest}`;
}

export function corridorStatusCopy(
  executable: boolean,
  uncatalogued = false
): string {
  if (uncatalogued) {
    return 'Unsupported pair';
  }
  return executable ? 'Executable corridor' : 'Coming soon — preview only';
}
