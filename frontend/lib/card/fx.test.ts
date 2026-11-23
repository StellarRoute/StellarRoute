import { describe, expect, it } from 'vitest';

import {
  CARD_FIAT_CURRENCIES,
  CARD_FX_MATRIX_CURRENCIES,
  CARD_FX_MATRIX_FIXTURE_DATE,
  computeCardFxQuotes,
  getIndicativeRatesMatrix,
} from './fx';

function amounts(usdcAmount: number) {
  return Object.fromEntries(
    getIndicativeRatesMatrix({ usdcAmount }).rows.map((row) => [
      row.fiatCurrency,
      row.convertedAmount,
    ]),
  );
}

describe('getIndicativeRatesMatrix', () => {
  it('converts a single USDC input into every matrix currency', () => {
    const matrix = getIndicativeRatesMatrix({ usdcAmount: 100 });

    expect(matrix.rows.map((row) => row.fiatCurrency)).toEqual([
      'EUR',
      'GBP',
      'JPY',
      'CAD',
    ]);
    expect(amounts(100)).toEqual({
      EUR: '92.06',
      GBP: '78.95',
      JPY: '15162',
      CAD: '138.74',
    });
  });

  it('rounds JPY to 0 decimals and EUR/GBP/CAD to 2 decimals', () => {
    expect(amounts(12.345)).toEqual({
      EUR: '11.36',
      GBP: '9.75',
      JPY: '1872',
      CAD: '17.13',
    });
  });

  it('exposes the fixture rate per USDC for each currency', () => {
    const rates = getIndicativeRatesMatrix({ usdcAmount: 1 }).rows.map(
      (row) => [row.fiatCurrency, row.ratePerUsdc],
    );

    expect(rates).toEqual([
      ['EUR', '0.9206'],
      ['GBP', '0.7895'],
      ['JPY', '151.62'],
      ['CAD', '1.3874'],
    ]);
  });

  it('marks the matrix as indicative and dated', () => {
    const matrix = getIndicativeRatesMatrix({ usdcAmount: 5 });

    expect(matrix.indicative).toBe(true);
    expect(matrix.fixtureDate).toBe(CARD_FX_MATRIX_FIXTURE_DATE);
  });

  it.each([0, -25, Number.NaN, Number.POSITIVE_INFINITY])(
    'treats %p as 0 USDC',
    (usdcAmount) => {
      const matrix = getIndicativeRatesMatrix({ usdcAmount });

      expect(matrix.usdcAmount).toBe(0);
      expect(amounts(usdcAmount)).toEqual({
        EUR: '0.00',
        GBP: '0.00',
        JPY: '0',
        CAD: '0.00',
      });
    },
  );

  it('limits rows to the requested target currencies', () => {
    const matrix = getIndicativeRatesMatrix({
      usdcAmount: 10,
      targetCurrencies: ['JPY', 'EUR'],
    });

    expect(matrix.rows.map((row) => row.fiatCurrency)).toEqual(['JPY', 'EUR']);
  });

  it('keeps the existing card FX quotes unchanged', () => {
    expect(CARD_FIAT_CURRENCIES).toEqual(['USD', 'EUR', 'GBP', 'NGN']);
    expect(CARD_FX_MATRIX_CURRENCIES).toEqual(['EUR', 'GBP', 'JPY', 'CAD']);
    expect(
      computeCardFxQuotes(0).quotes.map((quote) => quote.ratePerUsdc),
    ).toEqual(['1.0000', '0.9206', '0.7895', '1538.46']);
  });
});
