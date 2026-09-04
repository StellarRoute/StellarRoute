import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor, cleanup } from '@testing-library/react';

import { HistoryPageClient } from './HistoryPageClient';

const getSwapActivity = vi.fn();

// Stable reference, matching the real hook: `useStellarRouteClient` memoises on
// `network`, so the client identity must not change between renders or every
// consumer's fetch effect would re-run.
const mockClient = { getSwapActivity };

vi.mock('@/hooks/useStellarRouteClient', () => ({
  useStellarRouteClient: () => mockClient,
}));

const ADDRESS = 'GBRPDEJSTXWHLT2YTIU6X7E3E5B5O3N4CUXOAT76O4Q4WUPTFBJMDSZH';

function swap(overrides: Record<string, unknown> = {}) {
  return {
    event_id: 'evt-1',
    contract_id: 'contract-1',
    ledger: 100,
    ledger_closed_at: '2026-01-01T00:00:00Z',
    paging_token: 'abc123def456',
    sender: ADDRESS,
    amount_in: '100.0000000',
    amount_out: '10.5000000',
    fee_amount: '0.0000100',
    route: null,
    source_asset: 'XLM',
    destination_asset: 'USDC',
    ...overrides,
  };
}

describe('HistoryPageClient', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  it('shows an illustrated empty state when the wallet has no swaps', async () => {
    getSwapActivity.mockResolvedValue({ swaps: [] });

    render(<HistoryPageClient address={ADDRESS} />);

    await waitFor(() => {
      expect(screen.getByText('No transactions yet')).toBeInTheDocument();
    });

    const illustration = document.querySelector(
      'img[src="/illustrations/empty-history.svg"]'
    );
    expect(illustration).toBeInTheDocument();
    expect(illustration).toHaveAttribute('aria-hidden', 'true');
    expect(illustration).toHaveAttribute('alt', '');

    expect(
      screen.getByRole('link', { name: /make your first swap/i })
    ).toHaveAttribute('href', '/swap');
  });

  it('treats swaps from other senders as empty', async () => {
    getSwapActivity.mockResolvedValue({
      swaps: [swap({ sender: 'GSOMEONEELSE' })],
    });

    render(<HistoryPageClient address={ADDRESS} />);

    await waitFor(() => {
      expect(screen.getByText('No transactions yet')).toBeInTheDocument();
    });
  });

  it('renders the activity table when swaps exist, without a second fetch', async () => {
    getSwapActivity.mockResolvedValue({ swaps: [swap()] });

    render(<HistoryPageClient address={ADDRESS} />);

    await waitFor(() => {
      expect(screen.getByTestId('trade-row')).toBeInTheDocument();
    });

    expect(screen.queryByText('No transactions yet')).not.toBeInTheDocument();
    // The page owns the fetch and hands rows down via `initialData`.
    expect(getSwapActivity).toHaveBeenCalledTimes(1);
  });

  it('shows an error state when the activity request fails', async () => {
    getSwapActivity.mockRejectedValue(new Error('upstream down'));

    render(<HistoryPageClient address={ADDRESS} />);

    await waitFor(() => {
      expect(screen.getByText('Could not load history')).toBeInTheDocument();
    });
    expect(
      document.querySelector('img[src="/illustrations/empty-history.svg"]')
    ).not.toBeInTheDocument();
  });
});
