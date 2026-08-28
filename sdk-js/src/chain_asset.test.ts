import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { canonicalizeAssetId, looksLikeCaip } from './types.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const vectors = JSON.parse(
  readFileSync(join(__dirname, 'fixtures/chain_asset_vectors.json'), 'utf8'),
) as {
  canonical_round_trips: Array<{ input: string; canonical: string }>;
  must_reject: string[];
  v1_cache_legacy: Array<{ input: string; canonical: string }>;
};

describe('chain-aware asset helpers', () => {
  it('detects chain-scoped prefixes', () => {
    expect(looksLikeCaip('stellar:pubnet/slip44:148')).toBe(true);
    expect(looksLikeCaip('eip155:1/erc20:0xabc')).toBe(true);
    expect(looksLikeCaip('solana:mainnet/token:So1')).toBe(true);
    expect(looksLikeCaip('native')).toBe(false);
    expect(looksLikeCaip('USDC:GA')).toBe(false);
  });

  it('matches shared Rust/JS fixture vectors byte-for-byte', () => {
    for (const item of vectors.canonical_round_trips) {
      expect(canonicalizeAssetId(item.input)).toBe(item.canonical);
    }
    for (const input of vectors.must_reject) {
      expect(() => canonicalizeAssetId(input)).toThrow();
    }
  });

  it('maps legacy stellar ids without colliding across chains', () => {
    const issuer = 'GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5';
    const stellar = canonicalizeAssetId(`USDC:${issuer}`);
    const eth = canonicalizeAssetId(
      'eip155:1/erc20:0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48',
    );
    expect(stellar).toBe(`stellar:pubnet/stellar:USDC:${issuer}`);
    expect(eth).toBe(
      'eip155:1/erc20:0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48',
    );
    expect(stellar).not.toBe(eth);
  });

  it('preserves stellar issuer casing and rejects slip44:native', () => {
    const issuer = 'GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5';
    expect(canonicalizeAssetId(`usdc:${issuer}`)).toContain(issuer);
    expect(() => canonicalizeAssetId('eip155:1/slip44:native')).toThrow();
    expect(() => canonicalizeAssetId('stellar:pubnet/slip44:native')).toThrow();
  });

  it('normalizes XLM/native to stellar pubnet slip44:148', () => {
    expect(canonicalizeAssetId('XLM')).toBe('stellar:pubnet/slip44:148');
    expect(canonicalizeAssetId('native')).toBe('stellar:pubnet/slip44:148');
  });
});
