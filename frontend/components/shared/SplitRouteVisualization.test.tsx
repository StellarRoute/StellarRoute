import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { SplitRouteVisualization } from './SplitRouteVisualization';
import { SplitRouteData, RouteMetrics } from '@/types/route';

// ── Mock Data (Aligned with Demo Page Parser) ───────────────────────────────

const splitRouteDataTwoWay: SplitRouteData = {
  paths: [
    {
      percentage: 60,
      steps: [
        {
          from_asset: { asset_type: 'native' },
          to_asset: {
            asset_type: 'credit_alphanum4',
            asset_code: 'USDC',
            asset_issuer: 'GA5Z...',
          },
          price: '0.0850',
          source: 'sdex',
        },
        {
          from_asset: {
            asset_type: 'credit_alphanum4',
            asset_code: 'USDC',
            asset_issuer: 'GA5Z...',
          },
          to_asset: {
            asset_type: 'credit_alphanum4',
            asset_code: 'BTC',
            asset_issuer: 'GBVOL...',
          },
          price: '0.000015',
          source: 'sdex',
        },
      ],
      outputAmount: '0.000765',
    },
    {
      percentage: 40,
      steps: [
        {
          from_asset: { asset_type: 'native' },
          to_asset: {
            asset_type: 'credit_alphanum4',
            asset_code: 'BTC',
            asset_issuer: 'GBVOL...',
          },
          price: '0.00000128',
          source: 'amm:CDQR7XQJUGQP3VXV3YKQJMVXQXQXQXQXQXQXQXQXQXQXQXQXQXQXQXQX',
        },
      ],
      outputAmount: '0.000512',
    },
  ],
  totalOutput: '0.001277',
  totalFees: '0.00001',
  totalPriceImpact: '0.15%',
};

const splitRouteDataThreeWay: SplitRouteData = {
  paths: [
    ...splitRouteDataTwoWay.paths.map((p) => ({
      ...p,
      percentage: p.percentage / 2,
    })),
    {
      percentage: 50,
      steps: [
        {
          from_asset: { asset_type: 'native' },
          to_asset: {
            asset_type: 'credit_alphanum4',
            asset_code: 'BTC',
            asset_issuer: 'GBVOL...',
          },
          price: '0.00000128',
          source: 'sdex',
        },
      ],
      outputAmount: '0.000500',
    },
  ],
  totalOutput: '0.001777',
};

const splitRouteDataSingleVenue: SplitRouteData = {
  paths: [
    {
      percentage: 100,
      steps: [
        {
          from_asset: { asset_type: 'native' },
          to_asset: {
            asset_type: 'credit_alphanum4',
            asset_code: 'USDC',
            asset_issuer: 'GA5Z...',
          },
          price: '0.0850',
          source: 'sdex',
        },
      ],
      outputAmount: '100',
    },
  ],
  totalOutput: '100',
};

const mockMetrics: RouteMetrics = {
  totalFees: '0.00001 BTC',
  totalPriceImpact: '0.15%',
  netOutput: '0.001267 BTC',
  averageRate: '0.00000127',
};

describe('SplitRouteVisualization', () => {
  it('renders a 2-way split layout', () => {
    render(<SplitRouteVisualization splitRoute={splitRouteDataTwoWay} />);
    
    // Role-based assertions for paths
    const paths = screen.getAllByRole('region', { name: /Path \d/i });
    expect(paths).toHaveLength(2);
    
    // Verify percentage badges
    expect(screen.getByText('60% of trade')).toBeInTheDocument();
    expect(screen.getByText('40% of trade')).toBeInTheDocument();
    
    // Total hops calculation (Path 1 has 2 hops, Path 2 has 1 hop = 3 hops)
    expect(screen.getByText('3 Hops')).toBeInTheDocument();
  });

  it('renders a 3-way split layout', () => {
    render(<SplitRouteVisualization splitRoute={splitRouteDataThreeWay} />);
    
    const paths = screen.getAllByRole('region', { name: /Path \d/i });
    expect(paths).toHaveLength(3);
    
    expect(screen.getByText('30% of trade')).toBeInTheDocument();
    expect(screen.getByText('20% of trade')).toBeInTheDocument();
    expect(screen.getByText('50% of trade')).toBeInTheDocument();
  });

  it('renders edge case: single venue 100% split', () => {
    render(<SplitRouteVisualization splitRoute={splitRouteDataSingleVenue} />);
    
    const paths = screen.getAllByRole('region', { name: /Path 1/i });
    expect(paths).toHaveLength(1);
    
    expect(screen.getByText('100% of trade')).toBeInTheDocument();
    expect(screen.queryByText('Split Route')).not.toBeInTheDocument();
    
    // Single hop
    expect(screen.getByText('1 Hop')).toBeInTheDocument();
  });

  it('renders loading state via role assertions', () => {
    render(
      <SplitRouteVisualization
        splitRoute={splitRouteDataTwoWay}
        isLoading={true}
      />
    );
    expect(screen.getByRole('status', { name: 'Loading split route' })).toBeInTheDocument();
  });

  it('renders error state via role assertions', () => {
    render(
      <SplitRouteVisualization
        splitRoute={splitRouteDataTwoWay}
        error="Simulation failed"
      />
    );
    expect(screen.getByRole('alert')).toHaveTextContent('Simulation failed');
  });

  it('renders metrics summary when provided', () => {
    render(
      <SplitRouteVisualization
        splitRoute={splitRouteDataTwoWay}
        metrics={mockMetrics}
      />
    );
    
    const metricsRegion = screen.getByRole('group', { name: 'Route metrics summary' });
    expect(within(metricsRegion).getByText('0.00001 BTC')).toBeInTheDocument();
    expect(within(metricsRegion).getByText('0.15%')).toBeInTheDocument();
    expect(within(metricsRegion).getByText('0.001267 BTC')).toBeInTheDocument();
    expect(within(metricsRegion).getByText('0.00000127')).toBeInTheDocument();
  });

  it('toggles detailed breakdown panel using accessible controls', async () => {
    const user = userEvent.setup();
    render(<SplitRouteVisualization splitRoute={splitRouteDataTwoWay} />);
    
    const toggleButton = screen.getByRole('button', { name: /detailed breakdown/i });
    
    // Verify initial closed state
    expect(toggleButton).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByRole('region', { name: /Detailed per-path hops/i })).not.toBeInTheDocument();
    
    // Click to expand
    await user.click(toggleButton);
    expect(toggleButton).toHaveAttribute('aria-expanded', 'true');
    
    const detailsRegion = screen.getByRole('region', { name: /Detailed per-path hops/i });
    expect(detailsRegion).toBeInTheDocument();
    
    // Includes specific output details in expanded view
    expect(within(detailsRegion).getByText('Output: 0.000765')).toBeInTheDocument();
    expect(within(detailsRegion).getByText('Output: 0.000512')).toBeInTheDocument();
    
    // Click to collapse
    await user.click(toggleButton);
    expect(toggleButton).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByRole('region', { name: /Detailed per-path hops/i })).not.toBeInTheDocument();
  });
});
