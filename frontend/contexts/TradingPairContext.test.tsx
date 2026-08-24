import { describe, it, expect } from 'vitest';
import { render, screen, renderHook, act } from '@testing-library/react';
import { 
  TradingPairProvider, 
  useTradingPair, 
  useOptionalTradingPair 
} from './TradingPairContext';

describe('TradingPairContext', () => {
  describe('TradingPairProvider', () => {
    it('renders children', () => {
      render(
        <TradingPairProvider>
          <div>Test Child</div>
        </TradingPairProvider>
      );
      
      expect(screen.getByText('Test Child')).toBeInTheDocument();
    });

    it('provides initial null values', () => {
      const { result } = renderHook(() => useTradingPair(), {
        wrapper: TradingPairProvider,
      });

      expect(result.current.fromAsset).toBeNull();
      expect(result.current.toAsset).toBeNull();
    });
  });

  describe('useTradingPair', () => {
    it('throws error when used outside provider', () => {
      // Suppress console.error for this test
      const originalError = console.error;
      console.error = () => {};

      expect(() => {
        renderHook(() => useTradingPair());
      }).toThrow('useTradingPair must be used within a TradingPairProvider');

      console.error = originalError;
    });

    it('allows setting trading pair', () => {
      const { result } = renderHook(() => useTradingPair(), {
        wrapper: TradingPairProvider,
      });

      act(() => {
        result.current.setTradingPair('native', 'USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5');
      });

      expect(result.current.fromAsset).toBe('native');
      expect(result.current.toAsset).toBe('USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5');
    });

    it('allows clearing trading pair', () => {
      const { result } = renderHook(() => useTradingPair(), {
        wrapper: TradingPairProvider,
      });

      act(() => {
        result.current.setTradingPair('native', 'USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5');
      });

      expect(result.current.fromAsset).toBe('native');
      expect(result.current.toAsset).toBe('USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5');

      act(() => {
        result.current.clearTradingPair();
      });

      expect(result.current.fromAsset).toBeNull();
      expect(result.current.toAsset).toBeNull();
    });

    it('updates trading pair multiple times', () => {
      const { result } = renderHook(() => useTradingPair(), {
        wrapper: TradingPairProvider,
      });

      act(() => {
        result.current.setTradingPair('native', 'USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5');
      });

      expect(result.current.fromAsset).toBe('native');

      act(() => {
        result.current.setTradingPair('USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5', 'native');
      });

      expect(result.current.fromAsset).toBe('USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5');
      expect(result.current.toAsset).toBe('native');
    });
  });

  describe('useOptionalTradingPair', () => {
    it('returns undefined when used outside provider', () => {
      const { result } = renderHook(() => useOptionalTradingPair());
      
      expect(result.current).toBeUndefined();
    });

    it('returns context when used inside provider', () => {
      const { result } = renderHook(() => useOptionalTradingPair(), {
        wrapper: TradingPairProvider,
      });

      expect(result.current).toBeDefined();
      expect(result.current?.fromAsset).toBeNull();
      expect(result.current?.toAsset).toBeNull();
    });

    it('allows setting trading pair through optional hook', () => {
      const { result } = renderHook(() => useOptionalTradingPair(), {
        wrapper: TradingPairProvider,
      });

      act(() => {
        result.current?.setTradingPair('native', 'USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5');
      });

      expect(result.current?.fromAsset).toBe('native');
      expect(result.current?.toAsset).toBe('USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5');
    });
  });
});
