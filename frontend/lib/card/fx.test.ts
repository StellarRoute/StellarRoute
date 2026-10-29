import { describe, expect, it } from 'vitest';

import {
  CARD_FIAT_CURRENCIES,
  CARD_FX_TTL_SECONDS,
  computeCardFxQuotes,
  formatCardFxRate,
  FX_FIXTURE_DATE,
  FX_FIXTURE_RATES,
  getCardFxQuote,
  getCurrencyDecimals,
  getFixtureRate,
  quote,
} from './fx';

describe('card fx quote', () => {
  describe('quote', () => {
    it('yields 9.00 EUR for 10 USDC at the 0.90 fixture rate', () => {
      const result = quote({ usdc: 10, fiat: 'EUR' });

      expect(result.fiatAmount).toBe(9.0);
      expect(result.rate).toBe(0.9);
      expect(result.currency).toBe('EUR');
    });

    it('always includes indicative true', () => {
      const result = quote({ usdc: 10, fiat: 'EUR' });

      expect(result.indicative).toBe(true);
    });

    it('rounds JPY to whole units with no decimal part', () => {
      const result = quote({ usdc: 10, fiat: 'JPY' });

      expect(result.fiatAmount).toBe(1495);
      expect(result.fiatAmount).toBe(Math.trunc(result.fiatAmount));
      expect(result.currency).toBe('JPY');
    });

    it('rounds half away from zero (JPY)', () => {
      // 0.005 USDC * 149.5 = 0.7475 → rounds to 1
      const result = quote({ usdc: 0.005, fiat: 'JPY' });

      expect(result.fiatAmount).toBe(1);
    });

    it('rounds half away from zero (EUR)', () => {
      // 10.005 USDC * 0.90 = 9.0045 → rounds to 9.00 (2 decimals)
      const result = quote({ usdc: 10.005, fiat: 'EUR' });

      expect(result.fiatAmount).toBe(9.0);
    });

    it('returns the fixture rate in the result', () => {
      const result = quote({ usdc: 5, fiat: 'GBP' });

      expect(result.rate).toBe(0.78);
    });

    it('handles zero USDC amount', () => {
      const result = quote({ usdc: 0, fiat: 'EUR' });

      expect(result.fiatAmount).toBe(0);
      expect(result.indicative).toBe(true);
    });

    it('throws for a negative USDC amount', () => {
      expect(() => quote({ usdc: -1, fiat: 'EUR' })).toThrow();
    });

    it('throws for a NaN USDC amount', () => {
      expect(() => quote({ usdc: NaN, fiat: 'EUR' })).toThrow();
    });

    it('throws for an unsupported currency', () => {
      expect(() => quote({ usdc: 10, fiat: 'CAD' })).toThrow(
        'No fixture FX rate for currency "CAD".',
      );
    });

    it('is case-insensitive for currency codes', () => {
      const lower = quote({ usdc: 10, fiat: 'eur' });
      const upper = quote({ usdc: 10, fiat: 'EUR' });

      expect(lower).toEqual(upper);
    });

    it('quotes NGN at the fixture rate', () => {
      const result = quote({ usdc: 1, fiat: 'NGN' });

      expect(result.fiatAmount).toBe(1580.0);
      expect(result.currency).toBe('NGN');
    });

    it('quotes USD 1:1', () => {
      const result = quote({ usdc: 50, fiat: 'USD' });

      expect(result.fiatAmount).toBe(50);
      expect(result.rate).toBe(1.0);
    });
  });

  describe('getCurrencyDecimals', () => {
    it('returns 0 for JPY', () => {
      expect(getCurrencyDecimals('JPY')).toBe(0);
    });

    it('returns 2 for EUR', () => {
      expect(getCurrencyDecimals('EUR')).toBe(2);
    });

    it('defaults to 2 for unknown currencies', () => {
      expect(getCurrencyDecimals('XYZ')).toBe(2);
    });

    it('is case-insensitive', () => {
      expect(getCurrencyDecimals('jpy')).toBe(0);
    });
  });

  describe('getFixtureRate', () => {
    it('returns the rate for a known currency', () => {
      expect(getFixtureRate('EUR')).toBe(0.9);
    });

    it('throws for an unknown currency', () => {
      expect(() => getFixtureRate('CAD')).toThrow(
        'No fixture FX rate for currency "CAD".',
      );
    });
  });

  describe('fixture table', () => {
    it('has a dated fixture date', () => {
      expect(FX_FIXTURE_DATE).toMatch(/^\d{4}-\d{2}-\d{2}$/);
    });

    it('includes EUR, JPY, and NGN', () => {
      const currencies = FX_FIXTURE_RATES.map((r) => r.currency);
      expect(currencies).toContain('EUR');
      expect(currencies).toContain('JPY');
      expect(currencies).toContain('NGN');
    });

    it('sets JPY decimals to 0', () => {
      const jpy = FX_FIXTURE_RATES.find((r) => r.currency === 'JPY');
      expect(jpy?.decimals).toBe(0);
    });

    it('every entry has a positive rate', () => {
      for (const entry of FX_FIXTURE_RATES) {
        expect(entry.rate).toBeGreaterThan(0);
      }
    });
  });

  // ── Upstream CARD-48 snapshot functions (unchanged by CARD-10) ──────────

  describe('computeCardFxQuotes', () => {
    it('returns a snapshot with the expected currencies', () => {
      const snapshot = computeCardFxQuotes(Date.now());

      expect(snapshot.quotes).toHaveLength(CARD_FIAT_CURRENCIES.length);
      for (const q of snapshot.quotes) {
        expect(q.indicative).toBe(true);
        expect(CARD_FIAT_CURRENCIES).toContain(q.fiatCurrency);
      }
    });

    it('applies drift across revisions producing different rates', () => {
      const s0 = computeCardFxQuotes(0, 0);
      const s1 = computeCardFxQuotes(0, 1);

      const eur0 = s0.quotes.find((q) => q.fiatCurrency === 'EUR');
      const eur1 = s1.quotes.find((q) => q.fiatCurrency === 'EUR');
      expect(eur0?.ratePerUsdc).not.toBe(eur1?.ratePerUsdc);
    });

    it('revision 0 has zero drift (base rates)', () => {
      const snapshot = computeCardFxQuotes(0, 0);
      const usd = snapshot.quotes.find((q) => q.fiatCurrency === 'USD');
      expect(usd?.ratePerUsdc).toBe('1.0000');
    });
  });

  describe('formatCardFxRate', () => {
    it('formats a rate string', () => {
      const snapshot = computeCardFxQuotes(0, 0);
      const eur = snapshot.quotes.find((q) => q.fiatCurrency === 'EUR')!;

      expect(formatCardFxRate(eur)).toContain('1 USDC =');
      expect(formatCardFxRate(eur)).toContain('EUR');
    });
  });

  describe('getCardFxQuote', () => {
    it('retrieves a quote by currency', () => {
      const snapshot = computeCardFxQuotes(0, 0);
      const eur = getCardFxQuote(snapshot, 'EUR');

      expect(eur.fiatCurrency).toBe('EUR');
      expect(eur.indicative).toBe(true);
    });

    it('throws for a currency not in the snapshot', () => {
      const snapshot = computeCardFxQuotes(0, 0);

      expect(() => getCardFxQuote(snapshot, 'JPY' as 'EUR')).toThrow();
    });
  });

  describe('CARD_FX_TTL_SECONDS', () => {
    it('is a positive number', () => {
      expect(CARD_FX_TTL_SECONDS).toBeGreaterThan(0);
    });
  });
});
