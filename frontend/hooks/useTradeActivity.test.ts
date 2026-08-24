// frontend/src/hooks/useTradeActivity.test.ts
import { describe, it, expect } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useTradeActivity } from './useTradeActivity';
import { TradeRecord } from '../types/trade';

const mockData: TradeRecord[] = Array.from({ length: 15 }, (_, i) => ({
  id: `id-${i}`,
  txHash: `hash-${i}1234567890`,
  timestamp: new Date(2026, i, 1),
  action: i % 2 === 0 ? 'BUY' : 'SELL',
  amount: (10 + i).toString(),
  asset: 'XLM',
}));

describe('useTradeActivity hook', () => {
  it('should initialize with default states', () => {
    const { result } = renderHook(() => useTradeActivity({ address: 'G...', initialData: mockData }));
    expect(result.current.page).toBe(1);
    expect(result.current.totalPages).toBe(2);
    expect(result.current.data.length).toBe(10);
  });

  it('should handle pagination switching', () => {
    const { result } = renderHook(() => useTradeActivity({ address: 'G...', initialData: mockData }));
    act(() => {
      result.current.setPage(2);
    });
    expect(result.current.page).toBe(2);
    expect(result.current.data.length).toBe(5);
  });

  it('should handle field switching on sorting sorting', () => {
    const { result } = renderHook(() => useTradeActivity({ address: 'G...', initialData: mockData }));
    expect(result.current.sortField).toBe('timestamp');
    act(() => {
      result.current.handleSort('amount');
    });
    expect(result.current.sortField).toBe('amount');
  });
});