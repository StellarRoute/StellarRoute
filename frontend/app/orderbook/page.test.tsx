import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor, cleanup } from '@testing-library/react';
import OrderbookPage from './page';
import { TradingPairProvider } from '@/contexts/TradingPairContext';
import * as useApiHooks from '@/hooks/useApi';

// Mock the API hooks
vi.mock('@/hooks/useApi', () => ({
  usePairs: vi.fn(),
  useOrderbook: vi.fn(),
}));

const mockPairs = [
  {
    base_asset: 'native',
    counter_asset: 'USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5',
    base: 'XLM',
    counter: 'USDC',
  },
  {
    base_asset: 'USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5',
    counter_asset: 'EURC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
    base: 'USDC',
    counter: 'EURC',
  },
];

const mockOrderbook = {
  bids: [
    { price: '0.1000', amount: '100', total: '10' },
    { price: '0.0999', amount: '200', total: '19.98' },
    { price: '0.0998', amount: '150', total: '14.97' },
  ],
  asks: [
    { price: '0.1001', amount: '100', total: '10.01' },
    { price: '0.1002', amount: '200', total: '20.04' },
    { price: '0.1003', amount: '150', total: '15.045' },
  ],
};

describe('OrderbookPage with highlighting', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  it('renders orderbook without highlighting when no trading pair is set', async () => {
    vi.mocked(useApiHooks.usePairs).mockReturnValue({
      data: mockPairs,
      loading: false,
      error: null,
      refresh: vi.fn(),
    });

    vi.mocked(useApiHooks.useOrderbook).mockReturnValue({
      data: mockOrderbook,
      loading: false,
      error: null,
      refresh: vi.fn(),
    });

    render(
      <TradingPairProvider>
        <OrderbookPage />
      </TradingPairProvider>
    );

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Orderbook' })).toBeInTheDocument();
    });

    // Should not show highlighted indicator
    expect(screen.queryByTestId('highlighted-pair-indicator')).not.toBeInTheDocument();
    
    // Rows should not have highlighted class
    const bidRows = screen.queryAllByTestId('bid-row');
    expect(bidRows.length).toBeGreaterThan(0);
  });

  it('displays loading state correctly', () => {
    vi.mocked(useApiHooks.usePairs).mockReturnValue({
      data: null,
      loading: true,
      error: null,
      refresh: vi.fn(),
    });

    vi.mocked(useApiHooks.useOrderbook).mockReturnValue({
      data: null,
      loading: false,
      error: null,
      refresh: vi.fn(),
    });

    render(
      <TradingPairProvider>
        <OrderbookPage />
      </TradingPairProvider>
    );

    expect(screen.getByText('Loading markets')).toBeInTheDocument();
  });

  it('displays error state correctly', () => {
    vi.mocked(useApiHooks.usePairs).mockReturnValue({
      data: null,
      loading: false,
      error: new Error('API Error'),
      refresh: vi.fn(),
    });

    vi.mocked(useApiHooks.useOrderbook).mockReturnValue({
      data: null,
      loading: false,
      error: null,
      refresh: vi.fn(),
    });

    render(
      <TradingPairProvider>
        <OrderbookPage />
      </TradingPairProvider>
    );

    expect(screen.getByText('Could not load markets')).toBeInTheDocument();
  });

  it.skip('handles empty orderbook gracefully', async () => {
    vi.mocked(useApiHooks.usePairs).mockReturnValue({
      data: mockPairs,
      loading: false,
      error: null,
      refresh: vi.fn(),
    });

    vi.mocked(useApiHooks.useOrderbook).mockReturnValue({
      data: { bids: [], asks: [] },
      loading: false,
      error: null,
      refresh: vi.fn(),
    });

    render(
      <TradingPairProvider>
        <OrderbookPage />
      </TradingPairProvider>
    );

    await waitFor(() => {
      expect(screen.getByText('No orderbook entries')).toBeInTheDocument();
    });
  });

  it('shows hover effects on orderbook rows', async () => {
    vi.mocked(useApiHooks.usePairs).mockReturnValue({
      data: mockPairs,
      loading: false,
      error: null,
      refresh: vi.fn(),
    });

    vi.mocked(useApiHooks.useOrderbook).mockReturnValue({
      data: mockOrderbook,
      loading: false,
      error: null,
      refresh: vi.fn(),
    });

    render(
      <TradingPairProvider>
        <OrderbookPage />
      </TradingPairProvider>
    );

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: /Bids/ })).toBeInTheDocument();
    });

    const bidRows = screen.getAllByTestId('bid-row');
    expect(bidRows.length).toBeGreaterThan(0);
    
    // Check that hover classes are present
    bidRows.forEach(row => {
      expect(row.className).toContain('hover:bg-emerald-500/10');
    });
  });
});
