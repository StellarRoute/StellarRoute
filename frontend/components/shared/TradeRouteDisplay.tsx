'use client';

import { useState, useEffect } from 'react';
import { RouteVisualization } from './RouteVisualization';
import { SplitRouteVisualization } from './SplitRouteVisualization';
import { PathStep, PriceQuote } from '@/types';
import { SplitRouteData, RouteMetrics } from '@/types/route';

interface TradeRouteDisplayProps {
  quote: PriceQuote | null;
  isLoading?: boolean;
  error?: string;
  className?: string;
}

function isSplitRoute(_path: PathStep[]): boolean {
  // For now, we assume all routes are single-path
  // In the future, the API might return split route information
  return false;
}

function convertToSplitRoute(path: PathStep[]): SplitRouteData {
  // Placeholder conversion - actual implementation would parse API response
  return {
    paths: [
      {
        percentage: 100,
        steps: path,
      },
    ],
    totalOutput: '0',
  };
}

function calculateMetrics(quote: PriceQuote): RouteMetrics {
  // Calculate metrics from quote data
  const hops = Math.max(quote.path.length, 1);
  const totalFees = `${(hops * 0.00001).toFixed(5)} XLM`;
  const totalPriceImpact = quote.priceImpact != null
    ? `${quote.priceImpact}${quote.priceImpact.includes('%') ? '' : '%'}`
    : 'N/A';
  const netOutput = quote.total;
  const averageRate = quote.price;

  return {
    totalFees,
    totalPriceImpact,
    netOutput,
    averageRate,
  };
}

export function TradeRouteDisplay({
  quote,
  isLoading = false,
  error,
  className,
}: TradeRouteDisplayProps) {
  const [displayError, setDisplayError] = useState<string | undefined>(error);
  const breakdown = quote
    ? {
        hops: quote.path.length,
        totalFees: `${(Math.max(quote.path.length, 1) * 0.00001).toFixed(5)} XLM`,
        priceImpact:
          quote.priceImpact != null
            ? `${quote.priceImpact}${quote.priceImpact.includes('%') ? '' : '%'}`
            : 'N/A',
      }
    : undefined;

  useEffect(() => {
    setDisplayError(error);
  }, [error]);

  // Loading state
  if (isLoading) {
    return (
      <RouteVisualization path={[]} isLoading={true} className={className} />
    );
  }

  // Error state
  if (displayError) {
    return (
      <RouteVisualization
        path={[]}
        error={displayError}
        className={className}
      />
    );
  }

  // No quote
  if (!quote) {
    return <RouteVisualization path={[]} className={className} />;
  }

  // Check if split route
  if (isSplitRoute(quote.path)) {
    const splitRoute = convertToSplitRoute(quote.path);
    const metrics = calculateMetrics(quote);
    return (
      <SplitRouteVisualization
        splitRoute={splitRoute}
        metrics={metrics}
        className={className}
      />
    );
  }

  // Regular single-path route
  return (
    <RouteVisualization
      path={quote.path}
      className={className}
      breakdown={breakdown}
    />
  );
}

/**
 * Placeholder example — for live quotes use `useQuoteRefresh` from
 * `@/hooks/useQuoteRefresh` with `stellarRouteClient.getQuote` (see `DemoSwap`).
 * A future WebSocket quote stream can push updates into the same hook.
 */
export function TradeRouteExample() {
  return (
    <div className="space-y-4">
      <TradeRouteDisplay quote={null} />
    </div>
  );
}
