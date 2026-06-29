'use client';

import { useEffect, useState } from 'react';
import { RouteVisualization } from './RouteVisualization';
import { SplitRouteVisualization } from './SplitRouteVisualization';
import type { PriceQuote } from '@/types';
import type {
  ApiSplitPath,
  RouteMetrics,
  SplitRouteData,
  SplitRouteQuotePayload,
} from '@/types/route';

interface TradeRouteDisplayProps {
  quote: PriceQuote | null;
  isLoading?: boolean;
  error?: string;
  className?: string;
}

function getAllocation(path: ApiSplitPath): number | null {
  if (typeof path.allocation_bps === 'number') {
    return path.allocation_bps / 100;
  }
  if (typeof path.allocation_percent === 'number') {
    return path.allocation_percent;
  }
  if (typeof path.percentage === 'number') {
    return path.percentage;
  }
  if (typeof path.weight === 'number') {
    return path.weight <= 1 ? path.weight * 100 : path.weight;
  }
  return null;
}

/**
 * Converts the split route section of a live API quote into visualization
 * data. Invalid/empty paths are ignored and allocations are normalized to 100.
 */
export function parseSplitRoute(quote: PriceQuote): SplitRouteData | null {
  const payload = quote as PriceQuote & SplitRouteQuotePayload;
  const rawPaths =
    payload.split_paths ?? payload.splitPaths ?? payload.routes ?? [];
  const validPaths = rawPaths
    .map((path) => ({
      raw: path,
      steps: path.steps ?? path.path ?? [],
    }))
    .filter(({ steps }) => steps.length > 0);

  if (validPaths.length < 2) {
    return null;
  }

  const suppliedAllocations = validPaths.map(({ raw }) => getAllocation(raw));
  const suppliedTotal = suppliedAllocations.reduce<number>(
    (total, allocation) => total + (allocation ?? 0),
    0
  );
  const missingCount = suppliedAllocations.filter(
    (allocation) => allocation === null
  ).length;
  const missingAllocation =
    missingCount > 0 ? Math.max(0, 100 - suppliedTotal) / missingCount : 0;
  const allocations = suppliedAllocations.map(
    (allocation) => allocation ?? missingAllocation
  );
  const allocationTotal = allocations.reduce<number>(
    (total, allocation) => total + allocation,
    0
  );

  if (allocationTotal <= 0) {
    return null;
  }

  return {
    paths: validPaths.map(({ raw, steps }, index) => ({
      percentage: Math.round((allocations[index] / allocationTotal) * 100),
      steps,
      outputAmount: raw.output_amount ?? raw.outputAmount,
    })),
    totalOutput: quote.total,
  };
}

function calculateMetrics(quote: PriceQuote): RouteMetrics {
  // Calculate metrics from quote data
  const hops = Math.max(quote.path.length, 1);
  const totalFees = `${(hops * 0.00001).toFixed(5)} XLM`;
  const totalPriceImpact =
    quote.priceImpact != null
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

  const splitRoute = parseSplitRoute(quote);
  if (splitRoute) {
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
 * `@/hooks/useQuoteRefresh` with `stellarRouteClient.getQuote`.
 * A future WebSocket quote stream can push updates into the same hook.
 */
export function TradeRouteExample() {
  return (
    <div className="space-y-4">
      <TradeRouteDisplay quote={null} />
    </div>
  );
}
