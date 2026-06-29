import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const {
  getPriceHistoryMock,
  getRoutesMock,
  toastErrorMock,
} = vi.hoisted(() => ({
  getPriceHistoryMock: vi.fn(),
  getRoutesMock: vi.fn(),
  toastErrorMock: vi.fn(),
}));

vi.mock('sonner', () => ({
  toast: {
    error: toastErrorMock,
  },
}));

vi.mock('@/lib/api/client', () => {
  class StellarRouteApiError extends Error {
    public readonly status: number;
    public readonly code: string;

    constructor(status: number, code: string, message: string) {
      super(message);
      this.name = 'StellarRouteApiError';
      this.status = status;
      this.code = code;
    }
  }

  return {
    StellarRouteApiError,
    STATUS_PAGE_REFRESH_MS: 30_000,
    stellarRouteClient: {
      getPriceHistory: getPriceHistoryMock,
      getRoutes: getRoutesMock,
    },
  };
});

import { usePriceHistory, useRoutes } from './useApi';

describe('useApi hooks regression coverage', () => {
  beforeEach(() => {
    vi.useRealTimers();
    getPriceHistoryMock.mockReset();
    getRoutesMock.mockReset();
    toastErrorMock.mockReset();

    getPriceHistoryMock.mockResolvedValue({
      base_asset: { asset_type: 'native' },
      quote_asset: { asset_type: 'credit_alphanum4', asset_code: 'USDC' },
      window: '24h',
      source: 'test',
      generated_at: Date.now(),
      points: [],
    });

    getRoutesMock.mockResolvedValue({
      base_asset: { asset_type: 'native' },
      quote_asset: { asset_type: 'credit_alphanum4', asset_code: 'USDC' },
      amount: '10',
      timestamp: Date.now(),
      routes: [],
    });
  });

  it('useRoutes skips API call when pair is empty (no spurious mount call)', async () => {
    const { result } = renderHook(() => useRoutes('', '', 10));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(getRoutesMock).not.toHaveBeenCalled();
  });

  it('useRoutes skips API call when amount is invalid', async () => {
    const { result } = renderHook(() => useRoutes('native', 'USDC', Number.NaN));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(getRoutesMock).not.toHaveBeenCalled();
  });

  it('useRoutes performs API call when args are valid', async () => {
    renderHook(() => useRoutes('native', 'USDC', 10));

    await waitFor(() => {
      expect(getRoutesMock).toHaveBeenCalledTimes(1);
    });
  });

  it('useRoutes supports showToastOnError option', async () => {
    getRoutesMock.mockRejectedValueOnce(new Error('routes failed'));

    renderHook(() =>
      useRoutes('native', 'USDC', 10, 5, 3, { showToastOnError: true })
    );

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledTimes(1);
    });
  });

  it('usePriceHistory registers polling interval with refreshIntervalMs', async () => {
    const setIntervalSpy = vi.spyOn(window, 'setInterval');
    const clearIntervalSpy = vi.spyOn(window, 'clearInterval');

    const { unmount } = renderHook(() => usePriceHistory('native', 'USDC', 50));

    await waitFor(() => {
      expect(getPriceHistoryMock).toHaveBeenCalledTimes(1);
    });

    expect(setIntervalSpy).toHaveBeenCalledWith(expect.any(Function), 50);

    unmount();

    expect(clearIntervalSpy).toHaveBeenCalled();
  });

  it('usePriceHistory skip=true prevents requests', async () => {
    const { result } = renderHook(() =>
      usePriceHistory('native', 'USDC', 60_000, true)
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(getPriceHistoryMock).not.toHaveBeenCalled();
  });

  it('usePriceHistory skips when pair is undefined', async () => {
    const { result } = renderHook(() => usePriceHistory('', '', 60_000, false));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(getPriceHistoryMock).not.toHaveBeenCalled();
  });
});
