import { describe, it, expect } from 'vitest';
import {
  categoryFromMcc,
  buildCategoryBreakdown,
} from './categories';

describe('CARD-53 categories', () => {
  it('maps known MCCs', () => {
    expect(categoryFromMcc(5812)).toBe('Dining');
    expect(categoryFromMcc(5411)).toBe('Groceries');
    expect(categoryFromMcc(3001)).toBe('Travel');
    expect(categoryFromMcc(7372)).toBe('Tech/SaaS');
  });

  it('unknown MCC → General', () => {
    expect(categoryFromMcc(9999)).toBe('General');
    expect(categoryFromMcc(undefined)).toBe('General');
  });

  it('excludes CCTP deposits from breakdown', () => {
    const buckets = buildCategoryBreakdown([
      { type: 'cctp_deposit', usdcAmount: 100, mcc: 5812 },
      { type: 'pos_debit', usdcAmount: -20, mcc: 5812, status: 'settled' },
    ]);
    expect(buckets).toHaveLength(1);
    expect(buckets[0].category).toBe('Dining');
    expect(buckets[0].totalUsdc).toBe(20);
    expect(buckets[0].percent).toBe(100);
  });

  it('computes percentages across categories', () => {
    const buckets = buildCategoryBreakdown([
      { type: 'pos_debit', usdcAmount: -30, mcc: 5812, status: 'settled' },
      { type: 'pos_debit', usdcAmount: -70, mcc: 5411, status: 'settled' },
    ]);
    const sum = buckets.reduce((s, b) => s + b.percent, 0);
    expect(Math.round(sum)).toBe(100);
  });

  it('empty when no debits', () => {
    expect(buildCategoryBreakdown([])).toEqual([]);
  });
});