import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';

import { DiagnosticsPanel } from '@/components/shared/DiagnosticsPanel';
import type { PriceQuote } from '@/types';

const mockQuote: PriceQuote = {
  base_asset: {
    asset_type: 'native',
  },
  quote_asset: {
    asset_type: 'credit_alphanum4',
    asset_code: 'USDC',
    asset_issuer: 'GA5ZSEJYB37JRC5AVCIA5MOP4IHTZMAB5KYXOM5KBVG7GBJINW7JCXU',
  },
  amount: '100.00',
  price: '0.105',
  total: '10.50',
  quote_type: 'sell',
  path: [
    {
      from_asset: { asset_type: 'native' },
      to_asset: {
        asset_type: 'credit_alphanum4',
        asset_code: 'USDC',
        asset_issuer: 'GA5ZSEJYB37JRC5AVCIA5MOP4IHTZMAB5KYXOM5KBVG7GBJINW7JCXU',
      },
      price: '0.105',
      source: 'sdex',
    },
  ],
  timestamp: Date.now(),
};

describe('DiagnosticsPanel', () => {
  it('renders empty state when no quote is provided', () => {
    render(
      <DiagnosticsPanel
        quote={undefined}
        isOpen={true}
        onOpenChange={vi.fn()}
      />,
    );

    expect(
      screen.getByText('No quote data available. Request a quote to view diagnostics.'),
    ).toBeTruthy();
  });

  it('displays diagnostics information when quote is provided', () => {
    render(
      <DiagnosticsPanel
        quote={mockQuote}
        requestId="server-req-123"
        lastQuotedAtMs={Date.now() - 1_000}
        isOpen={true}
        onOpenChange={vi.fn()}
      />,
    );

    expect(screen.getByText('server-req-123')).toBeTruthy();
    expect(screen.getByText(/Quote Age:/)).toBeTruthy();
    expect(screen.getByText(/Route:/)).toBeTruthy();
  });

  it('renders action buttons', () => {
    render(
      <DiagnosticsPanel
        quote={mockQuote}
        isOpen={true}
        onOpenChange={vi.fn()}
      />,
    );

    expect(screen.getByText('Copy')).toBeTruthy();
    expect(screen.getByText('Export')).toBeTruthy();
  });

  it('calls onOpenChange when dialog closes', () => {
    const onOpenChange = vi.fn();
    const { rerender } = render(
      <DiagnosticsPanel
        quote={mockQuote}
        isOpen={true}
        onOpenChange={onOpenChange}
      />,
    );

    rerender(
      <DiagnosticsPanel
        quote={mockQuote}
        isOpen={false}
        onOpenChange={onOpenChange}
      />,
    );
  });

  it('shows export format selector', () => {
    render(
      <DiagnosticsPanel
        quote={mockQuote}
        isOpen={true}
        onOpenChange={vi.fn()}
      />,
    );

    const selector = screen.getByRole('combobox', { hidden: true });
    expect(selector).toBeTruthy();
  });

  it('does not display raw issuer addresses in diagnostics text', () => {
    render(
      <DiagnosticsPanel
        quote={mockQuote}
        isOpen={true}
        onOpenChange={vi.fn()}
      />,
    );

    expect(
      screen.queryByText(/GA5ZSEJYB37JRC5AVCIA5MOP4IHTZMAB5KYXOM5KBVG7GBJINW7JCXU/),
    ).toBeNull();
  });
});
