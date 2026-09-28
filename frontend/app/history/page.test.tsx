import { render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { TradeActivityTable } from '@/components/shared/TradeActivityTable';

const mockUseTradeActivity = vi.fn();

vi.mock('@/hooks/useTradeActivity', () => ({
  useTradeActivity: (args: unknown) => mockUseTradeActivity(args),
}));

describe('History page trade activity states', () => {
  beforeEach(() => {
    mockUseTradeActivity.mockReset();
  });

  it('shows a loading state while trade activity is pending', () => {
    mockUseTradeActivity.mockReturnValue({
      data: [],
      page: 1,
      totalPages: 1,
      setPage: vi.fn(),
      handleSort: vi.fn(),
      sortField: 'timestamp',
      sortDirection: 'desc',
      isLoading: true,
      isEmpty: false,
      error: null,
    });

    render(<TradeActivityTable address="GTESTADDRESS" />);

    expect(screen.getByTestId('loading-state')).toBeInTheDocument();
  });

  it('shows an empty state when no trade activity exists', () => {
    mockUseTradeActivity.mockReturnValue({
      data: [],
      page: 1,
      totalPages: 1,
      setPage: vi.fn(),
      handleSort: vi.fn(),
      sortField: 'timestamp',
      sortDirection: 'desc',
      isLoading: false,
      isEmpty: true,
      error: null,
    });

    render(<TradeActivityTable address="GTESTADDRESS" />);

    expect(screen.getByTestId('empty-state')).toBeInTheDocument();
  });

  it('shows an error state when trade activity fails to load', async () => {
    mockUseTradeActivity.mockReturnValue({
      data: [],
      page: 1,
      totalPages: 1,
      setPage: vi.fn(),
      handleSort: vi.fn(),
      sortField: 'timestamp',
      sortDirection: 'desc',
      isLoading: false,
      isEmpty: false,
      error: new Error('Network error'),
    });

    render(<TradeActivityTable address="GTESTADDRESS" />);

    await waitFor(() => {
      expect(screen.getByTestId('error-state')).toBeInTheDocument();
    });
  });

  it('renders the populated table structure with the expected rows', () => {
    const rows = [
      {
        id: 'trade-1',
        txHash: '123456789012345',
        timestamp: new Date('2026-01-01T00:00:00Z'),
        action: 'BUY',
        amount: '100',
        asset: 'XLM',
      },
      {
        id: 'trade-2',
        txHash: '234567890123456',
        timestamp: new Date('2026-01-02T00:00:00Z'),
        action: 'SELL',
        amount: '75',
        asset: 'USDC',
      },
    ];

    mockUseTradeActivity.mockReturnValue({
      data: rows,
      page: 1,
      totalPages: 1,
      setPage: vi.fn(),
      handleSort: vi.fn(),
      sortField: 'timestamp',
      sortDirection: 'desc',
      isLoading: false,
      isEmpty: false,
      error: null,
    });

    render(<TradeActivityTable address="GTESTADDRESS" />);

    expect(screen.getAllByTestId('trade-row')).toHaveLength(2);
    expect(screen.getByText('BUY')).toBeInTheDocument();
    expect(screen.getByText('SELL')).toBeInTheDocument();
    expect(screen.getByText('Page 1 of 1')).toBeInTheDocument();
  });
});
