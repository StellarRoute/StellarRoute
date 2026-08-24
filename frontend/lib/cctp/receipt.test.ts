import { describe, expect, it } from 'vitest';
import { cctpExplorerUrl, shortenAddress, shortenHash } from './receipt';

describe('cctp receipt helpers', () => {
  it('shortens EVM addresses for display', () => {
    expect(shortenAddress('0xa632da1234567890abcdef1234567890abcdef12')).toBe(
      '0xa632da…cdef12',
    );
  });

  it('builds Sepolia explorer URL for destination mint on stellar→evm', () => {
    expect(cctpExplorerUrl('0xabc', 'dest', 'stellar_to_evm')).toBe(
      'https://sepolia.etherscan.io/tx/0xabc',
    );
  });

  it('shortens long hashes', () => {
    expect(shortenHash('abcdefghijklmnopqrstuvwxyz0123456789')).toMatch(/…/);
  });
});
