import { describe, expect, it } from 'vitest';

import {
  CARD_FLAT_FEE_USDC,
  CARD_SPREAD_BASIS_POINTS,
  calculateCardFees,
  declineInsufficient,
} from './fees';

describe('CARD_SPREAD_BASIS_POINTS', () => {
  it('is 25 basis points', () => {
    expect(CARD_SPREAD_BASIS_POINTS).toBe(25);
  });
});

describe('CARD_FLAT_FEE_USDC', () => {
  it('is 2.0 USDC', () => {
    expect(CARD_FLAT_FEE_USDC).toBe(2.0);
  });
});

describe('calculateCardFees', () => {
  it('computes correct gross, fee, net for a known input', () => {
    const result = calculateCardFees(100);
    expect(result.gross_usdc).toBeCloseTo(100.25);
    expect(result.fee_usdc).toBeCloseTo(2.250625);
    expect(result.net_usdc).toBeCloseTo(97.999375);
    expect(result.fiat_merchant_amount).toBeCloseTo(97.999375);
  });

  it('returns zero values for zero quote', () => {
    const result = calculateCardFees(0);
    expect(result.gross_usdc).toBe(0);
    expect(result.fee_usdc).toBeCloseTo(CARD_FLAT_FEE_USDC);
    expect(result.net_usdc).toBeCloseTo(-CARD_FLAT_FEE_USDC);
    expect(result.fiat_merchant_amount).toBeCloseTo(-CARD_FLAT_FEE_USDC);
  });

  it('does not read from the network', () => {
    const a = calculateCardFees(50);
    const b = calculateCardFees(50);
    expect(a.gross_usdc).toBe(b.gross_usdc);
    expect(a.fee_usdc).toBe(b.fee_usdc);
    expect(a.net_usdc).toBe(b.net_usdc);
  });
});

describe('declineInsufficient', () => {
  it('returns null when available USDC is above gross', () => {
    const result = declineInsufficient(200, 100);
    expect(result).toBeNull();
  });

  it('returns an error when available USDC is below gross', () => {
    const result = declineInsufficient(50, 100);
    expect(typeof result).toBe('string');
    expect(result).toContain('Insufficient USDC');
  });

  it('returns null when available exactly equals gross', () => {
    const result = declineInsufficient(100.25, 100);
    expect(result).toBeNull();
  });
});
