/**
 * Card FX quote fixtures (CARD-10 companion for CARD-48).
 *
 * Rates are indicative fixtures, not a live price feed. StellarRoute never
 * holds keys or card PANs and does not settle against these numbers.
 */

import type { FiatCurrencyCode } from './types';

/** How long a card FX quote may be shown before it is considered stale. */
export const CARD_FX_TTL_SECONDS = 60;

export const CARD_FIAT_CURRENCIES: readonly FiatCurrencyCode[] = [
  'USD',
  'EUR',
  'GBP',
  'NGN',
];

const FIXTURE_BASE_RATES: Record<FiatCurrencyCode, number> = {
  USD: 1,
  EUR: 0.9206,
  GBP: 0.7895,
  NGN: 1538.46,
};

const FIXTURE_RATE_DECIMALS: Record<FiatCurrencyCode, number> = {
  USD: 4,
  EUR: 4,
  GBP: 4,
  NGN: 2,
};

// Basis-point drift applied per revision so a manual refresh visibly
// re-computes the fixture quotes instead of repeating the same numbers.
const FIXTURE_DRIFT_BPS = [0, 6, -4, 9, -7];

export interface CardFxQuote {
  fiatCurrency: FiatCurrencyCode;
  /** Fiat received per 1 USDC, as a decimal string. */
  ratePerUsdc: string;
  indicative: true;
}

export interface CardFxSnapshot {
  retrievedAtMs: number;
  revision: number;
  quotes: CardFxQuote[];
}

export function computeCardFxQuotes(
  retrievedAtMs: number,
  revision = 0,
): CardFxSnapshot {
  const driftBps =
    FIXTURE_DRIFT_BPS[Math.abs(revision) % FIXTURE_DRIFT_BPS.length];

  return {
    retrievedAtMs,
    revision,
    quotes: CARD_FIAT_CURRENCIES.map((fiatCurrency) => ({
      fiatCurrency,
      ratePerUsdc: (
        FIXTURE_BASE_RATES[fiatCurrency] *
        (1 + driftBps / 10_000)
      ).toFixed(FIXTURE_RATE_DECIMALS[fiatCurrency]),
      indicative: true as const,
    })),
  };
}

export function formatCardFxRate(quote: CardFxQuote): string {
  return `1 USDC = ${quote.ratePerUsdc} ${quote.fiatCurrency}`;
}

export function getCardFxQuote(
  snapshot: CardFxSnapshot,
  fiatCurrency: FiatCurrencyCode,
): CardFxQuote {
  const quote = snapshot.quotes.find(
    (candidate) => candidate.fiatCurrency === fiatCurrency,
  );
  if (!quote) {
    throw new Error(`No fixture quote for ${fiatCurrency}.`);
  }
  return quote;
}

export type CardFxMatrixCurrency = 'EUR' | 'GBP' | 'JPY' | 'CAD';

export const CARD_FX_MATRIX_CURRENCIES: readonly CardFxMatrixCurrency[] = [
  'EUR',
  'GBP',
  'JPY',
  'CAD',
];

export const CARD_FX_MATRIX_FIXTURE_DATE = '2026-11-01';

const MATRIX_FIXTURE_RATES: Record<CardFxMatrixCurrency, number> = {
  EUR: FIXTURE_BASE_RATES.EUR,
  GBP: FIXTURE_BASE_RATES.GBP,
  JPY: 151.62,
  CAD: 1.3874,
};

const MATRIX_RATE_DECIMALS: Record<CardFxMatrixCurrency, number> = {
  EUR: 4,
  GBP: 4,
  JPY: 2,
  CAD: 4,
};

const MATRIX_AMOUNT_DECIMALS: Record<CardFxMatrixCurrency, number> = {
  EUR: 2,
  GBP: 2,
  JPY: 0,
  CAD: 2,
};

const MATRIX_FLAGS: Record<CardFxMatrixCurrency, string> = {
  EUR: '🇪🇺',
  GBP: '🇬🇧',
  JPY: '🇯🇵',
  CAD: '🇨🇦',
};

export interface CardFxMatrixRow {
  fiatCurrency: CardFxMatrixCurrency;
  flag: string;
  ratePerUsdc: string;
  convertedAmount: string;
}

export interface CardFxMatrix {
  usdcAmount: number;
  fixtureDate: string;
  indicative: true;
  rows: CardFxMatrixRow[];
}

export interface IndicativeRatesMatrixInput {
  usdcAmount: number;
  targetCurrencies?: readonly CardFxMatrixCurrency[];
}

export function getIndicativeRatesMatrix({
  usdcAmount,
  targetCurrencies = CARD_FX_MATRIX_CURRENCIES,
}: IndicativeRatesMatrixInput): CardFxMatrix {
  const amount = Number.isFinite(usdcAmount) && usdcAmount > 0 ? usdcAmount : 0;

  return {
    usdcAmount: amount,
    fixtureDate: CARD_FX_MATRIX_FIXTURE_DATE,
    indicative: true,
    rows: targetCurrencies.map((fiatCurrency) => {
      const rate = MATRIX_FIXTURE_RATES[fiatCurrency];
      return {
        fiatCurrency,
        flag: MATRIX_FLAGS[fiatCurrency],
        ratePerUsdc: rate.toFixed(MATRIX_RATE_DECIMALS[fiatCurrency]),
        convertedAmount: (amount * rate).toFixed(
          MATRIX_AMOUNT_DECIMALS[fiatCurrency],
        ),
      };
    }),
  };
}
