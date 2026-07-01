'use client';

/**
 * Custom React hooks for StellarRoute data fetching.
 *
 * Each hook returns { data, loading, error } and handles:
 *  - Request cancellation on unmount (AbortController)
 *  - Auto-refresh intervals where appropriate
 *  - Debounced parameters for useQuote
 */

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import { toast } from 'sonner';

import { useWallet } from '@/components/providers/wallet-provider';
import {
  StellarRouteApiError,
  STATUS_PAGE_REFRESH_MS,
} from '@/lib/api/client';
import type { DepsHealthStatus } from '@/lib/api/client';
import { useStellarRouteClient } from '@/hooks/useStellarRouteClient';
import { getApiBaseUrl } from '@/lib/network-endpoints';
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
  CacheMetricsResponse,
  PoolStatsResponse,
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

  useEffect(() => {
    if (!refreshIntervalMs || skip) return;
    const id = setInterval(() => setTick((n) => n + 1), refreshIntervalMs);
    return () => clearInterval(id);
  }, [refreshIntervalMs, skip]);

  return { ...state, refresh };
}

function useDebounced<T>(value: T, delayMs: number): T {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const id = setTimeout(() => setDebounced(value), delayMs);
    return () => clearTimeout(id);
  }, [value, delayMs]);
  return debounced;
}

export function usePairs(): UseApiState<TradingPair[]> & {
  refresh: () => void;
} {
  const client = useStellarRouteClient();
  return useFetch(
    (signal) =>
      client
        .getPairs({ signal })
        .then((res: PairsResponse) => res.pairs),
    [client],
    { showToastOnError: true }
  );
}

export function useOrderbook(
  base: string,
  quote: string,
  refreshIntervalMs = 10_000
): UseApiState<Orderbook> & {
  refresh: () => void;
  midpoint?: string;
  spreadBps?: number;
} {
  const client = useStellarRouteClient();
  const result = useFetch(
    (signal) => client.getOrderbook(base, quote, { signal }),
    [client, base, quote],
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
  const client = useStellarRouteClient();
  return useFetch(
    (signal) => client.getPriceHistory(base, quote, { signal }),
    [client, base, quote],
    { refreshIntervalMs, skip: skip || !base || !quote }
  );
}

export function useRoutes(
  base: string,
  quote: string,
  amount?: number,
  limit = 5,
  maxHops = 3
): UseApiState<RoutesResponse> & { refresh: () => void } {
  const client = useStellarRouteClient();
  const debouncedAmount = useDebounced(amount, QUOTE_AMOUNT_DEBOUNCE_MS);
  const skip =
    !base ||
    !quote ||
    debouncedAmount === undefined ||
    Number.isNaN(debouncedAmount) ||
    debouncedAmount <= 0;
  return useFetch(
    (signal) =>
      client.getRoutes(base, quote, debouncedAmount, limit, maxHops, {
        signal,
      }),
    [client, base, quote, debouncedAmount, limit, maxHops],
    { skip }
  );
}

export function useQuote(
  base: string,
  quote: string,
  amount: number | undefined,
  type: QuoteType = 'sell',
  refreshIntervalMs?: number
): UseApiState<PriceQuote> & { refresh: () => void } {
  const client = useStellarRouteClient();
  const debouncedAmount = useDebounced(amount, QUOTE_AMOUNT_DEBOUNCE_MS);

  const skip =
    !base ||
    !quote ||
    debouncedAmount === undefined ||
    !Number.isFinite(debouncedAmount) ||
    debouncedAmount <= 0;

  return useFetch(
    (signal) =>
      client
        .getQuote(base, quote, debouncedAmount, type, {
          signal,
        })
        .then((result) => result.quote),
    [client, base, quote, debouncedAmount, type],
    { refreshIntervalMs, skip }
  );
}

import type { QuoteRequestItem, BatchQuoteResponse } from '@/lib/api/client';

export function useBatchQuote(
  requests: QuoteRequestItem[],
  skip = false,
  refreshIntervalMs?: number
): UseApiState<BatchQuoteResponse> & { refresh: () => void } {
  const client = useStellarRouteClient();
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
    (signal) => client.getQuotesBatch(requests, { signal }),
    [client, JSON.stringify(requests)],
    { refreshIntervalMs, skip: skip || !hasValidRequests }
  );
}

export function useHealth(
  refreshIntervalMs = STATUS_PAGE_REFRESH_MS,
): UseApiState<HealthStatus> & { refresh: () => void } {
  const client = useStellarRouteClient();
  return useFetch((signal) => client.getHealth({ signal }), [client], {
    refreshIntervalMs,
  });
}

export function useHealthDeps(
  refreshIntervalMs = STATUS_PAGE_REFRESH_MS,
): UseApiState<DepsHealthStatus> & { refresh: () => void } {
  const client = useStellarRouteClient();
  return useFetch(
    (signal) => client.getDepsHealth({ signal }),
    [client],
    { refreshIntervalMs }
  );
}

export function useCacheMetrics(
  refreshIntervalMs = 30_000,
  skip = false,
): UseApiState<CacheMetricsResponse> & { refresh: () => void } {
  const client = useStellarRouteClient();
  return useFetch(
    (signal) => client.getCacheMetrics({ signal }),
    [client],
    { refreshIntervalMs, skip }
  );
}

export function usePoolStats(
  refreshIntervalMs = 30_000,
  skip = false,
): UseApiState<PoolStatsResponse> & { refresh: () => void } {
  const client = useStellarRouteClient();
  return useFetch(
    (signal) => client.getPoolStats({ signal }),
    [client],
    { refreshIntervalMs, skip }
  );
}

export interface UseQuoteStreamResult {
  data: PriceQuote | undefined;
  isConnected: boolean;
  error: Error | null;
  wsAvailable: boolean;
}

export function useQuoteStream(
  base: string,
  quote: string,
  amount: number | undefined,
): UseQuoteStreamResult {
  const { network } = useWallet();
  const [data, setData] = useState<PriceQuote | undefined>(undefined);
  const [isConnected, setIsConnected] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const debouncedAmount = useDebounced(amount, QUOTE_AMOUNT_DEBOUNCE_MS);
  const apiBaseUrl = useMemo(() => getApiBaseUrl(network), [network]);
  const wsAvailable = Boolean(apiBaseUrl);

  useEffect(() => {
    const skip = !wsAvailable || !base || !quote;
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

      const wsProtocol = apiBaseUrl.startsWith('https') ? 'wss' : 'ws';
      const host = apiBaseUrl.replace(/^https?:\/\//, '').replace(/\/$/, '');
      const wsUrl = `${wsProtocol}://${host}/api/v1/ws`;

      try {
        ws = new WebSocket(wsUrl);

        ws.onopen = () => {
          if (!isMounted) return;
          setIsConnected(true);
          setError(null);
          retryCount = 0;

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
            const msg = JSON.parse(event.data as string);
            if (msg.type === 'subscription_confirmed') {
              subscriptionId = msg.subscription_id as string;
            } else if (msg.type === 'quote_update') {
              setData(msg.quote as PriceQuote);
            } else if (msg.type === 'error') {
              setError(new Error((msg.message as string) || 'WebSocket Error'));
            }
          } catch (err) {
            setError(err instanceof Error ? err : new Error('Parse error'));
          }
        };

        ws.onclose = () => {
          if (!isMounted) return;
          setIsConnected(false);
          subscriptionId = null;

          const delay = Math.min(1000 * Math.pow(2, retryCount), 30_000);
          retryCount++;
          reconnectTimer = setTimeout(connect, delay);
        };

        ws.onerror = () => {
          if (isMounted) {
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
  }, [base, quote, debouncedAmount, apiBaseUrl, wsAvailable]);

  return { data, isConnected, error, wsAvailable };
}
