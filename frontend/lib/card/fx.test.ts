import { describe, it, expect } from 'vitest';
import { requiredUsdcForFiat } from './fx';

describe('requiredUsdcForFiat (CARD-46)', () => {
  it('90.00 EUR at 0.90 rate with 0 balance → 100.00 USDC', () => {
    const result = requiredUsdcForFiat({
      targetFiat: 90,
      currency: 'EUR',
      currentUsdc: 0,
    });
    expect(result.requiredUsdc).toBe(100);
    expect(result.indicative).toBe(true);
  });

  it('90.00 EUR with 20.00 USDC balance → 80.00 USDC', () => {
    const result = requiredUsdcForFiat({
      targetFiat: 90,
      currency: 'EUR',
      currentUsdc: 20,
    });
    expect(result.requiredUsdc).toBe(80);
  });

  it('JPY uses whole-number fiat', () => {
    const result = requiredUsdcForFiat({
      targetFiat: 1500.4,
      currency: 'JPY',
      currentUsdc: 0,
    });
    expect(result.requiredUsdc).toBe(10);
  });

  it('full balance cover → 0 required', () => {
    const result = requiredUsdcForFiat({
      targetFiat: 90,
      currency: 'EUR',
      currentUsdc: 100,
    });
    expect(result.requiredUsdc).toBe(0);
  });
});