// frontend/hooks/useTradeActivity.ts
import { useState, useMemo, useEffect } from 'react';
import { TradeRecord } from '../types/trade';
import { useStellarRouteClient } from './useStellarRouteClient';

interface UseTradeActivityProps {
  address?: string;
  initialData?: TradeRecord[];
}

export function useTradeActivity({ address, initialData = [] }: UseTradeActivityProps) {
  const client = useStellarRouteClient();
  const [data, setData] = useState<TradeRecord[]>(initialData);
  const [page, setPage] = useState(1);
  const [sortField, setSortField] = useState<keyof TradeRecord>('timestamp');
  const [sortDirection, setSortDirection] = useState<'desc' | 'asc'>('desc');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  // Configuration knob to toggle between mock/offline and live data fetching (Issue #983)
  const useLiveData = true;

  useEffect(() => {
    if (!address || !useLiveData) {
      setData(initialData);
      setIsLoading(false);
      return;
    }

    let active = true;
    setIsLoading(true);
    setError(null);

    client.getSwapActivity({ limit: 100 })
      .then((res) => {
        if (!active) return;
        
        // Map SwapActivityItem -> TradeRecord
        const mapped: TradeRecord[] = res.swaps
          .filter((swap) => swap.sender.toLowerCase() === address.toLowerCase())
          .map((swap) => ({
            id: swap.event_id,
            txHash: swap.paging_token,
            timestamp: swap.ledger_closed_at ? new Date(swap.ledger_closed_at) : new Date(),
            action: 'SWAP',
            amount: swap.amount_in,
            asset: `${swap.source_asset || 'Unknown'} → ${swap.destination_asset || 'Unknown'}`,
          }));

        setData(mapped);
        setIsLoading(false);
      })
      .catch((err) => {
        if (!active) return;
        console.error('Failed to fetch swap activity:', err);
        setError(err instanceof Error ? err : new Error(String(err)));
        setIsLoading(false);
      });

    return () => {
      active = false;
    };
  }, [address, client, useLiveData, initialData]);

  const itemsPerPage = 10;

  const sortedData = useMemo(() => {
    return [...data].sort((a, b) => {
      const aValue = a[sortField];
      const bValue = b[sortField];

      // Clean, direct numeric comparison for Dates without reassigning types
      if (aValue instanceof Date && bValue instanceof Date) {
        return sortDirection === 'asc' 
          ? aValue.getTime() - bValue.getTime() 
          : bValue.getTime() - aValue.getTime();
      }

      // Safe string fallback comparison
      const aStr = String(aValue);
      const bStr = String(bValue);

      if (aStr < bStr) return sortDirection === 'asc' ? -1 : 1;
      if (aStr > bStr) return sortDirection === 'asc' ? 1 : -1;
      return 0;
    });
  }, [data, sortField, sortDirection]);

  const paginatedData = useMemo(() => {
    const startIndex = (page - 1) * itemsPerPage;
    return sortedData.slice(startIndex, startIndex + itemsPerPage);
  }, [sortedData, page]);

  const totalPages = Math.ceil(data.length / itemsPerPage);

  const handleSort = (field: keyof TradeRecord) => {
    if (field === sortField) {
      setSortDirection((prev) => (prev === 'asc' ? 'desc' : 'asc'));
    } else {
      setSortField(field);
      setSortDirection('desc');
    }
  };

  return {
    data: paginatedData,
    page,
    totalPages,
    setPage,
    handleSort,
    sortField,
    sortDirection,
    isLoading: isLoading || (!address && data.length === 0),
    isEmpty: data.length === 0 && !isLoading,
    error,
  };
}