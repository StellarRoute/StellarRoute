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