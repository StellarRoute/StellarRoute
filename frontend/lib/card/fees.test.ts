import { describe, expect, it } from 'vitest';
import {
  calculateCardFees,
  checkCardFeeAvailability,
  declineInsufficient,
  CARD_FLAT_FEE_USDC,
  CARD_SPREAD_BPS,
  CARD_SPREAD_BASIS_POINTS,
} from './fees';

describe('card fees', () => {
  it('calculates the spread, flat fee, net USDC, and fiat merchant amount', () => {
    const quote = calculateCardFees(100, 1_500);

    expect(quote).toEqual({
      grossUsdc: 100,
      feeUsdc: 1,
      netUsdc: 99,
      fiatMerchantAmount: 148_500,
    });
    expect(CARD_SPREAD_BPS).toBe(50);
    expect(CARD_FLAT_FEE_USDC).toBe(0.5);
  });

  it('marks an amount below gross as insufficient without changing the amount', () => {
    expect(checkCardFeeAvailability(9.99, 10)).toEqual({
      insufficient: true,
      shortfallUsdc: 0.01,
    });
  });

  it('does not read fee constants from the network', () => {
    expect(calculateCardFees(10, 2)).toEqual({
      grossUsdc: 10,
      feeUsdc: 0.55,
      netUsdc: 9.45,
      fiatMerchantAmount: 18.9,
    });
  });
});

describe('CARD-11 quote fee schedule', () => {
  it('uses a 25 bps spread when only a quote amount is provided', () => {
    expect(CARD_SPREAD_BASIS_POINTS).toBe(25);
    const result = calculateCardFees(100);
    expect(result.gross_usdc).toBeCloseTo(100.25);
    expect(result.fee_usdc).toBeCloseTo(2.250625);
    expect(result.net_usdc).toBeCloseTo(97.999375);
    expect(result.fiat_merchant_amount).toBeCloseTo(97.999375);
  });

  it('declines when available USDC is below the grossed-up quote', () => {
    expect(declineInsufficient(200, 100)).toBeNull();
    expect(declineInsufficient(50, 100)).toMatch(/Insufficient USDC/);
  });
});
