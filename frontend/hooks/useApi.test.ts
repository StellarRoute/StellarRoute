import { cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { stellarRouteClient } from '@/lib/api/client';
import { usePriceHistory, useRoutes } from './useApi';

vi.mock('@/lib/api/client', async () => {
  const actual = await vi.importActual<typeof import('@/lib/api/client')>(
    '@/lib/api/client',
  );

  return {
    ...actual,
    stellarRouteClient: {
      ...actual.stellarRouteClient,
      getPriceHistory: vi.fn(),
      getRoutes: vi.fn(),
    },
  };
});

describe('useApi hook option signatures', () => {
  afterEach(() => {
    cleanup();
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it('usePriceHistory honors refreshIntervalMs and uses options object shape', async () => {
    const getPriceHistoryMock = vi.mocked(stellarRouteClient.getPriceHistory);
    getPriceHistoryMock.mockResolvedValue({
      base_asset: { asset_type: 'native' },
      quote_asset: {
        asset_type: 'credit_alphanum4',
        asset_code: 'USDC',
        asset_issuer: 'G...USDC',
      },
      points: [{ timestamp: 1710000000000, price: '0.101' }],
    });

    renderHook(() =>
      usePriceHistory('native', 'USDC:G...USDC', {
        refreshIntervalMs: 20,
        query: { window: '24h', limit: 10 },
      }),
    );

    await waitFor(() => {
      expect(getPriceHistoryMock).toHaveBeenCalledTimes(1);
    });

    await waitFor(() => {
      expect(getPriceHistoryMock.mock.calls.length).toBeGreaterThanOrEqual(2);
    });
  });

  it('usePriceHistory skip works when pair is undefined', () => {
    const getPriceHistoryMock = vi.mocked(stellarRouteClient.getPriceHistory);

    const { result } = renderHook(() =>
      usePriceHistory(undefined, undefined, {
        refreshIntervalMs: 1_000,
      }),
    );

    expect(result.current.loading).toBe(false);
    expect(getPriceHistoryMock).not.toHaveBeenCalled();
  });

  it('useRoutes skip logic prevents spurious mount calls on invalid pair/amount', () => {
    const getRoutesMock = vi.mocked(stellarRouteClient.getRoutes);

    renderHook(() => useRoutes('', '', undefined));

    expect(getRoutesMock).not.toHaveBeenCalled();
  });

  it('useRoutes supports skip true and configurable refresh + error callback', async () => {
    const onError = vi.fn();
    const getRoutesMock = vi.mocked(stellarRouteClient.getRoutes);
    getRoutesMock.mockResolvedValue({
      base_asset: { asset_type: 'native' },
      quote_asset: {
        asset_type: 'credit_alphanum4',
        asset_code: 'USDC',
        asset_issuer: 'G...USDC',
      },
      amount: '100',
      routes: [],
      timestamp: Date.now(),
    });
    getRoutesMock
      .mockRejectedValueOnce(new Error('temporary backend failure'))
      .mockResolvedValueOnce({
        base_asset: { asset_type: 'native' },
        quote_asset: {
          asset_type: 'credit_alphanum4',
          asset_code: 'USDC',
          asset_issuer: 'G...USDC',
        },
        amount: '100',
        routes: [],
        timestamp: Date.now(),
      });

    const { rerender } = renderHook(
      ({ skip }) =>
        useRoutes('native', 'USDC:G...USDC', 100, {
          skip,
          refreshIntervalMs: 20,
          onError,
        }),
      {
        initialProps: { skip: true },
      },
    );

    expect(getRoutesMock).not.toHaveBeenCalled();

    rerender({ skip: false });

    await waitFor(() => {
      expect(getRoutesMock.mock.calls.length).toBeGreaterThanOrEqual(1);
      expect(onError.mock.calls.length).toBeGreaterThanOrEqual(1);
    });

    await waitFor(() => {
      expect(getRoutesMock.mock.calls.length).toBeGreaterThanOrEqual(2);
    });
  });
});
