'use client';

import { useMemo } from 'react';
import { useQuoteRefresh } from './useQuoteRefresh';
import type { Asset, QuoteType } from '@/types';

interface UseQuoteProps {
  fromToken: string; // asset identifier "native" or "CODE:ISSUER"
  toToken: string;
  amount: number | undefined;
  type?: QuoteType;
}

export interface QuoteResult {
  outputAmount: number;
  priceImpact: number;
  route: string[];
  fee: number;
  rate: number;
  loading: boolean;
  error: Error | null;
  isStale: boolean;
  isRecovering: boolean;
  retryAttempt: number;
  refresh: () => void;
  data: import('@/types').PriceQuote | undefined;
}

/**
 * Hook to fetch real-time swap quotes with debouncing and state management.
 * 
 * Adapts the robust useQuoteRefresh hook to the specific swap interface requirements.
 */
export function useQuote({ fromToken, toToken, amount, type = 'sell' }: UseQuoteProps): QuoteResult {
  const { data, loading, error, isStale, isRecovering, retryAttempt, refresh } = useQuoteRefresh(
    fromToken,
    toToken,
    amount,
    type,
    {
      debounceMs: 300,
      autoRefreshIntervalMs: 15000,
    }
  );

  const result = useMemo(() => {
    if (!data) {
      return {
        outputAmount: 0,
        priceImpact: 0,
        route: [],
        fee: 0,
        rate: 0,
      };
    }

    // Parse the data from the PriceQuote response
    const outputAmount = parseFloat(data.total) || 0;
    const priceImpact = parseFloat(data.price_impact || '0') || 0;
    
    // Extract route symbols from path
    const route = data.path.reduce((acc: string[], step) => {
      const fromCode = step.from_asset.asset_code || 'XLM';
      const toCode = step.to_asset.asset_code || 'XLM';
      if (acc.length === 0) {
        acc.push(fromCode);
      }
      acc.push(toCode);
      return acc;
    }, []);

    // Rate: units of toToken per 1 unit of fromToken
    const rate = parseFloat(data.price) || 0;

    // Fees calculation - in a real app this would be more complex and come from the API
    // For now, we'll sum up small illustrative fees if not provided by API
    const fee = 0.001 * (parseFloat(data.amount) || 0);

    return {
      outputAmount,
      priceImpact,
      route,
      fee,
      rate,
    };
  }, [data]);

  return {
    ...result,
    loading,
    error: error instanceof Error ? error : error ? new Error(String(error)) : null,
    isStale,
    isRecovering,
    retryAttempt,
    refresh,
    data,
  };
}
