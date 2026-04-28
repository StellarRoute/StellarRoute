import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useShareableQuote } from './useShareableQuote';
import { useRouter, useSearchParams } from 'next/navigation';

vi.mock('next/navigation', () => ({
  useRouter: vi.fn(),
  useSearchParams: vi.fn(),
}));

describe('useShareableQuote', () => {
  const mockPush = vi.fn();
  const mockSearchParams = new Map<string, string>();

  beforeEach(() => {
    vi.clearAllMocks();
    mockSearchParams.clear();
    
    vi.mocked(useRouter).mockReturnValue({
      push: mockPush,
    } as any);

    vi.mocked(useSearchParams).mockReturnValue({
      get: (key: string) => mockSearchParams.get(key) || null,
    } as any);
  });

  describe('parseParams', () => {
    it('returns null when required params missing', () => {
      const { result } = renderHook(() => useShareableQuote());
      expect(result.current.parseParams()).toBeNull();
    });

    it('parses valid params correctly', () => {
      mockSearchParams.set('from', 'native');
      mockSearchParams.set('to', 'USDC:GABC');
      mockSearchParams.set('amount', '100.5');
      mockSearchParams.set('slippage', '0.5');

      const { result } = renderHook(() => useShareableQuote());
      const params = result.current.parseParams();

      expect(params).toEqual({
        from: 'native',
        to: 'USDC:GABC',
        amount: '100.5',
        slippage: '0.5',
      });
    });

    it('sanitizes amount with invalid characters', () => {
      mockSearchParams.set('from', 'native');
      mockSearchParams.set('to', 'USDC:GABC');
      mockSearchParams.set('amount', '100.5abc');

      const { result } = renderHook(() => useShareableQuote());
      const params = result.current.parseParams();

      expect(params?.amount).toBe('100.5');
    });

    it('returns null for negative amount', () => {
      mockSearchParams.set('from', 'native');
      mockSearchParams.set('to', 'USDC:GABC');
      mockSearchParams.set('amount', '-100');

      const { result } = renderHook(() => useShareableQuote());
      expect(result.current.parseParams()).toBeNull();
    });

    it('validates slippage range', () => {
      mockSearchParams.set('from', 'native');
      mockSearchParams.set('to', 'USDC:GABC');
      mockSearchParams.set('amount', '100');
      mockSearchParams.set('slippage', '60');

      const { result } = renderHook(() => useShareableQuote());
      const params = result.current.parseParams();

      expect(params?.slippage).toBeUndefined();
    });

    it('detects stale quotes', () => {
      const oldTimestamp = Date.now() - 120000; // 2 minutes ago
      mockSearchParams.set('from', 'native');
      mockSearchParams.set('to', 'USDC:GABC');
      mockSearchParams.set('amount', '100');
      mockSearchParams.set('ts', oldTimestamp.toString());

      const { result } = renderHook(() => useShareableQuote());
      result.current.parseParams();

      expect(result.current.isStale).toBe(true);
    });
  });

  describe('generateShareableUrl', () => {
    beforeEach(() => {
      Object.defineProperty(window, 'location', {
        value: { origin: 'https://stellarroute.com' },
        writable: true,
      });
    });

    it('generates valid URL with all params', () => {
      const { result } = renderHook(() => useShareableQuote());
      const url = result.current.generateShareableUrl({
        from: 'native',
        to: 'USDC:GABC',
        amount: '100',
        slippage: '0.5',
      });

      expect(url).toContain('https://stellarroute.com/swap?');
      expect(url).toContain('from=native');
      expect(url).toContain('to=USDC%3AGABC');
      expect(url).toContain('amount=100');
      expect(url).toContain('slippage=0.5');
      expect(url).toContain('ts=');
    });

    it('returns null when required params missing', () => {
      const { result } = renderHook(() => useShareableQuote());
      const url = result.current.generateShareableUrl({
        from: 'native',
        to: '',
        amount: '100',
      });

      expect(url).toBeNull();
    });

    it('returns null when URL exceeds max length', () => {
      const { result } = renderHook(() => useShareableQuote());
      const longString = 'A'.repeat(2000);
      const url = result.current.generateShareableUrl({
        from: longString,
        to: longString,
        amount: '100',
      });

      expect(url).toBeNull();
    });
  });

  describe('applyParams', () => {
    it('navigates with correct params', () => {
      const { result } = renderHook(() => useShareableQuote());
      
      act(() => {
        result.current.applyParams({
          from: 'native',
          to: 'USDC:GABC',
          amount: '100',
          slippage: '0.5',
        });
      });

      expect(mockPush).toHaveBeenCalledWith(
        expect.stringContaining('/swap?')
      );
      expect(mockPush).toHaveBeenCalledWith(
        expect.stringContaining('from=native')
      );
    });
  });

  describe('clearParams', () => {
    it('navigates to clean swap page', () => {
      const { result } = renderHook(() => useShareableQuote());
      
      act(() => {
        result.current.clearParams();
      });

      expect(mockPush).toHaveBeenCalledWith('/swap');
    });

    it('resets stale state', () => {
      mockSearchParams.set('from', 'native');
      mockSearchParams.set('to', 'USDC:GABC');
      mockSearchParams.set('amount', '100');
      mockSearchParams.set('ts', (Date.now() - 120000).toString());

      const { result } = renderHook(() => useShareableQuote());
      result.current.parseParams();
      expect(result.current.isStale).toBe(true);

      act(() => {
        result.current.clearParams();
      });

      expect(result.current.isStale).toBe(false);
    });
  });

  describe('refreshQuote', () => {
    it('updates timestamp and clears stale flag', () => {
      mockSearchParams.set('from', 'native');
      mockSearchParams.set('to', 'USDC:GABC');
      mockSearchParams.set('amount', '100');
      mockSearchParams.set('ts', (Date.now() - 120000).toString());

      const { result } = renderHook(() => useShareableQuote());
      result.current.parseParams();
      expect(result.current.isStale).toBe(true);

      act(() => {
        result.current.refreshQuote();
      });

      expect(result.current.isStale).toBe(false);
      expect(mockPush).toHaveBeenCalled();
    });
  });
});
