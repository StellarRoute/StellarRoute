/**

 * CARD-46: Fiat spending target → required USDC calculator.
 * Uses indicative fixture rates only (not live FX).
 */

export type TargetFiatCurrency = 'EUR' | 'GBP' | 'JPY';

/** Fixture rates: units of fiat per 1 USDC (indicative). */
export const FIXTURE_FIAT_PER_USDC: Record<TargetFiatCurrency, number> = {
  EUR: 0.9, // 1 USDC = 0.90 EUR → 90 EUR needs 100 USDC
  GBP: 0.78,
  JPY: 150, // 1 USDC = 150 JPY
};

export interface RequiredUsdcForFiatInput {
  targetFiat: number;
  currency: TargetFiatCurrency;
  currentUsdc: number;
}

export interface RequiredUsdcForFiatResult {
  requiredUsdc: number;
  grossUsdc: number;
  rate: number;
  indicative: true;
}

/**
 * How many extra USDC are needed so the card can support `targetFiat` spending.
 * Rounds USDC up to 2 decimal places. JPY fiat amounts use whole numbers.
 */
export function requiredUsdcForFiat({
  targetFiat,
  currency,
  currentUsdc,
}: RequiredUsdcForFiatInput): RequiredUsdcForFiatResult {
  const rate = FIXTURE_FIAT_PER_USDC[currency];

  const normalizedFiat =
    currency === 'JPY' ? Math.round(targetFiat) : targetFiat;

  const grossUsdc = normalizedFiat / rate;
  const grossRounded = Math.ceil(grossUsdc * 100) / 100;

  const deficit = Math.max(0, grossRounded - currentUsdc);
  const requiredUsdc = Math.ceil(deficit * 100) / 100;

  return {
    requiredUsdc,
    grossUsdc: grossRounded,
    rate,
    indicative: true,
  };
}

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

