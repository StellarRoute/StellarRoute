import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { useSearchParams } from 'next/navigation';
import { QuotePrintPageClient } from './QuotePrintPageClient';
import { buildQuotePrintUrl } from './quote-print-payload';
import type { QuoteExportPayload } from '@/lib/quote-export';

vi.mock('next/navigation', () => ({
  useSearchParams: vi.fn(),
}));

const samplePayload: QuoteExportPayload = {
  exportedAt: '2026-08-28T09:32:00.000Z',
  market: {
    fromAsset: 'XLM',
    toAsset: 'USDC',
    fromAmount: '100',
    expectedToAmount: '9.87',
  },
  pricing: {
    rate: '1 XLM = 0.0987 USDC',
    priceImpactPct: '0.12%',
    minimumReceived: '9.82 USDC',
    networkFee: '0.00001 XLM',
  },
  route: {
    selectedVenue: 'SDEX',
    routeSummary: 'XLM->USDC',
  },
};

function mockSearchParams(query: string) {
  const params = new URLSearchParams(query);
  vi.mocked(useSearchParams).mockReturnValue(
    params as unknown as ReturnType<typeof useSearchParams>
  );
}

describe('QuotePrintPageClient', () => {
  it('renders an empty state when there is no data param', () => {
    mockSearchParams('');
    render(<QuotePrintPageClient />);

    expect(screen.getByText('No quote to print')).toBeInTheDocument();
  });

  it('renders an empty state when the data param is invalid', () => {
    mockSearchParams('data=not-valid');
    render(<QuotePrintPageClient />);

    expect(screen.getByText('No quote to print')).toBeInTheDocument();
  });

  it('renders the quote summary for a valid data param', () => {
    const url = buildQuotePrintUrl(samplePayload);
    mockSearchParams(url.split('?')[1]);

    render(<QuotePrintPageClient />);

    expect(screen.getByTestId('quote-print-page')).toBeInTheDocument();
    expect(screen.getByText('StellarRoute')).toBeInTheDocument();
    expect(screen.getByText('100 XLM')).toBeInTheDocument();
    expect(screen.getByText('9.87 USDC')).toBeInTheDocument();
    expect(screen.getByText('SDEX')).toBeInTheDocument();
  });

  it('renders a screen-only Print button', () => {
    const url = buildQuotePrintUrl(samplePayload);
    mockSearchParams(url.split('?')[1]);

    render(<QuotePrintPageClient />);

    expect(
      screen.getByRole('button', { name: 'Print' })
    ).toBeInTheDocument();
  });
});
