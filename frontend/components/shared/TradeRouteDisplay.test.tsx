import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import type { PathStep, PriceQuote } from '@/types';
import { parseSplitRoute, TradeRouteDisplay } from './TradeRouteDisplay';

const xlm = { asset_type: 'native' as const };
const usdc = {
  asset_type: 'credit_alphanum4' as const,
  asset_code: 'USDC',
  asset_issuer: 'GUSDC',
};
const aqua = {
  asset_type: 'credit_alphanum4' as const,
  asset_code: 'AQUA',
  asset_issuer: 'GAQUA',
};

const splitFixture = {
  base_asset: xlm,
  quote_asset: usdc,
  amount: '100',
  price: '0.1',
  total: '10',
  quote_type: 'sell' as const,
  timestamp: 1_700_000_000_000,
  price_impact: '0.12',
  path: [],
  split_paths: [
    {
      allocation_bps: 6000,
      output_amount: '6',
      path: [
        {
          from_asset: xlm,
          to_asset: usdc,
          price: '0.1',
          source: 'sdex',
        },
      ],
    },
    {
      allocation_bps: 4000,
      output_amount: '4',
      path: [
        {
          from_asset: xlm,
          to_asset: aqua,
          price: '1.5',
          source: 'amm:POOL',
        },
        {
          from_asset: aqua,
          to_asset: usdc,
          price: '0.066',
          source: 'sdex',
        },
      ],
    },
  ],
} satisfies PriceQuote & {
  split_paths: Array<{
    allocation_bps: number;
    output_amount: string;
    path: PathStep[];
  }>;
};

describe('TradeRouteDisplay', () => {
  it('parses recorded API split paths and basis-point allocations', () => {
    expect(parseSplitRoute(splitFixture)).toEqual({
      paths: [
        {
          percentage: 60,
          steps: splitFixture.split_paths[0].path,
          outputAmount: '6',
        },
        {
          percentage: 40,
          steps: splitFixture.split_paths[1].path,
          outputAmount: '4',
        },
      ],
      totalOutput: '10',
    });
  });

  it('renders proportional paths and venue badges from the API quote', () => {
    render(<TradeRouteDisplay quote={splitFixture} />);

    expect(screen.getByText('Split Route')).toBeInTheDocument();
    expect(screen.getByText('60% of trade')).toBeInTheDocument();
    expect(screen.getByText('40% of trade')).toBeInTheDocument();
    expect(screen.getAllByText('SDEX').length).toBeGreaterThan(0);
    expect(screen.getByText('Pool POOL...')).toBeInTheDocument();
  });

  it('uses the single-route visualization when no split payload exists', () => {
    const quote: PriceQuote = {
      ...splitFixture,
      path: splitFixture.split_paths[0].path,
    };
    delete (quote as PriceQuote & { split_paths?: unknown }).split_paths;

    render(<TradeRouteDisplay quote={quote} />);

    expect(screen.queryByText('Split Route')).not.toBeInTheDocument();
  });
});
