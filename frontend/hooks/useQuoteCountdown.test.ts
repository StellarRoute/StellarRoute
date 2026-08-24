import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useQuoteCountdown } from './useQuoteCountdown';

describe('useQuoteCountdown', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('returns initial state when expiresAtMs is undefined', () => {
    const { result } = renderHook(() => useQuoteCountdown(undefined));
    expect(result.current.remainingSeconds).toBe(0);
    expect(result.current.isExpired).toBe(false);
    expect(result.current.progress).toBe(1);
  });

  it('calculates remaining seconds correctly', () => {
    const now = Date.now();
    const expiresAtMs = now + 5000;
    
    const { result } = renderHook(() => useQuoteCountdown(expiresAtMs, 5000));
    
    expect(result.current.remainingSeconds).toBe(5);
    expect(result.current.isExpired).toBe(false);
    expect(result.current.progress).toBe(1);
  });

  it('updates countdown over time', () => {
    const now = Date.now();
    const expiresAtMs = now + 5000;
    
    const { result } = renderHook(() => useQuoteCountdown(expiresAtMs, 5000));
    
    act(() => {
      vi.advanceTimersByTime(2000);
    });
    
    expect(result.current.remainingSeconds).toBe(3);
    expect(result.current.progress).toBeCloseTo(0.6);
    expect(result.current.isExpired).toBe(false);
  });

  it('marks as expired when time passes', () => {
    const now = Date.now();
    const expiresAtMs = now + 5000;
    
    const { result } = renderHook(() => useQuoteCountdown(expiresAtMs, 5000));
    
    act(() => {
      vi.advanceTimersByTime(6000);
    });
    
    expect(result.current.remainingSeconds).toBe(0);
    expect(result.current.isExpired).toBe(true);
    expect(result.current.progress).toBe(0);
  });

  it('syncs correctly on expiresAtMs change without drift', () => {
    const now = Date.now();
    const expiresAtMs1 = now + 5000;
    
    const { result, rerender } = renderHook(
      ({ expiresAtMs }) => useQuoteCountdown(expiresAtMs, 5000),
      { initialProps: { expiresAtMs: expiresAtMs1 } }
    );
    
    expect(result.current.remainingSeconds).toBe(5);
    
    // Simulate some time passing
    act(() => {
      vi.advanceTimersByTime(2000);
    });
    expect(result.current.remainingSeconds).toBe(3);
    
    // New quote arrives with new expiration
    const expiresAtMs2 = Date.now() + 5000;
    rerender({ expiresAtMs: expiresAtMs2 });
    
    expect(result.current.remainingSeconds).toBe(5);
    expect(result.current.isExpired).toBe(false);
  });
});
