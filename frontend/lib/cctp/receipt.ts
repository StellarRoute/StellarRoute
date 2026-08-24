import type { CctpDirection } from './types';

export function shortenHash(
  value: string,
  head = 10,
  tail = 8,
): string {
  if (value.length <= head + tail + 1) return value;
  return `${value.slice(0, head)}…${value.slice(-tail)}`;
}

export function shortenAddress(value: string): string {
  if (value.startsWith('0x') && value.length >= 12) {
    return `${value.slice(0, 8)}…${value.slice(-6)}`;
  }
  if (value.length > 18) {
    return `${value.slice(0, 6)}…${value.slice(-6)}`;
  }
  return value;
}

export function cctpExplorerUrl(
  hash: string,
  leg: 'source' | 'dest',
  direction: CctpDirection,
): string {
  const encoded = encodeURIComponent(hash);
  const sourceIsStellar = direction !== 'evm_to_stellar';
  if (leg === 'source') {
    return sourceIsStellar
      ? `https://stellar.expert/explorer/testnet/tx/${encoded}`
      : `https://sepolia.etherscan.io/tx/${encoded}`;
  }
  return sourceIsStellar
    ? `https://sepolia.etherscan.io/tx/${encoded}`
    : `https://stellar.expert/explorer/testnet/tx/${encoded}`;
}
