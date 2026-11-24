import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { CardExportButton } from './CardExportButton';
import type { CardTransaction } from '@/lib/card/export';

const transactions: CardTransaction[] = [
  {
    timestamp: '2026-11-20T10:00:00.000Z',
    type: 'pos_debit',
    merchant: 'Coffee Shop',
    usdcAmount: '5.48',
    fiatAmount: '5.50',
    currency: 'USD',
    txHash: 'abc123',
  },
];

describe('CardExportButton', () => {
  beforeEach(() => {
    vi.spyOn(URL, 'createObjectURL').mockReturnValue('blob:mock-url');
    vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {});
    vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(
      () => {},
    );
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('is disabled when the card transaction list is empty', () => {
    render(<CardExportButton transactions={[]} />);
    const btn = screen.getByTestId('card-export-button') as HTMLButtonElement;
    expect(btn.disabled).toBe(true);
  });

  it('is enabled when transactions exist', () => {
    render(<CardExportButton transactions={transactions} />);
    const btn = screen.getByTestId('card-export-button') as HTMLButtonElement;
    expect(btn.disabled).toBe(false);
  });

  it('triggers a client-side Blob download without network requests', async () => {
    const fetchSpy = vi
      .spyOn(globalThis, 'fetch')
      .mockRejectedValue(new Error('must not fetch'));
    render(<CardExportButton transactions={transactions} />);
    fireEvent.click(screen.getByTestId('card-export-button'));

    expect(URL.createObjectURL).toHaveBeenCalledTimes(1);
    const blob = vi.mocked(URL.createObjectURL).mock.calls[0][0] as Blob;
    expect(blob).toBeInstanceOf(Blob);
    expect(blob.type).toBe('text/csv;charset=utf-8;');
    const text = await blob.text();
    expect(text).toContain('Timestamp,Type,Description/Merchant');
    expect(text).toContain('Coffee Shop');
    expect(URL.revokeObjectURL).toHaveBeenCalledWith('blob:mock-url');
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it('names the download stellarroute-card-activity-<YYYY-MM-DD>.csv', () => {
    const setAttrSpy = vi.spyOn(
      HTMLAnchorElement.prototype,
      'setAttribute',
    );
    render(<CardExportButton transactions={transactions} />);
    fireEvent.click(screen.getByTestId('card-export-button'));

    const downloadCall = setAttrSpy.mock.calls.find(
      ([name]) => name === 'download',
    );
    expect(downloadCall).toBeDefined();
    const today = new Date().toISOString().slice(0, 10);
    expect(downloadCall?.[1]).toBe(`stellarroute-card-activity-${today}.csv`);
  });
});
