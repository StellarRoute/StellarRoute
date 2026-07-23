'use client';

import { useEffect, useRef, useState } from 'react';

export const STALE_QUOTE_PRICE_IMPACT_THRESHOLD_PCT = 0.5;

export interface UseStaleQuoteOptions {
  /**
   * Unix timestamp (ms) when the current quote expires, as provided by the
   * server `expires_at` field.  When present this is the authoritative source
   * of expiry; when absent the caller's `isStale` flag is used instead.
   */
  expiresAtMs?: number;
  /**
   * Most-recent price-impact percentage (e.g. 1.23 for 1.23 %).
   * Tracked against the value that was recorded at the last fetch; when the
   * absolute difference exceeds {@link STALE_QUOTE_PRICE_IMPACT_THRESHOLD_PCT}
   * the quote is considered stale regardless of its expiry timestamp.
   */
  currentPriceImpact: number;
  /**
   * Falls back to this when `expiresAtMs` is not provided.
   * This is the value emitted by `useQuoteRefresh` / `useQuote`.
   */
  isStale: boolean;
  /** Signals that a successful fresh quote has just been received. */
  lastQuotedAtMs: number | null;
}

export interface UseStaleQuoteResult {
  /** True when the quote is expired OR price impact moved > threshold. */
  isQuoteStale: boolean;
  /** True specifically because `now >= expires_at`. */
  isExpired: boolean;
  /** True specifically because price impact changed > 0.5 % since last fetch. */
  isPriceImpactChanged: boolean;
  /** The baseline price-impact recorded at the last successful fetch. */
  baselinePriceImpact: number | null;
}

/**
 * Determines whether the swap Confirm button should be disabled due to a
 * stale quote condition:
 *
 *  - The server-provided `expires_at` has been exceeded (authoritative), OR
 *  - The polling-derived `isStale` flag is true (fallback), OR
 *  - The price impact has shifted by more than 0.5 % since the last fetch.
 *
 * Resets the baseline whenever `lastQuotedAtMs` advances (i.e. a fresh quote
 * was received).
 */
export function useStaleQuote({
  expiresAtMs,
  currentPriceImpact,
  isStale,
  lastQuotedAtMs,
}: UseStaleQuoteOptions): UseStaleQuoteResult {
  const [nowMs, setNowMs] = useState(() => Date.now());
  const [baselinePriceImpact, setBaselinePriceImpact] = useState<number | null>(null);

  // Tick every second to re-evaluate expiry.
  useEffect(() => {
    const id = setInterval(() => setNowMs(Date.now()), 1000);
    return () => clearInterval(id);
  }, []);

  // Track the previous lastQuotedAtMs so we can detect when it advances.
  const prevLastQuotedAtMsRef = useRef<number | null>(null);

  useEffect(() => {
    if (
      lastQuotedAtMs !== null &&
      lastQuotedAtMs !== prevLastQuotedAtMsRef.current
    ) {
      // Fresh quote arrived — record the new baseline.
      setBaselinePriceImpact(currentPriceImpact);
      prevLastQuotedAtMsRef.current = lastQuotedAtMs;
    }
  }, [lastQuotedAtMs, currentPriceImpact]);

  const isExpired =
    expiresAtMs != null ? nowMs >= expiresAtMs : isStale;

  const isPriceImpactChanged =
    baselinePriceImpact !== null &&
    Math.abs(currentPriceImpact - baselinePriceImpact) >
      STALE_QUOTE_PRICE_IMPACT_THRESHOLD_PCT;

  const isQuoteStale = isExpired || isPriceImpactChanged;

  return {
    isQuoteStale,
    isExpired,
    isPriceImpactChanged,
    baselinePriceImpact,
  };
}
