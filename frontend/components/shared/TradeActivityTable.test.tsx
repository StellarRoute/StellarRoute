// frontend/components/shared/TradeActivityTable.test.tsx
import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { TradeActivityTable } from './TradeActivityTable';
import { TradeRecord } from '../../types/trade';

const mockFeatureFlags = {
  transaction_history: false,
};

vi.mock('../../hooks/useFeatureFlag', () => ({
  useFeatureFlag: (flag: string) => ({
    enabled: mockFeatureFlags[flag as keyof typeof mockFeatureFlags] ?? false,
    loading: false,
  }),
}));

const getSwapActivity = vi.fn();

vi.mock('../../hooks/useStellarRouteClient', () => ({
  useStellarRouteClient: () => ({
    getSwapActivity,
  }),
}));

const mockData: TradeRecord[] = [
  {
    id: '1',
    txHash: '123456789012345',
    timestamp: new Date(2026, 0, 1),
    action: 'BUY',
    amount: '100',
    asset: 'XLM',
  },
  {
    id: '2',
    txHash: '678901234567890',
    timestamp: new Date(2026, 0, 2),
    action: 'SELL',
    amount: '50',
    asset: 'USDC',
  },
];

describe('TradeActivityTable component', () => {
  beforeEach(() => {
    getSwapActivity.mockReset();
    mockFeatureFlags.transaction_history = false;
  });

  it('should render offline initialData when address is missing', () => {
    render(<TradeActivityTable initialData={mockData} />);
    expect(screen.getAllByTestId('trade-row').length).toBe(2);
    expect(screen.getByText('BUY')).toBeDefined();
    // Filters should NOT be rendered when flag is false
    expect(screen.queryByTestId('history-filters')).toBeNull();
  });

  it('should render empty state after live fetch returns no swaps', async () => {
    getSwapActivity.mockResolvedValue({ swaps: [] });
    render(<TradeActivityTable address="GTESTADDRESS" initialData={[]} />);

    await waitFor(() => {
      expect(screen.getByTestId('empty-state')).toBeDefined();
    });
  });

  it('should render table rows from live swap activity', async () => {
    getSwapActivity.mockResolvedValue({
      swaps: [
        {
          event_id: '1',
          paging_token: '123456789012345',
          ledger_closed_at: '2026-01-01T00:00:00Z',
          sender: 'GTESTADDRESS',
          amount_in: '100',
          source_asset: 'XLM',
          destination_asset: 'USDC',
        },
      ],
    });

    render(<TradeActivityTable address="GTESTADDRESS" />);

    await waitFor(() => {
      expect(screen.getAllByTestId('trade-row').length).toBe(1);
    });
    expect(screen.getByText('SWAP')).toBeDefined();
  });

  it('should render filter controls when transaction_history flag is enabled', () => {
    mockFeatureFlags.transaction_history = true;
    render(<TradeActivityTable initialData={mockData} />);

    expect(screen.getByTestId('history-filters')).toBeDefined();
    expect(screen.getByTestId('filter-pair')).toBeDefined();
    expect(screen.getByTestId('filter-status')).toBeDefined();
    expect(screen.getByTestId('filter-date-from')).toBeDefined();
    expect(screen.getByTestId('filter-date-to')).toBeDefined();
  });

  it('should filter trades interactively when filters change and reset them', () => {
    mockFeatureFlags.transaction_history = true;
    render(<TradeActivityTable initialData={mockData} />);

    expect(screen.getAllByTestId('trade-row').length).toBe(2);

    // Filter by pair
    const pairInput = screen.getByTestId('filter-pair');
    fireEvent.change(pairInput, { target: { value: 'XLM' } });
    const rows = screen.getAllByTestId('trade-row');
    expect(rows.length).toBe(1);
    expect(rows[0].textContent).toContain('BUY');
    expect(rows[0].textContent).toContain('XLM');

    // Reset button should appear
    const resetButton = screen.getByTestId('filter-reset-button');
    fireEvent.click(resetButton);
    expect(screen.getAllByTestId('trade-row').length).toBe(2);
  });
});
