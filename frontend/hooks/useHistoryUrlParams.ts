import { useSearchParams } from 'next/navigation';
import { useMemo } from 'react';

export type TransactionStatus = 'success' | 'failed' | 'pending';

export interface HistoryUrlParams {
  asset?: string;
  status?: TransactionStatus;
}

/**
 * Hook to read history filter parameters from URL query string
 * 
 * Parameters:
 * - asset: Asset code to filter by (e.g., USDC)
 * - status: Transaction status to filter by (success, failed, pending)
 * 
 * Example URLs:
 * /history?asset=USDC
 * /history?status=failed
 * /history?asset=USDC&status=success
 */
export function useHistoryUrlParams(): HistoryUrlParams {
  const searchParams = useSearchParams();

  return useMemo(() => {
    const asset = searchParams.get('asset') || undefined;
    const statusParam = searchParams.get('status');
    const status = (
      statusParam === 'failed' || statusParam === 'pending' 
        ? statusParam 
        : statusParam === 'success' ? 'success' : undefined
    ) as TransactionStatus | undefined;

    return {
      asset: asset?.toUpperCase(),
      status,
    };
  }, [searchParams]);
}
