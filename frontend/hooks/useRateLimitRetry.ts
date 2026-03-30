'use client';

/**
 * Hook that detects rate-limit (429) errors and manages an auto-retry countdown.
 *
 * Usage:
 *   const rl = useRateLimitRetry(error, refresh);
 *   if (rl.isRateLimited) return <RateLimitBanner {...rl} />;
 */

import { useCallback, useEffect, useRef, useState } from 'react';

import { StellarRouteApiError } from '@/lib/api/client';

export interface UseRateLimitRetryResult {
  /** True when the current error is a 429 rate-limit. */
  isRateLimited: boolean;
  /** Countdown seconds remaining before auto-retry. 0 when not counting. */
  secondsRemaining: number;
  /** Trigger an immediate retry (resets countdown). */
  retry: () => void;
}

const DEFAULT_RETRY_AFTER_SECONDS = 5;

export function useRateLimitRetry(
  error: Error | null | undefined,
  onRetry: () => void,
): UseRateLimitRetryResult {
  const [secondsRemaining, setSecondsRemaining] = useState(0);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const onRetryRef = useRef(onRetry);
  onRetryRef.current = onRetry;

  const isRateLimited =
    error instanceof StellarRouteApiError && error.isRateLimit;

  const retryAfterSeconds =
    error instanceof StellarRouteApiError
      ? (error.retryAfterSeconds ?? DEFAULT_RETRY_AFTER_SECONDS)
      : DEFAULT_RETRY_AFTER_SECONDS;

  // Start countdown when a rate-limit error arrives
  useEffect(() => {
    if (!isRateLimited) {
      setSecondsRemaining(0);
      return;
    }

    setSecondsRemaining(retryAfterSeconds);

    timerRef.current = setInterval(() => {
      setSecondsRemaining((prev) => {
        if (prev <= 1) {
          // Countdown finished → auto-retry
          if (timerRef.current) clearInterval(timerRef.current);
          // Use setTimeout to avoid calling setState during render
          setTimeout(() => onRetryRef.current(), 0);
          return 0;
        }
        return prev - 1;
      });
    }, 1_000);

    return () => {
      if (timerRef.current) clearInterval(timerRef.current);
    };
  }, [isRateLimited, retryAfterSeconds]);

  const retry = useCallback(() => {
    if (timerRef.current) clearInterval(timerRef.current);
    setSecondsRemaining(0);
    onRetryRef.current();
  }, []);

  return { isRateLimited, secondsRemaining, retry };
}
