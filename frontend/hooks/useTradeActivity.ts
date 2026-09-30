// frontend/hooks/useTradeActivity.ts
import { useState, useMemo, useEffect } from 'react';
import { TradeRecord } from '../types/trade';
import { useStellarRouteClient } from './useStellarRouteClient';

export interface TradeActivityFilters {
  pair?: string;
  status?: string;
  dateFrom?: string | Date;
  dateTo?: string | Date;
}

interface UseTradeActivityProps {
  address?: string;
  initialData?: TradeRecord[];
  filters?: TradeActivityFilters;
}

export function useTradeActivity({ address, initialData, filters }: UseTradeActivityProps) {
  const client = useStellarRouteClient();
  const [data, setData] = useState<TradeRecord[]>(initialData ?? []);
  const [page, setPage] = useState(1);
  const [sortField, setSortField] = useState<keyof TradeRecord>('timestamp');
  const [sortDirection, setSortDirection] = useState<'desc' | 'asc'>('desc');
  // If initialData was explicitly provided (even as []), use it directly without fetching.
  // Only live-fetch when address is given and no initialData was supplied at all.
  const hasInitialData = initialData !== undefined;
  const [isLoading, setIsLoading] = useState(() => !!address && !hasInitialData);
  const [error, setError] = useState<Error | null>(null);

  // Configuration knob to toggle between mock/offline and live data fetching (Issue #983)
  const useLiveData = true;

  useEffect(() => {
    // No address or caller supplied their own initialData — no live fetch needed
    if (!address || !useLiveData || hasInitialData) {
      setData(initialData ?? []);
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
  }, [address, client, useLiveData, hasInitialData, initialData]);

  // Reset page to 1 whenever filters change
  useEffect(() => {
    setPage(1);
  }, [filters?.pair, filters?.status, filters?.dateFrom, filters?.dateTo]);

  const itemsPerPage = 10;

  const filteredData = useMemo(() => {
    if (!filters) return data;
    const { pair, status, dateFrom, dateTo } = filters;

    return data.filter((item) => {
      // Filter by pair / asset
      if (pair && pair.trim() !== '' && pair.toUpperCase() !== 'ALL') {
        const query = pair.toUpperCase().trim();
        const assetStr = (item.asset || '').toUpperCase();
        if (!assetStr.includes(query)) {
          return false;
        }
      }

      // Filter by status / action
      if (status && status.trim() !== '' && status.toUpperCase() !== 'ALL') {
        const queryStatus = status.toUpperCase().trim();
        const itemAction = (item.action || '').toUpperCase();
        if (itemAction !== queryStatus && !itemAction.includes(queryStatus)) {
          return false;
        }
      }

      // Filter by dateFrom
      if (dateFrom) {
        const fromDate = typeof dateFrom === 'string' ? new Date(dateFrom) : dateFrom;
        if (!isNaN(fromDate.getTime())) {
          // Normalize start of the day
          const start = new Date(fromDate);
          start.setHours(0, 0, 0, 0);
          if (item.timestamp.getTime() < start.getTime()) {
            return false;
          }
        }
      }

      // Filter by dateTo
      if (dateTo) {
        const toDate = typeof dateTo === 'string' ? new Date(dateTo) : dateTo;
        if (!isNaN(toDate.getTime())) {
          // Normalize end of the day
          const end = new Date(toDate);
          end.setHours(23, 59, 59, 999);
          if (item.timestamp.getTime() > end.getTime()) {
            return false;
          }
        }
      }

      return true;
    });
  }, [data, filters]);

  const sortedData = useMemo(() => {
    return [...filteredData].sort((a, b) => {
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
  }, [filteredData, sortField, sortDirection]);

  const paginatedData = useMemo(() => {
    const startIndex = (page - 1) * itemsPerPage;
    return sortedData.slice(startIndex, startIndex + itemsPerPage);
  }, [sortedData, page]);

  const totalPages = Math.ceil(filteredData.length / itemsPerPage);

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
    isLoading,
    isEmpty: filteredData.length === 0 && !isLoading,
    error,
  };
}