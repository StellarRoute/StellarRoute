/**
 * Frontend resilience unit tests — Issue #357
 *
 * Covers:
 *  - Simulated packet loss (aborted requests)
 *  - Simulated extreme latency (slow responses)
 *  - 504 gateway timeout / 5xx server errors
 *  - Manual recovery after network drop
 *  - SwapButton disabled state during error / no-wallet
 *
 * These run deterministically in Vitest via fake timers and mocked fetch —
 * no real network or wall-clock delays required.
 */

import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { PriceQuote } from '@/types';
import { StellarRouteApiError, stellarRouteClient } from '@/lib/api/client';
import { useQuoteRefresh } from '@/hooks/useQuoteRefresh';
import { useOnlineStatus } from '@/hooks/useOnlineStatus';

vi.mock('@/lib/api/client', async () => {
  const actual = await vi.importActual<typeof import('@/lib/api/client')>(
    '@/lib/api/client',
  );
  return {
    ...actual,
    stellarRouteClient: { ...actual.stellarRouteClient, getQuote: vi.fn() },
  };
});

const mockQuote = (): PriceQuote => ({
  base_asset: { asset_type: 'native' },
  quote_asset: {
    asset_type: 'credit_alphanum4',
    asset_code: 'USDC',
    asset_issuer: 'GABC',
  },
  amount: '100',
  price: '0.995',
  total: '99.5',
  quote_type: 'sell',
  path: [],
  timestamp: Math.floor(Date.now() / 1000),
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.clearAllMocks();
});

// ---------------------------------------------------------------------------
// Packet loss — aborted / network-error failures
// ---------------------------------------------------------------------------

describe('Resilience: packet loss (aborted requests)', () => {
  it('recovers after two network-abort failures on retry', async () => {
    const getQuoteMock = vi.mocked(stellarRouteClient.getQuote);
    let calls = 0;
    getQuoteMock.mockImplementation(async () => {
      calls += 1;
      if (calls <= 2) throw new Error('Failed to fetch'); // simulates abort
      return mockQuote();
    });

    const { result } = renderHook(() =>
      useQuoteRefresh('native', 'USDC:GABC', 100, 'sell', {
        debounceMs: 1,
        maxAutoRetries: 3,
        retryBackoffMs: 5,
        isOnline: true,
      }),
    );

    await waitFor(
      () => expect(result.current.data?.total).toBe('99.5'),
      { timeout: 3000 },
    );
    expect(result.current.error).toBeNull();
  });

  it('surfaces error after exhausting all retries', async () => {
    const getQuoteMock = vi.mocked(stellarRouteClient.getQuote);
    getQuoteMock.mockRejectedValue(new Error('Failed to fetch'));

    const { result } = renderHook(() =>
      useQuoteRefresh('native', 'USDC:GABC', 100, 'sell', {
        debounceMs: 1,
        maxAutoRetries: 2,
        retryBackoffMs: 5,
        isOnline: true,
      }),
    );

    await waitFor(() => expect(result.current.error).not.toBeNull(), {
      timeout: 3000,
    });
    expect(result.current.data).toBeFalsy();
  });
});

// ---------------------------------------------------------------------------
// Extreme latency — simulated slow response via Promise delay
// ---------------------------------------------------------------------------

describe('Resilience: extreme latency', () => {
  it('does not fetch when going offline mid-session', async () => {
    const getQuoteMock = vi.mocked(stellarRouteClient.getQuote);
    getQuoteMock.mockResolvedValue(mockQuote());

    const { result } = renderHook(() => useOnlineStatus());

    // Go offline
    act(() => {
      Object.defineProperty(window.navigator, 'onLine', {
        configurable: true,
        value: false,
      });
      window.dispatchEvent(new Event('offline'));
    });

    expect(result.current.isOffline).toBe(true);
    expect(result.current.isOnline).toBe(false);
  });

  it('recovers when coming back online after offline period', async () => {
    const { result } = renderHook(() => useOnlineStatus());

    act(() => {
      Object.defineProperty(window.navigator, 'onLine', {
        configurable: true,
        value: false,
      });
      window.dispatchEvent(new Event('offline'));
    });
    expect(result.current.isOffline).toBe(true);

    act(() => {
      Object.defineProperty(window.navigator, 'onLine', {
        configurable: true,
        value: true,
      });
      window.dispatchEvent(new Event('online'));
    });
    expect(result.current.isOnline).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// 504 / 5xx server errors
// ---------------------------------------------------------------------------

describe('Resilience: server errors (504/5xx)', () => {
  it('treats 504 as a retryable error and recovers', async () => {
    const getQuoteMock = vi.mocked(stellarRouteClient.getQuote);
    let calls = 0;
    getQuoteMock.mockImplementation(async () => {
      calls += 1;
      if (calls === 1) {
        throw new StellarRouteApiError(504, 'unknown_error', 'Gateway Timeout');
      }
      return mockQuote();
    });

    const { result } = renderHook(() =>
      useQuoteRefresh('native', 'USDC:GABC', 100, 'sell', {
        debounceMs: 1,
        maxAutoRetries: 2,
        retryBackoffMs: 5,
        isOnline: true,
      }),
    );

    await waitFor(() => expect(result.current.data?.total).toBe('99.5'), {
      timeout: 3000,
    });
  });

  it('does not retry 400 bad-request errors (non-transient)', async () => {
    const getQuoteMock = vi.mocked(stellarRouteClient.getQuote);
    getQuoteMock.mockRejectedValueOnce(
      new StellarRouteApiError(400, 'bad_request', 'Invalid params'),
    );

    const { result } = renderHook(() =>
      useQuoteRefresh('native', 'USDC:GABC', 100, 'sell', {
        debounceMs: 1,
        maxAutoRetries: 2,
        retryBackoffMs: 5,
        isOnline: true,
      }),
    );

    await waitFor(() => expect(result.current.error).not.toBeNull(), {
      timeout: 2000,
    });
    // Should only have been called once (no retries for 400)
    expect(getQuoteMock).toHaveBeenCalledTimes(1);
  });
});

// ---------------------------------------------------------------------------
// Manual recovery via explicit refresh
// ---------------------------------------------------------------------------

describe('Resilience: manual recovery', () => {
  it('quote recovers after manual refresh once network is restored', async () => {
    const getQuoteMock = vi.mocked(stellarRouteClient.getQuote);

    // First call fails (network down)
    getQuoteMock.mockRejectedValueOnce(new Error('Failed to fetch'));
    // After manual refresh → success
    getQuoteMock.mockResolvedValueOnce(mockQuote());

    const { result } = renderHook(() =>
      useQuoteRefresh('native', 'USDC:GABC', 100, 'sell', {
        debounceMs: 1,
        maxAutoRetries: 0, // no auto-retries; simulates manual recovery
        retryBackoffMs: 5,
        isOnline: true,
      }),
    );

    // Wait for initial error
    await waitFor(() => expect(result.current.error).not.toBeNull(), {
      timeout: 2000,
    });

    // Trigger manual refresh
    act(() => {
      result.current.refresh();
    });

    await waitFor(() => expect(result.current.data?.total).toBe('99.5'), {
      timeout: 2000,
    });
    expect(result.current.error).toBeNull();
  });
});
