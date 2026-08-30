import { describe, expect, it } from 'vitest';

import {
  buildOfframpQuotePreview,
  buildOfframpRouteSteps,
  findOfframpSource,
  isValidNigerianAccountNumber,
} from './index';

describe('offramp quote preview', () => {
  it('quotes direct Stellar USDC to NGN with fee', () => {
    const asset = findOfframpSource('stellar-usdc')!;
    const quote = buildOfframpQuotePreview({
      asset,
      amount: '100',
      mode: 'direct',
    });

    expect(quote).not.toBeNull();
    expect(quote!.mode).toBe('direct');
    expect(quote!.indicative).toBe(true);
    expect(quote!.feeUsdc).toBe('0.50');
    expect(quote!.netUsdc).toBe('99.50');
    // 99.50 * 1580 = 157,210
    expect(quote!.receiveNgn).toBe('157,210.00');
  });

  it('returns null for empty or invalid amounts', () => {
    const asset = findOfframpSource('stellar-usdc')!;
    expect(
      buildOfframpQuotePreview({ asset, amount: '', mode: 'direct' }),
    ).toBeNull();
    expect(
      buildOfframpQuotePreview({ asset, amount: '-1', mode: 'direct' }),
    ).toBeNull();
  });
});

describe('offramp route steps', () => {
  it('skips bridge for direct Stellar USDC', () => {
    const asset = findOfframpSource('stellar-usdc')!;
    const steps = buildOfframpRouteSteps(asset, 'direct');
    expect(steps.map((s) => s.id)).toEqual([
      'source',
      'settle_usdc',
      'payout',
    ]);
  });

  it('includes CCTP bridge for Ethereum USDC', () => {
    const asset = findOfframpSource('eth-usdc')!;
    const steps = buildOfframpRouteSteps(asset, 'bridge');
    expect(steps.map((s) => s.id)).toEqual([
      'source',
      'bridge',
      'settle_usdc',
      'payout',
    ]);
  });

  it('swaps XLM on Stellar without a bridge hop', () => {
    const asset = findOfframpSource('stellar-xlm')!;
    const steps = buildOfframpRouteSteps(asset, 'bridge');
    expect(steps.map((s) => s.id)).toEqual([
      'source',
      'settle_usdc',
      'payout',
    ]);
    expect(steps[1].label).toMatch(/Swap/i);
  });
});

describe('isValidNigerianAccountNumber', () => {
  it('accepts 10-digit NUBAN', () => {
    expect(isValidNigerianAccountNumber('0123456789')).toBe(true);
  });

  it('rejects short or non-numeric values', () => {
    expect(isValidNigerianAccountNumber('123')).toBe(false);
    expect(isValidNigerianAccountNumber('abcdefghij')).toBe(false);
  });
});
