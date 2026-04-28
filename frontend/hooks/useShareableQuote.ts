import { useCallback, useEffect, useState } from 'react';
import { useRouter, useSearchParams } from 'next/navigation';

export interface ShareableQuoteParams {
  from?: string;
  to?: string;
  amount?: string;
  slippage?: string;
}

const MAX_URL_LENGTH = 2048;
const STALE_THRESHOLD_MS = 60_000; // 1 minute

/**
 * Hook for generating and parsing shareable quote URLs
 * Validates and sanitizes URL params, detects stale quotes
 */
export function useShareableQuote() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const [isStale, setIsStale] = useState(false);

  // Parse current URL params
  const parseParams = useCallback((): ShareableQuoteParams | null => {
    const from = searchParams.get('from');
    const to = searchParams.get('to');
    const amount = searchParams.get('amount');
    const slippage = searchParams.get('slippage');
    const timestamp = searchParams.get('ts');

    // Validate required params
    if (!from || !to || !amount) {
      return null;
    }

    // Sanitize and validate amount
    const sanitizedAmount = amount.replace(/[^0-9.]/g, '');
    if (!sanitizedAmount || parseFloat(sanitizedAmount) <= 0) {
      return null;
    }

    // Sanitize and validate slippage
    let sanitizedSlippage: string | undefined;
    if (slippage) {
      sanitizedSlippage = slippage.replace(/[^0-9.]/g, '');
      const slippageNum = parseFloat(sanitizedSlippage);
      if (slippageNum < 0 || slippageNum > 50) {
        sanitizedSlippage = undefined;
      }
    }

    // Check staleness
    if (timestamp) {
      const ts = parseInt(timestamp, 10);
      if (!isNaN(ts) && Date.now() - ts > STALE_THRESHOLD_MS) {
        setIsStale(true);
      }
    }

    return {
      from,
      to,
      amount: sanitizedAmount,
      slippage: sanitizedSlippage,
    };
  }, [searchParams]);

  // Generate shareable URL
  const generateShareableUrl = useCallback(
    (params: ShareableQuoteParams): string | null => {
      const { from, to, amount, slippage } = params;

      if (!from || !to || !amount) {
        return null;
      }

      const urlParams = new URLSearchParams();
      urlParams.set('from', from);
      urlParams.set('to', to);
      urlParams.set('amount', amount);
      if (slippage) {
        urlParams.set('slippage', slippage);
      }
      urlParams.set('ts', Date.now().toString());

      const baseUrl = typeof window !== 'undefined' ? window.location.origin : '';
      const fullUrl = `${baseUrl}/swap?${urlParams.toString()}`;

      // Check URL length
      if (fullUrl.length > MAX_URL_LENGTH) {
        console.warn('Generated URL exceeds maximum length');
        return null;
      }

      return fullUrl;
    },
    []
  );

  // Apply params to current route
  const applyParams = useCallback(
    (params: ShareableQuoteParams) => {
      const { from, to, amount, slippage } = params;
      const urlParams = new URLSearchParams();

      if (from) urlParams.set('from', from);
      if (to) urlParams.set('to', to);
      if (amount) urlParams.set('amount', amount);
      if (slippage) urlParams.set('slippage', slippage);
      urlParams.set('ts', Date.now().toString());

      router.push(`/swap?${urlParams.toString()}`);
    },
    [router]
  );

  // Clear params
  const clearParams = useCallback(() => {
    router.push('/swap');
    setIsStale(false);
  }, [router]);

  // Refresh quote (update timestamp)
  const refreshQuote = useCallback(() => {
    const current = parseParams();
    if (current) {
      applyParams(current);
      setIsStale(false);
    }
  }, [parseParams, applyParams]);

  return {
    parseParams,
    generateShareableUrl,
    applyParams,
    clearParams,
    refreshQuote,
    isStale,
  };
}
