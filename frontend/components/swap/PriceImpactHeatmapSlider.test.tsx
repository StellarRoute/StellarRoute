import { render, screen, fireEvent, act, within, cleanup } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { PriceImpactHeatmapSlider } from './PriceImpactHeatmapSlider';
import { stellarRouteClient } from '@/lib/api/client';
import type { PriceQuote } from '@/types';

// Mock the API client
vi.mock('@/lib/api/client', () => {
  return {
    stellarRouteClient: {
      getQuotesBatch: vi.fn(),
    },
  };
});

describe('PriceImpactHeatmapSlider', () => {
  const mockOnChange = vi.fn();

  afterEach(() => {
    cleanup();
  });

  beforeEach(() => {
    vi.clearAllMocks();
  });

  const createMockQuote = (amount: string, priceImpact: string, price: string): PriceQuote => ({
    base_asset: { asset_type: 'native' },
    quote_asset: { asset_type: 'credit_alphanum4', asset_code: 'USDC', asset_issuer: 'G' },
    amount,
    price,
    total: (parseFloat(amount) * parseFloat(price)).toString(),
    quote_type: 'sell',
    price_impact: priceImpact,
    timestamp: Date.now(),
  });

  it('renders nothing if balance is 0', () => {
    const { container } = render(
      <PriceImpactHeatmapSlider
        fromToken="native"
        toToken="USDC:G"
        balance={0}
        currentAmount="0"
        onChangeAmount={mockOnChange}
      />
    );
    expect(container.firstChild).toBeNull();
  });

  it('fetches batch quotes on mount when tokens and balance are provided', async () => {
    const mockQuotes = Array.from({ length: 10 }, (_, i) =>
      createMockQuote(((i + 1) * 10).toString(), '0.5', '1.2')
    );
    vi.mocked(stellarRouteClient.getQuotesBatch).mockResolvedValueOnce({
      quotes: mockQuotes,
      total: 10,
    });

    render(
      <PriceImpactHeatmapSlider
        fromToken="native"
        toToken="USDC:G"
        balance={100}
        currentAmount="25"
        onChangeAmount={mockOnChange}
      />
    );

    expect(stellarRouteClient.getQuotesBatch).toHaveBeenCalledTimes(1);
    const callArgs = vi.mocked(stellarRouteClient.getQuotesBatch).mock.calls[0][0];
    expect(callArgs).toHaveLength(10);
    expect(callArgs[0]).toEqual({
      base: 'native',
      quote: 'USDC:G',
      amount: 10,
      quote_type: 'sell',
    });
    expect(callArgs[9]).toEqual({
      base: 'native',
      quote: 'USDC:G',
      amount: 100,
      quote_type: 'sell',
    });
  });

  it('renders slider and tracks percentage indicator correctly', async () => {
    vi.mocked(stellarRouteClient.getQuotesBatch).mockResolvedValueOnce({
      quotes: [],
      total: 0,
    });

    render(
      <PriceImpactHeatmapSlider
        fromToken="native"
        toToken="USDC:G"
        balance={100}
        currentAmount="50"
        onChangeAmount={mockOnChange}
      />
    );

    expect(screen.getByText('Amount Slider')).toBeInTheDocument();
    // The percentage indicator and the heatmap label both show '50%', so use getAllByText
    const matches = screen.getAllByText(/50\s*%/);
    expect(matches.length).toBeGreaterThanOrEqual(1);

    const slider = screen.getByTestId('price-impact-heatmap-slider-input');
    expect(slider).toHaveValue('50');
  });

  it('triggers onChangeAmount when sliding', async () => {
    vi.mocked(stellarRouteClient.getQuotesBatch).mockResolvedValueOnce({
      quotes: [],
      total: 0,
    });

    render(
      <PriceImpactHeatmapSlider
        fromToken="native"
        toToken="USDC:G"
        balance={100}
        currentAmount="0"
        onChangeAmount={mockOnChange}
      />
    );

    const slider = screen.getByTestId('price-impact-heatmap-slider-input');
    fireEvent.change(slider, { target: { value: '75' } });

    expect(mockOnChange).toHaveBeenCalledWith('75');
  });

  it('renders heatmap segments and maps colors to price impacts', async () => {
    // Generate quotes with varying price impacts:
    // Index 0: 10% (0.5% impact -> Safe -> green)
    // Index 4: 50% (2.0% impact -> Moderate -> yellow)
    // Index 7: 80% (4.0% impact -> High -> orange)
    // Index 9: 100% (6.5% impact -> Very High -> destructive/red)
    const mockQuotes = Array.from({ length: 10 }, (_, i) => {
      let impact = '0.5';
      if (i === 4) impact = '2.0';
      if (i === 7) impact = '4.0';
      if (i === 9) impact = '6.5';
      return createMockQuote(((i + 1) * 10).toString(), impact, '1.0');
    });

    let resolvePromise!: (val: any) => void;
    const promise = new Promise((resolve) => {
      resolvePromise = resolve;
    });

    vi.mocked(stellarRouteClient.getQuotesBatch).mockReturnValue(promise);

    const { container } = render(
      <PriceImpactHeatmapSlider
        fromToken="native"
        toToken="USDC:G"
        balance={100}
        currentAmount="50"
        onChangeAmount={mockOnChange}
      />
    );

    // Initial loading state: segments should have animate-pulse
    const seg10 = within(container).getByTestId('heatmap-segment-10');
    expect(seg10).toHaveClass('animate-pulse');

    // Resolve quotes and wait for state updates
    await act(async () => {
      resolvePromise({ quotes: mockQuotes, total: 10 });
    });

    const seg10Resolved = within(container).getByTestId('heatmap-segment-10');
    const seg50Resolved = within(container).getByTestId('heatmap-segment-50');
    const seg80Resolved = within(container).getByTestId('heatmap-segment-80');
    const seg100Resolved = within(container).getByTestId('heatmap-segment-100');

    expect(seg10Resolved).not.toHaveClass('animate-pulse');
    expect(seg10Resolved).toHaveClass('bg-emerald-500');
    expect(seg50Resolved).toHaveClass('bg-yellow-500');
    expect(seg80Resolved).toHaveClass('bg-orange-500');
    expect(seg100Resolved).toHaveClass('bg-destructive');
  });

  it('updates amount when a heatmap segment is clicked', async () => {
    vi.mocked(stellarRouteClient.getQuotesBatch).mockResolvedValueOnce({
      quotes: Array.from({ length: 10 }, (_, i) =>
        createMockQuote(((i + 1) * 10).toString(), '0.5', '1.0')
      ),
      total: 10,
    });

    const { container } = render(
      <PriceImpactHeatmapSlider
        fromToken="native"
        toToken="USDC:G"
        balance={100}
        currentAmount="10"
        onChangeAmount={mockOnChange}
      />
    );

    const seg50 = within(container).getByTestId('heatmap-segment-50');
    fireEvent.click(seg50);

    expect(mockOnChange).toHaveBeenCalledWith('50');
  });
});
