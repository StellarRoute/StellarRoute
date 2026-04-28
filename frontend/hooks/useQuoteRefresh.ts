'use client';

/**
 * Quote fetching with manual refresh (cooldown), optional auto-refresh, and stale detection.
 *
 * Uses `stellarRouteClient.getQuote` as the only HTTP path for quotes (same as `useQuote`).
 *
 * Extension point — real-time updates: when the API exposes WebSocket (or SSE) quote streams,
 * subscribe here alongside or instead of the auto-refresh interval; update `data` and reset
 * `lastQuotedAtMs` from pushed payloads while keeping manual refresh as a fallback.
 */

import { useCallback, useEffect, useMemo, useState } from 'react';

import {
  StellarRouteApiError,
  stellarRouteClient,
} from '@/lib/api/client';
import {
  calculateQuoteRetryDelayMs,
  emitQuoteRetryTelemetry,
  getQuoteRetryRequestKey,
  type QuoteRetryRequestContext,
  type QuoteRetryTelemetryEvent,
} from '@/lib/quote-retry';
import {
  isQuoteStale,
  QUOTE_AMOUNT_DEBOUNCE_MS,
  QUOTE_AUTO_REFRESH_INTERVAL_MS,
  QUOTE_MANUAL_REFRESH_COOLDOWN_MS,
  QUOTE_STALE_AFTER_MS,
} from '@/lib/quote-stale';
import type { PriceQuote, QuoteType } from '@/types';
import type { UseApiState } from '@/hooks/useApi';

function useDebounced<T>(value: T, delayMs: number): T {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const id = setTimeout(() => setDebounced(value), delayMs);
    return () => clearTimeout(id);
  }, [value, delayMs]);
  return debounced;
}

export interface UseQuoteRefreshOptions {
  staleAfterMs?: number;
  autoRefreshIntervalMs?: number;
  manualRefreshCooldownMs?: number;
  debounceMs?: number;
  /** Optional connectivity override. When false, network quote requests are paused. */
  isOnline?: boolean;
  /** Auto-retry attempts for transient online quote failures. */
  maxAutoRetries?: number;
  /** Base delay in ms for exponential retry backoff. */
  retryBackoffMs?: number;
  /** Upper bound for exponential retry backoff. */
  maxRetryBackoffMs?: number;
  /** Random jitter ratio applied symmetrically to the computed retry delay. */
  retryJitterRatio?: number;
  /** Optional deterministic random source for tests. */
  retryRandom?: () => number;
  /** Optional telemetry sink for retry lifecycle events. */
  onRetryEvent?: (event: QuoteRetryTelemetryEvent) => void;
}

export type UseQuoteRefreshState = UseApiState<PriceQuote> & {
  /** Manual refresh; blocked during cooldown or while inputs are invalid. */
  refresh: (options?: { force?: boolean }) => void;
  /** True after a manual refresh until the cooldown elapses. */
  manualRefreshCoolingDown: boolean;
  autoRefreshEnabled: boolean;
  setAutoRefreshEnabled: (enabled: boolean) => void;
  /** True when we have a quote and it is older than the stale TTL. */
  isStale: boolean;
  /** Wall-clock time of the last successful quote fetch, or null. */
  lastQuotedAtMs: number | null;
  /** True while transient online quote failures are being retried. */
  isRecovering: boolean;
  /** Current transient retry attempt count for the active request context. */
  retryAttempt: number;
  /** True when a retry is queued and waiting for its backoff window. */
  hasPendingRetry: boolean;
  /** Remaining wait time for the queued retry. */
  pendingRetryRemainingMs: number;
  /** Cancel the currently queued retry, if any. */
  cancelRetry: () => void;
  /** Remaining wait time from Retry-After, if the API is currently rate-limiting requests. */
  rateLimitRemainingMs: number;
};

interface PendingQuoteRetry {
  request: QuoteRetryRequestContext;
  key: string;
  attempt: number;
  dueAtMs: number;
  delayMs: number;
}

function isTransientQuoteError(err: Error): boolean {
  if (err instanceof StellarRouteApiError) {
    return err.status === 0 || err.status === 429 || err.status >= 500;
  }

  const message = err.message.toLowerCase();
  return (
    message.includes('network') ||
    message.includes('timeout') ||
    message.includes('failed to fetch')
  );
}

export function useQuoteRefresh(
  base: string,
  quoteAsset: string,
  amount: number | undefined,
  type: QuoteType = 'sell',
  options?: UseQuoteRefreshOptions,
): UseQuoteRefreshState {
  const staleAfterMs = options?.staleAfterMs ?? QUOTE_STALE_AFTER_MS;
  const autoRefreshIntervalMs =
    options?.autoRefreshIntervalMs ?? QUOTE_AUTO_REFRESH_INTERVAL_MS;
  const manualRefreshCooldownMs =
    options?.manualRefreshCooldownMs ?? QUOTE_MANUAL_REFRESH_COOLDOWN_MS;
  const debounceMs = options?.debounceMs ?? QUOTE_AMOUNT_DEBOUNCE_MS;
  const isOnline = options?.isOnline ?? true;
  const maxAutoRetries = options?.maxAutoRetries ?? 2;
  const retryBackoffMs = options?.retryBackoffMs ?? 1_000;
  const maxRetryBackoffMs = options?.maxRetryBackoffMs ?? 30_000;
  const retryJitterRatio = options?.retryJitterRatio ?? 0.2;
  const retryRandom = options?.retryRandom;
  const onRetryEvent = options?.onRetryEvent;

  const debouncedAmount = useDebounced(amount, debounceMs);
  const [tick, setTick] = useState(0);
  const [autoRefreshEnabled, setAutoRefreshEnabled] = useState(false);
  const [state, setState] = useState<UseApiState<PriceQuote>>({
    data: undefined,
    loading: false,
    error: null,
  });
  const [manualCooldownUntil, setManualCooldownUntil] = useState(0);
  const [lastQuotedAtMs, setLastQuotedAtMs] = useState<number | null>(null);
  const [isRecovering, setIsRecovering] = useState(false);
  const [retryAttempt, setRetryAttempt] = useState(0);
  const [nowMs, setNowMs] = useState(() => Date.now());
  const [rateLimitUntilMs, setRateLimitUntilMs] = useState(0);
  const [pendingRetry, setPendingRetry] = useState<PendingQuoteRetry | null>(
    null,
  );

  const hasValidInputs =
    Boolean(base) &&
    Boolean(quoteAsset) &&
    debouncedAmount !== undefined &&
    Number.isFinite(debouncedAmount) &&
    debouncedAmount > 0;
  const canRequest = hasValidInputs && isOnline;
  const requestContext = useMemo(
    () =>
      hasValidInputs && debouncedAmount !== undefined
        ? {
            base,
            quoteAsset,
            amount: debouncedAmount,
            type,
          }
        : null,
    [base, quoteAsset, debouncedAmount, hasValidInputs, type],
  );
  const requestKey = requestContext
    ? getQuoteRetryRequestKey(requestContext)
    : null;

  const emitRetryEvent = useCallback(
    (event: QuoteRetryTelemetryEvent) => {
      emitQuoteRetryTelemetry(event, onRetryEvent);
    },
    [onRetryEvent],
  );

  useEffect(() => {
    const id = setInterval(() => setNowMs(Date.now()), 1000);
    return () => clearInterval(id);
  }, []);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect -- intentional retry-state reset when request inputs change
    setRetryAttempt(0);
    setIsRecovering(false);
    setRateLimitUntilMs(0);
    setPendingRetry(null);
  }, [base, quoteAsset, debouncedAmount, type]);

  const cancelRetry = useCallback(() => {
    setPendingRetry((current) => {
      if (current) {
        emitRetryEvent({
          stage: 'cancelled',
          request: current.request,
          attempt: current.attempt,
          delayMs: current.delayMs,
        });
      }
      return null;
    });
    setIsRecovering(false);
    setRetryAttempt(0);
  }, [emitRetryEvent]);

  useEffect(() => {
    if (!pendingRetry || !requestKey || pendingRetry.key !== requestKey || !canRequest) {
      return;
    }

    const delayMs = Math.max(0, pendingRetry.dueAtMs - Date.now());
    const id = setTimeout(() => {
      setPendingRetry((current) =>
        current && current.key === pendingRetry.key ? null : current,
      );
      setTick((n) => n + 1);
    }, delayMs);

    return () => clearTimeout(id);
  }, [canRequest, pendingRetry, requestKey]);

  useEffect(() => {
    if (!canRequest) return;

    const controller = new AbortController();
    // Same pattern as `useFetch` in useApi.ts: set loading before starting the request.
    // eslint-disable-next-line react-hooks/set-state-in-effect -- intentional loading transition before async getQuote
    setState((prev) => ({ ...prev, loading: true, error: null }));

    stellarRouteClient
      .getQuote(base, quoteAsset, debouncedAmount, type, {
        signal: controller.signal,
      })
      .then((data) => {
        if (!controller.signal.aborted) {
          const t = Date.now();
          setLastQuotedAtMs(t);
          if (retryAttempt > 0 && requestContext) {
            emitRetryEvent({
              stage: 'succeeded',
              request: requestContext,
              attempt: retryAttempt,
              delayMs: 0,
            });
          }
          setRetryAttempt(0);
          setIsRecovering(false);
          setRateLimitUntilMs(0);
          setPendingRetry(null);
          setState({ data, loading: false, error: null });
        }
      })
      .catch((err: unknown) => {
        if (!controller.signal.aborted) {
          const normalizedError =
            err instanceof StellarRouteApiError || err instanceof Error
              ? err
              : new Error(String(err));
          const shouldRetry =
            isOnline &&
            isTransientQuoteError(normalizedError) &&
            retryAttempt < maxAutoRetries &&
            requestContext !== null &&
            requestKey !== null;
          const rateLimitDelayMs =
            normalizedError instanceof StellarRouteApiError &&
            normalizedError.isRateLimit
              ? normalizedError.retryAfterMs
              : null;

          setRateLimitUntilMs(
            rateLimitDelayMs ? Date.now() + rateLimitDelayMs : 0,
          );

          setState((prev) => ({
            // Preserve last successful quote so users can still act on stale-but-visible data.
            data: prev.data,
            loading: false,
            error: normalizedError,
          }));

          if (shouldRetry) {
            const nextAttempt = retryAttempt + 1;
            const delayMs = rateLimitDelayMs ?? calculateQuoteRetryDelayMs(
              nextAttempt,
              {
                baseDelayMs: retryBackoffMs,
                maxDelayMs: maxRetryBackoffMs,
                jitterRatio: retryJitterRatio,
              },
              retryRandom,
            );
            const scheduledAtMs = Date.now();
            setRetryAttempt(nextAttempt);
            setIsRecovering(true);
            setPendingRetry({
              request: requestContext,
              key: requestKey,
              attempt: nextAttempt,
              dueAtMs: scheduledAtMs + delayMs,
              delayMs,
            });
            emitRetryEvent({
              stage: 'scheduled',
              request: requestContext,
              attempt: nextAttempt,
              delayMs,
              errorMessage: normalizedError.message,
            });
            return;
          }

          if (retryAttempt > 0 && requestContext) {
            emitRetryEvent({
              stage: 'failed',
              request: requestContext,
              attempt: retryAttempt,
              delayMs: 0,
              errorMessage: normalizedError.message,
            });
          }

          setPendingRetry(null);
          setIsRecovering(false);
        }
      });

    return () => {
      controller.abort();
    };
  }, [
    base,
    quoteAsset,
    debouncedAmount,
    type,
    tick,
    canRequest,
    isOnline,
    maxAutoRetries,
    maxRetryBackoffMs,
    retryAttempt,
    retryBackoffMs,
    retryJitterRatio,
    retryRandom,
    emitRetryEvent,
    requestContext,
    requestKey,
  ]);

  useEffect(() => {
    if (manualCooldownUntil === 0) return;
    const remaining = manualCooldownUntil - Date.now();
    const id = setTimeout(
      () => setManualCooldownUntil(0),
      Math.max(0, remaining),
    );
    return () => clearTimeout(id);
  }, [manualCooldownUntil]);

  const refresh = useCallback((options?: { force?: boolean }) => {
    if (!canRequest) return;
    const t = Date.now();
    if (t < rateLimitUntilMs) return;
    if (!options?.force && t < manualCooldownUntil) return;
    setManualCooldownUntil(
      options?.force ? 0 : t + manualRefreshCooldownMs,
    );
    setRateLimitUntilMs(0);
    setTick((n) => n + 1);
  }, [
    canRequest,
    manualCooldownUntil,
    manualRefreshCooldownMs,
    rateLimitUntilMs,
  ]);

  useEffect(() => {
    if (!autoRefreshEnabled || !canRequest) return;

    const id = setInterval(() => {
      if (typeof document !== 'undefined' && document.visibilityState !== 'visible') {
        return;
      }
      setTick((n) => n + 1);
    }, autoRefreshIntervalMs);

    return () => clearInterval(id);
  }, [autoRefreshEnabled, autoRefreshIntervalMs, canRequest]);

  const manualRefreshCoolingDown =
    manualCooldownUntil > 0 && nowMs < manualCooldownUntil;

  const data = hasValidInputs ? state.data : undefined;
  const loading = canRequest && state.loading;
  const error = !hasValidInputs
    ? null
    : !isOnline
      ? new Error('You are offline. Reconnect to refresh quotes.')
      : state.error;

  const isStale =
    data !== undefined && isQuoteStale(lastQuotedAtMs, nowMs, staleAfterMs);
  const rateLimitRemainingMs =
    rateLimitUntilMs > nowMs ? rateLimitUntilMs - nowMs : 0;
  const pendingRetryRemainingMs = pendingRetry
    ? Math.max(0, pendingRetry.dueAtMs - nowMs)
    : 0;

  return {
    data,
    loading,
    error,
    refresh,
    manualRefreshCoolingDown,
    autoRefreshEnabled,
    setAutoRefreshEnabled,
    isStale,
    lastQuotedAtMs,
    isRecovering,
    retryAttempt,
    hasPendingRetry: pendingRetry !== null,
    pendingRetryRemainingMs,
    cancelRetry,
    rateLimitRemainingMs,
  };
}
