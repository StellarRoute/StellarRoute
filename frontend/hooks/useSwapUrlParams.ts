import { useSearchParams } from 'next/navigation';
import { useMemo } from 'react';

export interface SwapUrlParams {
  base?: string;
  quote?: string;
  amount?: string;
  type?: 'sell' | 'buy';
}

/**
 * Hook to read swap parameters from URL query string
 * 
 * Parameters:
 * - base: Token to sell (e.g., XLM)
 * - quote: Token to buy (e.g., USDC)
 * - amount: Amount to sell as string (e.g., "100.5")
 * - type: Trade direction "sell" or "buy" (default: "sell")
 * 
 * Example URL: /?base=XLM&quote=USDC&amount=100&type=sell
 */
export function useSwapUrlParams(): SwapUrlParams {
  const searchParams = useSearchParams();

  return useMemo(() => {
    const base = searchParams.get('base') || undefined;
    const quote = searchParams.get('quote') || undefined;
    const amount = searchParams.get('amount') || undefined;
    const typeParam = searchParams.get('type');
    const type = (typeParam === 'buy' ? 'buy' : 'sell') as 'sell' | 'buy';

    return {
      base: base?.toUpperCase(),
      quote: quote?.toUpperCase(),
      amount,
      type,
    };
  }, [searchParams]);
}
