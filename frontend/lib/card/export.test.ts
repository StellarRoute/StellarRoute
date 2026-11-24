import { describe, it, expect } from 'vitest';
import {
  exportCardTransactionsToCsv,
  escapeCardCsvField,
  type CardTransaction,
} from './export';

const debit: CardTransaction = {
  timestamp: '2026-11-20T10:00:00.000Z',
  type: 'pos_debit',
  merchant: 'Coffee Shop',
  usdcAmount: '5.48',
  fiatAmount: '5.50',
  currency: 'USD',
  txHash: 'abc123',
};

const topUp: CardTransaction = {
  timestamp: '2026-11-20T11:00:00.000Z',
  type: 'cctp_topup',
  merchant: 'CCTP Bridge Top-Up',
  usdcAmount: '100.00',
  fiatAmount: '100.00',
  currency: 'USD',
  txHash: 'def456',
};

describe('exportCardTransactionsToCsv', () => {
  it('generates the header plus one row per transaction', () => {
    const csv = exportCardTransactionsToCsv([debit, topUp]);
    const lines = csv.trim().split('\n');
    expect(lines[0]).toBe(
      'Timestamp,Type,Description/Merchant,USDC Amount,Fiat Amount,Currency,Transaction Hash',
    );
    expect(lines).toHaveLength(3);
  });

  it('renders POS debits with negative amounts and CCTP deposits with positive amounts', () => {
    const csv = exportCardTransactionsToCsv([debit, topUp]);
    expect(csv).toContain(
      '2026-11-20T10:00:00.000Z,POS Debit,Coffee Shop,-5.48,-5.50,USD,abc123',
    );
    expect(csv).toContain(
      '2026-11-20T11:00:00.000Z,CCTP Top-Up,CCTP Bridge Top-Up,100.00,100.00,USD,def456',
    );
  });

  it('escapes quotes, commas, and special characters in merchant names', () => {
    const tricky: CardTransaction = {
      ...debit,
      merchant: 'Café "Verde", Ltd.\nMain St',
    };
    const csv = exportCardTransactionsToCsv([tricky]);
    expect(csv).toContain('"Café ""Verde"", Ltd.\nMain St"');
  });

  it('returns only the header for an empty list', () => {
    const csv = exportCardTransactionsToCsv([]);
    expect(csv).toBe(
      'Timestamp,Type,Description/Merchant,USDC Amount,Fiat Amount,Currency,Transaction Hash\n',
    );
  });

  it('does not render negative zero for a zero-amount debit', () => {
    const zero: CardTransaction = {
      ...debit,
      usdcAmount: '0.00',
      fiatAmount: '0.00',
    };
    const csv = exportCardTransactionsToCsv([zero]);
    expect(csv).not.toContain('-0.00');
    expect(csv).toContain('0.00,0.00');
  });
});

describe('escapeCardCsvField', () => {
  it('leaves plain values untouched', () => {
    expect(escapeCardCsvField('Coffee Shop')).toBe('Coffee Shop');
  });

  it('quotes values with commas and doubles embedded quotes', () => {
    expect(escapeCardCsvField('a,"b"')).toBe('"a,""b"""');
  });
});
