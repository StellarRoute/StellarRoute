import { describe, expect, it } from 'vitest';
import { getSubscriptionCopy } from './subscribe-copy';

describe('getSubscriptionCopy', () => {
  it('renders monthly cadence with amount, asset, payee, and max count', () => {
    const copy = getSubscriptionCopy({
      amount: '15',
      asset: 'USDC',
      cadence: 'monthly',
      recipient: 'GABC',
      maxCount: 12,
    });
    expect(copy).toBe('15 USDC monthly to GABC for up to 12 payments. Each payment needs your confirmation.');
  });

  it('renders weekly cadence with amount, asset, payee, and max count', () => {
    const copy = getSubscriptionCopy({
      amount: '10',
      asset: 'XLM',
      cadence: 'weekly',
      recipient: 'GXYZ',
      maxCount: 52,
    });
    expect(copy).toBe('10 XLM weekly to GXYZ for up to 52 payments. Each payment needs your confirmation.');
  });

  it('includes the max payment count in the copy', () => {
    const copy = getSubscriptionCopy({
      amount: '100',
      asset: 'USDC',
      cadence: 'monthly',
      recipient: 'GABC',
      maxCount: 6,
    });
    expect(copy).toContain('up to 6 payments');
  });

  it('states each payment needs confirmation', () => {
    const copy = getSubscriptionCopy({
      amount: '15',
      asset: 'USDC',
      cadence: 'monthly',
      recipient: 'GABC',
      maxCount: 12,
    });
    expect(copy).toContain('Each payment needs your confirmation.');
  });

  it('uses the correct cadence label', () => {
    const monthlyCopy = getSubscriptionCopy({
      amount: '15',
      asset: 'USDC',
      cadence: 'monthly',
      recipient: 'GABC',
      maxCount: 12,
    });
    const weeklyCopy = getSubscriptionCopy({
      amount: '15',
      asset: 'USDC',
      cadence: 'weekly',
      recipient: 'GABC',
      maxCount: 52,
    });
    expect(monthlyCopy).toContain('monthly');
    expect(weeklyCopy).toContain('weekly');
    expect(monthlyCopy).not.toContain('weekly');
    expect(weeklyCopy).not.toContain('monthly');
  });
});
