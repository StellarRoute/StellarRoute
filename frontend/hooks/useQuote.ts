'use client';

import { useMemo } from 'react';
import { useQuoteRefresh } from './useQuoteRefresh';
import { useQuoteStream } from './useApi';
import type { QuoteType } from '@/types';

interface UseQuoteProps {
  fromToken: string; // asset identifier "native" or "CODE:ISSUER"
  toToken: string;
  amount: number | undefined;
  type?: QuoteType;
}

export interface QuoteResult {
  outputAmount: number;
  priceImpact: number;
  route: string[];
  fee: number;
  rate: number;
  loading: boolean;
  error: Error | null;
  isStale: boolean;
  isRecovering: boolean;
  retryAttempt: number;
  hasPendingRetry: boolean;
  pendingRetryRemainingMs: number;
  cancelRetry: () => void;
  refresh: (opts?: { force?: boolean }) => void;
  data: import('@/types').PriceQuote | undefined;
  lastQuotedAtMs: number | null;
  requestId: string | null;
  /** True when the quote data is being delivered via WebSocket (not HTTP polling). */
  wsConnected: boolean;
}

/**
 * Hook to fetch real-time swap quotes with debouncing and state management.
 *
 * When NEXT_PUBLIC_API_WS_URL (or NEXT_PUBLIC_API_URL) is configured the hook
 * subscribes to the WebSocket quote stream via useQuoteStream and uses those
 * pushed quotes as the primary data source. HTTP polling via useQuoteRefresh
 * remains active throughout and acts as the automatic fallback:
 *
 *   - WS connected & has data  →  stream data wins
 *   - WS disconnected / no env →  polling data is used transparently
 *   - isRecovering tracks HTTP retry only (WS outages do not sticky-reconnect)
 *
 * Callers can check `wsConnected` to know which path is active.
 */
export function useQuote({ fromToken, toToken, amount, type = 'sell' }: UseQuoteProps): QuoteResult {
  // ── HTTP polling (always active; fallback when WS is absent/down) ──────────
  const {
    data: pollingData,
    loading,
    error: pollingError,
    isStale,
    isRecovering: pollingIsRecovering,
    retryAttempt,
    hasPendingRetry,
    pendingRetryRemainingMs,
    cancelRetry,
    refresh,
    lastQuotedAtMs,
    requestId,
  } = useQuoteRefresh(
    fromToken,
    toToken,
    amount,
    type,
    {
      debounceMs: 300,
      autoRefreshIntervalMs: 15_000,
    },
  );

  // ── WebSocket stream (opt-in via NEXT_PUBLIC_QUOTE_WS=true) ────────────────
  const {
    data: wsData,
    isConnected: wsConnected,
    error: wsError,
  } = useQuoteStream(fromToken, toToken, amount);

  // Prefer WS data when the socket is healthy, otherwise fall back to polling.
  // Do NOT treat "WS configured but disconnected" as recovering — Cloudflare
  // quick tunnels / hosts without WS would otherwise stick on "Reconnecting".
  const isRecovering = pollingIsRecovering;

  // Prefer WS data when the socket is healthy, otherwise fall back to polling.
  const data = wsConnected && wsData ? wsData : pollingData;

  // Soft-fail background refresh: if we still have a displayable quote, do not
  // surface a hard error (that raced into "Error fetching quote" over good numbers).
  // First-load / no-data failures still propagate so the empty state can recover.
  const error = data
    ? null
    : pollingError ?? (wsConnected && wsError ? wsError : null);

  const result = useMemo(() => {
    if (!data) {
      return {
        outputAmount: 0,
        priceImpact: 0,
        route: [],
        fee: 0,
        rate: 0,
      };
    }

    // Parse the data from the PriceQuote response
    const outputAmount = parseFloat(data.total) || 0;
    const priceImpact = parseFloat(data.price_impact || '0') || 0;

    // Extract route symbols from path
    const route = data.path.reduce((acc: string[], step) => {
      const fromCode = step.from_asset.asset_code || 'XLM';
      const toCode = step.to_asset.asset_code || 'XLM';
      if (acc.length === 0) {
        acc.push(fromCode);
      }
      acc.push(toCode);
      return acc;
    }, []);

    // Rate: units of toToken per 1 unit of fromToken
    const rate = parseFloat(data.price) || 0;

    // Fees calculation — simplified; real fee breakdown comes from the API path steps
    const fee = 0.001 * (parseFloat(data.amount) || 0);

    return {
      outputAmount,
      priceImpact,
      route,
      fee,
      rate,
    };
  }, [data]);

  return {
    ...result,
    loading,
    error: error instanceof Error ? error : error ? new Error(String(error)) : null,
    isStale,
    isRecovering,
    retryAttempt,
    hasPendingRetry,
    pendingRetryRemainingMs,
    cancelRetry,
    refresh,
    data,
    lastQuotedAtMs,
    requestId,
    wsConnected,
  };
}
