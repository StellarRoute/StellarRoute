// frontend/hooks/useTradeActivity.ts
import { useState, useMemo } from 'react';
import { TradeRecord } from '../types/trade';

interface UseTradeActivityProps {
  address?: string;
  initialData?: TradeRecord[];
}

export function useTradeActivity({ address, initialData = [] }: UseTradeActivityProps) {
  const [data] = useState<TradeRecord[]>(initialData);
  const [page, setPage] = useState(1);
  const [sortField, setSortField] = useState<keyof TradeRecord>('timestamp');
  const [sortDirection, setSortDirection] = useState<'desc' | 'asc'>('desc');
  
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
    isLoading: !address,
    isEmpty: data.length === 0,
  };
}