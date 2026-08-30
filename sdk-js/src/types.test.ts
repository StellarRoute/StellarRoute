import { describe, it, expect } from 'vitest';
import {
  PriceQuote,
  isQuoteStale,
  isQuoteExpired,
  getTimeUntilExpiry,
  DEFAULT_STALENESS_CONFIG,
  QuoteStalenessConfig,
  stellarAssetToCanonical,
  pathStepToSimulationHop,
  API_ERROR_CODES,
} from './types.js';

describe('Quote staleness utilities', () => {
  const createQuote = (overrides: Partial<PriceQuote> = {}): PriceQuote => ({
    base_asset: { asset_type: 'native' },
    quote_asset: { asset_type: 'credit_alphanum4' as const, asset_code: 'USDC', asset_issuer: 'test' },
    amount: '100',
    price: '0.12',
    total: '12',
    quote_type: 'sell',
    path: [],
    timestamp: Date.now(),
    ...overrides,
  });

  describe('isQuoteStale', () => {
    it('should return false for fresh quotes', () => {
      const quote = createQuote({ timestamp: Date.now() });
      expect(isQuoteStale(quote)).toBe(false);
    });

    it('should return true for old quotes', () => {
      const oldTimestamp = Date.now() - 60000; // 60 seconds ago
      const quote = createQuote({ timestamp: oldTimestamp });
      expect(isQuoteStale(quote)).toBe(true);
    });

    it('should respect custom max_age_seconds config', () => {
      const config: QuoteStalenessConfig = { max_age_seconds: 10 };
      
      // Quote 5 seconds old - should be fresh
      const freshQuote = createQuote({ timestamp: Date.now() - 5000 });
      expect(isQuoteStale(freshQuote, config)).toBe(false);
      
      // Quote 15 seconds old - should be stale
      const staleQuote = createQuote({ timestamp: Date.now() - 15000 });
      expect(isQuoteStale(staleQuote, config)).toBe(true);
    });

    it('should use default config when not provided', () => {
      const quote = createQuote({ timestamp: Date.now() });
      expect(isQuoteStale(quote)).toBe(false);
      expect(DEFAULT_STALENESS_CONFIG.max_age_seconds).toBe(30);
    });
  });

  describe('isQuoteExpired', () => {
    it('should return false when no expires_at field', () => {
      const quote = createQuote();
      expect(isQuoteExpired(quote)).toBe(false);
    });

    it('should return false for future expiry', () => {
      const futureExpiry = Date.now() + 30000;
      const quote = createQuote({ expires_at: futureExpiry });
      expect(isQuoteExpired(quote)).toBe(false);
    });

    it('should return true for past expiry', () => {
      const pastExpiry = Date.now() - 1000;
      const quote = createQuote({ expires_at: pastExpiry });
      expect(isQuoteExpired(quote)).toBe(true);
    });
  });

  describe('getTimeUntilExpiry', () => {
    it('should return null when no expires_at field', () => {
      const quote = createQuote();
      expect(getTimeUntilExpiry(quote)).toBeNull();
    });

    it('should return remaining seconds for future expiry', () => {
      const futureExpiry = Date.now() + 5000;
      const quote = createQuote({ expires_at: futureExpiry });
      const remaining = getTimeUntilExpiry(quote);
      expect(remaining).toBeGreaterThanOrEqual(4);
      expect(remaining).toBeLessThanOrEqual(5);
    });

    it('should return 0 for past expiry', () => {
      const pastExpiry = Date.now() - 1000;
      const quote = createQuote({ expires_at: pastExpiry });
      expect(getTimeUntilExpiry(quote)).toBe(0);
    });
  });

  describe('PriceQuote with expiry fields', () => {
    it('should accept optional expires_at field', () => {
      const quote: PriceQuote = createQuote({
        expires_at: Date.now() + 30000,
      });
      expect(quote.expires_at).toBeDefined();
    });

    it('should accept optional source_timestamp field', () => {
      const quote: PriceQuote = createQuote({
        source_timestamp: Date.now(),
      });
      expect(quote.source_timestamp).toBeDefined();
    });

    it('should accept optional ttl_seconds field', () => {
      const quote: PriceQuote = createQuote({
        ttl_seconds: 30,
      });
      expect(quote.ttl_seconds).toBe(30);
    });
  });
});

describe('stellarAssetToCanonical / pathStepToSimulationHop', () => {
  it('maps native and credit assets', () => {
    expect(stellarAssetToCanonical({ asset_type: 'native' })).toBe('native');
    expect(
      stellarAssetToCanonical({
        asset_type: 'credit_alphanum4',
        asset_code: 'USDC',
        asset_issuer: 'GDUK',
      }),
    ).toBe('USDC:GDUK');
  });

  it('builds simulation hops from PathStep', () => {
    expect(
      pathStepToSimulationHop({
        from_asset: { asset_type: 'native' },
        to_asset: {
          asset_type: 'credit_alphanum4',
          asset_code: 'USDC',
          asset_issuer: 'GDUK',
        },
        price: '0.99',
        source: 'sdex',
        fee_bps: 30,
      }),
    ).toEqual({
      from_asset: 'native',
      to_asset: 'USDC:GDUK',
      source: 'sdex',
      fee_bps: 30,
      price: '0.99',
    });
  });
});

describe('API_ERROR_CODES classic swap codes', () => {
  it('includes dependency and unsupported classic codes', () => {
    expect(API_ERROR_CODES).toEqual(
      expect.arrayContaining([
        'dependency_unavailable',
        'unsupported_execution_mode',
        'unsupported_route',
        'quote_expired',
        'duplicate_quote',
      ]),
    );
  });
});
