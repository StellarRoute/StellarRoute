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

import { toast } from 'sonner';

import {
  StellarRouteApiError,
  stellarRouteClient,
  STATUS_PAGE_REFRESH_MS,
} from '@/lib/api/client';
import type { DepsHealthStatus } from '@/lib/api/client';
import { QUOTE_AMOUNT_DEBOUNCE_MS } from '@/lib/quote-stale';
import type {
  HealthStatus,
  Orderbook,
  PriceHistoryResponse,
  PairsResponse,
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
  error: StellarRouteApiError | Error | null;
}

// ---------------------------------------------------------------------------
// Internal: generic fetch hook
// ---------------------------------------------------------------------------

interface UseFetchOptions {
  refreshIntervalMs?: number;
  skip?: boolean;
  showToastOnError?: boolean;
}

function useFetch<T>(
  fetcher: (signal: AbortSignal) => Promise<T>,
  deps: unknown[],
  {
    refreshIntervalMs,
    skip = false,
    showToastOnError = false,
  }: UseFetchOptions = {}
): UseApiState<T> & { refresh: () => void } {
  const [state, setState] = useState<UseApiState<T>>({
    data: undefined,
    loading: true,
    error: null,
  });

  // Stable ref so the interval callback always sees the latest fetcher
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
          const finalError =
            err instanceof Error ? err : new Error(String(err));
          setState({
            data: undefined,
            loading: false,
            error: finalError,
          });

          if (showToastOnError) {
            toast.error(
              finalError instanceof StellarRouteApiError
                ? 'API Error'
                : 'Fetch Error',
              {
                description: finalError.message,
              }
            );
          }
        }
      });

    return () => controller.abort();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tick, skip, showToastOnError, ...deps]);

  // Auto-refresh
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
  const result = useFetch(
    (signal) =>
      stellarRouteClient
        .getPairs({ signal })
        .then((res: PairsResponse) => res.pairs),
    [],
    { showToastOnError: true }
  );
  return result;
}

// ---------------------------------------------------------------------------
// useOrderbook — fetch orderbook with auto-refresh every 10 s
// ---------------------------------------------------------------------------

export function useOrderbook(
  base: string,
  quote: string,
  refreshIntervalMs = 10_000
): UseApiState<Orderbook> & {
  refresh: () => void;
  midpoint?: string;
  spreadBps?: number;
} {
  const result = useFetch(
    (signal) => stellarRouteClient.getOrderbook(base, quote, { signal }),
    [base, quote],
    { refreshIntervalMs }
  );

  let midpoint: string | undefined = undefined;
  let spreadBps: number | undefined = undefined;

  if (result.data) {
    const bids = [...result.data.bids].sort(
      (a, b) => Number(b.price) - Number(a.price)
    );
    const asks = [...result.data.asks].sort(
      (a, b) => Number(a.price) - Number(b.price)
    );
    const bestBid = bids[0] ? Number(bids[0].price) : null;
    const bestAsk = asks[0] ? Number(asks[0].price) : null;

    if (
      bestBid !== null &&
      bestAsk !== null &&
      bestBid > 0 &&
      bestAsk > 0 &&
      bestAsk >= bestBid
    ) {
      const mid = (bestBid + bestAsk) / 2;
      midpoint = mid.toString();
      spreadBps = Math.round(((bestAsk - bestBid) / mid) * 10000);
    }
  }

  return {
    ...result,
    midpoint,
    spreadBps,
  };
}

export function usePriceHistory(
  base: string,
  quote: string,
  refreshIntervalMs = 60_000,
  skip = false
): UseApiState<PriceHistoryResponse> & { refresh: () => void } {
  return useFetch(
    (signal) => stellarRouteClient.getPriceHistory(base, quote, { signal }),
    [base, quote],
    { refreshIntervalMs, skip: skip || !base || !quote }
  );
}

// ---------------------------------------------------------------------------
// useRoutes — fetch ranked route candidates
// ---------------------------------------------------------------------------

export function useRoutes(
  base: string,
  quote: string,
  amount?: number,
  limit = 5,
  maxHops = 3
): UseApiState<RoutesResponse> & { refresh: () => void } {
  const debouncedAmount = useDebounced(amount, QUOTE_AMOUNT_DEBOUNCE_MS);
  const skip = !base || !quote;
  return useFetch(
    (signal) =>
      stellarRouteClient.getRoutes(base, quote, debouncedAmount, limit, maxHops, {
        signal,
      }),
    [base, quote, debouncedAmount, limit, maxHops],
    { skip },
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
  /** Optional polling interval. Prefer `useQuoteRefresh` for manual/auto refresh UX. */
  refreshIntervalMs?: number
): UseApiState<PriceQuote> & { refresh: () => void } {
  const debouncedAmount = useDebounced(amount, QUOTE_AMOUNT_DEBOUNCE_MS);

  const skip =
    !base ||
    !quote ||
    debouncedAmount === undefined ||
    !Number.isFinite(debouncedAmount) ||
    debouncedAmount <= 0;

  return useFetch(
    (signal) =>
      stellarRouteClient
        .getQuote(base, quote, debouncedAmount, type, {
          signal,
        })
        .then((result) => result.quote),
    [base, quote, debouncedAmount, type],
    { refreshIntervalMs, skip }
  );
}

// ---------------------------------------------------------------------------
// useBatchQuote — fetch multiple quotes at once
// ---------------------------------------------------------------------------

import type { QuoteRequestItem, BatchQuoteResponse } from '@/lib/api/client';

export function useBatchQuote(
  requests: QuoteRequestItem[],
  skip = false,
  refreshIntervalMs?: number
): UseApiState<BatchQuoteResponse> & { refresh: () => void } {
  const hasValidRequests =
    requests.length > 0 &&
    requests.every(
      ({ base, quote, amount, quote_type }) =>
        base.trim().length > 0 &&
        quote.trim().length > 0 &&
        base !== quote &&
        amount !== undefined &&
        Number.isFinite(amount) &&
        amount > 0 &&
        (quote_type === undefined ||
          quote_type === 'sell' ||
          quote_type === 'buy')
    );

  return useFetch(
    (signal) => stellarRouteClient.getQuotesBatch(requests, { signal }),
    [JSON.stringify(requests)],
    { refreshIntervalMs, skip: skip || !hasValidRequests }
  );
}

// ---------------------------------------------------------------------------
// useHealth — API health status
// ---------------------------------------------------------------------------

export function useHealth(
  refreshIntervalMs = STATUS_PAGE_REFRESH_MS,
): UseApiState<HealthStatus> & { refresh: () => void } {
  return useFetch((signal) => stellarRouteClient.getHealth({ signal }), [], {
    refreshIntervalMs,
  });
}

// ---------------------------------------------------------------------------
// useHealthDeps — external dependency health status
// ---------------------------------------------------------------------------

export function useHealthDeps(
  refreshIntervalMs = STATUS_PAGE_REFRESH_MS,
): UseApiState<DepsHealthStatus> & { refresh: () => void } {
  return useFetch(
    (signal) => stellarRouteClient.getDepsHealth({ signal }),
    [],
    { refreshIntervalMs },
  );
}

// ---------------------------------------------------------------------------
// useQuoteStream — WebSocket subscription for quotes
// ---------------------------------------------------------------------------

export function useQuoteStream(
  base: string,
  quote: string,
  amount: number | undefined
) {
  const [data, setData] = useState<PriceQuote | undefined>(undefined);
  const [isConnected, setIsConnected] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const debouncedAmount = useDebounced(amount, QUOTE_AMOUNT_DEBOUNCE_MS);

  useEffect(() => {
    const skip = !base || !quote;
    if (skip) {
      setData(undefined);
      setIsConnected(false);
      setError(null);
      return;
    }

    let ws: WebSocket | null = null;
    let reconnectTimer: ReturnType<typeof setTimeout>;
    let isMounted = true;
    let retryCount = 0;
    let subscriptionId: string | null = null;

    const connect = () => {
      if (!isMounted) return;

      const baseUrl =
        process.env.NEXT_PUBLIC_API_URL ?? 'http://localhost:8080';
      const wsProtocol = baseUrl.startsWith('https') ? 'wss' : 'ws';
      const host = baseUrl.replace(/^https?:\/\//, '');
      const wsUrl = `${wsProtocol}://${host}/api/v1/ws`;

      try {
        ws = new WebSocket(wsUrl);

        ws.onopen = () => {
          if (!isMounted) return;
          setIsConnected(true);
          setError(null);
          retryCount = 0;

          // Send subscribe message
          const subscribeMsg = {
            action: 'subscribe',
            subscription: {
              base,
              quote,
              amount:
                debouncedAmount !== undefined
                  ? String(debouncedAmount)
                  : undefined,
            },
          };
          ws?.send(JSON.stringify(subscribeMsg));
        };

        ws.onmessage = (event) => {
          if (!isMounted) return;
          try {
            const msg = JSON.parse(event.data);
            if (msg.type === 'subscription_confirmed') {
              subscriptionId = msg.subscription_id;
            } else if (msg.type === 'quote_update') {
              setData(msg.quote);
            } else if (msg.type === 'error') {
              setError(new Error(msg.message || 'WebSocket Error'));
            }
          } catch (err) {
            setError(err instanceof Error ? err : new Error('Parse error'));
          }
        };

        ws.onclose = () => {
          if (!isMounted) return;
          setIsConnected(false);
          subscriptionId = null;

          // Exponential backoff reconnect
          const delay = Math.min(1000 * Math.pow(2, retryCount), 30000);
          retryCount++;
          reconnectTimer = setTimeout(connect, delay);
        };

        ws.onerror = () => {
          if (isMounted && !error) {
            setError(new Error('WebSocket connection error'));
          }
        };
      } catch (err) {
        if (isMounted) {
          setError(
            err instanceof Error ? err : new Error('Failed to create WebSocket')
          );
        }
      }
    };

    connect();

    return () => {
      isMounted = false;
      clearTimeout(reconnectTimer);
      if (ws) {
        if (subscriptionId && ws.readyState === WebSocket.OPEN) {
          ws.send(
            JSON.stringify({
              action: 'unsubscribe',
              subscription_id: subscriptionId,
            })
          );
        }
        ws.close();
      }
    };
  }, [base, quote, debouncedAmount]);

  return { data, isConnected, error };
}
