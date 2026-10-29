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

// ── CARD-10: Indicative USDC → fiat quote ─────────────────────────────────
//
// The functions above (computeCardFxQuotes / CardFxSnapshot) produce rate
// snapshots for the card spend flow. The `quote()` function below is a
// standalone, additive utility that converts a USDC *amount* into an
// indicative fiat *amount*. It uses its own dated fixture rate table so the
// acceptance-criteria rates (e.g. EUR at 0.90) are stable and independent of
// the drift-based snapshot rates above.

/**
 * FIAT currency codes supported by the standalone `quote()` function.
 * Extends the card program's `FiatCurrencyCode` with JPY (required by
 * CARD-10 acceptance criteria — "JPY rounding has no decimal part").
 */
export type FxQuoteCurrency = FiatCurrencyCode | 'JPY';

export interface FxFixtureRate {
  /** ISO 4217 currency code, e.g. "EUR", "JPY" */
  currency: string;
  /** Number of decimal places for the currency (JPY = 0, most = 2) */
  decimals: number;
  /** Fixture rate: how many fiat units per 1 USDC */
  rate: number;
}

export interface FxQuoteInput {
  /** USDC amount to convert */
  usdc: number;
  /** Target fiat currency code (ISO 4217) */
  fiat: string;
}

export interface FxQuoteResult {
  /** Fiat amount after applying the fixture rate and rounding */
  fiatAmount: number;
  /** The fixture rate used (fiat units per 1 USDC) */
  rate: number;
  /** Always true — this quote is indicative, not a live rate */
  indicative: true;
  /** The fiat currency code */
  currency: string;
}

/** Date the fixture rates were set (ISO 8601). */
export const FX_FIXTURE_DATE = '2026-10-29';

/**
 * Dated fixture rate table for the standalone `quote()` function: fiat units
 * per 1 USDC. These are indicative placeholder rates, not live market rates.
 *
 * These rates are separate from `FIXTURE_BASE_RATES` above so that changes to
 * the snapshot drift logic do not affect `quote()` results and vice-versa.
 */
export const FX_FIXTURE_RATES: readonly FxFixtureRate[] = [
  { currency: 'USD', decimals: 2, rate: 1.0 },
  { currency: 'EUR', decimals: 2, rate: 0.9 },
  { currency: 'GBP', decimals: 2, rate: 0.78 },
  { currency: 'JPY', decimals: 0, rate: 149.5 },
  { currency: 'NGN', decimals: 2, rate: 1580.0 },
];

const quoteRateMap = new Map<string, FxFixtureRate>(
  FX_FIXTURE_RATES.map((r) => [r.currency.toUpperCase(), r]),
);

/**
 * Round a value to the given number of decimal places, half away from zero.
 *
 * Follows the same EPSILON mitigation pattern used in `fees.ts` to avoid
 * floating-point representation errors.
 */
function roundHalfAwayFromZero(value: number, decimals: number): number {
  const factor = Math.pow(10, decimals);
  return Math.round((value + Number.EPSILON) * factor) / factor;
}

/**
 * Get the number of decimal places for a currency in the quote fixture table.
 * Defaults to 2 for currencies not in the table.
 */
export function getCurrencyDecimals(currency: string): number {
  const entry = quoteRateMap.get(currency.toUpperCase());
  return entry ? entry.decimals : 2;
}

/**
 * Get the fixture rate (fiat per 1 USDC) from the quote rate table.
 * Throws if the currency is not in the quote fixture table.
 */
export function getFixtureRate(currency: string): number {
  const entry = quoteRateMap.get(currency.toUpperCase());
  if (!entry) {
    throw new Error(`No fixture FX rate for currency "${currency}".`);
  }
  return entry.rate;
}

/**
 * Produce an indicative FX quote from USDC to a fiat currency.
 *
 * The result always includes `indicative: true` because rates come from a
 * dated fixture table, not a live market feed. The fiat amount is rounded
 * to the currency's standard decimal places, half away from zero.
 *
 * @throws if `usdc` is not a finite non-negative number
 * @throws if `fiat` currency is not in the quote fixture table
 *
 * @example
 * quote({ usdc: 10, fiat: 'EUR' })
 * // → { fiatAmount: 9.00, rate: 0.9, indicative: true, currency: 'EUR' }
 */
export function quote({ usdc, fiat }: FxQuoteInput): FxQuoteResult {
  if (!Number.isFinite(usdc) || usdc < 0) {
    throw new Error('USDC amount must be a non-negative number.');
  }

  const entry = quoteRateMap.get(fiat.toUpperCase());
  if (!entry) {
    throw new Error(`No fixture FX rate for currency "${fiat}".`);
  }

  const fiatAmount = roundHalfAwayFromZero(usdc * entry.rate, entry.decimals);

  return {
    fiatAmount,
    rate: entry.rate,
    indicative: true,
    currency: entry.currency,
  };
}
