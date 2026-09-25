/**
 * CARD-11: Fee schedule on top of the FX quote.
 *
 * Pure functions that compute card-specific fees (spread bps + flat fee)
 * on top of an FX quote. All constants live in this module; no network
 * calls are made.
 */

/** Card spread in basis points (0.25% = 25 bps). */
export const CARD_SPREAD_BASIS_POINTS = 25;

/** Flat cross-border fee in USDC. */
export const CARD_FLAT_FEE_USDC = 2.0;

/** USDC asset identifier used throughout the card module. */
export const CARD_ASSET_USDC = 'USDC';

/**
 * Result of a fee calculation.
 */
export interface FeeCalculationResult {
  /** Gross USDC before fees (quote amount + spread). */
  gross_usdc: number;
  /** Total fee in USDC (spread + flat). */
  fee_usdc: number;
  /** Net USDC after deducting fees from gross. */
  net_usdc: number;
  /** Fiat merchant amount derived from net USDC (uses 1:1 USDC→USD assumption). */
  fiat_merchant_amount: number;
}

/**
 * Compute the card fee schedule on top of an FX quote.
 *
 * Applies a spread (in basis points) to the quote amount and adds a
 * flat cross-border fee. Returns gross, fee, net, and the fiat merchant
 * amount.
 *
 * @param quoteAmountUsdc - The quoted USDC amount before fees
 * @returns The fee calculation result
 */
export function calculateCardFees(
  quoteAmountUsdc: number
): FeeCalculationResult {
  const spreadBps = CARD_SPREAD_BASIS_POINTS / 10_000;
  const spreadAmount = quoteAmountUsdc * spreadBps;
  const grossUsdc = quoteAmountUsdc + spreadAmount;
  const feeUsdc = grossUsdc * spreadBps + CARD_FLAT_FEE_USDC;
  const netUsdc = grossUsdc - feeUsdc;

  return {
    gross_usdc: grossUsdc,
    fee_usdc: feeUsdc,
    net_usdc: netUsdc,
    fiat_merchant_amount: netUsdc,
  };
}

/**
 * Determine whether a card payment can proceed given the available USDC.
 *
 * Returns an error message string when available USDC is below the gross
 * amount; otherwise returns null.
 *
 * @param availableUsdc - The user's available USDC balance
 * @param quoteAmountUsdc - The quoted USDC amount before fees
 * @returns null if sufficient, or an error message string
 */
export function declineInsufficient(
  availableUsdc: number,
  quoteAmountUsdc: number
): string | null {
  const spreadBps = CARD_SPREAD_BASIS_POINTS / 10_000;
  const spreadAmount = quoteAmountUsdc * spreadBps;
  const grossUsdc = quoteAmountUsdc + spreadAmount;

  if (availableUsdc < grossUsdc) {
    return `Insufficient USDC: available ${availableUsdc.toFixed(2)} USDC, gross required ${grossUsdc.toFixed(2)} USDC`;
  }

  return null;
}
