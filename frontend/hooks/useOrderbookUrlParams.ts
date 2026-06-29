import { useSearchParams } from 'next/navigation';
import { useMemo } from 'react';

export interface OrderbookUrlParams {
  pair?: string;
}

/**
 * Hook to read orderbook parameters from URL query string
 * 
 * Parameters:
 * - pair: Trading pair in format "BASE/COUNTER" (e.g., XLM/USDC)
 * 
 * Example URL: /orderbook?pair=XLM/USDC
 */
export function useOrderbookUrlParams(): OrderbookUrlParams {
  const searchParams = useSearchParams();

  return useMemo(() => {
    const pair = searchParams.get('pair') || undefined;

    return {
      pair: pair?.toUpperCase(),
    };
  }, [searchParams]);
}
