import { renderHook, act } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';

import { StellarRouteApiError } from '@/lib/api/client';
import { useRateLimitRetry } from './useRateLimitRetry';

describe('useRateLimitRetry', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('returns isRateLimited=false for null error', () => {
    const onRetry = vi.fn();
    const { result } = renderHook(() => useRateLimitRetry(null, onRetry));

    expect(result.current.isRateLimited).toBe(false);
    expect(result.current.secondsRemaining).toBe(0);
  });

  it('returns isRateLimited=false for non-429 errors', () => {
    const onRetry = vi.fn();
    const error = new StellarRouteApiError(500, 'internal_error', 'Server Error');
    const { result } = renderHook(() => useRateLimitRetry(error, onRetry));

    expect(result.current.isRateLimited).toBe(false);
    expect(result.current.secondsRemaining).toBe(0);
  });

  it('returns isRateLimited=false for plain Error', () => {
    const onRetry = vi.fn();
    const error = new Error('Something broke');
    const { result } = renderHook(() => useRateLimitRetry(error, onRetry));

    expect(result.current.isRateLimited).toBe(false);
  });

  it('detects 429 StellarRouteApiError as rate-limited', () => {
    const onRetry = vi.fn();
    const error = new StellarRouteApiError(
      429,
      'rate_limit_exceeded',
      'Too many requests',
      undefined,
      5,
    );

    const { result } = renderHook(() => useRateLimitRetry(error, onRetry));

    expect(result.current.isRateLimited).toBe(true);
    expect(result.current.secondsRemaining).toBe(5);
  });

  it('uses default 5s when retryAfterSeconds is not provided on 429', () => {
    const onRetry = vi.fn();
    const error = new StellarRouteApiError(
      429,
      'rate_limit_exceeded',
      'Too many requests',
    );

    const { result } = renderHook(() => useRateLimitRetry(error, onRetry));

    expect(result.current.isRateLimited).toBe(true);
    expect(result.current.secondsRemaining).toBe(5);
  });

  it('counts down each second', () => {
    const onRetry = vi.fn();
    const error = new StellarRouteApiError(
      429,
      'rate_limit_exceeded',
      'Too many requests',
      undefined,
      3,
    );

    const { result } = renderHook(() => useRateLimitRetry(error, onRetry));
    expect(result.current.secondsRemaining).toBe(3);

    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(result.current.secondsRemaining).toBe(2);

    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(result.current.secondsRemaining).toBe(1);
  });

  it('auto-retries when countdown reaches 0', () => {
    const onRetry = vi.fn();
    const error = new StellarRouteApiError(
      429,
      'rate_limit_exceeded',
      'Too many requests',
      undefined,
      2,
    );

    renderHook(() => useRateLimitRetry(error, onRetry));

    // Advance through full countdown
    act(() => {
      vi.advanceTimersByTime(2000);
    });

    // The auto-retry uses setTimeout(..., 0), flush it
    act(() => {
      vi.advanceTimersByTime(1);
    });

    expect(onRetry).toHaveBeenCalledTimes(1);
  });

  it('manual retry resets countdown and fires immediately', () => {
    const onRetry = vi.fn();
    const error = new StellarRouteApiError(
      429,
      'rate_limit_exceeded',
      'Too many requests',
      undefined,
      10,
    );

    const { result } = renderHook(() => useRateLimitRetry(error, onRetry));
    expect(result.current.secondsRemaining).toBe(10);

    act(() => {
      result.current.retry();
    });

    expect(result.current.secondsRemaining).toBe(0);
    expect(onRetry).toHaveBeenCalledTimes(1);
  });

  it('clears countdown when error changes to non-rate-limit', () => {
    const onRetry = vi.fn();
    const rateLimitError = new StellarRouteApiError(
      429,
      'rate_limit_exceeded',
      'Too many requests',
      undefined,
      5,
    );

    const { result, rerender } = renderHook(
      ({ error }) => useRateLimitRetry(error, onRetry),
      { initialProps: { error: rateLimitError as Error | null } },
    );

    expect(result.current.isRateLimited).toBe(true);
    expect(result.current.secondsRemaining).toBe(5);

    // Error clears
    rerender({ error: null });

    expect(result.current.isRateLimited).toBe(false);
    expect(result.current.secondsRemaining).toBe(0);
  });
});
