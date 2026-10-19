import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { QuoteInspector, VenueQuote } from './QuoteInspector';

const MOCK_TIMESTAMP = 1713895200;

const mockQuotes: VenueQuote[] = [
  {
    base_asset: { asset_type: 'native' },
    quote_asset: { asset_type: 'credit_alphanum4', asset_code: 'USDC', asset_issuer: 'GA5Z' },
    amount: '1000',
    price: '0.1052',
    total: '105.20',
    quote_type: 'sell',
    timestamp: MOCK_TIMESTAMP,
    venueName: 'Stellar SDEX',
    path: [{ from_asset: { asset_type: 'native' }, to_asset: { asset_type: 'credit_alphanum4', asset_code: 'USDC', asset_issuer: 'GA5Z' }, price: '0.1052', source: 'sdex' }],
  },
  {
    base_asset: { asset_type: 'native' },
    quote_asset: { asset_type: 'credit_alphanum4', asset_code: 'USDC', asset_issuer: 'GA5Z' },
    amount: '1000',
    price: '0.1061',
    total: '106.10', // Optimal
    quote_type: 'sell',
    timestamp: MOCK_TIMESTAMP,
    venueName: 'Soroban AMM (Phoenix)',
    isAggregated: true,
    path: [{ from_asset: { asset_type: 'native' }, to_asset: { asset_type: 'credit_alphanum4', asset_code: 'USDC', asset_issuer: 'GA5Z' }, price: '0.1061', source: 'amm:phoenix' }],
  },
];

describe('QuoteInspector', () => {
  it('renders quotes and highlights the optimal one', () => {
    const handleSelect = vi.fn();
    render(<QuoteInspector quotes={mockQuotes} onSelect={handleSelect} />);
    
    // Both venues are rendered
    expect(screen.getByText('Stellar SDEX')).toBeInTheDocument();
    expect(screen.getByText('Soroban AMM (Phoenix)')).toBeInTheDocument();
    
    // Optimal tag is rendered for the best quote
    expect(screen.getByText('OPTIMAL')).toBeInTheDocument();
    
    // Shows reference tag
    expect(screen.getByText('Reference')).toBeInTheDocument();
  });

  it('handles empty quote payloads gracefully', () => {
    const handleSelect = vi.fn();
    render(<QuoteInspector quotes={[]} onSelect={handleSelect} />);
    
    expect(screen.getByText('0 Sources')).toBeInTheDocument();
    expect(screen.getByText('Select a venue to begin deterministic reconciliation')).toBeInTheDocument();
  });

  it('handles malformed quote payloads gracefully without crashing', () => {
    const handleSelect = vi.fn();
    const malformedQuotes = [
      ...mockQuotes,
      {
        ...mockQuotes[0],
        total: 'invalid_number', // malformed data
        venueName: 'Malformed Venue',
      }
    ];
    render(<QuoteInspector quotes={malformedQuotes} onSelect={handleSelect} />);
    
    // Should render without crashing
    expect(screen.getByText('Malformed Venue')).toBeInTheDocument();
  });

  it('renders an empty state for an empty route-list (no sources)', () => {
    const handleSelect = vi.fn();
    // Empty route-list fixture: an empty array of quotes. This renders the
    // dedicated empty state rather than any crash.
    render(<QuoteInspector quotes={[]} onSelect={handleSelect} />);

    expect(screen.getByText('0 Sources')).toBeInTheDocument();
    expect(
      screen.getByText('Select a venue to begin deterministic reconciliation')
    ).toBeInTheDocument();
  });

  it('renders malformed route hop fixtures without crashing', () => {
    const handleSelect = vi.fn();
    // Malformed payload: a hop with an empty/missing source string must render
    // without throwing. The component guards path access with optional chaining.
    const malformedHopQuotes: VenueQuote[] = [{
      ...mockQuotes[0],
      path: [{
        from_asset: { asset_type: 'native' },
        to_asset: { asset_type: 'credit_alphanum4', asset_code: 'USDC', asset_issuer: 'GA5Z' },
        price: '0.1052',
        source: '',
      }],
    }];

    const { container } = render(
      <QuoteInspector quotes={malformedHopQuotes} onSelect={handleSelect} />
    );

    expect(screen.getByText('Stellar SDEX')).toBeInTheDocument();
    expect(container.querySelector('.animate-pulse')).not.toBeInTheDocument();
  });

  it('renders malformed JSON payloads without throwing', () => {
    const handleSelect = vi.fn();
    // Malformed payload: non-numeric price/total fields plus extra unknown
    // fields. The component must render the row without throwing.
    const malformedPayload: VenueQuote[] = [{
      base_asset: { asset_type: 'native' },
      quote_asset: { asset_type: 'credit_alphanum4', asset_code: 'USDC', asset_issuer: 'GA5Z' },
      amount: 'NOT_A_NUMBER',
      price: 'not-a-price',
      total: 'NaN',
      quote_type: 'sell',
      timestamp: MOCK_TIMESTAMP,
      venueName: 'Broken Venue',
      path: [{
        from_asset: { asset_type: 'native' },
        to_asset: { asset_type: 'credit_alphanum4', asset_code: 'USDC', asset_issuer: 'GA5Z' },
        price: '0.1052',
        source: 'sdex',
      }],
    }];

    const { container } = render(
      <QuoteInspector quotes={malformedPayload} onSelect={handleSelect} />
    );

    // The row still renders rather than throwing.
    expect(screen.getByText('Broken Venue')).toBeInTheDocument();
    expect(container.querySelector('.animate-pulse')).not.toBeInTheDocument();
  });

  it('allows selection and shows reconciliation complete', async () => {
    const user = userEvent.setup();
    const handleSelect = vi.fn();
    render(<QuoteInspector quotes={mockQuotes} onSelect={handleSelect} />);
    
    const selectButtons = screen.getAllByRole('button', { name: /Reconcile|Select/i });
    // The second button is the "Reconcile" one since it's optimal
    await user.click(selectButtons[1]);
    
    expect(handleSelect).toHaveBeenCalledWith(mockQuotes[1]);
    expect(screen.getByText('Reconciliation Finalized')).toBeInTheDocument();
    expect(screen.getAllByText(/Soroban AMM \(Phoenix\)/).length).toBeGreaterThan(0);
  });

  it('renders loading skeleton', () => {
    const handleSelect = vi.fn();
    const { container } = render(<QuoteInspector quotes={mockQuotes} onSelect={handleSelect} isLoading={true} />);
    
    // Verify it renders loading skeleton and not the actual content
    expect(screen.queryByText('Cross-Venue Quote Inspector')).not.toBeInTheDocument();
    expect(container.querySelector('.animate-pulse')).toBeInTheDocument();
  });

  // NOTE: JSON tree expansion, copy actions, and sensitive field redaction 
  // are implemented in DiagnosticsPanel.tsx, not QuoteInspector.tsx. 
  // Tests for those features belong in DiagnosticsPanel.test.tsx.
});
