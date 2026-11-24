export const CARD_SPREAD_BPS = 50;
export const CARD_FLAT_FEE_USDC = 0.5;

/** CARD-11 quote spread (0.25%). Kept beside the settled 50 bps card spread. */
export const CARD_SPREAD_BASIS_POINTS = 25;

/** Flat fee used when pricing a quote before it is grossed up (CARD-11). */
export const CARD_QUOTE_FLAT_FEE_USDC = 2;

export const CARD_ASSET_USDC = 'USDC';

export interface CardFeeQuote {
  grossUsdc: number;
  feeUsdc: number;
  netUsdc: number;
  fiatMerchantAmount: number;
}

export interface FeeCalculationResult {
  gross_usdc: number;
  fee_usdc: number;
  net_usdc: number;
  fiat_merchant_amount: number;
}

export interface CardFeeAvailability {
  insufficient: boolean;
  shortfallUsdc: number;
}

function roundUsdc(value: number): number {
  return Math.round((value + Number.EPSILON) * 10_000_000) / 10_000_000;
}

export function calculateCardFees(grossUsdc: number, fiatPerUsdc: number): CardFeeQuote;
export function calculateCardFees(quoteAmountUsdc: number): FeeCalculationResult;
export function calculateCardFees(
  amount: number,
  fiatPerUsdc?: number,
): CardFeeQuote | FeeCalculationResult {
  if (fiatPerUsdc === undefined) {
    if (!Number.isFinite(amount) || amount < 0) {
      throw new Error('Quote USDC amount must be a non-negative number.');
    }
    const spreadRate = CARD_SPREAD_BASIS_POINTS / 10_000;
    const grossUsdc = amount + amount * spreadRate;
    const feeUsdc = grossUsdc * spreadRate + CARD_QUOTE_FLAT_FEE_USDC;
    const netUsdc = grossUsdc - feeUsdc;
    return {
      gross_usdc: grossUsdc,
      fee_usdc: feeUsdc,
      net_usdc: netUsdc,
      fiat_merchant_amount: netUsdc,
    };
  }

  if (!Number.isFinite(amount) || amount < 0) {
    throw new Error('Gross USDC amount must be a non-negative number.');
  }
  if (!Number.isFinite(fiatPerUsdc) || fiatPerUsdc < 0) {
    throw new Error('Fiat rate must be a non-negative number.');
  }

  const feeUsdc = roundUsdc(amount * (CARD_SPREAD_BPS / 10_000) + CARD_FLAT_FEE_USDC);
  const netUsdc = roundUsdc(Math.max(0, amount - feeUsdc));

  return {
    grossUsdc: roundUsdc(amount),
    feeUsdc,
    netUsdc,
    fiatMerchantAmount: roundUsdc(netUsdc * fiatPerUsdc),
  };
}

export function checkCardFeeAvailability(
  availableUsdc: number,
  grossUsdc: number,
): CardFeeAvailability {
  if (!Number.isFinite(availableUsdc) || availableUsdc < 0) {
    throw new Error('Available USDC must be a non-negative number.');
  }
  if (!Number.isFinite(grossUsdc) || grossUsdc < 0) {
    throw new Error('Gross USDC amount must be a non-negative number.');
  }

  return {
    insufficient: availableUsdc < grossUsdc,
    shortfallUsdc: roundUsdc(Math.max(0, grossUsdc - availableUsdc)),
  };
}

/** Decline a payment when available USDC cannot cover the grossed-up quote. */
export function declineInsufficient(
  availableUsdc: number,
  quoteAmountUsdc: number,
): string | null {
  const spreadRate = CARD_SPREAD_BASIS_POINTS / 10_000;
  const grossUsdc = quoteAmountUsdc + quoteAmountUsdc * spreadRate;
  if (availableUsdc < grossUsdc) {
    return `Insufficient USDC: available ${availableUsdc.toFixed(2)} USDC, gross required ${grossUsdc.toFixed(2)} USDC`;
  }
  return null;
}
