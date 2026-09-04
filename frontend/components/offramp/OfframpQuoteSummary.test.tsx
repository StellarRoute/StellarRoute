import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';

import { OfframpQuoteSummary } from './OfframpQuoteSummary';

const NGN_QUOTE_FIXTURE = {
  sourceAmount: '100',
  sourceSymbol: 'USDC',
  usdcAmount: '100',
  feeUsdc: '0.50',
  netUsdc: '99.50',
  rateNgn: 1580,
  receiveNgn: '157,210.00',
  etaLabel: '~2 minutes',
  mode: 'direct' as const,
  indicative: true as const,
};

describe('OfframpQuoteSummary', () => {
  it('renders the empty state when quote is null', () => {
    render(<OfframpQuoteSummary quote={null} />);
    expect(screen.getByTestId('offramp-quote-empty')).toBeInTheDocument();
    expect(
      screen.getByText(/Enter an amount to preview Naira/i),
    ).toBeInTheDocument();
  });

  it('renders a fixture NGN quote with all amounts and codes', () => {
    render(<OfframpQuoteSummary quote={NGN_QUOTE_FIXTURE} />);
    const summary = screen.getByTestId('offramp-quote-summary');
    expect(summary).toBeInTheDocument();

    expect(summary).toHaveTextContent('157,210.00');
    expect(summary).toHaveTextContent('99.50');
    expect(summary).toHaveTextContent('USDC');
    expect(summary).toHaveTextContent('100');
    expect(summary).toHaveTextContent('0.50');
    expect(summary).toHaveTextContent('~2 minutes');
  });

  it('displays the direct path label for direct mode', () => {
    render(<OfframpQuoteSummary quote={NGN_QUOTE_FIXTURE} />);
    expect(screen.getByText(/Direct Stellar USDC/i)).toBeInTheDocument();
  });

  it('displays the bridge path label for bridge mode', () => {
    const bridgeQuote = { ...NGN_QUOTE_FIXTURE, mode: 'bridge' as const };
    render(<OfframpQuoteSummary quote={bridgeQuote} />);
    expect(screen.getByText(/Bridge\s*→\s*offramp/i)).toBeInTheDocument();
  });

  it('does not make any Paycrest network calls', () => {
    const spy = vi.spyOn(globalThis, 'fetch');
    render(<OfframpQuoteSummary quote={NGN_QUOTE_FIXTURE} />);
    expect(spy).not.toHaveBeenCalled();
  });
});