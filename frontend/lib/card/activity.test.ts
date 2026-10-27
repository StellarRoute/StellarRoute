import { describe, it, expect } from 'vitest';
import {
  normalizeCardActivity,
  sortActivityNewestFirst,
  formatUsdcSigned,
  stellarExpertUrl,
  type CardActivityItem,
} from './activity';

describe('CARD-44 activity', () => {
  const sample = normalizeCardActivity([
    {
      id: '1',
      type: 'pos_debit',
      createdAt: '2026-10-01T10:00:00Z',
      usdcAmount: -12.5,
      merchantName: 'Cafe',
      fiatAmount: 11.2,
      fiatCurrency: 'EUR',
    },
    {
      id: '2',
      type: 'cctp_deposit',
      createdAt: '2026-10-02T12:00:00Z',
      usdcAmount: 50,
      sourceChain: 'base',
      stellarTxHash: 'STELLARHASH',
      evmTxHash: '0xabc',
    },
  ]);

  it('normalizes pos_debit and cctp_deposit', () => {
    expect(sample).toHaveLength(2);
    expect(sample[0].type).toBe('cctp_deposit'); // newer first
    expect(sample[1].type).toBe('pos_debit');
  });

  it('sorts most recent first', () => {
    const sorted = sortActivityNewestFirst(sample as CardActivityItem[]);
    expect(sorted[0].id).toBe('2');
    expect(sorted[1].id).toBe('1');
  });

  it('formats positive CCTP amounts with +', () => {
    expect(formatUsdcSigned(50)).toBe('+50.00 USDC');
  });

  it('formats pos debit as negative', () => {
    expect(formatUsdcSigned(-12.5)).toBe('-12.50 USDC');
  });

  it('builds Stellar Expert link', () => {
    expect(stellarExpertUrl('STELLARHASH')).toContain(
      'stellar.expert/explorer/public/tx/STELLARHASH',
    );
  });
});