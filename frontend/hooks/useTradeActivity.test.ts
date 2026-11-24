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

  it('should filter by pair / asset', () => {
    const mixedData: TradeRecord[] = [
      { id: '1', txHash: 'h1', timestamp: new Date(2026, 0, 1), action: 'SWAP', amount: '10', asset: 'XLM → USDC' },
      { id: '2', txHash: 'h2', timestamp: new Date(2026, 0, 2), action: 'SWAP', amount: '20', asset: 'BTC → ETH' },
    ];
    const { result, rerender } = renderHook(
      ({ filters }) => useTradeActivity({ initialData: mixedData, filters }),
      { initialProps: { filters: { pair: 'USDC' } } }
    );
    expect(result.current.data.length).toBe(1);
    expect(result.current.data[0].id).toBe('1');

    // Change filter
    rerender({ filters: { pair: 'ETH' } });
    expect(result.current.data.length).toBe(1);
    expect(result.current.data[0].id).toBe('2');

    // ALL should return both
    rerender({ filters: { pair: 'ALL' } });
    expect(result.current.data.length).toBe(2);
  });

  it('should filter by status / action', () => {
    const mixedData: TradeRecord[] = [
      { id: '1', txHash: 'h1', timestamp: new Date(2026, 0, 1), action: 'BUY', amount: '10', asset: 'XLM' },
      { id: '2', txHash: 'h2', timestamp: new Date(2026, 0, 2), action: 'SELL', amount: '20', asset: 'XLM' },
    ];
    const { result } = renderHook(() =>
      useTradeActivity({ initialData: mixedData, filters: { status: 'BUY' } })
    );
    expect(result.current.data.length).toBe(1);
    expect(result.current.data[0].action).toBe('BUY');
  });

  it('should filter by dateFrom and dateTo range', () => {
    const datesData: TradeRecord[] = [
      { id: '1', txHash: 'h1', timestamp: new Date('2026-01-05T10:00:00Z'), action: 'SWAP', amount: '10', asset: 'XLM' },
      { id: '2', txHash: 'h2', timestamp: new Date('2026-01-15T10:00:00Z'), action: 'SWAP', amount: '20', asset: 'XLM' },
      { id: '3', txHash: 'h3', timestamp: new Date('2026-01-25T10:00:00Z'), action: 'SWAP', amount: '30', asset: 'XLM' },
    ];
    const { result } = renderHook(() =>
      useTradeActivity({
        initialData: datesData,
        filters: {
          dateFrom: '2026-01-10',
          dateTo: '2026-01-20',
        },
      })
    );
    expect(result.current.data.length).toBe(1);
    expect(result.current.data[0].id).toBe('2');
  });
});