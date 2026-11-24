import { describe, expect, it } from 'vitest';
import type { QuoteExportPayload } from '@/lib/quote-export';
import {
  buildQuotePrintUrl,
  isQuoteExportPayload,
  parseQuotePrintPayload,
} from './quote-print-payload';

const samplePayload: QuoteExportPayload = {
  exportedAt: '2026-08-28T12:00:00.000Z',
  market: {
    fromAsset: 'XLM',
    toAsset: 'USDC',
    fromAmount: '100',
    expectedToAmount: '9.87',
  },
  pricing: {
    rate: '1 XLM = 0.0987 USDC',
    priceImpactPct: '0.12%',
    minimumReceived: '9.82 USDC',
    networkFee: '0.00001 XLM',
  },
  route: {
    selectedVenue: 'SDEX',
    routeSummary: 'XLM->USDC',
  },
};

describe('isQuoteExportPayload', () => {
  it('accepts a well-formed payload', () => {
    expect(isQuoteExportPayload(samplePayload)).toBe(true);
  });

  it('rejects null/undefined/non-objects', () => {
    expect(isQuoteExportPayload(null)).toBe(false);
    expect(isQuoteExportPayload(undefined)).toBe(false);
    expect(isQuoteExportPayload('a string')).toBe(false);
    expect(isQuoteExportPayload(42)).toBe(false);
  });

  it('rejects a payload missing a required nested field', () => {
    const broken = {
      ...samplePayload,
      market: { ...samplePayload.market, fromAsset: '' },
    };
    expect(isQuoteExportPayload(broken)).toBe(false);
  });

  it('rejects a payload missing a whole section', () => {
    const withoutRoute: Record<string, unknown> = { ...samplePayload };
    delete withoutRoute.route;
    expect(isQuoteExportPayload(withoutRoute)).toBe(false);
  });
});

describe('buildQuotePrintUrl / parseQuotePrintPayload round trip', () => {
  it('round-trips a valid payload', () => {
    const url = buildQuotePrintUrl(samplePayload);
    expect(url.startsWith('/quote/print?data=')).toBe(true);

    const params = new URLSearchParams(url.split('?')[1]);
    const decoded = parseQuotePrintPayload(params.get('data'));
    expect(decoded).toEqual(samplePayload);
  });

  it('round-trips values with unicode characters', () => {
    const withUnicode: QuoteExportPayload = {
      ...samplePayload,
      route: { selectedVenue: 'SDEX \u{1F680}', routeSummary: 'XLM->USDC->\u20AC' },
    };
    const url = buildQuotePrintUrl(withUnicode);
    const params = new URLSearchParams(url.split('?')[1]);
    const decoded = parseQuotePrintPayload(params.get('data'));
    expect(decoded).toEqual(withUnicode);
  });
});

describe('parseQuotePrintPayload', () => {
  it('returns null for a missing param', () => {
    expect(parseQuotePrintPayload(null)).toBeNull();
  });

  it('returns null for garbage input instead of throwing', () => {
    expect(parseQuotePrintPayload('not-valid-base64!!!')).toBeNull();
  });

  it('returns null for valid base64 that is not JSON', () => {
    const encoded = btoa('not json');
    expect(parseQuotePrintPayload(encoded)).toBeNull();
  });

  it('returns null for well-formed JSON that does not match the shape', () => {
    const encoded = btoa(JSON.stringify({ hello: 'world' }));
    expect(parseQuotePrintPayload(encoded)).toBeNull();
  });
});
