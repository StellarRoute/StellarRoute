'use client';

/**
 * Custom React hooks for StellarRoute data fetching.
 *
 * Each hook returns { data, loading, error } and handles:
 *  - Request cancellation on unmount (AbortController)
 *  - Auto-refresh intervals where appropriate
 *  - Debounced parameters for useQuote
 */

import { useCallback, useEffect, useRef, useState } from 'react';

import {
  stellarRouteClient,
  type BatchQuoteResponse,
  type PriceHistoryQueryOptions,
  type QuoteRequestItem,
  type RoutesQueryOptions,
} from '@/lib/api/client';
import { QUOTE_AMOUNT_DEBOUNCE_MS } from '@/lib/quote-stale';
import type {
  HealthStatus,
  Orderbook,
  PairsResponse,
  PriceHistoryResponse,
  PriceQuote,
  QuoteType,
  RoutesResponse,
  TradingPair,
} from '@/types';

// ---------------------------------------------------------------------------
// Shared state shape
// ---------------------------------------------------------------------------

export interface UseApiState<T> {
  data: T | undefined;
  loading: boolean;
  error: Error | null;
}

export interface UseFetchOptions {
  refreshIntervalMs?: number;
  skip?: boolean;
  onError?: (error: Error) => void;
}

// ---------------------------------------------------------------------------
// Internal: generic fetch hook
// ---------------------------------------------------------------------------

function useFetch<T>(
  fetcher: (signal: AbortSignal) => Promise<T>,
  deps: unknown[],
  options: UseFetchOptions = {},
): UseApiState<T> & { refresh: () => void } {
  const { refreshIntervalMs, skip = false, onError } = options;

  const [state, setState] = useState<UseApiState<T>>({
    data: undefined,
    loading: !skip,
    error: null,
  });

  const fetcherRef = useRef(fetcher);
  useEffect(() => {
    fetcherRef.current = fetcher;
  }, [fetcher]);

  const [tick, setTick] = useState(0);
  const refresh = useCallback(() => setTick((n) => n + 1), []);

  useEffect(() => {
    if (skip) {
      setState({ data: undefined, loading: false, error: null });
      return;
    }

    const controller = new AbortController();
    setState((prev) => ({ ...prev, loading: true, error: null }));

    fetcherRef
      .current(controller.signal)
      .then((data) => {
        if (!controller.signal.aborted) {
          setState({ data, loading: false, error: null });
        }
      })
      .catch((err: unknown) => {
        if (!controller.signal.aborted) {
          const normalizedError =
            err instanceof Error ? err : new Error(String(err));
          onError?.(normalizedError);
          setState({
            data: undefined,
            loading: false,
            error: normalizedError,
          });
        }
      });

    return () => controller.abort();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tick, skip, ...deps]);

  useEffect(() => {
    if (!refreshIntervalMs || skip) return;
    const id = setInterval(() => setTick((n) => n + 1), refreshIntervalMs);
    return () => clearInterval(id);
  }, [refreshIntervalMs, skip]);

  return { ...state, refresh };
}

// ---------------------------------------------------------------------------
// Internal: simple debounce hook
// ---------------------------------------------------------------------------

function useDebounced<T>(value: T, delayMs: number): T {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const id = setTimeout(() => setDebounced(value), delayMs);
    return () => clearTimeout(id);
  }, [value, delayMs]);
  return debounced;
}

// ---------------------------------------------------------------------------
// usePairs — fetch and cache trading pairs
// ---------------------------------------------------------------------------

export function usePairs(): UseApiState<TradingPair[]> & {
  refresh: () => void;
} {
  return useFetch(
    (signal) =>
      stellarRouteClient
        .getPairs({ signal })
        .then((res: PairsResponse) => res.pairs),
    [],
  );
}

// ---------------------------------------------------------------------------
// useOrderbook — fetch orderbook with auto-refresh
// ---------------------------------------------------------------------------

export function useOrderbook(
  base: string,
  quote: string,
  options: UseFetchOptions = { refreshIntervalMs: 10_000 },
): UseApiState<Orderbook> & { refresh: () => void } {
  return useFetch(
    (signal) => stellarRouteClient.getOrderbook(base, quote, { signal }),
    [base, quote],
    {
      ...options,
      skip: options.skip || !base || !quote,
    },
  );
}

// ---------------------------------------------------------------------------
// useQuote — debounced amount; no request while input is invalid / empty
// ---------------------------------------------------------------------------

export function useQuote(
  base: string,
  quote: string,
  amount: number | undefined,
  type: QuoteType = 'sell',
  options: UseFetchOptions = {},
): UseApiState<PriceQuote> & { refresh: () => void } {
  const debouncedAmount = useDebounced(amount, QUOTE_AMOUNT_DEBOUNCE_MS);

  const invalidAmount =
    debouncedAmount === undefined ||
    !Number.isFinite(debouncedAmount) ||
    debouncedAmount <= 0;

  return useFetch(
    (signal) =>
      stellarRouteClient.getQuote(base, quote, debouncedAmount, type, {
        signal,
      }),
    [base, quote, debouncedAmount, type],
    {
      ...options,
      skip: options.skip || !base || !quote || invalidAmount,
    },
  );
}

// ---------------------------------------------------------------------------
// usePriceHistory — fetch OHLC/price series points
// ---------------------------------------------------------------------------

export interface UsePriceHistoryOptions extends UseFetchOptions {
  query?: PriceHistoryQueryOptions;
}

export function usePriceHistory(
  base: string | undefined,
  quote: string | undefined,
  options: UsePriceHistoryOptions = {},
): UseApiState<PriceHistoryResponse> & { refresh: () => void } {
  const { query, ...fetchOptions } = options;
  const skip = fetchOptions.skip || !base || !quote;

  return useFetch(
    (signal) =>
      stellarRouteClient.getPriceHistory(base ?? '', quote ?? '', query, {
        signal,
      }),
    [base, quote, JSON.stringify(query ?? {})],
    {
      ...fetchOptions,
      skip,
    },
  );
}

// ---------------------------------------------------------------------------
// useRoutes — fetch ranked route candidates
// ---------------------------------------------------------------------------

export interface UseRoutesOptions extends UseFetchOptions {
  query?: Omit<RoutesQueryOptions, 'amount'>;
}

export function useRoutes(
  base: string,
  quote: string,
  amount: number | undefined,
  options: UseRoutesOptions = {},
): UseApiState<RoutesResponse> & { refresh: () => void } {
  const { query, ...fetchOptions } = options;
  const invalidAmount =
    amount === undefined || !Number.isFinite(amount) || amount <= 0;

  return useFetch(
    (signal) =>
      stellarRouteClient.getRoutes(
        base,
        quote,
        {
          ...query,
          amount,
        },
        { signal },
      ),
    [base, quote, amount, JSON.stringify(query ?? {})],
    {
      ...fetchOptions,
      skip: fetchOptions.skip || !base || !quote || invalidAmount,
    },
  );
}

// ---------------------------------------------------------------------------
// useBatchQuote — fetch multiple quotes at once
// ---------------------------------------------------------------------------

export function useBatchQuote(
  requests: QuoteRequestItem[],
  options: UseFetchOptions = {},
): UseApiState<BatchQuoteResponse> & { refresh: () => void } {
  return useFetch(
    (signal) => stellarRouteClient.getQuotesBatch(requests, { signal }),
    [JSON.stringify(requests)],
    {
      ...options,
      skip: options.skip || requests.length === 0,
    },
  );
}

// ---------------------------------------------------------------------------
// useHealth — API health status
// ---------------------------------------------------------------------------

export function useHealth(
  options: UseFetchOptions = { refreshIntervalMs: 60_000 },
): UseApiState<HealthStatus> & { refresh: () => void } {
  return useFetch(
    (signal) => stellarRouteClient.getHealth({ signal }),
    [],
    options,
  );
}
